use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum PathSafetyError {
    AbsolutePath,
    ParentDirectory,
    UnsafeName,
}

impl fmt::Display for PathSafetyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AbsolutePath => formatter.write_str("backup path must be relative"),
            Self::ParentDirectory => {
                formatter.write_str("backup path may not contain parent directory components")
            }
            Self::UnsafeName => {
                formatter.write_str("backup file name contains unsupported characters")
            }
        }
    }
}

impl std::error::Error for PathSafetyError {}

impl From<PathSafetyError> for crate::Error {
    fn from(error: PathSafetyError) -> Self {
        Self::message(error.to_string())
    }
}

pub fn safe_backup_relative_path(
    project: &str,
    deployment: &str,
    file_name: &str,
) -> Result<PathBuf, PathSafetyError> {
    for segment in [project, deployment, file_name] {
        if !is_safe_segment(segment) {
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

fn is_safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
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

    #[test]
    fn rejects_empty_and_non_ascii_segments() {
        assert_eq!(
            safe_backup_relative_path("", "prod", "backup.zip").unwrap_err(),
            PathSafetyError::UnsafeName
        );
        assert_eq!(
            safe_backup_relative_path("projet", "prod", "backup-ñ.zip").unwrap_err(),
            PathSafetyError::UnsafeName
        );
    }
}
