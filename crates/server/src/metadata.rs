use axum::Json;

pub(crate) async fn openapi_spec() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "ConvexAutoBackup API", "version": env!("CARGO_PKG_VERSION")},
        "paths": {
            "/api/v1/health": {"get": {"summary": "Service health"}},
            "/api/v1/bootstrap": {"post": {"summary": "Create first owner"}},
            "/api/v1/login": {"post": {"summary": "Create API token with email/password"}},
            "/api/v1/users": {"get": {"summary": "List users"}, "post": {"summary": "Create user"}},
            "/api/v1/tokens": {"get": {"summary": "List API token metadata"}, "post": {"summary": "Create API token"}},
            "/api/v1/tokens/{token_id}": {"delete": {"summary": "Revoke API token"}},
            "/api/v1/secrets": {"get": {"summary": "List encrypted secret metadata"}, "post": {"summary": "Store encrypted secret"}},
            "/api/v1/projects": {"get": {"summary": "List projects"}, "post": {"summary": "Create project"}},
            "/api/v1/targets": {"get": {"summary": "List Convex targets"}},
            "/api/v1/targets/cloud": {"post": {"summary": "Create Convex Cloud target"}},
            "/api/v1/destinations": {"get": {"summary": "List backup destinations"}},
            "/api/v1/destinations/local": {"post": {"summary": "Create local filesystem destination"}},
            "/api/v1/destinations/s3": {"post": {"summary": "Create S3-compatible destination"}},
            "/api/v1/jobs": {"get": {"summary": "List backup jobs"}, "post": {"summary": "Create backup job"}},
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

pub(crate) async fn capabilities() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "convex_targets": ["cloud", "self_hosted"],
        "storage_destinations": ["local_filesystem", "s3_compatible"],
        "schedule_modes": ["interval_minutes", "daily", "weekly", "cron"],
        "backup_defaults": {"include_file_storage": true, "missed_run_policy": "run_once_on_resume"},
        "agent_interfaces": ["cli_json", "http_api", "mcp_stdio"],
        "database_modes": ["sqlite"],
        "secret_backends": ["encrypted_database", "environment_reference"],
        "implemented_api_resources": ["bootstrap", "login", "users", "tokens", "secrets", "projects", "targets", "destinations", "jobs", "schedules", "runs", "restore", "audit", "dr_report"]
    }))
}
