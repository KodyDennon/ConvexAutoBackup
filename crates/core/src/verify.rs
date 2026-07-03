use crate::models::StorageKind;
use crate::storage::{s3_client_from_destination, s3_object_key_from_uri};
use crate::{AppDatabase, BackupManifest};
use crate::{Result, ResultContext, error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationResult {
    pub run_id: Uuid,
    pub ok: bool,
    pub archive_uri: String,
    pub expected_sha256: String,
    pub actual_sha256: String,
    pub expected_size_bytes: u64,
    pub actual_size_bytes: u64,
}

pub async fn verify_run(database: &AppDatabase, run_id: Uuid) -> Result<VerificationResult> {
    let run = database.get_run_record(run_id)?;
    let manifest_json = run
        .manifest_json
        .ok_or_else(|| error!("run {run_id} does not have a manifest"))?;
    let manifest: BackupManifest =
        serde_json::from_str(&manifest_json).context("stored manifest JSON is invalid")?;
    let archive_bytes = read_archive_bytes(database, run.run.job_id, &manifest.storage_uri).await?;
    let actual_sha256 = format!("{:x}", Sha256::digest(&archive_bytes));
    let actual_size_bytes = archive_bytes.len() as u64;
    Ok(VerificationResult {
        run_id,
        ok: actual_sha256 == manifest.sha256 && actual_size_bytes == manifest.archive_size_bytes,
        archive_uri: manifest.storage_uri,
        expected_sha256: manifest.sha256,
        actual_sha256,
        expected_size_bytes: manifest.archive_size_bytes,
        actual_size_bytes,
    })
}

async fn read_archive_bytes(
    database: &AppDatabase,
    job_id: Uuid,
    storage_uri: &str,
) -> Result<Vec<u8>> {
    if storage_uri.starts_with("file://") {
        let archive_path = file_uri_to_path(storage_uri)?;
        return std::fs::read(&archive_path)
            .with_context(|| format!("failed to read archive {}", archive_path.display()));
    }
    if storage_uri.starts_with("s3://") {
        let bundle = database.get_job_bundle(job_id)?;
        match bundle.destination.kind {
            StorageKind::S3Compatible { .. } => {
                let client = s3_client_from_destination(database, &bundle.destination)?;
                let key = s3_object_key_from_uri(storage_uri)?;
                let bytes = client
                    .get_object(&key)
                    .await
                    .context("failed to read S3 archive")?;
                return Ok(bytes);
            }
            _ => return Err(error!("run is not associated with an S3 destination")),
        }
    }
    Err(error!("unsupported archive URI {storage_uri}"))
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    uri.strip_prefix("file://")
        .map(PathBuf::from)
        .ok_or_else(|| error!("verification currently supports file:// archives"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BackupEngine, ConvexExporter, ConvexIoFuture, CreateCloudTarget, CreateLocalDestination,
        CreateProject, CreateScheduledJob, ExportRequest, RetentionPolicy,
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
                tokio::fs::write(output_path, b"verified export").await?;
                Ok("verified".to_string())
            })
        }
    }

    #[tokio::test]
    async fn verifies_successful_local_backup() {
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
        let run = engine.run_job(job.id, &FixtureExporter).await.unwrap();

        let verification = verify_run(&db, run.run_id).await.unwrap();

        assert!(verification.ok);
        assert_eq!(
            verification.expected_size_bytes,
            verification.actual_size_bytes
        );
    }
}
