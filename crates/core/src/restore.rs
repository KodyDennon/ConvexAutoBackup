use crate::convex::{ConvexImporter, ImportRequest, resolve_deploy_key};
use crate::secrets::SecretVault;
use crate::{AppDatabase, BackupManifest, ConvexTarget, VerificationResult, verify_run};
use crate::{Result, ResultContext, error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RestoreEngine {
    database: AppDatabase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreResult {
    pub run_id: Uuid,
    pub target_id: Uuid,
    pub target_deployment: String,
    pub archive_path: String,
    pub verification: VerificationResult,
    pub import_output: String,
}

impl RestoreEngine {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub async fn restore_run_to_target(
        &self,
        run_id: Uuid,
        target_id: Uuid,
        confirmed_deployment: &str,
        importer: &dyn ConvexImporter,
    ) -> Result<RestoreResult> {
        let target = self.database.get_target(target_id)?;
        if confirmed_deployment != target.deployment {
            return Err(error!(
                "deployment confirmation mismatch: expected {}",
                target.deployment
            ));
        }
        let verification = verify_run(&self.database, run_id).await?;
        if !verification.ok {
            return Err(error!("backup verification failed; restore blocked"));
        }
        let run = self.database.get_run_record(run_id)?;
        let manifest: BackupManifest = serde_json::from_str(
            run.manifest_json
                .as_deref()
                .ok_or_else(|| error!("run {run_id} does not have a manifest"))?,
        )
        .context("stored manifest JSON is invalid")?;
        let archive_path = file_uri_to_path(&manifest.storage_uri)?;
        let deploy_key = resolve_deploy_key_from_store(&self.database, &target)
            .or_else(|_| resolve_deploy_key(&target))?;
        let import_output = importer
            .import_from_path(
                ImportRequest {
                    target: target.clone(),
                    deploy_key,
                },
                &archive_path,
            )
            .await?;
        self.database.record_audit(
            "system",
            "restore.completed",
            "run",
            Some(run_id),
            &format!("restored backup to {}", target.deployment),
        )?;
        Ok(RestoreResult {
            run_id,
            target_id,
            target_deployment: target.deployment,
            archive_path: archive_path.display().to_string(),
            verification,
            import_output,
        })
    }
}

fn resolve_deploy_key_from_store(database: &AppDatabase, target: &ConvexTarget) -> Result<String> {
    SecretVault::from_env(database.clone())?.get_secret(target.secret.id)
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    uri.strip_prefix("file://")
        .map(PathBuf::from)
        .ok_or_else(|| error!("restore currently supports file:// archives"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BackupEngine, ConvexExporter, ConvexIoFuture, CreateCloudTarget, CreateLocalDestination,
        CreateProject, CreateScheduledJob, ExportRequest, ImportRequest, RetentionPolicy,
    };
    use std::path::Path;

    struct FixtureExporter;
    struct FixtureImporter;

    impl ConvexExporter for FixtureExporter {
        fn export_to_path<'a>(
            &'a self,
            _request: ExportRequest,
            output_path: &'a Path,
        ) -> ConvexIoFuture<'a> {
            Box::pin(async move {
                tokio::fs::write(output_path, b"restore export").await?;
                Ok("exported".to_string())
            })
        }
    }

    impl ConvexImporter for FixtureImporter {
        fn import_from_path<'a>(
            &'a self,
            _request: ImportRequest,
            archive_path: &'a Path,
        ) -> ConvexIoFuture<'a> {
            Box::pin(async move {
                assert!(archive_path.exists());
                Ok("imported".to_string())
            })
        }
    }

    #[tokio::test]
    async fn restore_requires_confirmation_and_verified_archive() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let project = db
            .create_project(CreateProject {
                name: "Client A".to_string(),
                description: None,
            })
            .unwrap();
        let destination = db
            .create_local_destination(CreateLocalDestination {
                name: "Local".to_string(),
                root: dir.path().join("backups").to_string_lossy().to_string(),
                retention: RetentionPolicy::default(),
            })
            .unwrap();
        let target = db
            .create_cloud_target(CreateCloudTarget {
                project_id: project.id,
                name: "Prod".to_string(),
                deployment: "prod:careful-otter-123".to_string(),
                deploy_key_env: Some("PATH".to_string()),
                deploy_key_secret_id: None,
            })
            .unwrap();
        let job = db
            .create_job(CreateScheduledJob {
                project_id: project.id,
                target_id: target.id,
                destination_id: destination.id,
                name: "Manual".to_string(),
                include_file_storage: true,
            })
            .unwrap();
        let backup = BackupEngine::new(db.clone(), dir.path().join("staging"))
            .run_job(job.id, &FixtureExporter)
            .await
            .unwrap();
        let restore = RestoreEngine::new(db.clone());

        assert!(
            restore
                .restore_run_to_target(backup.run_id, target.id, "wrong", &FixtureImporter)
                .await
                .is_err()
        );
        let result = restore
            .restore_run_to_target(
                backup.run_id,
                target.id,
                "prod:careful-otter-123",
                &FixtureImporter,
            )
            .await
            .unwrap();

        assert_eq!(result.import_output, "imported");
        assert!(result.verification.ok);
    }
}
