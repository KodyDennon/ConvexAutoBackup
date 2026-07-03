use crate::{
    AppDatabase, BackupManifest, SecretVault, StorageDestination, StorageKind,
    paths::safe_backup_relative_path,
};
use anyhow::{Context, anyhow};
use chrono::Utc;
use object_store::{
    ObjectStoreExt,
    aws::{AmazonS3, AmazonS3Builder},
    path::Path as ObjectPath,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StoredBackup {
    pub archive_path: PathBuf,
    pub manifest_path: PathBuf,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3CredentialSecret {
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub async fn store_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> anyhow::Result<StoredBackup> {
    match &destination.kind {
        StorageKind::LocalFilesystem { .. } => store_local_backup(
            destination,
            project_name,
            deployment,
            archive_bytes,
            manifest,
        ),
        StorageKind::S3Compatible { .. } => {
            store_s3_backup(
                database,
                destination,
                project_name,
                deployment,
                archive_bytes,
                manifest,
            )
            .await
        }
    }
}

pub fn store_local_backup(
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> anyhow::Result<StoredBackup> {
    let StorageKind::LocalFilesystem { root } = &destination.kind else {
        return Err(anyhow!(
            "destination {} is not local filesystem",
            destination.id
        ));
    };

    let archive_name = format!(
        "{}-{}.zip",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        manifest.run_id
    );
    let manifest_name = format!("{archive_name}.manifest.json");
    let project_segment = safe_segment(project_name);
    let deployment_segment = safe_segment(deployment);
    let relative_archive =
        safe_backup_relative_path(&project_segment, &deployment_segment, &archive_name)?;
    let relative_manifest =
        safe_backup_relative_path(&project_segment, &deployment_segment, &manifest_name)?;
    let archive_path = Path::new(root).join(&relative_archive);
    let manifest_path = Path::new(root).join(&relative_manifest);
    let parent = archive_path
        .parent()
        .ok_or_else(|| anyhow!("archive path has no parent"))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create backup directory {}", parent.display()))?;

    let tmp_archive = archive_path.with_extension("zip.tmp");
    std::fs::write(&tmp_archive, archive_bytes)
        .with_context(|| format!("failed to write {}", tmp_archive.display()))?;
    std::fs::rename(&tmp_archive, &archive_path)
        .with_context(|| format!("failed to commit {}", archive_path.display()))?;

    let manifest_json = serde_json::to_vec_pretty(manifest)?;
    let tmp_manifest = manifest_path.with_extension("json.tmp");
    std::fs::write(&tmp_manifest, manifest_json)
        .with_context(|| format!("failed to write {}", tmp_manifest.display()))?;
    std::fs::rename(&tmp_manifest, &manifest_path)
        .with_context(|| format!("failed to commit {}", manifest_path.display()))?;

    Ok(StoredBackup {
        storage_uri: format!("file://{}", archive_path.display()),
        archive_path,
        manifest_path,
    })
}

pub async fn store_s3_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> anyhow::Result<StoredBackup> {
    let StorageKind::S3Compatible {
        bucket,
        region: _,
        endpoint: _,
        prefix,
        credentials: _,
    } = &destination.kind
    else {
        return Err(anyhow!(
            "destination {} is not S3-compatible",
            destination.id
        ));
    };

    let store = s3_store_from_destination(database, destination)?;

    let archive_name = format!(
        "{}-{}.zip",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        manifest.run_id
    );
    let manifest_name = format!("{archive_name}.manifest.json");
    let base_key = object_key(prefix.as_deref(), project_name, deployment);
    let archive_key = format!("{base_key}/{archive_name}");
    let manifest_key = format!("{base_key}/{manifest_name}");

    store
        .put(
            &ObjectPath::from(archive_key.as_str()),
            archive_bytes.to_vec().into(),
        )
        .await
        .context("failed to upload S3 archive")?;
    store
        .put(
            &ObjectPath::from(manifest_key.as_str()),
            serde_json::to_vec_pretty(manifest)?.into(),
        )
        .await
        .context("failed to upload S3 manifest")?;

    Ok(StoredBackup {
        archive_path: PathBuf::from(&archive_key),
        manifest_path: PathBuf::from(&manifest_key),
        storage_uri: format!("s3://{bucket}/{archive_key}"),
    })
}

pub fn s3_store_from_destination(
    database: &AppDatabase,
    destination: &StorageDestination,
) -> anyhow::Result<AmazonS3> {
    let StorageKind::S3Compatible {
        bucket,
        region,
        endpoint,
        credentials,
        ..
    } = &destination.kind
    else {
        return Err(anyhow!(
            "destination {} is not S3-compatible",
            destination.id
        ));
    };
    let secret_json = SecretVault::from_env(database.clone())?.get_secret(credentials.id)?;
    let secret: S3CredentialSecret =
        serde_json::from_str(&secret_json).context("S3 credential secret must be JSON")?;
    let mut builder = AmazonS3Builder::new()
        .with_bucket_name(bucket)
        .with_access_key_id(secret.access_key_id)
        .with_secret_access_key(secret.secret_access_key)
        .with_region(region.clone().unwrap_or_else(|| "auto".to_string()));
    if let Some(endpoint) = endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    builder.build().context("failed to build S3 object store")
}

pub fn s3_object_key_from_uri(uri: &str) -> anyhow::Result<String> {
    let without_scheme = uri
        .strip_prefix("s3://")
        .ok_or_else(|| anyhow!("S3 URI must start with s3://"))?;
    let (_, key) = without_scheme
        .split_once('/')
        .ok_or_else(|| anyhow!("S3 URI must include bucket and key"))?;
    Ok(key.to_string())
}

fn object_key(prefix: Option<&str>, project_name: &str, deployment: &str) -> String {
    [
        prefix.unwrap_or("").trim_matches('/'),
        &safe_segment(project_name),
        &safe_segment(deployment),
    ]
    .into_iter()
    .filter(|segment| !segment.is_empty())
    .collect::<Vec<_>>()
    .join("/")
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EncryptionMode, RetentionPolicy};
    use chrono::{NaiveDate, TimeZone};
    use uuid::Uuid;

    #[test]
    fn local_storage_writes_archive_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: Uuid::now_v7(),
            name: "Local".to_string(),
            kind: StorageKind::LocalFilesystem {
                root: dir.path().to_string_lossy().to_string(),
            },
            encryption: EncryptionMode::Disabled,
            retention: RetentionPolicy::default(),
        };
        let started_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
        );
        let manifest = BackupManifest {
            schema_version: 1,
            project_id: Uuid::now_v7(),
            target_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            deployment: "prod:careful-otter-123".to_string(),
            convex_cli_version: "test".to_string(),
            include_file_storage: true,
            archive_size_bytes: 5,
            sha256: "abc".to_string(),
            started_at,
            finished_at: started_at,
            duration_seconds: 0,
            storage_uri: "pending".to_string(),
        };

        let stored = store_local_backup(
            &destination,
            "Client A",
            "prod:careful-otter-123",
            b"bytes",
            &manifest,
        )
        .unwrap();

        assert_eq!(std::fs::read(&stored.archive_path).unwrap(), b"bytes");
        assert!(
            std::fs::read_to_string(&stored.manifest_path)
                .unwrap()
                .contains("careful-otter")
        );
    }
}
