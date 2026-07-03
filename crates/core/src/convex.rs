use crate::{ConvexTarget, ConvexTargetKind};
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
}

#[derive(Debug, Clone)]
pub struct CommandConvexImporter {
    program: String,
}

impl Default for CommandConvexExporter {
    fn default() -> Self {
        Self {
            program: std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN")
                .unwrap_or_else(|_| "npx".to_string()),
        }
    }
}

impl Default for CommandConvexImporter {
    fn default() -> Self {
        Self {
            program: std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN")
                .unwrap_or_else(|_| "npx".to_string()),
        }
    }
}

impl CommandConvexExporter {
    pub fn command_args(request: &ExportRequest, output_path: &Path) -> Vec<String> {
        let mut args = vec![
            "convex".to_string(),
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
    pub fn command_args(request: &ImportRequest, archive_path: &Path) -> Vec<String> {
        let mut args = vec![
            "convex".to_string(),
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
                "convex",
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
                "convex",
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
