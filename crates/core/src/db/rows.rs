use super::{AuditEvent, DueJobSchedule, RunRecord};
use crate::models::{
    BackupJob, ConvexTarget, ConvexTargetKind, EncryptionMode, JobStatus, Project, SecretRef,
    StorageDestination, StorageKind,
};
use crate::schedule::MissedRunPolicy;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

pub(super) fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        team_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        description: row.get(3)?,
        created_at: parse_datetime(row.get::<_, String>(4)?)?,
    })
}

pub(super) fn target_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConvexTarget> {
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

pub(super) fn destination_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StorageDestination> {
    let kind_json: String = row.get(3)?;
    let encryption_json: String = row.get(4)?;
    let retention_json: String = row.get(5)?;
    Ok(StorageDestination {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        team_id: parse_uuid(row.get::<_, String>(1)?)?,
        name: row.get(2)?,
        kind: parse_json::<StorageKind>(3, &kind_json)?,
        encryption: parse_json::<EncryptionMode>(4, &encryption_json)?,
        retention: parse_json(5, &retention_json)?,
    })
}

pub(super) fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackupJob> {
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

pub(super) fn run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRecord> {
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

pub(super) fn schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DueJobSchedule> {
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

pub(super) fn audit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: parse_uuid(row.get::<_, String>(0)?)?,
        actor: row.get(1)?,
        action: row.get(2)?,
        resource_type: row.get(3)?,
        resource_id: row
            .get::<_, Option<String>>(4)?
            .map(parse_uuid)
            .transpose()?,
        message: row.get(5)?,
        created_at: parse_datetime(row.get::<_, String>(6)?)?,
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

pub(super) fn status_to_str(status: &JobStatus) -> &'static str {
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

pub(super) fn missed_policy_to_str(policy: &MissedRunPolicy) -> &'static str {
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
