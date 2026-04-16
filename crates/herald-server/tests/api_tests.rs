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
        "text": "HELLO WORLD",
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
        .clone()
        .oneshot(
            authed_request("POST", "/api/messages")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Neither text nor grid → 400
    let body = json!({ "h_align": "center" });
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
        "text": "TEST MESSAGE",
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
        "text": "MSG ONE",
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
    let msg_body = json!({ "text": "TEST" });
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

// ── WebSocket ────────────────────────────────────────────────────

#[tokio::test]
async fn ws_route_exists_and_is_public() {
    let (app, db_path) = test_app().await;

    // GET /ws without upgrade headers reaches the WS handler (not auth-blocked)
    let response = app
        .oneshot(Request::builder().uri("/ws").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Route exists (not 404) and no auth required (not 401)
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    cleanup(&db_path);
}

#[tokio::test]
async fn ws_upgrade_succeeds() {
    let (app, db_path) = test_app().await;

    // Start a real TCP server so the WebSocket handshake can complete
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect with a real WebSocket client
    let url = format!("ws://{addr}/ws");
    let (ws_stream, response) = tokio_tungstenite::connect_async(&url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    // Clean shutdown
    drop(ws_stream);
    cleanup(&db_path);
}

#[tokio::test]
async fn ws_broadcast_delivers_message() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect a WS client
    let url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    // Broadcast a board update through the channel
    let board_state = herald_common::BoardState::default();
    let msg = herald_common::ServerMessage::BoardUpdate(board_state);
    state.broadcast_tx().send(msg).unwrap();

    // Client should receive the message
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for WS message")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "board_update");
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    // Clean up the second db created by this test's state
    cleanup(&db_path);
}

// ── WS broadcast integration tests ──────────────────────────────

#[tokio::test]
async fn ws_broadcast_on_message_create() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect a WS client
    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // Drain the initial board state message sent on connect
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next()).await;

    // Create a message via HTTP
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/messages"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "text": "BROADCAST",
            "h_align": "center",
            "v_align": "middle"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    // WS client should receive a board_update broadcast
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for broadcast")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "board_update");
        assert!(parsed["grid"].is_array());
        assert!(parsed["current_item"].is_object());
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    cleanup(&db_path);
}

#[tokio::test]
async fn ws_broadcast_on_message_delete() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // Drain the initial board state message sent on connect
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next()).await;

    // Create a message
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/messages"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "text": "DELETE ME",
            "h_align": "center",
            "v_align": "middle"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let msg: serde_json::Value = resp.json().await.unwrap();
    let msg_id = msg["id"].as_str().unwrap().to_string();

    // Drain the create broadcast (BoardUpdate + QueueInfo)
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next()).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next()).await;

    // Delete the message
    let resp = client
        .delete(format!("http://{addr}/api/messages/{msg_id}"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::NO_CONTENT);

    // WS client should receive a board_update with empty board
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for delete broadcast")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "board_update");
        // Queue is now empty, so current_item should be null
        assert!(parsed["current_item"].is_null());
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    cleanup(&db_path);
}

#[tokio::test]
async fn ws_broadcast_on_countdown_create() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // Drain the initial board state message sent on connect
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next()).await;

    // Create a countdown via HTTP
    let client = reqwest::Client::new();
    let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
    let resp = client
        .post(format!("http://{addr}/api/countdowns"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "label": "Launch",
            "target": future_time.to_rfc3339()
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    // WS client should receive a board_update
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for broadcast")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "board_update");
        assert_eq!(parsed["current_item"]["kind"], "countdown");
        assert_eq!(parsed["current_item"]["label"], "Launch");
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    cleanup(&db_path);
}

// ── WS initial board state tests ────────────────────────────────

#[tokio::test]
async fn ws_initial_state_empty_board() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Connect WS client to a fresh (empty) database
    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // First message should be the initial board state
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for initial board state")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify message format has all expected fields
        assert_eq!(parsed["type"], "board_update");
        assert!(parsed["grid"].is_array(), "missing 'grid' field");
        assert!(
            parsed.get("previous_grid").is_some(),
            "missing 'previous_grid' field"
        );
        assert!(
            parsed.get("timestamp").is_some(),
            "missing 'timestamp' field"
        );

        // Empty board: no queue items (field omitted by skip_serializing_if or null)
        assert!(
            parsed.get("current_item").is_none() || parsed["current_item"].is_null(),
            "expected current_item to be absent or null on empty board"
        );

        // Grid should be 6 rows × 22 columns of blank cells
        let grid = parsed["grid"].as_array().unwrap();
        assert_eq!(grid.len(), 6, "grid should have 6 rows");
        for row in grid {
            let cols = row.as_array().unwrap();
            assert_eq!(cols.len(), 22, "each row should have 22 columns");
        }
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    cleanup(&db_path);
}

