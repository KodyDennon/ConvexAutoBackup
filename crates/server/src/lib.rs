use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use convex_autobackup_core::{
    AppDatabase, AuthService, BackupEngine, CommandConvexExporter, CommandConvexImporter,
    CreateCloudTarget, CreateJobSchedule, CreateLocalDestination, CreateProject,
    CreateS3Destination, CreateScheduledJob, CreateUser, RestoreEngine, Role, SchedulerService,
    SecretKind, SecretVault, User, generate_dr_report, verify_run,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

#[derive(Debug, Clone)]
pub struct AppState {
    pub version: &'static str,
    pub database: AppDatabase,
    pub staging_dir: PathBuf,
}

impl AppState {
    pub fn open_default() -> anyhow::Result<Self> {
        let data_dir = default_data_dir();
        Ok(Self {
            version: env!("CARGO_PKG_VERSION"),
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
        .route("/api/v1/tokens", post(create_api_token))
        .route("/api/v1/secrets", get(list_secrets).post(put_secret))
        .route("/api/v1/projects", get(list_projects).post(create_project))
        .route("/api/v1/targets/cloud", post(create_cloud_target))
        .route("/api/v1/destinations/local", post(create_local_destination))
        .route("/api/v1/destinations/s3", post(create_s3_destination))
        .route("/api/v1/jobs", post(create_job))
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

async fn openapi_spec() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "ConvexAutoBackup API", "version": env!("CARGO_PKG_VERSION")},
        "paths": {
            "/api/v1/health": {"get": {"summary": "Service health"}},
            "/api/v1/bootstrap": {"post": {"summary": "Create first owner"}},
            "/api/v1/tokens": {"post": {"summary": "Create API token"}},
            "/api/v1/secrets": {"get": {"summary": "List encrypted secret metadata"}, "post": {"summary": "Store encrypted secret"}},
            "/api/v1/projects": {"get": {"summary": "List projects"}, "post": {"summary": "Create project"}},
            "/api/v1/targets/cloud": {"post": {"summary": "Create Convex Cloud target"}},
            "/api/v1/destinations/local": {"post": {"summary": "Create local filesystem destination"}},
            "/api/v1/destinations/s3": {"post": {"summary": "Create S3-compatible destination"}},
            "/api/v1/jobs": {"post": {"summary": "Create backup job"}},
            "/api/v1/schedules": {"get": {"summary": "List schedules"}, "post": {"summary": "Create schedule"}},
            "/api/v1/schedules/run-due": {"post": {"summary": "Run due schedules"}},
            "/api/v1/runs/{run_id}/verify": {"post": {"summary": "Verify backup run"}},
            "/api/v1/restore": {"post": {"summary": "Restore verified backup run"}},
            "/api/v1/dr/report": {"get": {"summary": "Generate DR evidence report"}},
            "/api/v1/audit": {"get": {"summary": "List audit events"}},
            "/api/v1/jobs/{job_id}/run": {"post": {"summary": "Run backup job"}},
            "/api/v1/runs": {"get": {"summary": "List backup runs"}}
        }
    }))
}

async fn capabilities() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "convex_targets": ["cloud", "self_hosted"],
        "storage_destinations": ["local_filesystem", "s3_compatible"],
        "schedule_modes": ["interval_minutes", "daily", "weekly", "cron"],
        "backup_defaults": {"include_file_storage": true, "missed_run_policy": "run_once_on_resume"},
        "agent_interfaces": ["cli_json", "http_api", "mcp_stdio"],
        "database_modes": ["sqlite"],
        "secret_backends": ["encrypted_database", "environment_reference"],
        "implemented_api_resources": ["bootstrap", "tokens", "secrets", "projects", "targets", "destinations", "jobs", "schedules", "runs"]
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
    let vault = SecretVault::from_env(state.database.clone())?;
    Ok(Json(
        serde_json::json!({ "secrets": vault.list_secrets()? }),
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
    let exporter = CommandConvexExporter::default();
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
    let exporter = CommandConvexExporter::default();
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
    let importer = CommandConvexImporter::default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use tower::ServiceExt;

    fn test_router() -> Router {
        let dir = std::env::temp_dir().join(format!("convex-autobackup-test-{}", Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let state = AppState {
            version: env!("CARGO_PKG_VERSION"),
            database: AppDatabase::open(dir.join("app.db")).unwrap(),
            staging_dir: dir.join("staging"),
        };
        router_with_state(state)
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok_payload() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn projects_require_bearer_token_after_bootstrap() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bootstrap_token_and_project_api_flow() {
        let app = test_router();
        let bootstrap_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/bootstrap")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "email": "owner@example.com",
                            "password": "very-secure-password",
                            "role": "viewer"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bootstrap_response.status(), StatusCode::OK);
        let bootstrap_body = axum::body::to_bytes(bootstrap_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let bootstrap_json: serde_json::Value = serde_json::from_slice(&bootstrap_body).unwrap();
        let user_id = bootstrap_json["user"]["id"].as_str().unwrap();
        let token = bootstrap_json["api_token"]["token"].as_str().unwrap();

        let token_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tokens")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "user_id": user_id,
                            "name": "api-test"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(token_response.status(), StatusCode::OK);

        let create_project_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "name": "API Project",
                            "description": "created in integration test"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_project_response.status(), StatusCode::OK);

        let list_projects_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/projects")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_projects_response.status(), StatusCode::OK);
    }
}
