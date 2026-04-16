use futures::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use gloo_timers::future::TimeoutFuture;
use herald_common::{BOARD_COLS, BOARD_ROWS, BoardState, CellContent, ServerMessage};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// State exposed from the WebSocket connection to UI components.
#[derive(Clone)]
pub struct WebSocketState {
    /// Current grid — 6×22 grid of signals, one per cell
    pub grid: Vec<Vec<RwSignal<CellContent>>>,
    /// Previous grid for animation diffing
    pub previous_grid: Vec<Vec<RwSignal<CellContent>>>,
    /// Whether we're currently connected
    pub connected: RwSignal<bool>,
    /// Trigger signal that increments on each board update (used to start animations)
    pub update_counter: RwSignal<u64>,
}

/// Derive the WebSocket URL from the current page location.
fn ws_url() -> String {
    let window = web_sys::window().expect("no window");
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".into());
    let host = location.host().unwrap_or_else(|_| "localhost:3000".into());
    let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
    format!("{ws_protocol}//{host}/ws")
}

/// Create and manage a WebSocket connection with reconnection logic.
/// Returns reactive state that updates when board state changes.
pub fn use_websocket() -> WebSocketState {
    let grid: Vec<Vec<RwSignal<CellContent>>> = (0..BOARD_ROWS)
        .map(|_| {
            (0..BOARD_COLS)
                .map(|_| RwSignal::new(CellContent::Blank))
                .collect()
        })
        .collect();

    let previous_grid: Vec<Vec<RwSignal<CellContent>>> = (0..BOARD_ROWS)
        .map(|_| {
            (0..BOARD_COLS)
                .map(|_| RwSignal::new(CellContent::Blank))
                .collect()
        })
        .collect();

    let connected = RwSignal::new(false);
    let update_counter = RwSignal::new(0u64);

    let state = WebSocketState {
        grid,
        previous_grid,
        connected,
        update_counter,
    };

    let state_clone = state.clone();
    spawn_local(async move {
        connection_loop(state_clone).await;
    });

    state
}

/// Reconnection loop with exponential backoff.
async fn connection_loop(state: WebSocketState) {
    let mut attempt: u32 = 0;

    loop {
        let url = ws_url();
        log::info!("WebSocket connecting to {url} (attempt {attempt})...");

        match WebSocket::open(&url) {
            Ok(ws) => {
                attempt = 0;
                state.connected.set(true);
                log::info!("WebSocket connected");

                let (_write, mut read) = ws.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            handle_message(&text, &state);
                        }
                        Ok(Message::Bytes(bytes)) => {
                            if let Ok(text) = String::from_utf8(bytes) {
                                handle_message(&text, &state);
                            }
                        }
                        Err(e) => {
                            log::error!("WebSocket error: {e:?}");
                            break;
                        }
                    }
                }

                state.connected.set(false);
                log::warn!("WebSocket disconnected");
            }
            Err(e) => {
                log::error!("WebSocket connection failed: {e:?}");
                state.connected.set(false);
            }
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s max
        let delay_ms = std::cmp::min(1000 * 2u32.saturating_pow(attempt), 30_000);
        log::info!("Reconnecting in {delay_ms}ms...");
        TimeoutFuture::new(delay_ms).await;
        attempt += 1;
    }
}

/// Process a single incoming server message.
fn handle_message(text: &str, state: &WebSocketState) {
    match serde_json::from_str::<ServerMessage>(text) {
        Ok(ServerMessage::BoardUpdate(board_state)) => {
            apply_board_update(&board_state, state);
        }
        Ok(ServerMessage::Heartbeat { .. }) => {
            log::trace!("Heartbeat received");
        }
        Ok(ServerMessage::QueueInfo { .. }) => {
            // Could display in status bar — future enhancement
        }
        Ok(ServerMessage::Shutdown { reason }) => {
            log::warn!("Server shutting down: {reason}");
        }
        Ok(ServerMessage::Error { message }) => {
            log::error!("Server error: {message}");
        }
        Err(e) => {
            log::warn!("Failed to parse server message: {e}");
        }
    }
}

/// Apply a board update to the reactive grid signals.
fn apply_board_update(board_state: &BoardState, state: &WebSocketState) {
    // Copy current grid to previous_grid signals before updating
    for row in 0..BOARD_ROWS {
        for col in 0..BOARD_COLS {
            let current = state.grid[row][col].get_untracked();
            state.previous_grid[row][col].set(current);
        }
    }

    // Update grid signals — only cells that changed will trigger re-renders
    for row in 0..BOARD_ROWS {
        for col in 0..BOARD_COLS {
            let new_cell = board_state.grid.0[row][col];
            state.grid[row][col].set(new_cell);
        }
    }

    // Increment update counter to trigger animations
    state.update_counter.update(|c| *c += 1);
}
