mod components;
mod ws;

use components::Board;
use leptos::prelude::*;
use ws::WebSocketState;

/// Root application component.
#[component]
fn App() -> impl IntoView {
    let ws_state = ws::use_websocket();

    view! {
        <div class="herald-board" role="img" aria-label="Herald message board">
            <Board ws_state=ws_state.clone() />
            <StatusBar ws_state=ws_state />
        </div>
    }
}

/// Connection status bar below the board.
#[component]
fn StatusBar(ws_state: WebSocketState) -> impl IntoView {
    let status_class = move || match ws_state.connected.get() {
        true => "status-indicator connected",
        false => "status-indicator connecting",
    };

    let status_text = move || {
        if ws_state.connected.get() {
            "Connected".to_string()
        } else {
            "Reconnecting...".to_string()
        }
    };

    view! {
        <div class="board-status">
            <span class=status_class></span>
            <span class="status-text">{status_text}</span>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    log::info!("Herald web viewer starting...");
    leptos::mount::mount_to_body(App);
}
