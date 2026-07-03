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
    default_data_dir_from(|name| std::env::var(name).ok())
}

fn default_data_dir_from(mut env: impl FnMut(&str) -> Option<String>) -> PathBuf {
    if let Some(path) = env("CONVEX_AUTOBACKUP_DATA_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }

    platform_data_home(&mut env)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("convex-autobackup")
}

#[cfg(windows)]
fn platform_data_home(env: &mut impl FnMut(&str) -> Option<String>) -> Option<PathBuf> {
    env("APPDATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join("AppData").join("Roaming"))
        })
}

#[cfg(not(windows))]
fn platform_data_home(env: &mut impl FnMut(&str) -> Option<String>) -> Option<PathBuf> {
    env("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".local").join("share"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_dir(values: &[(&str, &str)]) -> PathBuf {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        default_data_dir_from(|key| values.get(key).cloned())
    }

    #[test]
    fn explicit_data_dir_wins() {
        assert_eq!(
            env_dir(&[
                ("CONVEX_AUTOBACKUP_DATA_DIR", "/custom/data"),
                ("XDG_DATA_HOME", "/xdg")
            ]),
            PathBuf::from("/custom/data")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_uses_xdg_then_home_fallback() {
        assert_eq!(
            env_dir(&[("XDG_DATA_HOME", "/xdg")]),
            PathBuf::from("/xdg/convex-autobackup")
        );
        assert_eq!(
            env_dir(&[("HOME", "/home/operator")]),
            PathBuf::from("/home/operator/.local/share/convex-autobackup")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_uses_appdata_then_home_fallback() {
        assert_eq!(
            env_dir(&[("APPDATA", r"C:\Users\Operator\AppData\Roaming")]),
            PathBuf::from(r"C:\Users\Operator\AppData\Roaming\convex-autobackup")
        );
        assert_eq!(
            env_dir(&[("HOME", r"C:\Users\Operator")]),
            PathBuf::from(r"C:\Users\Operator\AppData\Roaming\convex-autobackup")
        );
    }
}
