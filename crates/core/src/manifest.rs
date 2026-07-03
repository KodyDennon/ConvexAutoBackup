use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestInput {
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub run_id: Uuid,
    pub deployment: String,
    pub convex_cli_version: String,
    pub include_file_storage: bool,
    pub archive_bytes: Vec<u8>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub run_id: Uuid,
    pub deployment: String,
    pub convex_cli_version: String,
    pub include_file_storage: bool,
    pub archive_size_bytes: u64,
    pub sha256: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_seconds: i64,
    pub storage_uri: String,
}

impl BackupManifest {
    pub fn from_input(input: ManifestInput) -> Self {
        let sha256 = Sha256::digest(&input.archive_bytes);
        Self {
            schema_version: 1,
            project_id: input.project_id,
            target_id: input.target_id,
            run_id: input.run_id,
            deployment: input.deployment,
            convex_cli_version: input.convex_cli_version,
            include_file_storage: input.include_file_storage,
            archive_size_bytes: input.archive_bytes.len() as u64,
            sha256: format!("{sha256:x}"),
            duration_seconds: (input.finished_at - input.started_at).num_seconds(),
            started_at: input.started_at,
            finished_at: input.finished_at,
            storage_uri: input.storage_uri,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    #[test]
    fn manifest_records_checksum_size_and_duration() {
        let started_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
        );
        let finished_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 2, 0)
                .unwrap(),
        );
        let manifest = BackupManifest::from_input(ManifestInput {
            project_id: Uuid::now_v7(),
            target_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            deployment: "prod:careful-otter-123".to_string(),
            convex_cli_version: "1.28.0".to_string(),
            include_file_storage: true,
            archive_bytes: b"convex backup bytes".to_vec(),
            started_at,
            finished_at,
            storage_uri: "file:///backups/prod.zip".to_string(),
        });

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.archive_size_bytes, 19);
        assert_eq!(manifest.duration_seconds, 120);
        assert_eq!(
            manifest.sha256,
            "f953a3e53e0ec7e939ccacc7fb7c0bf3b60874d2a954fe8aa07f1d418fee6f85"
        );
    }
}