#[tokio::test]
async fn ws_initial_state_with_existing_message() {
    use futures_util::StreamExt;

    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Create a message via HTTP BEFORE connecting the WS client
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/messages"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "text": "INITIAL",
            "h_align": "center",
            "v_align": "middle"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    // Now connect WS client — it should receive the current board state
    let ws_url = format!("ws://{addr}/ws");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // First message should be board_update with the existing message
    let received = tokio::time::timeout(std::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for initial board state")
        .expect("stream ended")
        .expect("WS error");

    if let tokio_tungstenite::tungstenite::Message::Text(text) = received {
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(parsed["type"], "board_update");
        assert!(parsed["grid"].is_array());

        // current_item should be populated with the message we created
        assert!(
            parsed["current_item"].is_object(),
            "expected current_item to be populated"
        );
        assert_eq!(parsed["current_item"]["kind"], "message");

        // Grid should contain non-blank content (the message we posted)
        let grid = parsed["grid"].as_array().unwrap();
        let has_non_blank = grid.iter().any(|row| {
            row.as_array().unwrap().iter().any(|cell| {
                // A non-blank cell is anything other than a default blank
                cell != &serde_json::json!({"Blank": null})
                    && cell != &serde_json::json!("Blank")
                    && cell != &serde_json::json!(null)
            })
        });
        assert!(
            has_non_blank,
            "grid should contain the message content, not be entirely blank"
        );
    } else {
        panic!("expected text message, got {:?}", received);
    }

    drop(ws_stream);
    cleanup(&db_path);
}

// ── Rotation & expiry tests ─────────────────────────────────────

#[tokio::test]
async fn rotation_advance_cycles_through_queue() {
    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Create 3 messages (A, B, C) via HTTP
    let client = reqwest::Client::new();
    let mut message_ids = Vec::new();
    for label in &["AAA", "BBB", "CCC"] {
        let resp = client
            .post(format!("http://{addr}/api/messages"))
            .header("authorization", format!("Bearer {TEST_TOKEN}"))
            .json(&serde_json::json!({
                "text": label,
                "h_align": "center",
                "v_align": "middle"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
        let body: serde_json::Value = resp.json().await.unwrap();
        message_ids.push(body["id"].as_str().unwrap().to_string());
    }

    let pool = state.pool();

    // Initial board state should show the first message (index 0)
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    let item = board
        .current_item
        .as_ref()
        .expect("should have a current item");
    assert_eq!(item.id.to_string(), message_ids[0]);

    // Advance → should show message B
    herald_server::db::advance_to_next_valid_item(pool)
        .await
        .unwrap();
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    let item = board
        .current_item
        .as_ref()
        .expect("should have a current item");
    assert_eq!(item.id.to_string(), message_ids[1]);

    // Advance → should show message C
    herald_server::db::advance_to_next_valid_item(pool)
        .await
        .unwrap();
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    let item = board
        .current_item
        .as_ref()
        .expect("should have a current item");
    assert_eq!(item.id.to_string(), message_ids[2]);

    // Advance again → should wrap back to message A
    herald_server::db::advance_to_next_valid_item(pool)
        .await
        .unwrap();
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    let item = board
        .current_item
        .as_ref()
        .expect("should have a current item");
    assert_eq!(item.id.to_string(), message_ids[0]);

    cleanup(&db_path);
}

#[tokio::test]
async fn rotation_skips_and_deletes_expired_messages() {
    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Create a normal (non-expiring) message
    let resp = client
        .post(format!("http://{addr}/api/messages"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "text": "KEEP",
            "h_align": "center",
            "v_align": "middle"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let keep_id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create an already-expired message
    let resp = client
        .post(format!("http://{addr}/api/messages"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "text": "EXPIRED",
            "h_align": "center",
            "v_align": "middle",
            "expires_at": "2020-01-01T00:00:00Z"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    let pool = state.pool();

    // Before cleanup, queue should have 2 items
    let queue = herald_server::db::get_queue(pool).await.unwrap();
    assert_eq!(queue.len(), 2);

    // advance_to_next_valid_item deletes expired messages
    let deleted = herald_server::db::advance_to_next_valid_item(pool)
        .await
        .unwrap();
    assert!(
        deleted >= 1,
        "should have deleted at least 1 expired message"
    );

    // Queue should now have only the non-expired message
    let queue = herald_server::db::get_queue(pool).await.unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id.to_string(), keep_id);

    // Board state should show the kept message
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    let item = board
        .current_item
        .as_ref()
        .expect("should have a current item");
    assert_eq!(item.id.to_string(), keep_id);

    cleanup(&db_path);
}

#[tokio::test]
async fn rotation_all_expired_results_in_empty_board() {
    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Create 2 already-expired messages
    for label in &["GONE1", "GONE2"] {
        let resp = client
            .post(format!("http://{addr}/api/messages"))
            .header("authorization", format!("Bearer {TEST_TOKEN}"))
            .json(&serde_json::json!({
                "text": label,
                "h_align": "center",
                "v_align": "middle",
                "expires_at": "2020-01-01T00:00:00Z"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    }

    let pool = state.pool();

    // Before cleanup, queue should have 2 items
    let queue = herald_server::db::get_queue(pool).await.unwrap();
    assert_eq!(queue.len(), 2);

    // advance_to_next_valid_item should delete both expired messages
    let deleted = herald_server::db::advance_to_next_valid_item(pool)
        .await
        .unwrap();
    assert_eq!(deleted, 2, "should have deleted 2 expired messages");

    // Queue should be empty
    let queue = herald_server::db::get_queue(pool).await.unwrap();
    assert!(
        queue.is_empty(),
        "queue should be empty after all messages expired"
    );

    // Board state should be default (no current_item)
    let board = herald_server::db::build_board_state(pool).await.unwrap();
    assert!(
        board.current_item.is_none(),
        "current_item should be None when all messages are expired"
    );

    cleanup(&db_path);
}

// ── Countdown rendering tests ───────────────────────────────────

#[tokio::test]
async fn countdown_renders_on_board_state() {
    let db_path = format!(
        "herald_test_{}.db",
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let database_url = format!("sqlite:{db_path}");
    let pool = herald_server::db::init_pool(&database_url).await.unwrap();
    let state = herald_server::state::AppState::new(pool, TEST_TOKEN.to_string());
    let app = herald_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Create a countdown 1 hour in the future
    let client = reqwest::Client::new();
    let future_time = chrono::Utc::now() + chrono::Duration::hours(1);
    let resp = client
        .post(format!("http://{addr}/api/countdowns"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .json(&serde_json::json!({
            "label": "LAUNCH",
            "target": future_time.to_rfc3339()
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    // Build the board state — should have the countdown rendered (not blank)
    let board_state = herald_server::db::build_board_state(state.pool())
        .await
        .unwrap();

    // current_item should be the countdown
    let item = board_state.current_item.unwrap();
    assert_eq!(item.kind, herald_common::QueueItemKind::Countdown);
    assert_eq!(item.label, "LAUNCH");

    // Grid should NOT be entirely blank (countdown renders label + time)
    let has_content = board_state.grid.0.iter().any(|row| {
        row.iter()
            .any(|cell| *cell != herald_common::CellContent::Blank)
    });
    assert!(
        has_content,
        "countdown grid should have rendered content, not be blank"
    );

    // Row 0 should contain the label "LAUNCH"
    let row0_chars: String = board_state.grid.0[0]
        .iter()
        .filter_map(|c| match c {
            herald_common::CellContent::Char(ch) => Some(*ch),
            _ => None,
        })
        .collect();
    assert!(
        row0_chars.contains("LAUNCH"),
        "row 0 should contain the label LAUNCH, got: {row0_chars}"
    );

    // Row 3 should contain time digits (at least "00" from hours/minutes/seconds)
    let row3_chars: String = board_state.grid.0[3]
        .iter()
        .filter_map(|c| match c {
            herald_common::CellContent::Char(ch) => Some(*ch),
            _ => None,
        })
        .collect();
    assert!(
        !row3_chars.is_empty(),
        "row 3 should contain formatted time"
    );

    // Rows 2 and 5 should be blank (separator rows)
    for col in 0..herald_common::BOARD_COLS {
        assert_eq!(
            board_state.grid.0[2][col],
            herald_common::CellContent::Blank
        );
        assert_eq!(
            board_state.grid.0[5][col],
            herald_common::CellContent::Blank
        );
    }

    cleanup(&db_path);
}
