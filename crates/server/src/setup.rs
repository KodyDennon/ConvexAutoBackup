//! Setup-wizard endpoints: deploy key validation, storage presets and install checks.

use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct CheckDeployKeyRequest {
    deploy_key: String,
}

/// Validates a deploy key before anything is saved: parses the deployment name and
/// proves the key works with a read-only table listing.
pub(crate) async fn check_deploy_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CheckDeployKeyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let key = input.deploy_key.trim();
    let Some(deployment) =
        convex_autobackup_core::convex::extract_deployment_name_from_deploy_key(key)
            .filter(|_| key.contains('|'))
    else {
        return Ok(Json(serde_json::json!({
            "ok": false,
            "message": "That doesn't look like a Convex deploy key. Copy it from Convex dashboard → Settings → Deploy keys (it looks like prod:name-123|…)."
        })));
    };
    let kind = key
        .split_once(':')
        .map(|(kind, _)| kind)
        .filter(|kind| !kind.contains('|'));
    let exporter = CommandConvexExporter::for_data_dir(&state.data_dir);
    Ok(Json(match exporter.check_connection(key).await {
        Ok(check) => serde_json::json!({
            "ok": true,
            "deployment": deployment,
            "key_kind": kind,
            "table_count": check.tables.len(),
            "tables": check.tables.iter().take(50).collect::<Vec<_>>(),
            "message": format!("Connected to {deployment}: {} tables visible.", check.tables.len())
        }),
        Err(error) => {
            serde_json::json!({ "ok": false, "deployment": deployment, "message": format!("{error:#}") })
        }
    }))
}

/// Offsite storage the operator pre-provisioned through the environment.
struct R2Preset {
    bucket: String,
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
}

fn r2_preset() -> Option<R2Preset> {
    let var = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    };
    Some(R2Preset {
        bucket: var("CONVEX_AUTOBACKUP_PRESET_R2_BUCKET")?,
        endpoint: var("CONVEX_AUTOBACKUP_PRESET_R2_ENDPOINT")?,
        access_key_id: var("CONVEX_AUTOBACKUP_PRESET_R2_ACCESS_KEY_ID")?,
        secret_access_key: var("CONVEX_AUTOBACKUP_PRESET_R2_SECRET_ACCESS_KEY")?,
    })
}

pub(crate) async fn setup_presets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let r2 = r2_preset()
        .map(|preset| serde_json::json!({ "bucket": preset.bucket, "endpoint": preset.endpoint }));
    Ok(Json(serde_json::json!({
        "r2": r2,
        "default_local_root": state.data_dir.join("backups"),
        "min_passphrase_len": convex_autobackup_core::MIN_PASSPHRASE_LEN,
    })))
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateR2PresetRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    retention: Option<convex_autobackup_core::RetentionPolicy>,
    #[serde(default)]
    encryption_passphrase: Option<String>,
}

pub(crate) async fn create_r2_preset_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateR2PresetRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let preset = r2_preset()
        .ok_or_else(|| Error::message("no Cloudflare R2 preset is configured on this server"))?;
    let passphrase = non_empty(input.encryption_passphrase);
    if let Some(passphrase) = &passphrase {
        validate_passphrase(passphrase)?;
    }
    let name = non_empty(input.name).unwrap_or_else(|| "Cloudflare R2 (offsite)".to_string());
    let vault = SecretVault::from_env(state.database.clone())?;
    let secret = vault.put_secret(
        &format!("{name} S3 credentials"),
        SecretKind::S3Credentials,
        &serde_json::json!({
            "access_key_id": preset.access_key_id,
            "secret_access_key": preset.secret_access_key,
        })
        .to_string(),
    )?;
    let destination = state.database.create_s3_destination(CreateS3Destination {
        name: name.clone(),
        bucket: preset.bucket,
        region: Some("auto".to_string()),
        endpoint: Some(preset.endpoint),
        prefix: non_empty(input.prefix),
        credentials_secret_id: Some(secret.id),
        retention: input.retention.unwrap_or_default(),
    })?;
    let destination = match passphrase {
        Some(passphrase) => apply_passphrase(&state, destination.id, &name, &passphrase)?,
        None => destination,
    };
    Ok(Json(serde_json::json!({ "destination": destination })))
}

#[derive(Debug, Serialize)]
struct SystemCheck {
    name: &'static str,
    ok: bool,
    detail: String,
}

/// Install health for the setup wizard: master key, Convex CLI, writable storage.
pub(crate) async fn system_checks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    let mut checks = Vec::new();
    let master_key = std::env::var("CONVEX_AUTOBACKUP_MASTER_KEY").unwrap_or_default();
    checks.push(SystemCheck {
        name: "Secret vault key",
        ok: master_key.len() >= 32,
        detail: if master_key.len() >= 32 {
            "Master key is set; stored secrets are encrypted.".to_string()
        } else {
            "CONVEX_AUTOBACKUP_MASTER_KEY is missing or shorter than 32 characters.".to_string()
        },
    });
    let convex_bin = std::env::var("CONVEX_AUTOBACKUP_CONVEX_BIN")
        .ok()
        .filter(|bin| !bin.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| convex_autobackup_core::managed_convex_bin(&state.data_dir));
    let cli_version = tokio::process::Command::new(&convex_bin)
        .arg("--version")
        .output()
        .await
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    checks.push(SystemCheck {
        name: "Convex CLI",
        ok: cli_version.is_some(),
        detail: match &cli_version {
            Some(version) => format!("Convex CLI {version} is installed."),
            None => format!("Convex CLI not found at {}.", convex_bin.display()),
        },
    });
    let probe = state
        .data_dir
        .join(format!(".write-probe-{}", Uuid::now_v7()));
    let writable = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    checks.push(SystemCheck {
        name: "Data storage",
        ok: writable,
        detail: if writable {
            format!("{} is writable.", state.data_dir.display())
        } else {
            format!("{} is not writable.", state.data_dir.display())
        },
    });
    checks.push(SystemCheck {
        name: "Offsite storage preset",
        ok: r2_preset().is_some(),
        detail: if r2_preset().is_some() {
            "Cloudflare R2 bucket is pre-configured and ready to add.".to_string()
        } else {
            "No R2 preset; you can still add any S3-compatible bucket manually.".to_string()
        },
    });
    Ok(Json(serde_json::json!({
        "checks": checks,
        "version": state.version,
        "container": std::env::var_os("CONVEX_AUTOBACKUP_CONTAINER").is_some(),
    })))
}
