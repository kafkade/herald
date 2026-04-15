use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use herald_server::{build_router, db, state::AppState};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TEST_TOKEN: &str = "test-secret-token";

/// Create a test app with a temp SQLite database.
async fn test_app() -> (axum::Router, String) {
    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = db::init_pool(&database_url).await.unwrap();
    let state = AppState::new(pool, TEST_TOKEN.to_string());
    let router = build_router(state);
    (router, db_path)
}

/// Clean up test database files.
fn cleanup(db_path: &str) {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{db_path}-wal"));
    let _ = std::fs::remove_file(format!("{db_path}-shm"));
}

/// Helper to make a JSON request with auth.
fn authed_request(method: &str, uri: &str) -> http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .header("content-type", "application/json")
}

/// Helper to extract JSON body from response.
async fn json_body(response: axum::response::Response) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Build a grid with text in the first row.
fn text_grid(text: &str) -> Value {
    let mut grid_data = Vec::new();
    for r in 0..6 {
        let mut row = Vec::new();
        for c in 0..22 {
            if r == 0 && c < text.len() {
                let ch = text.chars().nth(c).unwrap();
                row.push(json!({"type": "char", "value": ch.to_string()}));
            } else {
                row.push(json!({"type": "blank", "value": null}));
            }
        }
        grid_data.push(row);
    }
    json!(grid_data)
}

// ── Health ────────────────────────────────────────────────────────

#[tokio::test]
async fn health_check_no_auth() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].is_number());

    cleanup(&db_path);
}

// ── Auth ──────────────────────────────────────────────────────────

#[tokio::test]
async fn auth_rejects_missing_token() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/messages")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    cleanup(&db_path);
}

#[tokio::test]
async fn auth_rejects_wrong_token() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/messages")
                .header("authorization", "Bearer wrong-token")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    cleanup(&db_path);
}

// ── Messages CRUD ─────────────────────────────────────────────────

#[tokio::test]
async fn create_message_returns_201() {
    let (app, db_path) = test_app().await;

    let body = json!({
        "grid": text_grid("HELLO WORLD"),
        "h_align": "center",
        "v_align": "middle"
    });

    let response = app
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let msg = json_body(response).await;
    assert!(msg["id"].is_string());
    assert_eq!(msg["h_align"], "center");
    assert_eq!(msg["queue_position"], 0);

    cleanup(&db_path);
}

#[tokio::test]
async fn create_message_invalid_grid_returns_400() {
    let (app, db_path) = test_app().await;

    // Grid with wrong dimensions (only 3 rows)
    let bad_grid: Vec<Value> = (0..3)
        .map(|_| {
            (0..22)
                .map(|_| json!({"type": "blank", "value": null}))
                .collect::<Vec<_>>()
        })
        .map(|r| json!(r))
        .collect();

    let body = json!({ "grid": bad_grid });

    let response = app
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    cleanup(&db_path);
}

#[tokio::test]
async fn list_messages_empty() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            authed_request("GET", "/api/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["total"], 0);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);

    cleanup(&db_path);
}

