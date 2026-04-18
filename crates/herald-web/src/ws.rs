use std::cell::RefCell;
use std::rc::Rc;

use crate::components::SoundEngine;
use futures::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use gloo_timers::future::TimeoutFuture;
use herald_common::{BOARD_COLS, BOARD_ROWS, BoardState, CellContent, ServerMessage, ThemeKind};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
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
    /// Whether we've received at least one board update
    pub has_received_update: RwSignal<bool>,
    /// Trigger signal that increments on each board update (used to start animations)
    pub update_counter: RwSignal<u64>,
    /// Columns that changed in the most recent board update (for sound effects)
    pub changed_cols: RwSignal<Vec<usize>>,
    /// Current board theme
    pub theme: RwSignal<ThemeKind>,
    /// Custom theme colors (only populated when theme is Custom)
    pub theme_colors: RwSignal<Option<herald_common::ThemeColors>>,
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
pub fn use_websocket(sound: SoundEngine) -> WebSocketState {
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
    let has_received_update = RwSignal::new(false);
    let update_counter = RwSignal::new(0u64);
    let changed_cols = RwSignal::new(Vec::new());
    let theme = RwSignal::new(ThemeKind::default());
    let theme_colors = RwSignal::new(None);

    let state = WebSocketState {
        grid,
        previous_grid,
        connected,
        has_received_update,
        update_counter,
        changed_cols,
        theme,
        theme_colors,
    };

    let state_clone = state.clone();
    let batcher = RafBatcher::new(state_clone, sound);
    spawn_local(async move {
        connection_loop(batcher).await;
    });

    state
}

/// Batches board updates into a single `requestAnimationFrame` callback.
///
/// If multiple `BoardUpdate` messages arrive within the same animation frame,
/// only the latest state is applied — earlier intermediate states are dropped.
/// This is app-lifetime and outlives reconnect cycles.
struct RafBatcher {
    pending: Rc<RefCell<Option<BoardState>>>,
    raf_scheduled: Rc<RefCell<bool>>,
    state: WebSocketState,
    sound: SoundEngine,
}

impl RafBatcher {
    fn new(state: WebSocketState, sound: SoundEngine) -> Self {
        Self {
            pending: Rc::new(RefCell::new(None)),
            raf_scheduled: Rc::new(RefCell::new(false)),
            state,
            sound,
        }
    }

    /// Queue a board update to be applied on the next animation frame.
    fn schedule_update(&self, board_state: BoardState) {
        *self.pending.borrow_mut() = Some(board_state);

        if !*self.raf_scheduled.borrow() {
            *self.raf_scheduled.borrow_mut() = true;

            let pending = Rc::clone(&self.pending);
            let raf_scheduled = Rc::clone(&self.raf_scheduled);
            let state = self.state.clone();
            let sound = self.sound.clone();

            let closure = Closure::once(move || {
                *raf_scheduled.borrow_mut() = false;
                if let Some(board_state) = pending.borrow_mut().take() {
                    apply_board_update(&board_state, &state);
                    // Trigger sound for changed columns
                    let cols = state.changed_cols.get_untracked();
                    if !cols.is_empty() {
                        sound.play_cascade(&cols);
                    }
                }
            });

            web_sys::window()
                .expect("no window")
                .request_animation_frame(closure.as_ref().unchecked_ref())
                .expect("requestAnimationFrame failed");

            closure.forget();
        }
    }
}

/// Reconnection loop with exponential backoff.
/// The `RafBatcher` is created once and reused across reconnect cycles.
async fn connection_loop(batcher: RafBatcher) {
    let mut attempt: u32 = 0;

    loop {
        let url = ws_url();
        log::info!("WebSocket connecting to {url} (attempt {attempt})...");

        match WebSocket::open(&url) {
            Ok(ws) => {
                attempt = 0;
                batcher.state.connected.set(true);
                log::info!("WebSocket connected");

                let (_write, mut read) = ws.split();

                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            handle_message(&text, &batcher);
                        }
                        Ok(Message::Bytes(bytes)) => {
                            if let Ok(text) = String::from_utf8(bytes) {
                                handle_message(&text, &batcher);
                            }
                        }
                        Err(e) => {
                            log::error!("WebSocket error: {e:?}");
                            break;
                        }
                    }
                }

                batcher.state.connected.set(false);
                log::warn!("WebSocket disconnected");
            }
            Err(e) => {
                log::error!("WebSocket connection failed: {e:?}");
                batcher.state.connected.set(false);
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
fn handle_message(text: &str, batcher: &RafBatcher) {
    match serde_json::from_str::<ServerMessage>(text) {
        Ok(ServerMessage::BoardUpdate(board_state)) => {
            batcher.schedule_update(board_state);
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
    // Compute changed columns for sound by comparing current client display with new grid.
    // Only compute if we've already received at least one update (suppress initial/reconnect).
    let already_received = state.has_received_update.get_untracked();
    if already_received {
        let mut cols = Vec::new();
        for col in 0..BOARD_COLS {
            for row in 0..BOARD_ROWS {
                let current = state.grid[row][col].get_untracked();
                if current != board_state.grid.0[row][col] {
                    cols.push(col);
                    break;
                }
            }
        }
        state.changed_cols.set(cols);
    } else {
        state.changed_cols.set(vec![]);
    }

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

    // Update theme
    state.theme.set(board_state.theme.clone());
    state.theme_colors.set(board_state.theme_colors.clone());

    // Mark that we've received at least one update
    if !state.has_received_update.get_untracked() {
        state.has_received_update.set(true);
    }

    // Increment update counter to trigger animations
    state.update_counter.update(|c| *c += 1);
}
