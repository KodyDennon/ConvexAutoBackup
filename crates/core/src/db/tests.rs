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
    assert!(
        db.list_audit_events(20)
            .unwrap()
            .iter()
            .any(|event| event.action == "job.create")
    );
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