#[tokio::test]
async fn message_crud_full_lifecycle() {
    let (app, db_path) = test_app().await;

    // CREATE
    let create_body = json!({
        "grid": text_grid("TEST MESSAGE"),
        "h_align": "left"
    });
    let response = app
        .clone()
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = json_body(response).await;
    let id = created["id"].as_str().unwrap();

    // GET by ID
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", &format!("/api/messages/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let fetched = json_body(response).await;
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["h_align"], "left");

    // UPDATE
    let update_body = json!({ "h_align": "right" });
    let response = app
        .clone()
        .oneshot(
            authed_request("PUT", &format!("/api/messages/{id}"))
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated = json_body(response).await;
    assert_eq!(updated["h_align"], "right");

    // LIST (should have 1)
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", "/api/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = json_body(response).await;
    assert_eq!(list["total"], 1);

    // DELETE
    let response = app
        .clone()
        .oneshot(
            authed_request("DELETE", &format!("/api/messages/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // GET after delete → 404
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", &format!("/api/messages/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup(&db_path);
}

#[tokio::test]
async fn get_nonexistent_message_returns_404() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            authed_request("GET", "/api/messages/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    cleanup(&db_path);
}

// ── Countdowns CRUD ───────────────────────────────────────────────

#[tokio::test]
async fn create_countdown_returns_201() {
    let (app, db_path) = test_app().await;

    let body = json!({
        "label": "NEW YEAR",
        "target": "2026-01-01T00:00:00Z"
    });

    let response = app
        .oneshot(
            authed_request("POST", "/api/countdowns")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let cd = json_body(response).await;
    assert_eq!(cd["label"], "NEW YEAR");
    assert!(cd["id"].is_string());
    assert_eq!(cd["zero_behavior"]["action"], "show_zero");

    cleanup(&db_path);
}

#[tokio::test]
async fn create_countdown_empty_label_returns_400() {
    let (app, db_path) = test_app().await;

    let body = json!({
        "label": "",
        "target": "2026-01-01T00:00:00Z"
    });

    let response = app
        .oneshot(
            authed_request("POST", "/api/countdowns")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    cleanup(&db_path);
}

#[tokio::test]
async fn countdown_crud_lifecycle() {
    let (app, db_path) = test_app().await;

    // CREATE
    let create_body = json!({
        "label": "LAUNCH",
        "target": "2026-06-15T12:00:00Z",
        "zero_behavior": {"action": "remove"}
    });
    let response = app
        .clone()
        .oneshot(
            authed_request("POST", "/api/countdowns")
                .body(Body::from(create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = json_body(response).await;
    let id = created["id"].as_str().unwrap();

    // UPDATE
    let update_body = json!({ "label": "LIFTOFF" });
    let response = app
        .clone()
        .oneshot(
            authed_request("PUT", &format!("/api/countdowns/{id}"))
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let updated = json_body(response).await;
    assert_eq!(updated["label"], "LIFTOFF");

    // LIST
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", "/api/countdowns")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = json_body(response).await;
    assert_eq!(list["total"], 1);

    // DELETE
    let response = app
        .clone()
        .oneshot(
            authed_request("DELETE", &format!("/api/countdowns/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    cleanup(&db_path);
}

// ── Queue ─────────────────────────────────────────────────────────

#[tokio::test]
async fn queue_list_empty() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            authed_request("GET", "/api/queue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["total"], 0);
    assert_eq!(body["current_index"], 0);

    cleanup(&db_path);
}

#[tokio::test]
async fn queue_merged_and_reorder() {
    let (app, db_path) = test_app().await;

    // Create a message
    let msg_body = json!({
        "grid": text_grid("MSG ONE"),
    });
    let response = app
        .clone()
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(msg_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let msg = json_body(response).await;
    let msg_id = msg["id"].as_str().unwrap().to_string();

    // Create a countdown
    let cd_body = json!({
        "label": "EVENT",
        "target": "2026-12-31T00:00:00Z"
    });
    let response = app
        .clone()
        .oneshot(
            authed_request("POST", "/api/countdowns")
                .body(Body::from(cd_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let cd = json_body(response).await;
    let cd_id = cd["id"].as_str().unwrap().to_string();

    // Queue should have 2 items
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", "/api/queue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let queue = json_body(response).await;
    assert_eq!(queue["total"], 2);

    // Reorder: countdown first, then message
    let reorder_body = json!({ "order": [cd_id, msg_id] });
    let response = app
        .clone()
        .oneshot(
            authed_request("PUT", "/api/queue/reorder")
                .body(Body::from(reorder_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let reordered = json_body(response).await;
    assert_eq!(reordered["items"][0]["id"], cd_id);
    assert_eq!(reordered["items"][1]["id"], msg_id);

    cleanup(&db_path);
}

#[tokio::test]
async fn queue_reorder_incomplete_ids_returns_400() {
    let (app, db_path) = test_app().await;

    // Create a message
    let msg_body = json!({ "grid": text_grid("TEST") });
    app.clone()
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(msg_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to reorder with empty list
    let reorder_body = json!({ "order": [] });
    let response = app
        .clone()
        .oneshot(
            authed_request("PUT", "/api/queue/reorder")
                .body(Body::from(reorder_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup(&db_path);
}

// ── Config ────────────────────────────────────────────────────────

#[tokio::test]
async fn config_get_returns_defaults() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            authed_request("GET", "/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let config = json_body(response).await;
    assert!(
        config["rotation_interval_seconds"].is_string()
            || config["rotation_interval_seconds"].is_number()
    );
    assert!(config["default_h_align"].is_string());

    cleanup(&db_path);
}

#[tokio::test]
async fn config_update_and_read_back() {
    let (app, db_path) = test_app().await;

    let update_body = json!({ "rotation_interval_seconds": "30" });
    let response = app
        .clone()
        .oneshot(
            authed_request("PUT", "/api/config")
                .body(Body::from(update_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let config = json_body(response).await;
    // Value may be stored as number or string depending on JSON parsing
    let val = &config["rotation_interval_seconds"];
    assert!(val == "30" || val == 30, "expected 30, got {val}");

    // Read back
    let response = app
        .clone()
        .oneshot(
            authed_request("GET", "/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let config = json_body(response).await;
    let val = &config["rotation_interval_seconds"];
    assert!(val == "30" || val == 30, "expected 30, got {val}");

    cleanup(&db_path);
}

#[tokio::test]
async fn config_update_empty_returns_400() {
    let (app, db_path) = test_app().await;

    let response = app
        .oneshot(
            authed_request("PUT", "/api/config")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cleanup(&db_path);
}
