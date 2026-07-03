use crate::{ConvexTarget, ConvexTargetKind, managed_convex_bin};
use anyhow::{Context, anyhow};
use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub target: ConvexTarget,
    pub include_file_storage: bool,
    pub deploy_key: String,
}

#[derive(Debug, Clone)]
pub struct ImportRequest {
    pub target: ConvexTarget,
    pub deploy_key: String,
}

#[async_trait]
pub trait ConvexExporter: Send + Sync {
    async fn export_to_path(
        &self,
        request: ExportRequest,
        output_path: &Path,
    ) -> anyhow::Result<String>;
}

#[async_trait]
pub trait ConvexImporter: Send + Sync {
    async fn import_from_path(
        &self,
        request: ImportRequest,
        archive_path: &Path,
    ) -> anyhow::Result<String>;
}

#[derive(Debug, Clone)]
pub struct CommandConvexExporter {
    program: String,
    prefix_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CommandConvexImporter {
    program: String,
    prefix_args: Vec<String>,
}

impl Default for CommandConvexExporter {
    fn default() -> Self {
        Self::from_program(
            std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN").unwrap_or_else(|_| "npx".to_string()),
        )
    }
}

impl Default for CommandConvexImporter {
    fn default() -> Self {
        Self::from_program(
            std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN").unwrap_or_else(|_| "npx".to_string()),
        )
    }
}

impl CommandConvexExporter {
    pub fn for_data_dir(data_dir: &Path) -> Self {
        if let Ok(program) = std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN") {
            return Self::from_program(program);
        }
        let managed = managed_convex_bin(data_dir);
        if managed.is_file() {
            return Self {
                program: managed.display().to_string(),
                prefix_args: Vec::new(),
            };
        }
        Self::default()
    }

    fn from_program(program: String) -> Self {
        let prefix_args = if program.ends_with("npx") || program.ends_with("npx.cmd") {
            vec!["convex".to_string()]
        } else {
            Vec::new()
        };
        Self {
            program,
            prefix_args,
        }
    }

    pub fn command_args(request: &ExportRequest, output_path: &Path) -> Vec<String> {
        let mut args = vec![
            "export".to_string(),
            "--path".to_string(),
            output_path.to_string_lossy().to_string(),
        ];
        if request.include_file_storage {
            args.push("--include-file-storage".to_string());
        }
        match request.target.kind {
            ConvexTargetKind::Cloud => {
                args.push("--deployment-name".to_string());
                args.push(request.target.deployment.clone());
            }
            ConvexTargetKind::SelfHosted => {
                if let Some(url) = &request.target.url {
                    args.push("--url".to_string());
                    args.push(url.clone());
                }
            }
        }
        args
    }
}

impl CommandConvexImporter {
    pub fn for_data_dir(data_dir: &Path) -> Self {
        if let Ok(program) = std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN") {
            return Self::from_program(program);
        }
        let managed = managed_convex_bin(data_dir);
        if managed.is_file() {
            return Self {
                program: managed.display().to_string(),
                prefix_args: Vec::new(),
            };
        }
        Self::default()
    }

    fn from_program(program: String) -> Self {
        let prefix_args = if program.ends_with("npx") || program.ends_with("npx.cmd") {
            vec!["convex".to_string()]
        } else {
            Vec::new()
        };
        Self {
            program,
            prefix_args,
        }
    }

    pub fn command_args(request: &ImportRequest, archive_path: &Path) -> Vec<String> {
        let mut args = vec![
            "import".to_string(),
            "--path".to_string(),
            archive_path.to_string_lossy().to_string(),
            "--replace".to_string(),
        ];
        match request.target.kind {
            ConvexTargetKind::Cloud => {
                args.push("--deployment-name".to_string());
                args.push(request.target.deployment.clone());
            }
            ConvexTargetKind::SelfHosted => {
                if let Some(url) = &request.target.url {
                    args.push("--url".to_string());
                    args.push(url.clone());
                }
            }
        }
        args
    }
}

#[async_trait]
impl ConvexExporter for CommandConvexExporter {
    async fn export_to_path(
        &self,
        request: ExportRequest,
        output_path: &Path,
    ) -> anyhow::Result<String> {
        let args = Self::command_args(&request, output_path);
        let mut command = Command::new(&self.program);
        command.args(&self.prefix_args);
        command.args(&args);
        command.env("CONVEX_DEPLOY_KEY", &request.deploy_key);

        let output = command
            .output()
            .await
            .with_context(|| format!("failed to execute {}", self.program))?;
        if !output.status.success() {
            return Err(anyhow!(
                "convex export failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait]
impl ConvexImporter for CommandConvexImporter {
    async fn import_from_path(
        &self,
        request: ImportRequest,
        archive_path: &Path,
    ) -> anyhow::Result<String> {
        let args = Self::command_args(&request, archive_path);
        let mut command = Command::new(&self.program);
        command.args(&self.prefix_args);
        command.args(&args);
        command.env("CONVEX_DEPLOY_KEY", &request.deploy_key);

        let output = command
            .output()
            .await
            .with_context(|| format!("failed to execute {}", self.program))?;
        if !output.status.success() {
            return Err(anyhow!(
                "convex import failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

pub fn resolve_deploy_key(target: &ConvexTarget) -> anyhow::Result<String> {
    std::env::var(&target.secret.label).with_context(|| {
        format!(
            "deploy key environment variable {} is not set",
            target.secret.label
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecretRef;
    use uuid::Uuid;

    #[test]
    fn cloud_export_command_includes_deployment_and_file_storage() {
        let request = ExportRequest {
            target: ConvexTarget {
                id: Uuid::now_v7(),
                project_id: Uuid::now_v7(),
                name: "Prod".to_string(),
                kind: ConvexTargetKind::Cloud,
                deployment: "prod:careful-otter-123".to_string(),
                url: None,
                secret: SecretRef {
                    id: Uuid::now_v7(),
                    label: "CONVEX_DEPLOY_KEY".to_string(),
                },
            },
            include_file_storage: true,
            deploy_key: "secret".to_string(),
        };

        let args = CommandConvexExporter::command_args(&request, Path::new("/tmp/out.zip"));
        assert_eq!(
            args,
            vec![
                "export",
                "--path",
                "/tmp/out.zip",
                "--include-file-storage",
                "--deployment-name",
                "prod:careful-otter-123"
            ]
        );
    }

    #[test]
    fn cloud_import_command_requires_replace_and_deployment() {
        let request = ImportRequest {
            target: ConvexTarget {
                id: Uuid::now_v7(),
                project_id: Uuid::now_v7(),
                name: "Prod".to_string(),
                kind: ConvexTargetKind::Cloud,
                deployment: "prod:careful-otter-123".to_string(),
                url: None,
                secret: SecretRef {
                    id: Uuid::now_v7(),
                    label: "CONVEX_DEPLOY_KEY".to_string(),
                },
            },
            deploy_key: "secret".to_string(),
        };

        let args = CommandConvexImporter::command_args(&request, Path::new("/tmp/in.zip"));
        assert_eq!(
            args,
            vec![
                "import",
                "--path",
                "/tmp/in.zip",
                "--replace",
                "--deployment-name",
                "prod:careful-otter-123"
            ]
        );
    }
}
