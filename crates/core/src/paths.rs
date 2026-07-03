use regex::Regex;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathSafetyError {
    #[error("backup path must be relative")]
    AbsolutePath,
    #[error("backup path may not contain parent directory components")]
    ParentDirectory,
    #[error("backup file name contains unsupported characters")]
    UnsafeName,
}

pub fn safe_backup_relative_path(
    project: &str,
    deployment: &str,
    file_name: &str,
) -> Result<PathBuf, PathSafetyError> {
    let safe_segment = Regex::new(r"^[A-Za-z0-9._-]+$").expect("static regex compiles");
    for segment in [project, deployment, file_name] {
        if !safe_segment.is_match(segment) {
            return Err(PathSafetyError::UnsafeName);
        }
    }

    let path = PathBuf::from(project).join(deployment).join(file_name);
    ensure_relative_safe(&path)?;
    Ok(path)
}

pub fn ensure_relative_safe(path: &Path) -> Result<(), PathSafetyError> {
    if path.is_absolute() {
        return Err(PathSafetyError::AbsolutePath);
    }

    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(PathSafetyError::ParentDirectory);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_safe_project_deployment_path() {
        let path = safe_backup_relative_path("project-a", "prod", "backup.zip").unwrap();
        assert_eq!(path, PathBuf::from("project-a/prod/backup.zip"));
    }

    #[test]
    fn rejects_parent_directory_escape() {
        let err = ensure_relative_safe(Path::new("project/../backup.zip")).unwrap_err();
        assert_eq!(err, PathSafetyError::ParentDirectory);
    }

    #[test]
    fn rejects_absolute_path() {
        let err = ensure_relative_safe(Path::new("/tmp/backup.zip")).unwrap_err();
        assert_eq!(err, PathSafetyError::AbsolutePath);
    }

    #[test]
    fn rejects_unsafe_segments() {
        let err = safe_backup_relative_path("project one", "prod", "backup.zip").unwrap_err();
        assert_eq!(err, PathSafetyError::UnsafeName);
    }
}
