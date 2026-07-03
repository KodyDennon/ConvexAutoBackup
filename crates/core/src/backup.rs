use crate::convex::{ConvexExporter, ExportRequest, resolve_deploy_key};
use crate::db::AppDatabase;
use crate::manifest::{BackupManifest, ManifestInput};
use crate::models::{ConvexTarget, JobStatus};
use crate::secrets::SecretVault;
use crate::storage::{prune_local_retention, store_backup};
use crate::{Error, Result, ResultContext};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BackupEngine {
    database: AppDatabase,
    staging_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupRunResult {
    pub run_id: Uuid,
    pub job_id: Uuid,
    pub status: JobStatus,
    pub archive_uri: Option<String>,
    pub manifest_path: Option<String>,
    pub error: Option<String>,
}

impl BackupEngine {
    pub fn new(database: AppDatabase, staging_dir: impl Into<PathBuf>) -> Self {
        Self {
            database,
            staging_dir: staging_dir.into(),
        }
    }

    pub async fn run_job(
        &self,
        job_id: Uuid,
        exporter: &dyn ConvexExporter,
    ) -> Result<BackupRunResult> {
        std::fs::create_dir_all(&self.staging_dir).with_context(|| {
            format!(
                "failed to create staging dir {}",
                self.staging_dir.display()
            )
        })?;
        let bundle = self.database.get_job_bundle(job_id)?;
        let run = self.database.insert_run(job_id)?;
        let archive_path = self.staging_dir.join(format!("{}.zip", run.id));
        let started_at = Utc::now();

        let result = async {
            let deploy_key = resolve_deploy_key_from_store(&self.database, &bundle.target)
                .or_else(|_| resolve_deploy_key(&bundle.target))?;
            exporter
                .export_to_path(
                    ExportRequest {
                        target: bundle.target.clone(),
                        include_file_storage: bundle.job.include_file_storage,
                        deploy_key,
                    },
                    &archive_path,
                )
                .await?;
            let archive_bytes = tokio::fs::read(&archive_path)
                .await
                .with_context(|| format!("failed to read {}", archive_path.display()))?;
            let finished_at = Utc::now();
            let mut manifest = BackupManifest::from_input(ManifestInput {
                project_id: bundle.project.id,
                target_id: bundle.target.id,
                run_id: run.id,
                deployment: bundle.target.deployment.clone(),
                convex_cli_version: "managed-cli".to_string(),
                include_file_storage: bundle.job.include_file_storage,
                archive_bytes: archive_bytes.clone(),
                started_at,
                finished_at,
                storage_uri: format!("preupload://{}", run.id),
            });
            let stored = store_backup(
                &self.database,
                &bundle.destination,
                &bundle.project.name,
                &bundle.target.deployment,
                &archive_bytes,
                &manifest,
            )
            .await?;
            manifest.storage_uri = stored.storage_uri.clone();
            let manifest_json = serde_json::to_string_pretty(&manifest)?;
            self.database.finish_run(
                run.id,
                JobStatus::Succeeded,
                Some(stored.manifest_path.to_string_lossy().to_string()),
                Some(manifest_json),
                None,
            )?;
            let _ = prune_local_retention(
                &bundle.destination,
                &bundle.project.name,
                &bundle.target.deployment,
            )?;
            Ok::<_, Error>((stored.storage_uri, stored.manifest_path))
        }
        .await;

        let _ = std::fs::remove_file(&archive_path);

        match result {
            Ok((archive_uri, manifest_path)) => Ok(BackupRunResult {
                run_id: run.id,
                job_id,
                status: JobStatus::Succeeded,
                archive_uri: Some(archive_uri),
                manifest_path: Some(manifest_path.to_string_lossy().to_string()),
                error: None,
            }),
            Err(error) => {
                let error_message = error.to_string();
                self.database.finish_run(
                    run.id,
                    JobStatus::Failed,
                    None,
                    None,
                    Some(error_message.clone()),
                )?;
                Ok(BackupRunResult {
                    run_id: run.id,
                    job_id,
                    status: JobStatus::Failed,
                    archive_uri: None,
                    manifest_path: None,
                    error: Some(error_message),
                })
            }
        }
    }
}

fn resolve_deploy_key_from_store(database: &AppDatabase, target: &ConvexTarget) -> Result<String> {
    SecretVault::from_env(database.clone())?.get_secret(target.secret.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConvexIoFuture, CreateCloudTarget, CreateLocalDestination, CreateProject,
        CreateScheduledJob, RetentionPolicy,
    };
    use std::path::Path;

    struct FixtureExporter;

    impl ConvexExporter for FixtureExporter {
        fn export_to_path<'a>(
            &'a self,
            _request: ExportRequest,
            output_path: &'a Path,
        ) -> ConvexIoFuture<'a> {
            Box::pin(async move {
                tokio::fs::write(output_path, b"fixture convex export").await?;
                Ok("fixture export complete".to_string())
            })
        }
    }

    #[tokio::test]
    async fn backup_engine_runs_job_and_persists_successful_manifest() {
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
        let engine = BackupEngine::new(db.clone(), dir.path().join("staging"));

        let result = engine.run_job(job.id, &FixtureExporter).await.unwrap();

        assert_eq!(result.status, JobStatus::Succeeded);
        assert!(result.archive_uri.unwrap().starts_with("file://"));
        let runs = db.list_runs().unwrap();
        assert_eq!(runs.len(), 1);
        assert!(
            runs[0]
                .manifest_json
                .as_ref()
                .unwrap()
                .contains("careful-otter")
        );
    }
}
