mod admin;
mod components;
mod ws;

use admin::AdminPanel;
use components::{Board, SoundEngine};
use herald_common::ThemeKind;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use ws::WebSocketState;

/// Simple client-side path tracking without leptos_router.
fn use_location_path() -> ReadSignal<String> {
    let (path, set_path) = signal(current_path());

    // Listen for popstate events (back/forward navigation)
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
        set_path.set(current_path());
    }) as Box<dyn Fn(_)>);

    web_sys::window()
        .unwrap()
        .add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    path
}

fn current_path() -> String {
    web_sys::window()
        .unwrap()
        .location()
        .pathname()
        .unwrap_or_else(|_| "/".to_string())
}

/// Root application component with client-side routing.
#[component]
fn App() -> impl IntoView {
    let path = use_location_path();

    view! {
        {move || {
            if path.get().starts_with("/admin") {
                view! { <AdminPanel /> }.into_any()
            } else {
                view! { <BoardView /> }.into_any()
            }
        }}
    }
}

/// The board viewer page (existing functionality).
#[component]
fn BoardView() -> impl IntoView {
    let sound = SoundEngine::new();
    let ws_state = ws::use_websocket(sound.clone());

    let ws_for_theme = ws_state.clone();
    let ws_for_style = ws_state.clone();

    view! {
        <div
            class="herald-board"
            role="img"
            aria-label="Herald message board"
            attr:data-theme=move || theme_attr(&ws_for_theme)
            style=move || theme_custom_style(&ws_for_style)
        >
            <Board ws_state=ws_state.clone() />
        </div>
        <SoundToggle sound=sound />
        <StatusBar ws_state=ws_state />
    }
}

/// Map theme to data-theme attribute value.
fn theme_attr(ws: &WebSocketState) -> String {
    match ws.theme.get() {
        ThemeKind::Classic => "classic".to_string(),
        ThemeKind::Dark => "dark".to_string(),
        ThemeKind::Custom => "custom".to_string(),
    }
}

/// Generate inline CSS custom properties for custom theme colors.
fn theme_custom_style(ws: &WebSocketState) -> String {
    if ws.theme.get() != ThemeKind::Custom {
        return String::new();
    }

    let colors = ws.theme_colors.get();
    let colors = match colors.as_ref() {
        Some(c) => c,
        None => return String::new(),
    };

    let mut style = String::new();
    if let Some(bg) = &colors.board_bg {
        style.push_str(&format!("--board-bg:{bg};--body-bg:{bg};"));
    }
    if let Some(tile) = &colors.tile_bg {
        style.push_str(&format!("--tile-top-bg:{tile};--tile-bottom-bg:{tile};"));
    }
    if let Some(ch) = &colors.char_color {
        style.push_str(&format!("--tile-char-color:{ch};"));
    }
    style
}

/// Sound mute/unmute toggle button (bottom-left corner).
#[component]
fn SoundToggle(sound: SoundEngine) -> impl IntoView {
    if !sound.supported {
        return view! {}.into_any();
    }

    let on_click = {
        let sound = sound.clone();
        move |_| sound.toggle()
    };

    let icon = move || {
        if sound.enabled.get() {
            "\u{1F50A}" // 🔊
        } else {
            "\u{1F507}" // 🔇
        }
    };

    let label = move || {
        if sound.enabled.get() {
            "Mute flip sound"
        } else {
            "Enable flip sound"
        }
    };

    view! {
        <button
            class="sound-toggle"
            on:click=on_click
            aria-label=label
            title=label
        >
            {icon}
        </button>
    }
    .into_any()
}

/// Fixed-position connection status indicator (bottom-right).
/// Auto-hides 3 seconds after a successful connection.
#[component]
fn StatusBar(ws_state: WebSocketState) -> impl IntoView {
    let is_visible = RwSignal::new(true);

    // Auto-hide when connected for 3 seconds
    Effect::new(move |_| {
        let connected = ws_state.connected.get();
        let has_update = ws_state.has_received_update.get();
        if connected && has_update {
            set_timeout(
                move || {
                    if ws_state.connected.get_untracked() {
                        is_visible.set(false);
                    }
                },
                std::time::Duration::from_secs(3),
            );
        } else {
            is_visible.set(true);
        }
    });

    let container_class = move || {
        let mut classes = vec!["board-status"];
        if !is_visible.get() {
            classes.push("status-hidden");
        }
        classes.join(" ")
    };

    let indicator_class = move || {
        if !ws_state.has_received_update.get() {
            "status-indicator connecting"
        } else if ws_state.connected.get() {
            "status-indicator connected"
        } else {
            "status-indicator disconnected"
        }
    };

    let status_text = move || {
        if !ws_state.has_received_update.get() {
            "Connecting...".to_string()
        } else if ws_state.connected.get() {
            "Connected".to_string()
        } else {
            "Reconnecting...".to_string()
        }
    };

    view! {
        <div class=container_class>
            <span class=indicator_class></span>
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
