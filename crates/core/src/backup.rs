use crate::convex::{ConvexExporter, ExportRequest, resolve_deploy_key};
use crate::crypto::encrypt_for_destination;
use crate::db::{AppDatabase, JobBundle, RunCopy};
use crate::manifest::{BackupManifest, ManifestInput};
use crate::models::{ConvexTarget, JobStatus, StorageDestination};
use crate::secrets::SecretVault;
use crate::storage::{StoredBackup, prune_retention, store_backup};
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
            crate::convex::validate_deploy_key_matches_deployment(
                &deploy_key,
                &bundle.target.deployment,
            )?;
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
            let manifest = BackupManifest::from_input(ManifestInput {
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

            let mut stored_copies = Vec::new();
            let mut copy_errors = Vec::new();
            for (position, destination) in bundle.destinations.iter().enumerate() {
                let copy = self
                    .store_copy(&bundle, destination, &archive_bytes, &manifest)
                    .await;
                let (status, stored, error) = match copy {
                    Ok(stored) => (JobStatus::Succeeded, Some(stored), None),
                    Err(error) => {
                        let message = format!("{}: {error:#}", destination.name);
                        copy_errors.push(message.clone());
                        (JobStatus::Failed, None, Some(message))
                    }
                };
                self.database.record_run_copy(&RunCopy {
                    run_id: run.id,
                    destination_id: destination.id,
                    position: position as u32,
                    status,
                    storage_uri: stored
                        .as_ref()
                        .map(|(stored, _)| stored.storage_uri.clone()),
                    manifest_path: stored
                        .as_ref()
                        .map(|(stored, _)| stored.manifest_path.to_string_lossy().to_string()),
                    manifest_json: stored.as_ref().map(|(_, json)| json.clone()),
                    error,
                })?;
                if let Some(stored) = stored {
                    stored_copies.push(stored);
                }
            }

            let Some((primary, primary_manifest_json)) = stored_copies.first().cloned() else {
                return Err(Error::message(format!(
                    "backup could not be stored in any destination: {}",
                    copy_errors.join("; ")
                )));
            };
            let status = if copy_errors.is_empty() {
                JobStatus::Succeeded
            } else {
                JobStatus::Partial
            };
            let error = (!copy_errors.is_empty()).then(|| copy_errors.join("; "));
            self.database.finish_run(
                run.id,
                status.clone(),
                Some(primary.manifest_path.to_string_lossy().to_string()),
                Some(primary_manifest_json),
                error.clone(),
            )?;
            Ok::<_, Error>((status, primary.storage_uri, primary.manifest_path, error))
        }
        .await;

        let _ = std::fs::remove_file(&archive_path);

        match result {
            Ok((status, archive_uri, manifest_path, error)) => Ok(BackupRunResult {
                run_id: run.id,
                job_id,
                status,
                archive_uri: Some(archive_uri),
                manifest_path: Some(manifest_path.to_string_lossy().to_string()),
                error,
            }),
            Err(error) => {
                let error_message = format!("{error:#}");
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

impl BackupEngine {
    /// Encrypts (if configured), stores and prunes one destination's copy.
    /// Returns the stored location and the manifest JSON for that copy.
    async fn store_copy(
        &self,
        bundle: &JobBundle,
        destination: &StorageDestination,
        archive_bytes: &[u8],
        manifest: &BackupManifest,
    ) -> Result<(StoredBackup, String)> {
        let (stored_bytes, encryption) =
            encrypt_for_destination(&self.database, &destination.encryption, archive_bytes)?;
        let copy_manifest = manifest.for_stored_copy(destination.id, encryption, &stored_bytes);
        let stored = store_backup(
            &self.database,
            destination,
            &bundle.project.name,
            &bundle.target.deployment,
            &stored_bytes,
            &copy_manifest,
        )
        .await?;
        let mut final_manifest = copy_manifest;
        final_manifest.storage_uri = stored.storage_uri.clone();
        let manifest_json = serde_json::to_string_pretty(&final_manifest)?;
        if let Err(error) = prune_retention(
            &self.database,
            destination,
            &bundle.project.name,
            &bundle.target.deployment,
        )
        .await
        {
            // Retention failures must not fail a backup that was stored successfully.
            let _ = self.database.record_audit(
                "system",
                "retention.failed",
                "destination",
                Some(destination.id),
                &format!("retention pruning failed: {error:#}"),
            );
        }
        Ok((stored, manifest_json))
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

    fn test_master_key() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            // SAFETY: set once before any test reads it; the value never changes.
            unsafe {
                std::env::set_var(
                    "CONVEX_AUTOBACKUP_MASTER_KEY",
                    "test-master-key-test-master-key-0123",
                )
            };
        });
    }

    fn local_destination(db: &AppDatabase, root: &Path, name: &str) -> crate::StorageDestination {
        db.create_local_destination(CreateLocalDestination {
            name: name.to_string(),
            root: root.to_string_lossy().to_string(),
            retention: RetentionPolicy {
                keep_last: Some(2),
                keep_days: None,
                keep_weeklies: None,
                keep_monthlies: None,
            },
        })
        .unwrap()
    }

    #[tokio::test]
    async fn encrypted_fan_out_writes_every_destination_and_verifies() {
        test_master_key();
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let project = db
            .create_project(CreateProject {
                name: "Client A".to_string(),
                description: None,
            })
            .unwrap();
        let plain = local_destination(&db, &dir.path().join("plain"), "Plain");
        let encrypted = local_destination(&db, &dir.path().join("encrypted"), "Encrypted");
        let secret = crate::SecretVault::from_env(db.clone())
            .unwrap()
            .put_secret(
                "backup passphrase",
                crate::SecretKind::EncryptionKey,
                "correct horse battery staple",
            )
            .unwrap();
        db.set_destination_encryption(
            encrypted.id,
            &crate::EncryptionMode::Passphrase {
                key_ref: crate::SecretRef {
                    id: secret.id,
                    label: secret.label.clone(),
                },
            },
        )
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
        // Encrypted destination is primary so verify reads the encrypted copy first.
        let job = db
            .create_job(CreateScheduledJob {
                project_id: project.id,
                target_id: target.id,
                destination_id: encrypted.id,
                name: "Fan out".to_string(),
                include_file_storage: true,
                additional_destination_ids: vec![plain.id, encrypted.id],
            })
            .unwrap();
        assert_eq!(job.additional_destination_ids, vec![plain.id]);
        let engine = BackupEngine::new(db.clone(), dir.path().join("staging"));

        for _ in 0..3 {
            let result = engine.run_job(job.id, &FixtureExporter).await.unwrap();
            assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
            tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        }

        let runs = db.list_runs().unwrap();
        let latest = &runs[0];
        let copies = db.list_run_copies(latest.run.id).unwrap();
        assert_eq!(copies.len(), 2);
        assert!(
            copies
                .iter()
                .all(|copy| copy.status == JobStatus::Succeeded)
        );

        let deployment_dir = |root: &str| {
            dir.path()
                .join(root)
                .join("Client-A")
                .join("prod-careful-otter-123")
        };
        let encrypted_files = std::fs::read_dir(deployment_dir("encrypted"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        // keep_last = 2 pruned the oldest of three runs.
        assert_eq!(
            encrypted_files
                .iter()
                .filter(|name| name.ends_with(".zip.age"))
                .count(),
            2
        );
        assert_eq!(encrypted_files.len(), 4, "{encrypted_files:?}");
        let archive = encrypted_files
            .iter()
            .find(|name| name.ends_with(".zip.age"))
            .unwrap();
        let ciphertext = std::fs::read(deployment_dir("encrypted").join(archive)).unwrap();
        assert!(crate::crypto::is_age_ciphertext(&ciphertext));
        assert_eq!(
            crate::crypto::decrypt_with_passphrase(&ciphertext, "correct horse battery staple")
                .unwrap(),
            b"fixture convex export"
        );
        let plain_count = std::fs::read_dir(deployment_dir("plain"))
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".zip")
            })
            .count();
        assert_eq!(plain_count, 2);

        let verification = crate::verify_run(&db, latest.run.id).await.unwrap();
        assert!(verification.ok);
        assert!(verification.encrypted);
        assert_eq!(verification.destination_name.as_deref(), Some("Encrypted"));
    }

    #[tokio::test]
    async fn failed_copy_marks_run_partial_and_other_copy_still_verifies() {
        test_master_key();
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let project = db
            .create_project(CreateProject {
                name: "Client A".to_string(),
                description: None,
            })
            .unwrap();
        let good = local_destination(&db, &dir.path().join("good"), "Good");
        // An S3 destination with no credentials secret cannot store anything.
        let broken = db
            .create_s3_destination(crate::CreateS3Destination {
                name: "Broken".to_string(),
                bucket: "missing".to_string(),
                region: None,
                endpoint: Some("http://127.0.0.1:9".to_string()),
                prefix: None,
                credentials_secret_id: None,
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
                destination_id: broken.id,
                name: "Partial".to_string(),
                include_file_storage: true,
                additional_destination_ids: vec![good.id],
            })
            .unwrap();

        let result = BackupEngine::new(db.clone(), dir.path().join("staging"))
            .run_job(job.id, &FixtureExporter)
            .await
            .unwrap();

        assert_eq!(result.status, JobStatus::Partial);
        assert!(result.error.unwrap().contains("Broken"));
        let verification = crate::verify_run(&db, result.run_id).await.unwrap();
        assert!(verification.ok);
        assert_eq!(verification.destination_name.as_deref(), Some("Good"));
    }
}
