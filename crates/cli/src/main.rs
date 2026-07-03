mod args;

use anyhow::Context;
use args::*;
use clap::Parser;
use convex_autobackup_core::{
    AppDatabase, AuthService, BackupEngine, CONVEX_CLI_PACKAGE, CONVEX_CLI_VERSION,
    CommandConvexExporter, CommandConvexImporter, CreateCloudTarget, CreateJobSchedule,
    CreateLocalDestination, CreateProject, CreateS3Destination, CreateScheduledJob, CreateUser,
    MissedRunPolicy, RestoreEngine, RetentionPolicy, Role, Schedule, SchedulerService, SecretKind,
    SecretVault, convex_runner_dir, generate_dr_report, list_secret_metadata, npm_program,
    runner_status, verify_run,
};
use convex_autobackup_server::AppState;
use serde::Serialize;
use std::{path::PathBuf, time::Duration};
use tokio::{net::TcpListener, process::Command as TokioCommand, time::sleep};

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
                    data_dir,
                    database,
                    staging_dir,
                }),
            )
            .await?;
        }
        Command::Supervise { bind, poll_seconds } => {
            tracing_subscriber::fmt::init();
            let database = AppDatabase::open(&database_path)?;
            let listener = TcpListener::bind(bind)
                .await
                .with_context(|| format!("failed to bind {bind}"))?;
            let scheduler = SchedulerService::new(
                database.clone(),
                BackupEngine::new(database.clone(), staging_dir.clone()),
            );
            let exporter = CommandConvexExporter::for_data_dir(&data_dir);
            let server = axum::serve(
                listener,
                convex_autobackup_server::router_with_state(AppState {
                    version: env!("CARGO_PKG_VERSION"),
                    data_dir: data_dir.clone(),
                    database,
                    staging_dir,
                }),
            );
            eprintln!("ConvexAutoBackup supervised service listening at http://{bind}");
            tokio::select! {
                result = server => result?,
                result = run_scheduler_loop(scheduler, exporter, poll_seconds) => result?,
                signal = tokio::signal::ctrl_c() => signal?,
            }
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
        Command::Doctor { bind, json } => {
            let output = run_doctor(&data_dir, &database_path, bind).await;
            let has_error = output.checks.iter().any(|check| check.status == "error");
            print_output(json, &output)?;
            if has_error {
                anyhow::bail!("doctor found install problems");
            }
        }
        Command::Runner { command } => match command {
            RunnerCommand::Install { json } => {
                let status = install_runner(&data_dir).await?;
                print_output(json, &serde_json::json!({ "runner": status }))?;
            }
            RunnerCommand::Status { json } => {
                print_output(
                    json,
                    &serde_json::json!({ "runner": runner_status(&data_dir) }),
                )?;
            }
        },
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
            match command {
                SecretCommand::Put {
                    label,
                    kind,
                    value,
                    json,
                } => {
                    let vault = SecretVault::from_env(database)?;
                    let secret = vault.put_secret(&label, parse_secret_kind(&kind)?, &value)?;
                    print_output(json, &serde_json::json!({ "secret": secret }))?;
                }
                SecretCommand::List(args) => {
                    print_output(
                        args.json,
                        &serde_json::json!({ "secrets": list_secret_metadata(&database)? }),
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
                    let exporter = CommandConvexExporter::for_data_dir(&data_dir);
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
                    let exporter = CommandConvexExporter::for_data_dir(&data_dir);
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
            let importer = CommandConvexImporter::for_data_dir(&data_dir);
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

async fn run_scheduler_loop(
    scheduler: SchedulerService,
    exporter: CommandConvexExporter,
    poll_seconds: u64,
) -> anyhow::Result<()> {
    loop {
        if let Err(error) = scheduler.run_due_once(&exporter).await {
            eprintln!("scheduler pass failed: {error}");
        }
        sleep(Duration::from_secs(poll_seconds)).await;
    }
}

async fn install_runner(
    data_dir: &std::path::Path,
) -> anyhow::Result<convex_autobackup_core::ManagedRunnerStatus> {
    let runner_dir = convex_runner_dir(data_dir);
    tokio::fs::create_dir_all(&runner_dir).await?;
    let package_json = serde_json::json!({
        "private": true,
        "dependencies": {
            CONVEX_CLI_PACKAGE: CONVEX_CLI_VERSION
        }
    });
    tokio::fs::write(
        runner_dir.join("package.json"),
        serde_json::to_string_pretty(&package_json)?,
    )
    .await?;
    let output = TokioCommand::new(npm_program())
        .arg("install")
        .arg("--omit=dev")
        .arg("--no-audit")
        .arg("--no-fund")
        .current_dir(&runner_dir)
        .output()
        .await
        .with_context(|| format!("failed to execute {}", npm_program()))?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to install managed Convex runner: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let status = runner_status(data_dir);
    if !status.installed {
        anyhow::bail!("managed Convex runner install completed but binary was not found");
    }
    Ok(status)
}

async fn run_doctor(
    data_dir: &std::path::Path,
    database_path: &std::path::Path,
    bind: std::net::SocketAddr,
) -> DoctorOutput {
    let mut checks = Vec::new();
    checks.push(match tokio::fs::create_dir_all(data_dir).await {
        Ok(()) => DoctorCheck {
            name: "data_dir",
            status: "ok",
            detail: data_dir.display().to_string(),
        },
        Err(error) => DoctorCheck {
            name: "data_dir",
            status: "error",
            detail: error.to_string(),
        },
    });
    checks.push(match AppDatabase::open(database_path) {
        Ok(database) => DoctorCheck {
            name: "database",
            status: "ok",
            detail: database.path().display().to_string(),
        },
        Err(error) => DoctorCheck {
            name: "database",
            status: "error",
            detail: error.to_string(),
        },
    });
    checks.push(match std::env::var("CONVEX_AUTOBACKUP_MASTER_KEY") {
        Ok(value) if value.len() >= 32 => DoctorCheck {
            name: "master_key",
            status: "ok",
            detail: "CONVEX_AUTOBACKUP_MASTER_KEY is set".to_string(),
        },
        Ok(_) => DoctorCheck {
            name: "master_key",
            status: "error",
            detail: "CONVEX_AUTOBACKUP_MASTER_KEY is shorter than 32 characters".to_string(),
        },
        Err(_) => DoctorCheck {
            name: "master_key",
            status: "error",
            detail: "CONVEX_AUTOBACKUP_MASTER_KEY is not set".to_string(),
        },
    });
    let runner = runner_status(data_dir);
    checks.push(DoctorCheck {
        name: "managed_runner",
        status: if runner.installed { "ok" } else { "error" },
        detail: runner.convex_bin,
    });
    checks.push(DoctorCheck {
        name: "worker",
        status: "ok",
        detail: "scheduler is available through the supervise command".to_string(),
    });
    let host = if bind.ip().is_unspecified() {
        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
    } else {
        bind.ip()
    };
    let health_addr = std::net::SocketAddr::new(host, bind.port());
    checks.push(match tokio::net::TcpStream::connect(health_addr).await {
        Ok(_) => DoctorCheck {
            name: "service",
            status: "ok",
            detail: format!("reachable at {health_addr}"),
        },
        Err(error) => DoctorCheck {
            name: "service",
            status: "error",
            detail: format!("not reachable at {health_addr}: {error}"),
        },
    });
    let status = if checks.iter().any(|check| check.status == "error") {
        "error"
    } else {
        "ok"
    };
    DoctorOutput {
        status,
        version: env!("CARGO_PKG_VERSION"),
        checks,
    }
}

fn default_data_dir() -> PathBuf {
    convex_autobackup_core::default_data_dir()
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
