use leptos::prelude::*;

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
