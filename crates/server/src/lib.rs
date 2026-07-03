mod metadata;
#[cfg(test)]
mod tests;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use convex_autobackup_core::{
    AppDatabase, AuthService, BackupEngine, CommandConvexExporter, CommandConvexImporter,
    CreateCloudTarget, CreateJobSchedule, CreateLocalDestination, CreateProject,
    CreateS3Destination, CreateScheduledJob, CreateUser, RestoreEngine, Role, SchedulerService,
    SecretKind, SecretVault, User, generate_dr_report, list_secret_metadata, verify_run,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use metadata::{capabilities, openapi_spec};

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

#[derive(Debug, Clone)]
pub struct AppState {
    pub version: &'static str,
    pub data_dir: PathBuf,
    pub database: AppDatabase,
    pub staging_dir: PathBuf,
}

impl AppState {
    pub fn open_default() -> anyhow::Result<Self> {
        let data_dir = default_data_dir();
        Ok(Self {
            version: env!("CARGO_PKG_VERSION"),
            data_dir: data_dir.clone(),
            database: AppDatabase::open(data_dir.join("convex-autobackup.sqlite3"))?,
            staging_dir: data_dir.join("staging"),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub database_path: String,
    pub users_configured: bool,
}

pub fn router() -> anyhow::Result<Router> {
    Ok(router_with_state(AppState::open_default()?))
}

pub fn router_with_state(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/", get(index))
        .route("/assets/{*path}", get(static_asset))
        .route("/api/v1/health", get(health))
        .route("/api/v1/openapi.json", get(openapi_spec))
        .route("/api/v1/capabilities", get(capabilities))
        .route("/api/v1/bootstrap", post(bootstrap_owner))
        .route("/api/v1/login", post(login))
        .route("/api/v1/users", get(list_users).post(create_user))
        .route(
            "/api/v1/tokens",
            get(list_api_tokens).post(create_api_token),
        )
        .route("/api/v1/tokens/{token_id}", delete(revoke_api_token))
        .route("/api/v1/secrets", get(list_secrets).post(put_secret))
        .route("/api/v1/projects", get(list_projects).post(create_project))
        .route("/api/v1/targets", get(list_targets))
        .route("/api/v1/targets/cloud", post(create_cloud_target))
        .route("/api/v1/destinations", get(list_destinations))
        .route("/api/v1/destinations/local", post(create_local_destination))
        .route("/api/v1/destinations/s3", post(create_s3_destination))
        .route("/api/v1/jobs", get(list_jobs).post(create_job))
        .route(
            "/api/v1/schedules",
            get(list_schedules).post(create_schedule),
        )
        .route("/api/v1/schedules/run-due", post(run_due_schedules))
        .route("/api/v1/runs/{run_id}/verify", post(verify_backup_run))
        .route("/api/v1/restore", post(restore_backup_run))
        .route("/api/v1/dr/report", get(dr_report))
        .route("/api/v1/audit", get(list_audit_events))
        .route("/api/v1/jobs/{job_id}/run", post(run_job))
        .route("/api/v1/runs", get(list_runs))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn index() -> Response {
    embedded_asset("index.html").unwrap_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, "web UI is not embedded").into_response()
    })
}

async fn static_asset(Path(path): Path<String>) -> Response {
    let asset_path = format!("assets/{path}");
    embedded_asset(&asset_path).unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
}

fn embedded_asset(path: &str) -> Option<Response> {
    WebAssets::get(path).map(|asset| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        (
            [(header::CONTENT_TYPE, mime.as_ref())],
            asset.data.into_owned(),
        )
            .into_response()
    })
}

async fn health(State(state): State<Arc<AppState>>) -> Result<Json<HealthResponse>, ApiError> {
    Ok(Json(HealthResponse {
        status: "ok",
        service: "convex-autobackup",
        version: state.version,
        database_path: state.database.path().display().to_string(),
        users_configured: state.database.user_count()? > 0,
    }))
}

async fn bootstrap_owner(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateUser>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if state.database.user_count()? != 0 {
        return Err(ApiError::bad_request("bootstrap is already complete"));
    }
    let auth = AuthService::new(state.database.clone());
    let user = auth.create_user(CreateUser {
        role: Role::Owner,
        ..input
    })?;
    let api_token = auth.create_api_token(user.id, "bootstrap")?;
    Ok(Json(serde_json::json!({
        "user": user,
        "api_token": api_token
    })))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = AuthService::new(state.database.clone());
    let user = auth.verify_password(&input.email, &input.password)?;
    let api_token = auth.create_api_token(user.id, "web-login")?;
    Ok(Json(serde_json::json!({
        "user": user,
        "api_token": api_token
    })))
}

#[derive(Debug, Deserialize)]
struct CreateTokenRequest {
    user_id: Uuid,
    name: String,
}

async fn create_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateTokenRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let auth = AuthService::new(state.database.clone());
    Ok(Json(serde_json::json!({
        "api_token": auth.create_api_token(input.user_id, &input.name)?
    })))
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let auth = AuthService::new(state.database.clone());
    Ok(Json(serde_json::json!({
        "users": auth.list_users()?
    })))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateUser>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let auth = AuthService::new(state.database.clone());
    Ok(Json(serde_json::json!({
        "user": auth.create_user(input)?
    })))
}

