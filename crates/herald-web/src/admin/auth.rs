use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Get the stored admin token from localStorage.
fn get_stored_token() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item("herald_admin_token")
        .ok()?
}

/// Store the admin token in localStorage.
fn store_token(token: &str) {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
    {
        let _ = storage.set_item("herald_admin_token", token);
    }
}

/// Clear the stored admin token.
pub fn clear_token() {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
    {
        let _ = storage.remove_item("herald_admin_token");
    }
}

/// The main admin panel component.
/// Shows login dialog if no token stored, otherwise shows the admin interface.
#[component]
pub fn AdminPanel() -> impl IntoView {
    let token = RwSignal::new(get_stored_token());
    let show_login = RwSignal::new(token.get_untracked().is_none());

    // Provide token to child components via context
    provide_context(token);
    provide_context(show_login);

    view! {
        <div class="admin-container">
            <header class="admin-header">
                <h1 class="admin-title">"Herald Admin"</h1>
                <ViewerBadge />
                <nav class="admin-nav">
                    <a href="/" class="admin-nav-link">"← Board"</a>
                    {move || {
                        if token.get().is_some() {
                            Some(view! {
                                <button class="admin-logout-btn"
                                    on:click=move |_| {
                                        clear_token();
                                        token.set(None);
                                        show_login.set(true);
                                    }
                                >"Logout"</button>
                            })
                        } else {
                            None
                        }
                    }}
                </nav>
            </header>

            {move || {
                if show_login.get() {
                    view! { <LoginDialog token=token show_login=show_login /> }.into_any()
                } else {
                    view! {
                        <div class="admin-panels">
                            <super::MessageComposer />
                            <super::CountdownManager />
                            <super::QueueManager />
                            <super::ConfigPanel />
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Login dialog component.
#[component]
fn LoginDialog(token: RwSignal<Option<String>>, show_login: RwSignal<bool>) -> impl IntoView {
    let input_value = RwSignal::new(String::new());
    let error_msg = RwSignal::new(Option::<String>::None);

    let do_submit = move || {
        let val = input_value.get_untracked();
        if val.trim().is_empty() {
            error_msg.set(Some("Token cannot be empty".to_string()));
            return;
        }
        store_token(val.trim());
        token.set(Some(val.trim().to_string()));
        show_login.set(false);
        error_msg.set(None);
    };

    view! {
        <div class="login-overlay">
            <div class="login-dialog">
                <div class="login-header-group">
                    <h2 class="login-title">"Admin Login"</h2>
                    <p class="login-subtitle">"Enter your Herald admin token"</p>
                    {move || error_msg.get().map(|msg| view! {
                        <div class="login-error">{msg}</div>
                    })}
                </div>

                <input
                    type="password"
                    class="login-input"
                    placeholder="Bearer token"
                    prop:value=move || input_value.get()
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        input_value.set(val);
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            do_submit();
                        }
                    }
                />

                <button class="login-btn" on:click=move |_| do_submit()>
                    "Login"
                </button>
            </div>
        </div>
    }
}

/// Fetch connected viewer count from /api/stats.
async fn fetch_viewer_count(token: &str) -> Result<u64, String> {
    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::SameOrigin);

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/stats"), &opts)
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "not a Response")?;

    if !resp.ok() {
        return Err(format!("{}", resp.status()));
    }

    let text = JsFuture::from(resp.text().unwrap())
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text_str = text.as_string().ok_or("not a string")?;
    let stats: herald_common::StatsResponse =
        serde_json::from_str(&text_str).map_err(|e| e.to_string())?;
    Ok(stats.connected_viewers as u64)
}

/// Live-updating viewer count badge for the admin header.
#[component]
fn ViewerBadge() -> impl IntoView {
    let token_signal = expect_context::<RwSignal<Option<String>>>();
    let viewer_count = RwSignal::new(Option::<u64>::None);

    let do_fetch = move || {
        let token = match token_signal.get_untracked() {
            Some(t) => t,
            None => return,
        };
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(count) = fetch_viewer_count(&token).await {
                viewer_count.set(Some(count));
            }
        });
    };

    // Fetch on mount
    do_fetch();

    // Poll every 10 seconds
    let handle = set_interval_with_handle(move || do_fetch(), std::time::Duration::from_secs(10));
    on_cleanup(move || {
        if let Ok(h) = handle {
            h.clear();
        }
    });

    move || {
        viewer_count.get().map(|count| {
            let label = if count == 1 { "viewer" } else { "viewers" };
            view! {
                <span class="admin-viewer-badge">
                    "👁 " {count} " " {label}
                </span>
            }
        })
    }
}
