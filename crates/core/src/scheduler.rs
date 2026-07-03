use crate::backup::{BackupEngine, BackupRunResult};
use crate::convex::ConvexExporter;
use crate::db::AppDatabase;
pub use crate::db::{CreateJobSchedule, DueJobSchedule};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct SchedulerService {
    database: AppDatabase,
    backup_engine: BackupEngine,
}

impl SchedulerService {
    pub fn new(database: AppDatabase, backup_engine: BackupEngine) -> Self {
        Self {
            database,
            backup_engine,
        }
    }

    pub async fn run_due_once(
        &self,
        exporter: &dyn ConvexExporter,
    ) -> anyhow::Result<Vec<BackupRunResult>> {
        let due = self.database.due_schedules(Utc::now())?;
        let mut results = Vec::with_capacity(due.len());
        for schedule in due {
            let result = self
                .backup_engine
                .run_job(schedule.job_id, exporter)
                .await?;
            self.database.advance_schedule(schedule.id, Utc::now())?;
            results.push(result);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CreateCloudTarget, CreateLocalDestination, CreateProject, CreateScheduledJob,
        MissedRunPolicy, RetentionPolicy, Schedule,
    };
    use async_trait::async_trait;
    use std::path::Path;

    struct FixtureExporter;

    #[async_trait]
    impl ConvexExporter for FixtureExporter {
        async fn export_to_path(
            &self,
            _request: crate::ExportRequest,
            output_path: &Path,
        ) -> anyhow::Result<String> {
            tokio::fs::write(output_path, b"scheduled export").await?;
            Ok("scheduled".to_string())
        }
    }

    #[tokio::test]
    async fn scheduler_runs_due_jobs_and_advances_schedule() {
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
        let schedule = db
            .create_schedule(CreateJobSchedule {
                job_id: job.id,
                schedule: Schedule::IntervalMinutes { every: 1 },
                missed_run_policy: MissedRunPolicy::RunOnceOnResume,
                enabled: true,
            })
            .unwrap();
        db.advance_schedule(schedule.id, Utc::now() - chrono::Duration::minutes(2))
            .unwrap();
        let backup_engine = BackupEngine::new(db.clone(), dir.path().join("staging"));
        let scheduler = SchedulerService::new(db.clone(), backup_engine);

        let results = scheduler.run_due_once(&FixtureExporter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(db.list_runs().unwrap().len(), 1);
    }
}
