use crate::models::{
    BackupJob, ConvexTarget, ConvexTargetKind, EncryptionMode, JobStatus, Project, RetentionPolicy,
    SecretRef, StorageDestination, StorageKind,
};
use crate::schedule::{MissedRunPolicy, Schedule};
use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

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

    pub fn create_project(&self, input: CreateProject) -> anyhow::Result<Project> {
        require_non_empty("project name", &input.name)?;
        let connection = self.connection()?;
        let project = Project {
            id: Uuid::now_v7(),
            team_id: self.default_team_id()?,
            name: input.name,
            description: input.description,
            created_at: Utc::now(),
        };
        connection.execute(
            "INSERT INTO projects (id, team_id, name, description, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.id.to_string(),
                project.team_id.to_string(),
                project.name,
                project.description,
                project.created_at.to_rfc3339()
            ],
        )?;
        Ok(project)
    }

    pub fn list_projects(&self) -> anyhow::Result<Vec<Project>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, team_id, name, description, created_at FROM projects ORDER BY created_at ASC",
        )?;
        let rows = statement.query_map([], project_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn create_cloud_target(&self, input: CreateCloudTarget) -> anyhow::Result<ConvexTarget> {
        require_non_empty("target name", &input.name)?;
        require_non_empty("deployment", &input.deployment)?;
        if input.deploy_key_env.is_none() && input.deploy_key_secret_id.is_none() {
            return Err(anyhow!(
                "deploy_key_env or deploy_key_secret_id is required"
            ));
        }
        self.require_project(input.project_id)?;
        if let Some(secret_id) = input.deploy_key_secret_id {
            self.require_secret(secret_id)?;
        }

        let secret = if let Some(secret_id) = input.deploy_key_secret_id {
            SecretRef {
                id: secret_id,
                label: "encrypted_secret".to_string(),
            }
        } else {
            let deploy_key_env = input
                .deploy_key_env
                .as_ref()
                .ok_or_else(|| anyhow!("deploy key env is required"))?;
            require_non_empty("deploy key env", deploy_key_env)?;
            SecretRef {
                id: Uuid::now_v7(),
                label: deploy_key_env.clone(),
            }
        };
        let target = ConvexTarget {
            id: Uuid::now_v7(),
            project_id: input.project_id,
            name: input.name,
            kind: ConvexTargetKind::Cloud,
            deployment: input.deployment,
            url: None,
            secret,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO targets (id, project_id, name, kind, deployment, url, secret_id, secret_label)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                target.id.to_string(),
                target.project_id.to_string(),
                target.name,
                "cloud",
                target.deployment,
                target.url,
                target.secret.id.to_string(),
                target.secret.label
            ],
        )?;
        Ok(target)
    }

    pub fn create_local_destination(
        &self,
        input: CreateLocalDestination,
    ) -> anyhow::Result<StorageDestination> {
        require_non_empty("destination name", &input.name)?;
        require_non_empty("destination root", &input.root)?;
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: self.default_team_id()?,
            name: input.name,
            kind: StorageKind::LocalFilesystem { root: input.root },
            encryption: EncryptionMode::Disabled,
            retention: input.retention,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO destinations (id, team_id, name, kind_json, encryption_json, retention_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                destination.id.to_string(),
                destination.team_id.to_string(),
                destination.name,
                serde_json::to_string(&destination.kind)?,
                serde_json::to_string(&destination.encryption)?,
                serde_json::to_string(&destination.retention)?,
            ],
        )?;
        Ok(destination)
    }

    pub fn create_s3_destination(
        &self,
        input: CreateS3Destination,
    ) -> anyhow::Result<StorageDestination> {
        require_non_empty("destination name", &input.name)?;
        require_non_empty("bucket", &input.bucket)?;
        self.require_secret(input.credentials_secret_id)?;
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: self.default_team_id()?,
            name: input.name,
            kind: StorageKind::S3Compatible {
                bucket: input.bucket,
                region: input.region,
                endpoint: input.endpoint,
                prefix: input.prefix,
                credentials: SecretRef {
                    id: input.credentials_secret_id,
                    label: "s3_credentials".to_string(),
                },
            },
            encryption: EncryptionMode::Disabled,
            retention: input.retention,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO destinations (id, team_id, name, kind_json, encryption_json, retention_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                destination.id.to_string(),
                destination.team_id.to_string(),
                destination.name,
                serde_json::to_string(&destination.kind)?,
                serde_json::to_string(&destination.encryption)?,
                serde_json::to_string(&destination.retention)?,
            ],
        )?;
        Ok(destination)
    }

    pub fn create_job(&self, input: CreateScheduledJob) -> anyhow::Result<BackupJob> {
        require_non_empty("job name", &input.name)?;
        self.require_project(input.project_id)?;
        self.require_target(input.target_id)?;
        self.require_destination(input.destination_id)?;
        let job = BackupJob {
            id: Uuid::now_v7(),
            project_id: input.project_id,
            target_id: input.target_id,
            destination_id: input.destination_id,
            name: input.name,
            include_file_storage: input.include_file_storage,
            schedule_enabled: true,
        };
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO jobs (id, project_id, target_id, destination_id, name, include_file_storage, schedule_enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                job.id.to_string(),
                job.project_id.to_string(),
                job.target_id.to_string(),
                job.destination_id.to_string(),
                job.name,
                job.include_file_storage,
                job.schedule_enabled
            ],
        )?;
        Ok(job)
    }

    pub fn get_job_bundle(&self, job_id: Uuid) -> anyhow::Result<JobBundle> {
        let connection = self.connection()?;
        let job = self.get_job(job_id)?;
        let project = connection.query_row(
            "SELECT id, team_id, name, description, created_at FROM projects WHERE id = ?1",
            params![job.project_id.to_string()],
            project_from_row,
        )?;
        let target = connection.query_row(
            "SELECT id, project_id, name, kind, deployment, url, secret_id, secret_label FROM targets WHERE id = ?1",
            params![job.target_id.to_string()],
            target_from_row,
        )?;
        let destination = connection.query_row(
            "SELECT id, team_id, name, kind_json, encryption_json, retention_json FROM destinations WHERE id = ?1",
            params![job.destination_id.to_string()],
            destination_from_row,
        )?;
        Ok(JobBundle {
            project,
            target,
            destination,
            job,
        })
    }

    pub fn get_target(&self, target_id: Uuid) -> anyhow::Result<ConvexTarget> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, project_id, name, kind, deployment, url, secret_id, secret_label FROM targets WHERE id = ?1",
                params![target_id.to_string()],
                target_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("target {target_id} does not exist"))
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
        Ok(())
    }

    fn get_job(&self, job_id: Uuid) -> anyhow::Result<BackupJob> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, project_id, target_id, destination_id, name, include_file_storage, schedule_enabled
                 FROM jobs WHERE id = ?1",
                params![job_id.to_string()],
                job_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("job {job_id} does not exist"))
    }

    fn require_project(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("projects", id, "project")
    }

    fn require_target(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("targets", id, "target")
    }

    fn require_destination(&self, id: Uuid) -> anyhow::Result<()> {
        self.require_exists("destinations", id, "destination")
    }

    fn require_secret(&self, id: Uuid) -> anyhow::Result<()> {
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

fn require_non_empty(label: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} is required"));
    }
    Ok(())
}

fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        team_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        description: row.get(3)?,
        created_at: parse_datetime(row.get::<_, String>(4)?)?,
    })
}

fn target_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConvexTarget> {
    let kind: String = row.get(3)?;
    Ok(ConvexTarget {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        project_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        kind: match kind.as_str() {
            "cloud" => ConvexTargetKind::Cloud,
            "self_hosted" => ConvexTargetKind::SelfHosted,
            other => {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    format!("unknown target kind {other}").into(),
                ));
            }
        },
        deployment: row.get(4)?,
        url: row.get(5)?,
        secret: SecretRef {
            id: parse_uuid(row.get::<_, String>(6)?)?,
            label: row.get(7)?,
        },
    })
}

fn destination_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StorageDestination> {
    let kind_json: String = row.get(3)?;
    let encryption_json: String = row.get(4)?;
    let retention_json: String = row.get(5)?;
    Ok(StorageDestination {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        team_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        kind: parse_json(3, &kind_json)?,
        encryption: parse_json(4, &encryption_json)?,
        retention: parse_json(5, &retention_json)?,
    })
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackupJob> {
    Ok(BackupJob {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        project_id: parse_uuid(row.get::<_, String>(1)?)?,
        target_id: parse_uuid(row.get::<_, String>(2)?)?,
        destination_id: parse_uuid(row.get::<_, String>(3)?)?,
        name: row.get(4)?,
        include_file_storage: row.get(5)?,
        schedule_enabled: row.get(6)?,
    })
}

fn run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRecord> {
    let status: String = row.get(2)?;
    Ok(RunRecord {
        run: crate::models::BackupRun {
            id: parse_uuid(row.get::<_, String>(0)?)?,
            job_id: parse_uuid(row.get::<_, String>(1)?)?,
            status: status_from_str(&status).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
            started_at: parse_datetime(row.get::<_, String>(3)?)?,
            finished_at: row
                .get::<_, Option<String>>(4)?
                .map(parse_datetime)
                .transpose()?,
            manifest_path: row.get(5)?,
            error: row.get(7)?,
        },
        manifest_json: row.get(6)?,
    })
}

fn schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DueJobSchedule> {
    let schedule_json: String = row.get(2)?;
    let missed_policy: String = row.get(3)?;
    Ok(DueJobSchedule {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        job_id: parse_uuid(row.get::<_, String>(1)?)?,
        schedule: parse_json(2, &schedule_json)?,
        missed_run_policy: missed_policy_from_str(&missed_policy).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, error.into())
        })?,
        next_due_at: parse_datetime(row.get::<_, String>(4)?)?,
    })
}

