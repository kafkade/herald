use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio::sync::broadcast;

use crate::state::AppState;

/// Handle WebSocket upgrade request.
/// This endpoint is unauthenticated — any viewer can connect.
pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

/// Manage a single WebSocket connection lifecycle.
async fn handle_connection(mut socket: WebSocket, state: AppState) {
    let viewer_count = state.add_viewer();
    tracing::info!("WebSocket client connected (viewers: {viewer_count})");

    let mut broadcast_rx = state.subscribe_broadcast();

    loop {
        tokio::select! {
            // Forward broadcast messages to this client
            msg = broadcast_rx.recv() => {
                match msg {
                    Ok(server_msg) => {
                        match serde_json::to_string(&server_msg) {
                            Ok(text) => {
                                if socket.send(Message::Text(text.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => tracing::error!("Serialize error: {e}"),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Viewer lagged, skipped {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Process incoming client messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(herald_common::ClientMessage::Pong) = serde_json::from_str(&text) {
                            tracing::trace!("Received pong from viewer");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::debug!("WebSocket receive error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let viewer_count = state.remove_viewer();
    tracing::info!("WebSocket client disconnected (viewers: {viewer_count})");
}
