use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use herald_common::{BoardState, ServerMessage};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

const INITIAL_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 30;

pub struct WsClient {
    server_url: String,
    board_tx: watch::Sender<BoardState>,
}

impl WsClient {
    /// Create a new WsClient and return it along with a watch::Receiver
    /// that will be updated whenever a new BoardState arrives from the server.
    pub fn new(server_url: String) -> (Self, watch::Receiver<BoardState>) {
        let (board_tx, board_rx) = watch::channel(BoardState::default());
        let client = Self {
            server_url,
            board_tx,
        };
        (client, board_rx)
    }

    /// Run the connection loop. Connects to the server, receives messages,
    /// and reconnects with exponential backoff on disconnection.
    /// Stops automatically when all watch receivers are dropped.
    /// Should be spawned as a tokio task.
    pub async fn run(&self) {
        let mut backoff_secs = INITIAL_BACKOFF_SECS;

        loop {
            if self.board_tx.is_closed() {
                tracing::info!("All receivers dropped, stopping WS client");
                return;
            }

            match tokio_tungstenite::connect_async(&self.server_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to {}", self.server_url);
                    backoff_secs = INITIAL_BACKOFF_SECS;

                    let (mut sink, mut stream) = ws_stream.split();

                    loop {
                        match stream.next().await {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(ServerMessage::BoardUpdate(board_state)) => {
                                        if self.board_tx.send(board_state).is_err() {
                                            tracing::info!(
                                                "All receivers dropped, stopping WS client"
                                            );
                                            return;
                                        }
                                    }
                                    Ok(_other) => {
                                        tracing::trace!("Received non-board message, skipping");
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            "Failed to deserialize server message: {err}"
                                        );
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(data))) => {
                                if let Err(err) = sink.send(Message::Pong(data)).await {
                                    tracing::warn!("Failed to send pong: {err}");
                                    break;
                                }
                            }
                            Some(Ok(Message::Close(_))) => {
                                tracing::info!("Server closed connection to {}", self.server_url);
                                break;
                            }
                            Some(Ok(_)) => {
                                // Binary, Pong, Frame — ignore
                            }
                            Some(Err(err)) => {
                                tracing::warn!("WebSocket error: {err}");
                                break;
                            }
                            None => {
                                tracing::info!("WebSocket stream ended for {}", self.server_url);
                                break;
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Connection to {} failed: {err}. Retrying in {backoff_secs}s...",
                        self.server_url
                    );
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                    continue;
                }
            }

            // Brief pause before reconnecting after a clean disconnect.
            // Only reached after a successful connection that later disconnected.
            // The `continue` in the Err branch skips this to avoid double-sleeping.
            tokio::time::sleep(Duration::from_secs(INITIAL_BACKOFF_SECS)).await;
        }
    }
}
