mod resources;
mod rows;
#[cfg(test)]
mod tests;

use crate::models::{
    BackupJob, ConvexTarget, JobStatus, Project, RetentionPolicy, StorageDestination,
};
use crate::schedule::{MissedRunPolicy, Schedule};
use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use rows::{audit_from_row, missed_policy_to_str, run_from_row, schedule_from_row, status_to_str};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCloudTarget {
    pub project_id: Uuid,
    pub name: String,
    pub deployment: String,
    #[serde(default)]
    pub deploy_key_env: Option<String>,
    #[serde(default)]
    pub deploy_key_secret_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateLocalDestination {
    pub name: String,
    pub root: String,
    pub retention: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateS3Destination {
    pub name: String,
    pub bucket: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub prefix: Option<String>,
    pub credentials_secret_id: Uuid,
    pub retention: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateScheduledJob {
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub destination_id: Uuid,
    pub name: String,
    pub include_file_storage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobBundle {
    pub project: Project,
    pub target: ConvexTarget,
    pub destination: StorageDestination,
    pub job: BackupJob,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRecord {
    pub run: crate::models::BackupRun,
    pub manifest_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: Uuid,
    pub actor: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateJobSchedule {
    pub job_id: Uuid,
    pub schedule: Schedule,
    pub missed_run_policy: MissedRunPolicy,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DueJobSchedule {
    pub id: Uuid,
    pub job_id: Uuid,
    pub schedule: Schedule,
    pub missed_run_policy: MissedRunPolicy,
    pub next_due_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AppDatabase {
    path: std::path::PathBuf,
}

impl AppDatabase {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let db = Self { path };
        db.migrate()?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection(&self) -> anyhow::Result<Connection> {
        let connection = Connection::open(&self.path)
            .with_context(|| format!("failed to open SQLite database {}", self.path.display()))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    pub fn migrate(&self) -> anyhow::Result<()> {
        let connection = self.connection()?;
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id),
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS targets (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id),
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                deployment TEXT NOT NULL,
                url TEXT,
                secret_id TEXT NOT NULL,
                secret_label TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS destinations (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id),
                name TEXT NOT NULL,
                kind_json TEXT NOT NULL,
                encryption_json TEXT NOT NULL,
                retention_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id),
                target_id TEXT NOT NULL REFERENCES targets(id),
                destination_id TEXT NOT NULL REFERENCES destinations(id),
                name TEXT NOT NULL,
                include_file_storage INTEGER NOT NULL,
                schedule_enabled INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id),
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                manifest_path TEXT,
                manifest_json TEXT,
                error TEXT
            );

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS api_tokens (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                token_hash TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL,
                revoked_at TEXT
            );

            CREATE TABLE IF NOT EXISTS secrets (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                kind TEXT NOT NULL,
                nonce_b64 TEXT NOT NULL,
                ciphertext_b64 TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schedules (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id),
                schedule_json TEXT NOT NULL,
                missed_run_policy TEXT NOT NULL,
                next_due_at TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                resource_id TEXT,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;

        let existing: Option<String> = connection
            .query_row("SELECT id FROM teams LIMIT 1", [], |row| row.get(0))
            .optional()?;
        if existing.is_none() {
            let team_id = Uuid::now_v7();
            connection.execute(
                "INSERT INTO teams (id, name, created_at) VALUES (?1, ?2, ?3)",
                params![team_id.to_string(), "Default", Utc::now().to_rfc3339()],
            )?;
        }

        Ok(())
    }

    pub fn default_team_id(&self) -> anyhow::Result<Uuid> {
        let connection = self.connection()?;
        let id: String =
            connection.query_row("SELECT id FROM teams LIMIT 1", [], |row| row.get(0))?;
        Uuid::parse_str(&id).context("stored team id is not a UUID")
    }

    pub fn user_count(&self) -> anyhow::Result<u64> {
        let connection = self.connection()?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn list_runs(&self) -> anyhow::Result<Vec<RunRecord>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, job_id, status, started_at, finished_at, manifest_path, manifest_json, error
             FROM runs ORDER BY started_at DESC",
        )?;
        let rows = statement.query_map([], run_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_run_record(&self, run_id: Uuid) -> anyhow::Result<RunRecord> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, job_id, status, started_at, finished_at, manifest_path, manifest_json, error
                 FROM runs WHERE id = ?1",
                params![run_id.to_string()],
                run_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("run {run_id} does not exist"))
    }

    pub fn create_schedule(&self, input: CreateJobSchedule) -> anyhow::Result<DueJobSchedule> {
        self.require_exists("jobs", input.job_id, "job")?;
        let now = Utc::now();
        let next_due_at = input.schedule.next_after(now)?;
        let schedule = DueJobSchedule {
            id: Uuid::now_v7(),
            job_id: input.job_id,
            schedule: input.schedule,
            missed_run_policy: input.missed_run_policy,
            next_due_at,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO schedules (id, job_id, schedule_json, missed_run_policy, next_due_at, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                schedule.id.to_string(),
                schedule.job_id.to_string(),
                serde_json::to_string(&schedule.schedule)?,
                missed_policy_to_str(&schedule.missed_run_policy),
                schedule.next_due_at.to_rfc3339(),
                input.enabled,
                now.to_rfc3339(),
                now.to_rfc3339()
            ],
        )?;
        self.record_audit(
            "system",
            "schedule.create",
            "schedule",
            Some(schedule.id),
            &format!("created schedule for job {}", schedule.job_id),
        )?;
        Ok(schedule)
    }

    pub fn list_schedules(&self) -> anyhow::Result<Vec<DueJobSchedule>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, job_id, schedule_json, missed_run_policy, next_due_at FROM schedules ORDER BY next_due_at ASC",
        )?;
        let rows = statement.query_map([], schedule_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn due_schedules(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<DueJobSchedule>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, job_id, schedule_json, missed_run_policy, next_due_at
             FROM schedules WHERE enabled = 1 AND next_due_at <= ?1 ORDER BY next_due_at ASC",
        )?;
        let rows = statement.query_map(params![now.to_rfc3339()], schedule_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn advance_schedule(&self, schedule_id: Uuid, after: DateTime<Utc>) -> anyhow::Result<()> {
        let connection = self.connection()?;
        let schedule = connection.query_row(
            "SELECT id, job_id, schedule_json, missed_run_policy, next_due_at FROM schedules WHERE id = ?1",
            params![schedule_id.to_string()],
            schedule_from_row,
        )?;
        let next_due_at = schedule.schedule.next_after(after)?;
        connection.execute(
            "UPDATE schedules SET next_due_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                next_due_at.to_rfc3339(),
                Utc::now().to_rfc3339(),
                schedule_id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn insert_run(&self, job_id: Uuid) -> anyhow::Result<crate::models::BackupRun> {
        let run = crate::models::BackupRun {
            id: Uuid::now_v7(),
            job_id,
            status: JobStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
            manifest_path: None,
            error: None,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO runs (id, job_id, status, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                run.id.to_string(),
                run.job_id.to_string(),
                status_to_str(&run.status),
                run.started_at.to_rfc3339(),
            ],
        )?;
        self.record_audit(
            "system",
            "backup.run.start",
            "run",
            Some(run.id),
            &format!("started backup run for job {}", run.job_id),
        )?;
        Ok(run)
    }

    pub fn finish_run(
        &self,
        run_id: Uuid,
        status: JobStatus,
        manifest_path: Option<String>,
        manifest_json: Option<String>,
        error: Option<String>,
    ) -> anyhow::Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "UPDATE runs
             SET status = ?1, finished_at = ?2, manifest_path = ?3, manifest_json = ?4, error = ?5
             WHERE id = ?6",
            params![
                status_to_str(&status),
                Utc::now().to_rfc3339(),
                manifest_path,
                manifest_json,
                error,
                run_id.to_string(),
            ],
        )?;
        self.record_audit(
            "system",
            match status {
                JobStatus::Succeeded => "backup.run.succeeded",
                JobStatus::Failed => "backup.run.failed",
                JobStatus::Canceled => "backup.run.canceled",
                JobStatus::Partial => "backup.run.partial",
                JobStatus::Queued => "backup.run.queued",
                JobStatus::Running => "backup.run.running",
            },
            "run",
            Some(run_id),
            error.as_deref().unwrap_or("backup run finished"),
        )?;
        Ok(())
    }

    pub fn record_audit(
        &self,
        actor: &str,
        action: &str,
        resource_type: &str,
        resource_id: Option<Uuid>,
        message: &str,
    ) -> anyhow::Result<AuditEvent> {
        require_non_empty("audit actor", actor)?;
        require_non_empty("audit action", action)?;
        require_non_empty("audit resource type", resource_type)?;
        let event = AuditEvent {
            id: Uuid::now_v7(),
            actor: actor.to_string(),
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id,
            message: message.to_string(),
            created_at: Utc::now(),
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO audit_events (id, actor, action, resource_type, resource_id, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id.to_string(),
                event.actor,
                event.action,
                event.resource_type,
                event.resource_id.map(|id| id.to_string()),
                event.message,
                event.created_at.to_rfc3339()
            ],
        )?;
        Ok(event)
    }

    pub fn list_audit_events(&self, limit: u32) -> anyhow::Result<Vec<AuditEvent>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, actor, action, resource_type, resource_id, message, created_at
             FROM audit_events ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], audit_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub(super) fn require_project(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("projects", id, "project")
    }

    pub(super) fn require_target(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("targets", id, "target")
    }

    pub(super) fn require_destination(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("destinations", id, "destination")
    }

    pub(super) fn require_secret(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("secrets", id, "secret")
    }

    fn require_exists(&self, table: &str, id: Uuid, label: &str) -> anyhow::Result<()> {
        let connection = self.connection()?;
        let count: i64 = connection.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1"),
            params![id.to_string()],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Err(anyhow!("{label} {id} does not exist"));
        }
        Ok(())
    }
}

pub(super) fn require_non_empty(label: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} is required"));
    }
    Ok(())
}