fn parse_uuid(value: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, error.into())
    })
}

fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, error.into())
        })
}

fn parse_json<T: for<'de> Deserialize<'de>>(index: usize, value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, error.into())
    })
}

fn status_to_str(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Failed => "failed",
        JobStatus::Canceled => "canceled",
        JobStatus::Partial => "partial",
    }
}

fn status_from_str(value: &str) -> Result<JobStatus, String> {
    match value {
        "queued" => Ok(JobStatus::Queued),
        "running" => Ok(JobStatus::Running),
        "succeeded" => Ok(JobStatus::Succeeded),
        "failed" => Ok(JobStatus::Failed),
        "canceled" => Ok(JobStatus::Canceled),
        "partial" => Ok(JobStatus::Partial),
        other => Err(format!("unknown job status {other}")),
    }
}

fn missed_policy_to_str(policy: &MissedRunPolicy) -> &'static str {
    match policy {
        MissedRunPolicy::RunOnceOnResume => "run_once_on_resume",
        MissedRunPolicy::Skip => "skip",
    }
}

fn missed_policy_from_str(value: &str) -> Result<MissedRunPolicy, String> {
    match value {
        "run_once_on_resume" => Ok(MissedRunPolicy::RunOnceOnResume),
        "skip" => Ok(MissedRunPolicy::Skip),
        other => Err(format!("unknown missed run policy {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_persists_project_destination_target_and_job() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();

        let project = db
            .create_project(CreateProject {
                name: "Client A".to_string(),
                description: Some("Production app".to_string()),
            })
            .unwrap();
        let destination = db
            .create_local_destination(CreateLocalDestination {
                name: "Local Vault".to_string(),
                root: dir.path().join("backups").to_string_lossy().to_string(),
                retention: RetentionPolicy::default(),
            })
            .unwrap();
        let target = db
            .create_cloud_target(CreateCloudTarget {
                project_id: project.id,
                name: "Production".to_string(),
                deployment: "prod:careful-otter-123".to_string(),
                deploy_key_env: Some("CONVEX_DEPLOY_KEY_CLIENT_A".to_string()),
                deploy_key_secret_id: None,
            })
            .unwrap();
        let job = db
            .create_job(CreateScheduledJob {
                project_id: project.id,
                target_id: target.id,
                destination_id: destination.id,
                name: "Nightly full backup".to_string(),
                include_file_storage: true,
            })
            .unwrap();

        let bundle = db.get_job_bundle(job.id).unwrap();
        assert_eq!(bundle.project.name, "Client A");
        assert_eq!(bundle.target.secret.label, "CONVEX_DEPLOY_KEY_CLIENT_A");
        assert!(bundle.job.include_file_storage);
        assert_eq!(db.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn database_persists_and_finds_due_schedules() {
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

        db.advance_schedule(schedule.id, Utc::now()).unwrap();
        assert_eq!(db.list_schedules().unwrap().len(), 1);
    }
}
