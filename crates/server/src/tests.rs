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
