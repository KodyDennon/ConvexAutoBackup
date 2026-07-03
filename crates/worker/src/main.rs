use clap::{Parser, Subcommand};
use convex_autobackup_worker::QueuePolicy;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "convex-autobackup-worker")]
#[command(about = "Backup worker process for ConvexAutoBackup")]
struct Cli {
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
}

#[derive(Debug, Serialize)]
struct PolicyOutput {
    status: &'static str,
    policy: QueuePolicy,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Policy { json } => {
            let output = PolicyOutput {
                status: "ready",
                policy: QueuePolicy::default(),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", serde_json::to_string(&output)?);
            }
        }
    }
    Ok(())
}
