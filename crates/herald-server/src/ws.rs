use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::broadcast;

use crate::state::AppState;

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

/// Handle WebSocket upgrade request.
/// This endpoint is unauthenticated — any viewer can connect.
pub async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

/// Manage a single WebSocket connection lifecycle.
async fn handle_connection(mut socket: WebSocket, state: AppState) {
    let client_id = state.next_client_id();
    let viewer_count = state.add_viewer();
    tracing::info!(
        client_id,
        viewers = viewer_count,
        "WebSocket client connected"
    );

    let mut broadcast_rx = state.subscribe_broadcast();

    // Send the current board state as the first message so the client
    // renders immediately without waiting for the next broadcast.
    match crate::db::build_board_state(state.pool()).await {
        Ok(board_state) => {
            let msg = herald_common::ServerMessage::BoardUpdate(board_state);
            match serde_json::to_string(&msg) {
                Ok(text) => {
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        let viewer_count = state.remove_viewer();
                        tracing::info!(
                            client_id,
                            viewers = viewer_count,
                            "WebSocket client disconnected during initial state send"
                        );
                        return;
                    }
                }
                Err(e) => {
                    tracing::error!(client_id, "Failed to serialize initial board state: {e}")
                }
            }
        }
        Err(e) => tracing::warn!(client_id, "Failed to build initial board state: {e}"),
    }

    // Heartbeat state
    let mut ping_interval = tokio::time::interval(PING_INTERVAL);
    ping_interval.tick().await; // consume the immediate first tick
    let mut awaiting_pong = false;
    let pong_deadline = tokio::time::sleep(Duration::from_secs(86400));
    tokio::pin!(pong_deadline);

    loop {
        tokio::select! {
            // Ping interval fires
            _ = ping_interval.tick() => {
                if awaiting_pong {
                    tracing::warn!(client_id, "Client failed to respond to ping, closing connection");
                    break;
                }
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
                awaiting_pong = true;
                pong_deadline.as_mut().reset(tokio::time::Instant::now() + PONG_TIMEOUT);
            }

            // Pong timeout (only active when awaiting_pong is true)
            _ = &mut pong_deadline, if awaiting_pong => {
                tracing::warn!(client_id, "Pong timeout after {}s, closing connection", PONG_TIMEOUT.as_secs());
                break;
            }

            // Forward broadcast messages to this client
            msg = broadcast_rx.recv() => {
                match msg {
                    Ok(json) => {
                        if socket.send(Message::Text(json.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(client_id, "Viewer lagged, skipped {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Process incoming client messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        awaiting_pong = false;
                        pong_deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(86400));
                        tracing::trace!(client_id, "Received pong");
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(herald_common::ClientMessage::Pong) = serde_json::from_str(&text) {
                            tracing::trace!(client_id, "Received application-level pong");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::debug!(client_id, "WebSocket receive error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let viewer_count = state.remove_viewer();
    tracing::info!(
        client_id,
        viewers = viewer_count,
        "WebSocket client disconnected"
    );
}
