//! Destination endpoints: creation with optional passphrase encryption, encryption changes and probes.

use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct CreateLocalDestinationRequest {
    #[serde(flatten)]
    destination: CreateLocalDestination,
    /// Optional passphrase; when set, archives are age-encrypted before storage.
    #[serde(default)]
    encryption_passphrase: Option<String>,
}

pub(crate) async fn create_local_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateLocalDestinationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let passphrase = non_empty(input.encryption_passphrase);
    if let Some(passphrase) = &passphrase {
        validate_passphrase(passphrase)?;
    }
    let destination = state.database.create_local_destination(input.destination)?;
    let destination = match passphrase {
        Some(passphrase) => {
            apply_passphrase(&state, destination.id, &destination.name, &passphrase)?
        }
        None => destination,
    };
    Ok(Json(serde_json::json!({ "destination": destination })))
}

pub(crate) fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

/// Stores `passphrase` as a new vault secret and points the destination at it.
/// Older secrets are kept so archives written under them stay readable.
pub(crate) fn apply_passphrase(
    state: &AppState,
    destination_id: Uuid,
    destination_name: &str,
    passphrase: &str,
) -> AppResult<StorageDestination> {
    validate_passphrase(passphrase)?;
    let vault = SecretVault::from_env(state.database.clone())?;
    let secret = vault.put_secret(
        &format!("{destination_name} backup passphrase"),
        SecretKind::EncryptionKey,
        passphrase,
    )?;
    state.database.set_destination_encryption(
        destination_id,
        &EncryptionMode::Passphrase {
            key_ref: SecretRef {
                id: secret.id,
                label: secret.label,
            },
        },
    )
}

#[derive(Debug, Deserialize)]
pub(crate) struct SetEncryptionRequest {
    /// New passphrase, or null/empty to stop encrypting new backups.
    #[serde(default)]
    passphrase: Option<String>,
}

pub(crate) async fn set_destination_encryption(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(destination_id): Path<Uuid>,
    Json(input): Json<SetEncryptionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let existing = state.database.get_destination(destination_id)?;
    let destination = match non_empty(input.passphrase) {
        Some(passphrase) => apply_passphrase(&state, destination_id, &existing.name, &passphrase)?,
        None => state
            .database
            .set_destination_encryption(destination_id, &EncryptionMode::Disabled)?,
    };
    Ok(Json(serde_json::json!({ "destination": destination })))
}

pub(crate) async fn test_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(destination_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let destination = state.database.get_destination(destination_id)?;
    match convex_autobackup_core::storage::test_destination(&state.database, &destination).await {
        Ok(detail) => Ok(Json(serde_json::json!({ "ok": true, "detail": detail }))),
        Err(error) => Ok(Json(
            serde_json::json!({ "ok": false, "detail": format!("{error:#}") }),
        )),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateS3DestinationRequest {
    #[serde(flatten)]
    destination: CreateS3Destination,
    /// Inline credentials; stored as a new encrypted `s3_credentials` secret.
    #[serde(default)]
    access_key_id: Option<String>,
    #[serde(default)]
    secret_access_key: Option<String>,
    #[serde(default)]
    encryption_passphrase: Option<String>,
}

pub(crate) async fn create_s3_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateS3DestinationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let passphrase = non_empty(input.encryption_passphrase);
    if let Some(passphrase) = &passphrase {
        validate_passphrase(passphrase)?;
    }
    let mut request = input.destination;
    match (
        non_empty(input.access_key_id),
        non_empty(input.secret_access_key),
    ) {
        (Some(access_key_id), Some(secret_access_key)) => {
            let vault = SecretVault::from_env(state.database.clone())?;
            let secret = vault.put_secret(
                &format!("{} S3 credentials", request.name),
                SecretKind::S3Credentials,
                &serde_json::json!({
                    "access_key_id": access_key_id.trim(),
                    "secret_access_key": secret_access_key.trim(),
                })
                .to_string(),
            )?;
            request.credentials_secret_id = Some(secret.id);
        }
        (None, None) => {}
        _ => {
            return Err(Error::message(
                "access_key_id and secret_access_key must be provided together",
            )
            .into());
        }
    }
    if request.credentials_secret_id.is_none() {
        return Err(Error::message("S3 destinations require credentials").into());
    }
    let destination = state.database.create_s3_destination(request)?;
    let destination = match passphrase {
        Some(passphrase) => {
            apply_passphrase(&state, destination.id, &destination.name, &passphrase)?
        }
        None => destination,
    };
    Ok(Json(serde_json::json!({ "destination": destination })))
}
