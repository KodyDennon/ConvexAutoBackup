use crate::{AppDatabase, JobStatus, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrReport {
    pub generated_at: DateTime<Utc>,
    pub total_runs: usize,
    pub successful_runs: usize,
    pub failed_runs: usize,
    pub latest_success_at: Option<DateTime<Utc>>,
    pub latest_failure_at: Option<DateTime<Utc>>,
    pub latest_manifest_path: Option<String>,
    pub readiness: DrReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrReadiness {
    Ready,
    AtRisk,
    NoBackups,
}

pub fn generate_dr_report(database: &AppDatabase) -> Result<DrReport> {
    let runs = database.list_runs()?;
    let successful_runs = runs
        .iter()
        .filter(|record| record.run.status == JobStatus::Succeeded)
        .count();
    let failed_runs = runs
        .iter()
        .filter(|record| record.run.status == JobStatus::Failed)
        .count();
    let latest_success = runs
        .iter()
        .filter(|record| record.run.status == JobStatus::Succeeded)
        .max_by_key(|record| record.run.finished_at.or(Some(record.run.started_at)));
    let latest_failure = runs
        .iter()
        .filter(|record| record.run.status == JobStatus::Failed)
        .max_by_key(|record| record.run.finished_at.or(Some(record.run.started_at)));
    let readiness = if successful_runs == 0 {
        DrReadiness::NoBackups
    } else if latest_failure.is_some_and(|failure| {
        latest_success
            .map(|success| failure.run.started_at > success.run.started_at)
            .unwrap_or(true)
    }) {
        DrReadiness::AtRisk
    } else {
        DrReadiness::Ready
    };
    Ok(DrReport {
        generated_at: Utc::now(),
        total_runs: runs.len(),
        successful_runs,
        failed_runs,
        latest_success_at: latest_success.and_then(|record| record.run.finished_at),
        latest_failure_at: latest_failure.and_then(|record| record.run.finished_at),
        latest_manifest_path: latest_success.and_then(|record| record.run.manifest_path.clone()),
        readiness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_database_reports_no_backups() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();

        let report = generate_dr_report(&db).unwrap();

        assert_eq!(report.total_runs, 0);
        assert_eq!(report.readiness, DrReadiness::NoBackups);
    }
}
