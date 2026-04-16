use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use herald_common::{BoardState, ServerMessage};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

const INITIAL_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 30;

/// Rotation metadata for the status bar.
#[derive(Clone, Debug, Default)]
pub struct QueueInfoState {
    pub current_index: usize,
    pub total_items: usize,
    pub next_rotation_seconds: u32,
    pub is_countdown_active: bool,
}

/// Connection state visible to the UI layer.
#[derive(Clone, Debug)]
pub enum ConnectionState {
    /// Initial connection attempt in progress.
    Connecting,
    /// WebSocket connection is active.
    Connected,
    /// Connection lost, attempting to reconnect.
    Reconnecting { attempt: u32, next_retry_secs: u64 },
    /// Disconnected (all receivers dropped or permanently failed).
    Disconnected,
}

pub struct WsClient {
    server_url: String,
    board_tx: watch::Sender<BoardState>,
    conn_tx: watch::Sender<ConnectionState>,
    queue_tx: watch::Sender<QueueInfoState>,
}

impl WsClient {
    /// Create a new WsClient and return it along with watch receivers for
    /// board state updates and connection state changes.
    pub fn new(
        server_url: String,
    ) -> (
        Self,
        watch::Receiver<BoardState>,
        watch::Receiver<ConnectionState>,
        watch::Receiver<QueueInfoState>,
    ) {
        let (board_tx, board_rx) = watch::channel(BoardState::default());
        let (conn_tx, conn_rx) = watch::channel(ConnectionState::Connecting);
        let (queue_tx, queue_rx) = watch::channel(QueueInfoState::default());
        let client = Self {
            server_url,
            board_tx,
            conn_tx,
            queue_tx,
        };
        (client, board_rx, conn_rx, queue_rx)
    }

    /// Run the connection loop. Connects to the server, receives messages,
    /// and reconnects with exponential backoff on disconnection.
    /// Stops automatically when all watch receivers are dropped.
    /// Should be spawned as a tokio task.
    pub async fn run(&self) {
        let mut backoff_secs = INITIAL_BACKOFF_SECS;
        let mut attempt: u32 = 0;

        loop {
            if self.board_tx.is_closed() {
                let _ = self.conn_tx.send(ConnectionState::Disconnected);
                tracing::info!("All receivers dropped, stopping WS client");
                return;
            }

            if attempt == 0 {
                let _ = self.conn_tx.send(ConnectionState::Connecting);
            } else {
                let _ = self.conn_tx.send(ConnectionState::Reconnecting {
                    attempt,
                    next_retry_secs: backoff_secs,
                });
            }

            match tokio_tungstenite::connect_async(&self.server_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to {}", self.server_url);
                    let _ = self.conn_tx.send(ConnectionState::Connected);
                    backoff_secs = INITIAL_BACKOFF_SECS;
                    attempt = 0;

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
                                    Ok(ServerMessage::QueueInfo {
                                        current_index,
                                        total_items,
                                        next_rotation_seconds,
                                        is_countdown_active,
                                    }) => {
                                        let _ = self.queue_tx.send(QueueInfoState {
                                            current_index,
                                            total_items,
                                            next_rotation_seconds,
                                            is_countdown_active,
                                        });
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
                    attempt += 1;
                    tracing::warn!(
                        "Connection to {} failed: {err}. Retrying in {backoff_secs}s...",
                        self.server_url
                    );
                    let _ = self.conn_tx.send(ConnectionState::Reconnecting {
                        attempt,
                        next_retry_secs: backoff_secs,
                    });
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                    continue;
                }
            }

            // After a successful connection ends, signal reconnecting before the pause.
            attempt += 1;
            let _ = self.conn_tx.send(ConnectionState::Reconnecting {
                attempt,
                next_retry_secs: INITIAL_BACKOFF_SECS,
            });
            tokio::time::sleep(Duration::from_secs(INITIAL_BACKOFF_SECS)).await;
        }
    }
}
