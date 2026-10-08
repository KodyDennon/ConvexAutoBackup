use super::{AppState, router_with_state};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use convex_autobackup_core::AppDatabase;
use tower::ServiceExt;
use uuid::Uuid;

fn test_router() -> Router {
    let dir = std::env::temp_dir().join(format!("convex-autobackup-test-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = AppState {
        version: env!("CARGO_PKG_VERSION"),
        data_dir: dir.clone(),
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

#[tokio::test]
async fn login_returns_token_for_valid_password() {
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

    let login_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "email": "owner@example.com",
                        "password": "very-secure-password"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json["api_token"]["token"]
            .as_str()
            .unwrap()
            .starts_with("cab_")
    );
}

fn test_master_key() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // SAFETY: set once before any test reads it; the value never changes.
        unsafe {
            std::env::set_var(
                "CONVEX_AUTOBACKUP_MASTER_KEY",
                "test-master-key-test-master-key-0123",
            )
        };
    });
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => request.body(Body::empty()).unwrap(),
    };
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test]
async fn destination_encryption_and_probe_api_flow() {
    test_master_key();
    let app = test_router();
    let (status, bootstrap) = call(
        &app,
        "POST",
        "/api/v1/bootstrap",
        None,
        Some(serde_json::json!({
            "email": "owner@example.com",
            "password": "very-secure-password",
            "role": "owner"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = bootstrap["api_token"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let root = std::env::temp_dir().join(format!("convex-autobackup-dest-{}", Uuid::now_v7()));

    let (status, body) = call(
        &app,
        "POST",
        "/api/v1/destinations/local",
        Some(&token),
        Some(serde_json::json!({
            "name": "Encrypted local",
            "root": root.to_string_lossy(),
            "encryption_passphrase": "short"
        })),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::OK,
        "short passphrase must be rejected: {body}"
    );

    let (status, body) = call(
        &app,
        "POST",
        "/api/v1/destinations/local",
        Some(&token),
        Some(serde_json::json!({
            "name": "Encrypted local",
            "root": root.to_string_lossy(),
            "encryption_passphrase": "correct horse battery staple"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let destination = &body["destination"];
    assert!(destination["encryption"]["passphrase"]["key_ref"]["id"].is_string());
    let destination_id = destination["id"].as_str().unwrap().to_string();
    let key_id = destination["encryption"]["passphrase"]["key_ref"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = call(
        &app,
        "POST",
        &format!("/api/v1/destinations/{destination_id}/test"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true, "{body}");

    let (status, _) = call(
        &app,
        "DELETE",
        &format!("/api/v1/secrets/{key_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_ne!(
        status,
        StatusCode::OK,
        "in-use passphrase secret must not be deletable"
    );

    let (status, body) = call(
        &app,
        "PUT",
        &format!("/api/v1/destinations/{destination_id}/encryption"),
        Some(&token),
        Some(serde_json::json!({ "passphrase": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["destination"]["encryption"], "disabled");

    let (status, body) = call(
        &app,
        "POST",
        "/api/v1/destinations/s3",
        Some(&token),
        Some(serde_json::json!({
            "name": "R2",
            "bucket": "b",
            "endpoint": "https://acct.r2.cloudflarestorage.com",
            "region": "auto",
            "access_key_id": "id"
        })),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::OK,
        "half-specified credentials must be rejected: {body}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn cross_origin_requests_are_not_allowed() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/bootstrap")
                .header(header::ORIGIN, "https://evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "API must not grant CORS to other origins"
    );
}
