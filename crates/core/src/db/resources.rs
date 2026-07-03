use super::{
    AppDatabase, CreateCloudTarget, CreateLocalDestination, CreateProject, CreateS3Destination,
    CreateScheduledJob, JobBundle, require_non_empty,
};
use crate::models::RetentionPolicy;
use crate::models::{
    BackupJob, ConvexTarget, ConvexTargetKind, EncryptionMode, Project, SecretRef,
    StorageDestination, StorageKind,
};
use crate::{Result, ResultContext, error};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use super::rows::{destination_from_row, job_from_row, project_from_row, target_from_row};

impl AppDatabase {
    pub fn create_project(&self, input: CreateProject) -> Result<Project> {
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
        self.record_audit(
            "system",
            "project.create",
            "project",
            Some(project.id),
            &format!("created project {}", project.name),
        )?;
        Ok(project)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, team_id, name, description, created_at FROM projects ORDER BY created_at ASC",
        )?;
        let rows = statement.query_map([], project_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_cloud_target(&self, input: CreateCloudTarget) -> Result<ConvexTarget> {
        require_non_empty("target name", &input.name)?;
        require_non_empty("deployment", &input.deployment)?;
        if input.deploy_key_env.is_none() && input.deploy_key_secret_id.is_none() {
            return Err(error!("deploy_key_env or deploy_key_secret_id is required"));
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
                .ok_or_else(|| error!("deploy key env is required"))?;
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
        self.record_audit(
            "system",
            "target.create",
            "target",
            Some(target.id),
            &format!("created target {} for {}", target.name, target.deployment),
        )?;
        Ok(target)
    }

    pub fn list_targets(&self) -> Result<Vec<ConvexTarget>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, name, kind, deployment, url, secret_id, secret_label
             FROM targets ORDER BY name ASC",
        )?;
        let rows = statement.query_map([], target_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_local_destination(
        &self,
        input: CreateLocalDestination,
    ) -> Result<StorageDestination> {
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
        self.insert_destination(&destination, "created local destination")?;
        Ok(destination)
    }

    pub fn create_s3_destination(&self, input: CreateS3Destination) -> Result<StorageDestination> {
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
        self.insert_destination(&destination, "created S3-compatible destination")?;
        Ok(destination)
    }

    pub fn list_destinations(&self) -> Result<Vec<StorageDestination>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, team_id, name, kind_json, encryption_json, retention_json
             FROM destinations ORDER BY name ASC",
        )?;
        let rows = statement.query_map([], destination_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn create_job(&self, input: CreateScheduledJob) -> Result<BackupJob> {
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
        self.record_audit(
            "system",
            "job.create",
            "job",
            Some(job.id),
            &format!("created backup job {}", job.name),
        )?;
        Ok(job)
    }

    pub fn list_jobs(&self) -> Result<Vec<BackupJob>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, project_id, target_id, destination_id, name, include_file_storage, schedule_enabled
             FROM jobs ORDER BY name ASC",
        )?;
        let rows = statement.query_map([], job_from_row)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_job_bundle(&self, job_id: Uuid) -> Result<JobBundle> {
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

    pub fn get_target(&self, target_id: Uuid) -> Result<ConvexTarget> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, project_id, name, kind, deployment, url, secret_id, secret_label FROM targets WHERE id = ?1",
                params![target_id.to_string()],
                target_from_row,
            )
            .optional()?
            .ok_or_else(|| error!("target {target_id} does not exist"))
    }

    fn get_job(&self, job_id: Uuid) -> Result<BackupJob> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, project_id, target_id, destination_id, name, include_file_storage, schedule_enabled
                 FROM jobs WHERE id = ?1",
                params![job_id.to_string()],
                job_from_row,
            )
            .optional()?
            .ok_or_else(|| error!("job {job_id} does not exist"))
    }

    fn insert_destination(
        &self,
        destination: &StorageDestination,
        audit_prefix: &str,
    ) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO destinations (id, team_id, name, kind_json, encryption_json, retention_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                destination.id.to_string(),
                destination.team_id.to_string(),
                destination.name,
                serde_json::to_string(&destination.kind).context("destination kind serialization failed")?,
                serde_json::to_string(&destination.encryption)
                    .context("destination encryption serialization failed")?,
                serde_json::to_string(&destination.retention)
                    .context("destination retention serialization failed")?,
            ],
        )?;
        self.record_audit(
            "system",
            "destination.create",
            "destination",
            Some(destination.id),
            &format!("{audit_prefix} {}", destination.name),
        )?;
        Ok(())
    }
}

#[allow(dead_code)]
fn _retention_policy_is_part_of_public_input(_: RetentionPolicy) {}
