use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use convex_autobackup_core::{
    AppDatabase, AuthService, BackupEngine, CommandConvexExporter, CommandConvexImporter,
    CreateCloudTarget, CreateJobSchedule, CreateLocalDestination, CreateProject,
    CreateS3Destination, CreateScheduledJob, CreateUser, MissedRunPolicy, RestoreEngine,
    RetentionPolicy, Role, Schedule, SchedulerService, SecretKind, SecretVault, generate_dr_report,
    verify_run,
};
use convex_autobackup_server::AppState;
use serde::Serialize;
use std::{net::SocketAddr, path::PathBuf};
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "convex-autobackup")]
#[command(about = "Self-hosted Convex backup and disaster recovery service")]
struct Cli {
    #[arg(long, env = "CONVEX_AUTOBACKUP_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the local/self-hosted web service.
    Serve {
        #[arg(long, env = "CONVEX_AUTOBACKUP_BIND", default_value = "0.0.0.0:8976")]
        bind: SocketAddr,
    },
    /// Initialize the database and print paths.
    Init(JsonArg),
    /// Print service health in agent-friendly form.
    Health(JsonArg),
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
struct JsonArg {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
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
enum UserCommand {
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
enum TokenCommand {
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
enum SecretCommand {
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
enum TargetCommand {
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
enum DestinationCommand {
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
enum JobCommand {
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
enum BackupCommand {
    Run {
        #[arg(long)]
        job_id: Uuid,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ScheduleCommand {
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

#[derive(Debug, Serialize)]
struct InitOutput {
    status: &'static str,
    app_name: &'static str,
    data_dir: String,
    database_path: String,
    staging_dir: String,
}

#[derive(Debug, Serialize)]
struct CliHealth {
    status: &'static str,
    service: &'static str,
    cli_version: &'static str,
    database_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.unwrap_or_else(default_data_dir);
    let database_path = data_dir.join("convex-autobackup.sqlite3");
    let staging_dir = data_dir.join("staging");

    match cli.command {
        Command::Serve { bind } => {
            tracing_subscriber::fmt::init();
            let database = AppDatabase::open(&database_path)?;
            let listener = TcpListener::bind(bind)
                .await
                .with_context(|| format!("failed to bind {bind}"))?;
            eprintln!("ConvexAutoBackup listening at http://{bind}");
            axum::serve(
                listener,
                convex_autobackup_server::router_with_state(AppState {
                    version: env!("CARGO_PKG_VERSION"),
                    database,
                    staging_dir,
                }),
            )
            .await?;
        }
        Command::Init(args) => {
            let database = AppDatabase::open(&database_path)?;
            let output = InitOutput {
                status: "initialized",
                app_name: "ConvexAutoBackup",
                data_dir: data_dir.display().to_string(),
                database_path: database.path().display().to_string(),
                staging_dir: staging_dir.display().to_string(),
            };
            print_output(args.json, &output)?;
        }
        Command::Health(args) => {
            let database = AppDatabase::open(&database_path)?;
            let health = CliHealth {
                status: "ok",
                service: "convex-autobackup",
                cli_version: env!("CARGO_PKG_VERSION"),
                database_path: database.path().display().to_string(),
            };
            print_output(args.json, &health)?;
        }
        Command::Project { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                ProjectCommand::Create {
                    name,
                    description,
                    json,
                } => {
                    let project = database.create_project(CreateProject { name, description })?;
                    print_output(json, &serde_json::json!({ "project": project }))?;
                }
                ProjectCommand::List(args) => {
                    print_output(
                        args.json,
                        &serde_json::json!({ "projects": database.list_projects()? }),
                    )?;
                }
            }
        }
        Command::User { command } => {
            let database = AppDatabase::open(&database_path)?;
            let auth = AuthService::new(database);
            match command {
                UserCommand::Create {
                    email,
                    password,
                    role,
                    json,
                } => {
                    let user = auth.create_user(CreateUser {
                        email,
                        password,
                        role: parse_role(&role)?,
                    })?;
                    print_output(json, &serde_json::json!({ "user": user }))?;
                }
            }
        }
        Command::Token { command } => {
            let database = AppDatabase::open(&database_path)?;
            let auth = AuthService::new(database);
            match command {
                TokenCommand::Create {
                    user_id,
                    name,
                    json,
                } => {
                    let token = auth.create_api_token(user_id, &name)?;
                    print_output(json, &serde_json::json!({ "api_token": token }))?;
                }
            }
        }
        Command::Secret { command } => {
            let database = AppDatabase::open(&database_path)?;
            let vault = SecretVault::from_env(database)?;
            match command {
                SecretCommand::Put {
                    label,
                    kind,
                    value,
                    json,
                } => {
                    let secret = vault.put_secret(&label, parse_secret_kind(&kind)?, &value)?;
                    print_output(json, &serde_json::json!({ "secret": secret }))?;
                }
                SecretCommand::List(args) => {
                    print_output(
                        args.json,
                        &serde_json::json!({ "secrets": vault.list_secrets()? }),
                    )?;
                }
            }
        }
        Command::Target { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                TargetCommand::CreateCloud {
                    project_id,
                    name,
                    deployment,
                    deploy_key_env,
                    deploy_key_secret_id,
                    json,
                } => {
                    let target = database.create_cloud_target(CreateCloudTarget {
                        project_id,
                        name,
                        deployment,
                        deploy_key_env,
                        deploy_key_secret_id,
                    })?;
                    print_output(json, &serde_json::json!({ "target": target }))?;
                }
            }
        }
        Command::Destination { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                DestinationCommand::CreateLocal { name, root, json } => {
                    let destination =
                        database.create_local_destination(CreateLocalDestination {
                            name,
                            root: root.display().to_string(),
                            retention: RetentionPolicy::default(),
                        })?;
                    print_output(json, &serde_json::json!({ "destination": destination }))?;
                }
                DestinationCommand::CreateS3 {
                    name,
                    bucket,
                    region,
                    endpoint,
                    prefix,
                    credentials_secret_id,
                    json,
                } => {
                    let destination = database.create_s3_destination(CreateS3Destination {
                        name,
                        bucket,
                        region,
                        endpoint,
                        prefix,
                        credentials_secret_id,
                        retention: RetentionPolicy::default(),
                    })?;
                    print_output(json, &serde_json::json!({ "destination": destination }))?;
                }
            }
        }
        Command::Job { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                JobCommand::Create {
                    project_id,
                    target_id,
                    destination_id,
                    name,
                    include_file_storage,
                    json,
                } => {
                    let job = database.create_job(CreateScheduledJob {
                        project_id,
                        target_id,
                        destination_id,
                        name,
                        include_file_storage,
                    })?;
                    print_output(json, &serde_json::json!({ "job": job }))?;
                }
            }
        }
        Command::Backup { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                BackupCommand::Run { job_id, json } => {
                    let engine = BackupEngine::new(database, staging_dir);
                    let exporter = CommandConvexExporter::default();
                    let run = engine.run_job(job_id, &exporter).await?;
                    print_output(json, &serde_json::json!({ "run": run }))?;
                }
            }
        }
        Command::Schedule { command } => {
            let database = AppDatabase::open(&database_path)?;
            match command {
                ScheduleCommand::CreateInterval {
                    job_id,
                    every_minutes,
                    enabled,
                    json,
                } => {
                    let schedule = database.create_schedule(CreateJobSchedule {
                        job_id,
                        schedule: Schedule::IntervalMinutes {
                            every: every_minutes,
                        },
                        missed_run_policy: MissedRunPolicy::RunOnceOnResume,
                        enabled,
                    })?;
                    print_output(json, &serde_json::json!({ "schedule": schedule }))?;
                }
                ScheduleCommand::List(args) => {
                    print_output(
                        args.json,
                        &serde_json::json!({ "schedules": database.list_schedules()? }),
                    )?;
                }
                ScheduleCommand::RunDue { json } => {
                    let backup_engine = BackupEngine::new(database.clone(), staging_dir);
                    let scheduler = SchedulerService::new(database, backup_engine);
                    let exporter = CommandConvexExporter::default();
                    let runs = scheduler.run_due_once(&exporter).await?;
                    print_output(json, &serde_json::json!({ "runs": runs }))?;
                }
            }
        }
        Command::Verify { run_id, json } => {
            let database = AppDatabase::open(&database_path)?;
            let result = verify_run(&database, run_id).await?;
            print_output(json, &serde_json::json!({ "verification": result }))?;
        }
        Command::Restore {
            run_id,
            target_id,
            confirm_deployment,
            json,
        } => {
            let database = AppDatabase::open(&database_path)?;
            let restore = RestoreEngine::new(database);
            let importer = CommandConvexImporter::default();
            let result = restore
                .restore_run_to_target(run_id, target_id, &confirm_deployment, &importer)
                .await?;
            print_output(json, &serde_json::json!({ "restore": result }))?;
        }
        Command::DrReport { json } => {
            let database = AppDatabase::open(&database_path)?;
            print_output(
                json,
                &serde_json::json!({ "dr_report": generate_dr_report(&database)? }),
            )?;
        }
        Command::Audit { limit, json } => {
            let database = AppDatabase::open(&database_path)?;
            print_output(
                json,
                &serde_json::json!({ "audit_events": database.list_audit_events(limit)? }),
            )?;
        }
        Command::Runs(args) => {
            let database = AppDatabase::open(&database_path)?;
            print_output(
                args.json,
                &serde_json::json!({ "runs": database.list_runs()? }),
            )?;
        }
    }
    Ok(())
}

fn print_output<T: Serialize>(json: bool, value: &T) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", serde_json::to_string(value)?);
    }
    Ok(())
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("convex-autobackup")
}

fn parse_role(value: &str) -> anyhow::Result<Role> {
    match value {
        "owner" => Ok(Role::Owner),
        "admin" => Ok(Role::Admin),
        "operator" => Ok(Role::Operator),
        "viewer" => Ok(Role::Viewer),
        other => anyhow::bail!("unknown role {other}"),
    }
}

fn parse_secret_kind(value: &str) -> anyhow::Result<SecretKind> {
    match value {
        "convex_deploy_key" => Ok(SecretKind::ConvexDeployKey),
        "s3_credentials" => Ok(SecretKind::S3Credentials),
        "webhook_token" => Ok(SecretKind::WebhookToken),
        "encryption_key" => Ok(SecretKind::EncryptionKey),
        other => anyhow::bail!("unknown secret kind {other}"),
    }
}
