use clap::{Parser, Subcommand};
use convex_autobackup_core::{AppDatabase, BackupEngine, CommandConvexExporter, SchedulerService};
use convex_autobackup_worker::QueuePolicy;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::time::sleep;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(name = "convex-autobackup-worker")]
#[command(about = "Backup worker process for ConvexAutoBackup")]
#[command(version)]
struct Cli {
    #[arg(long, env = "CONVEX_AUTOBACKUP_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the worker queue policy used by the service.
    Policy {
        #[arg(long)]
        json: bool,
    },
    /// Run the persisted schedule polling loop until interrupted.
    Run {
        #[arg(long, default_value_t = 30)]
        poll_seconds: u64,
    },
    /// Run one due-schedule pass and exit.
    RunOnce {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct PolicyOutput {
    status: &'static str,
    policy: QueuePolicy,
}

#[derive(Debug, Serialize)]
struct RunOnceOutput {
    status: &'static str,
    attempted_runs: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.unwrap_or_else(default_data_dir);
    match cli.command {
        Command::Policy { json } => {
            let output = PolicyOutput {
                status: "ready",
                policy: QueuePolicy::default(),
            };
            print_output(json, &output)?;
        }
        Command::Run { poll_seconds } => {
            fmt()
                .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
                .init();
            let scheduler = scheduler_for_data_dir(&data_dir)?;
            let exporter = CommandConvexExporter::for_data_dir(&data_dir);
            tracing::info!(
                data_dir = %data_dir.display(),
                poll_seconds,
                "starting scheduler worker"
            );
            loop {
                tokio::select! {
                    result = scheduler.run_due_once(&exporter) => {
                        match result {
                            Ok(runs) => tracing::info!(attempted_runs = runs.len(), "scheduler pass complete"),
                            Err(error) => tracing::error!(%error, "scheduler pass failed"),
                        }
                    }
                    signal = tokio::signal::ctrl_c() => {
                        signal?;
                        tracing::info!("scheduler worker shutdown requested");
                        break;
                    }
                }
                sleep(Duration::from_secs(poll_seconds)).await;
            }
        }
        Command::RunOnce { json } => {
            let scheduler = scheduler_for_data_dir(&data_dir)?;
            let exporter = CommandConvexExporter::for_data_dir(&data_dir);
            let runs = scheduler.run_due_once(&exporter).await?;
            print_output(
                json,
                &RunOnceOutput {
                    status: "complete",
                    attempted_runs: runs.len(),
                },
            )?;
        }
    }
    Ok(())
}

fn scheduler_for_data_dir(data_dir: &Path) -> anyhow::Result<SchedulerService> {
    let database = AppDatabase::open(data_dir.join("convex-autobackup.sqlite3"))?;
    let backup_engine = BackupEngine::new(database.clone(), data_dir.join("staging"));
    Ok(SchedulerService::new(database, backup_engine))
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
    convex_autobackup_core::default_data_dir()
}
