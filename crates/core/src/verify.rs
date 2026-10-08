use crate::crypto::decrypt_archive;
use crate::models::StorageDestination;
use crate::storage::read_stored_archive;
use crate::{AppDatabase, BackupManifest};
use crate::{Result, ResultContext, error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    /// True when the stored copy was encrypted and decrypted successfully for checking.
    #[serde(default)]
    pub encrypted: bool,
    /// Destination the verified copy was read from, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_name: Option<String>,
}

/// A run archive fetched from one stored copy and decrypted to the plaintext zip.
pub struct ResolvedArchive {
    pub manifest: BackupManifest,
    pub plaintext: Vec<u8>,
    pub stored_sha256_ok: bool,
    pub destination_name: Option<String>,
}

/// Verifies a run's archive against its manifest, trying each stored copy in order.
pub async fn verify_run(database: &AppDatabase, run_id: Uuid) -> Result<VerificationResult> {
    let resolved = resolve_run_archive(database, run_id).await?;
    let actual_sha256 = format!("{:x}", Sha256::digest(&resolved.plaintext));
    let actual_size_bytes = resolved.plaintext.len() as u64;
    let manifest = resolved.manifest;
    Ok(VerificationResult {
        run_id,
        ok: resolved.stored_sha256_ok
            && actual_sha256 == manifest.sha256
            && actual_size_bytes == manifest.archive_size_bytes,
        archive_uri: manifest.storage_uri,
        expected_sha256: manifest.sha256,
        actual_sha256,
        expected_size_bytes: manifest.archive_size_bytes,
        actual_size_bytes,
        encrypted: manifest.encryption.is_some(),
        destination_name: resolved.destination_name,
    })
}

/// Fetches and decrypts a run's archive from the first readable stored copy
/// (primary destination first). Falls back to the run's legacy single manifest.
pub async fn resolve_run_archive(database: &AppDatabase, run_id: Uuid) -> Result<ResolvedArchive> {
    let run = database.get_run_record(run_id)?;
    let mut candidates: Vec<(BackupManifest, Option<StorageDestination>)> = Vec::new();
    for copy in database.list_run_copies(run_id)? {
        let Some(json) = copy.manifest_json else {
            continue;
        };
        let manifest: BackupManifest =
            serde_json::from_str(&json).context("stored copy manifest JSON is invalid")?;
        candidates.push((manifest, database.get_destination(copy.destination_id).ok()));
    }
    if candidates.is_empty() {
        let manifest_json = run
            .manifest_json
            .ok_or_else(|| error!("run {run_id} does not have a manifest"))?;
        let manifest: BackupManifest =
            serde_json::from_str(&manifest_json).context("stored manifest JSON is invalid")?;
        let destination = match manifest.destination_id {
            Some(id) => database.get_destination(id).ok(),
            None => database
                .get_job_bundle(run.run.job_id)
                .ok()
                .map(|bundle| bundle.destination),
        };
        candidates.push((manifest, destination));
    }

    let mut failures = Vec::new();
    for (manifest, destination) in candidates {
        let label = destination
            .as_ref()
            .map(|destination| destination.name.clone())
            .unwrap_or_else(|| manifest.storage_uri.clone());
        let attempt = async {
            let stored =
                read_stored_archive(database, destination.as_ref(), &manifest.storage_uri).await?;
            let stored_sha256_ok = manifest
                .stored_sha256
                .as_ref()
                .is_none_or(|expected| *expected == format!("{:x}", Sha256::digest(&stored)));
            let plaintext = decrypt_archive(database, manifest.encryption.as_ref(), stored)?;
            Ok::<_, crate::Error>((stored_sha256_ok, plaintext))
        }
        .await;
        match attempt {
            Ok((stored_sha256_ok, plaintext)) => {
                return Ok(ResolvedArchive {
                    destination_name: destination.map(|destination| destination.name),
                    manifest,
                    plaintext,
                    stored_sha256_ok,
                });
            }
            Err(error) => failures.push(format!("{label}: {error:#}")),
        }
    }
    Err(error!(
        "no readable copy of run {run_id}: {}",
        failures.join("; ")
    ))
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
                url: None,
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
                additional_destination_ids: Vec::new(),
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
