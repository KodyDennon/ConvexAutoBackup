use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::{net::SocketAddr, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "convex-autobackup")]
#[command(about = "Self-hosted Convex backup and disaster recovery service")]
#[command(version)]
pub(crate) struct Cli {
    #[arg(long, env = "CONVEX_AUTOBACKUP_DATA_DIR")]
    pub(crate) data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Start the local/self-hosted web service.
    Serve {
        #[arg(long, env = "CONVEX_AUTOBACKUP_BIND", default_value = "0.0.0.0:8976")]
        bind: SocketAddr,
    },
    /// Start the web service and scheduler worker in one supervised process.
    Supervise {
        #[arg(long, env = "CONVEX_AUTOBACKUP_BIND", default_value = "0.0.0.0:8976")]
        bind: SocketAddr,
        #[arg(long, default_value_t = 30)]
        poll_seconds: u64,
    },
    /// Initialize the database and print paths.
    Init(JsonArg),
    /// Print service health in agent-friendly form.
    Health(JsonArg),
    /// Validate install, service, worker, and managed runner readiness.
    Doctor {
        #[arg(long, env = "CONVEX_AUTOBACKUP_BIND", default_value = "0.0.0.0:8976")]
        bind: SocketAddr,
        #[arg(long)]
        json: bool,
    },
    /// Manage the app-provisioned Convex CLI runner.
    Runner {
        #[command(subcommand)]
        command: RunnerCommand,
    },
    /// Manage projects.
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Manage API tokens.
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
    /// Manage encrypted secrets.
    Secret {
        #[command(subcommand)]
        command: SecretCommand,
    },
    /// Manage projects.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Manage Convex targets.
    Target {
        #[command(subcommand)]
        command: TargetCommand,
    },
    /// Manage storage destinations.
    Destination {
        #[command(subcommand)]
        command: DestinationCommand,
    },
    /// Manage backup jobs.
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    /// Manage persisted schedules.
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },
    /// Run or inspect backups.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// Verify stored backup archives against manifests.
    Verify {
        #[arg(long)]
        run_id: Uuid,
        #[arg(long)]
        json: bool,
    },
    /// Restore a verified backup to a Convex target.
    Restore {
        #[arg(long)]
        run_id: Uuid,
        #[arg(long)]
        target_id: Uuid,
        #[arg(long)]
        confirm_deployment: String,
        #[arg(long)]
        json: bool,
    },
    /// Generate disaster recovery evidence.
    DrReport {
        #[arg(long)]
        json: bool,
    },
    /// Inspect audit events.
    Audit {
        #[arg(long, default_value_t = 100)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    /// Inspect run history.
    Runs(JsonArg),
}

#[derive(Debug, Args)]
pub(crate) struct JsonArg {
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        json: bool,
    },
    List(JsonArg),
}

#[derive(Debug, Subcommand)]
pub(crate) enum UserCommand {
    Create {
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
        #[arg(long, default_value = "owner")]
        role: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TokenCommand {
    Create {
        #[arg(long)]
        user_id: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SecretCommand {
    Put {
        #[arg(long)]
        label: String,
        #[arg(long, default_value = "convex_deploy_key")]
        kind: String,
        #[arg(long)]
        value: String,
        #[arg(long)]
        json: bool,
    },
    List(JsonArg),
}

#[derive(Debug, Subcommand)]
pub(crate) enum TargetCommand {
    CreateCloud {
        #[arg(long)]
        project_id: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long)]
        deployment: String,
        #[arg(long)]
        deploy_key_env: Option<String>,
        #[arg(long)]
        deploy_key_secret_id: Option<Uuid>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum DestinationCommand {
    CreateLocal {
        #[arg(long)]
        name: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        json: bool,
    },
    CreateS3 {
        #[arg(long)]
        name: String,
        #[arg(long)]
        bucket: String,
        #[arg(long)]
        region: Option<String>,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        credentials_secret_id: Uuid,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum JobCommand {
    Create {
        #[arg(long)]
        project_id: Uuid,
        #[arg(long)]
        target_id: Uuid,
        #[arg(long)]
        destination_id: Uuid,
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = true)]
        include_file_storage: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum BackupCommand {
    Run {
        #[arg(long)]
        job_id: Uuid,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ScheduleCommand {
    CreateInterval {
        #[arg(long)]
        job_id: Uuid,
        #[arg(long)]
        every_minutes: u32,
        #[arg(long, default_value_t = true)]
        enabled: bool,
        #[arg(long)]
        json: bool,
    },
    List(JsonArg),
    RunDue {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RunnerCommand {
    /// Install the pinned Convex CLI runner into the app data directory.
    Install {
        #[arg(long)]
        json: bool,
    },
    /// Print managed runner status.
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct InitOutput {
    pub(crate) status: &'static str,
    pub(crate) app_name: &'static str,
    pub(crate) data_dir: String,
    pub(crate) database_path: String,
    pub(crate) staging_dir: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CliHealth {
    pub(crate) status: &'static str,
    pub(crate) service: &'static str,
    pub(crate) cli_version: &'static str,
    pub(crate) database_path: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DoctorCheck {
    pub(crate) name: &'static str,
    pub(crate) status: &'static str,
    pub(crate) detail: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DoctorOutput {
    pub(crate) status: &'static str,
    pub(crate) version: &'static str,
    pub(crate) checks: Vec<DoctorCheck>,
}
