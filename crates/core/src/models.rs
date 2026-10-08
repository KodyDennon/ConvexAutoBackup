use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConvexTarget {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub kind: ConvexTargetKind,
    pub deployment: String,
    pub url: Option<String>,
    pub secret: SecretRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConvexTargetKind {
    Cloud,
    SelfHosted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretRef {
    pub id: Uuid,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageDestination {
    pub id: Uuid,
    pub team_id: Uuid,
    pub name: String,
    pub kind: StorageKind,
    pub encryption: EncryptionMode,
    pub retention: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageKind {
    LocalFilesystem {
        root: String,
    },
    S3Compatible {
        bucket: String,
        region: Option<String>,
        endpoint: Option<String>,
        prefix: Option<String>,
        credentials: SecretRef,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionMode {
    Disabled,
    AgeX25519 {
        recipient: String,
    },
    ManagedKey {
        key_ref: SecretRef,
    },
    /// age scrypt passphrase encryption; the passphrase lives in the secret vault.
    Passphrase {
        key_ref: SecretRef,
    },
}

impl EncryptionMode {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub keep_last: Option<u32>,
    pub keep_days: Option<u32>,
    pub keep_weeklies: Option<u32>,
    pub keep_monthlies: Option<u32>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_last: Some(20),
            keep_days: Some(30),
            keep_weeklies: Some(12),
            keep_monthlies: Some(12),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub destination_id: Uuid,
    /// Extra destinations that receive a copy of the same export.
    #[serde(default)]
    pub additional_destination_ids: Vec<Uuid>,
    pub name: String,
    pub include_file_storage: bool,
    pub schedule_enabled: bool,
}

impl BackupJob {
    /// Primary destination first, then additional destinations.
    pub fn all_destination_ids(&self) -> Vec<Uuid> {
        let mut ids = vec![self.destination_id];
        for id in &self.additional_destination_ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        ids
    }

    pub fn full_backup(
        project_id: Uuid,
        target_id: Uuid,
        destination_id: Uuid,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            project_id,
            target_id,
            destination_id,
            additional_destination_ids: Vec::new(),
            name: name.into(),
            include_file_storage: true,
            schedule_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupRun {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub manifest_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
    Partial,
}