async fn list_api_tokens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let auth = AuthService::new(state.database.clone());
    Ok(Json(serde_json::json!({
        "api_tokens": auth.list_api_tokens()?
    })))
}

async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let auth = AuthService::new(state.database.clone());
    Ok(Json(serde_json::json!({
        "api_token": auth.revoke_api_token(token_id)?
    })))
}

#[derive(Debug, Deserialize)]
struct PutSecretRequest {
    label: String,
    kind: SecretKind,
    value: String,
}

async fn put_secret(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<PutSecretRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let vault = SecretVault::from_env(state.database.clone())?;
    Ok(Json(serde_json::json!({
        "secret": vault.put_secret(&input.label, input.kind, &input.value)?
    })))
}

async fn list_secrets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "secrets": list_secret_metadata(&state.database)? }),
    ))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(
        serde_json::json!({ "projects": state.database.list_projects()? }),
    ))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateProject>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "project": state.database.create_project(input)? }),
    ))
}

async fn create_cloud_target(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateCloudTarget>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "target": state.database.create_cloud_target(input)? }),
    ))
}

async fn list_targets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(serde_json::json!({
        "targets": state.database.list_targets()?
    })))
}

async fn create_local_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateLocalDestination>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "destination": state.database.create_local_destination(input)? }),
    ))
}

async fn list_destinations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(serde_json::json!({
        "destinations": state.database.list_destinations()?
    })))
}

async fn create_s3_destination(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateS3Destination>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "destination": state.database.create_s3_destination(input)? }),
    ))
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(serde_json::json!({
        "jobs": state.database.list_jobs()?
    })))
}

async fn create_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateScheduledJob>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(
        serde_json::json!({ "job": state.database.create_job(input)? }),
    ))
}

async fn run_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::RunBackup)?;
    let engine = BackupEngine::new(state.database.clone(), state.staging_dir.clone());
    let exporter = CommandConvexExporter::for_data_dir(&state.data_dir);
    Ok(Json(serde_json::json!({
        "run": engine.run_job(job_id, &exporter).await?
    })))
}

async fn create_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateJobSchedule>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(serde_json::json!({
        "schedule": state.database.create_schedule(input)?
    })))
}

async fn list_schedules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(serde_json::json!({
        "schedules": state.database.list_schedules()?
    })))
}

async fn run_due_schedules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::RunBackup)?;
    let backup_engine = BackupEngine::new(state.database.clone(), state.staging_dir.clone());
    let scheduler = SchedulerService::new(state.database.clone(), backup_engine);
    let exporter = CommandConvexExporter::for_data_dir(&state.data_dir);
    Ok(Json(serde_json::json!({
        "runs": scheduler.run_due_once(&exporter).await?
    })))
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(
        serde_json::json!({ "runs": state.database.list_runs()? }),
    ))
}

async fn verify_backup_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(run_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::RunBackup)?;
    Ok(Json(serde_json::json!({
        "verification": verify_run(&state.database, run_id).await?
    })))
}

#[derive(Debug, Deserialize)]
struct RestoreRequest {
    run_id: Uuid,
    target_id: Uuid,
    confirm_deployment: String,
}

async fn restore_backup_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    let restore = RestoreEngine::new(state.database.clone());
    let importer = CommandConvexImporter::for_data_dir(&state.data_dir);
    Ok(Json(serde_json::json!({
        "restore": restore
            .restore_run_to_target(
                input.run_id,
                input.target_id,
                &input.confirm_deployment,
                &importer
            )
            .await?
    })))
}

async fn dr_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Authenticated)?;
    Ok(Json(serde_json::json!({
        "dr_report": generate_dr_report(&state.database)?
    })))
}

async fn list_audit_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&state, &headers, RoleRequirement::Manage)?;
    Ok(Json(serde_json::json!({
        "audit_events": state.database.list_audit_events(100)?
    })))
}

#[derive(Debug, Clone, Copy)]
enum RoleRequirement {
    Authenticated,
    Manage,
    RunBackup,
}

fn require_role(
    state: &AppState,
    headers: &HeaderMap,
    requirement: RoleRequirement,
) -> Result<User, ApiError> {
    let token =
        bearer_token(headers).ok_or_else(|| ApiError::unauthorized("missing bearer token"))?;
    let user = AuthService::new(state.database.clone())
        .authenticate_token(token)
        .map_err(|_| ApiError::unauthorized("invalid bearer token"))?;
    let allowed = match requirement {
        RoleRequirement::Authenticated => true,
        RoleRequirement::Manage => user.role.can_manage(),
        RoleRequirement::RunBackup => user.role.can_run_backup(),
    };
    if !allowed {
        return Err(ApiError::forbidden("insufficient role"));
    }
    Ok(user)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self::bad_request(error.into().to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.message
            })),
        )
            .into_response()
    }
}

fn default_data_dir() -> PathBuf {
    std::env::var("CONVEX_AUTOBACKUP_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("convex-autobackup")
        })
}
