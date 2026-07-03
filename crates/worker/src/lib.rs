use convex_autobackup_core::BackupJob;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueuePolicy {
    pub global_concurrency: usize,
    pub per_target_concurrency: usize,
    pub per_destination_concurrency: usize,
}

impl Default for QueuePolicy {
    fn default() -> Self {
        Self {
            global_concurrency: 2,
            per_target_concurrency: 1,
            per_destination_concurrency: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueuedBackup {
    pub queue_id: Uuid,
    pub job: BackupJob,
    pub attempt: u32,
}

impl QueuedBackup {
    pub fn new(job: BackupJob) -> Self {
        Self {
            queue_id: Uuid::now_v7(),
            job,
            attempt: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_policy_defaults_to_safe_serialization_per_target_and_destination() {
        let policy = QueuePolicy::default();
        assert_eq!(policy.global_concurrency, 2);
        assert_eq!(policy.per_target_concurrency, 1);
        assert_eq!(policy.per_destination_concurrency, 1);
    }
}
