use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONVEX_CLI_PACKAGE: &str = "convex";
pub const CONVEX_CLI_VERSION: &str = "1.30.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedRunnerStatus {
    pub installed: bool,
    pub runner_dir: String,
    pub convex_bin: String,
    pub package: &'static str,
    pub version: &'static str,
}

pub fn default_data_dir() -> PathBuf {
    std::env::var("CONVEX_AUTOBACKUP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("convex-autobackup")
        })
}

pub fn convex_runner_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("runner")
}

pub fn managed_convex_bin(data_dir: &Path) -> PathBuf {
    let bin_name = if cfg!(windows) {
        "convex.cmd"
    } else {
        "convex"
    };
    convex_runner_dir(data_dir)
        .join("node_modules")
        .join(".bin")
        .join(bin_name)
}

pub fn npm_program() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

pub fn runner_status(data_dir: &Path) -> ManagedRunnerStatus {
    let bin = managed_convex_bin(data_dir);
    ManagedRunnerStatus {
        installed: bin.is_file(),
        runner_dir: convex_runner_dir(data_dir).display().to_string(),
        convex_bin: bin.display().to_string(),
        package: CONVEX_CLI_PACKAGE,
        version: CONVEX_CLI_VERSION,
    }
}
