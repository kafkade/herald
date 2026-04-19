use herald_common::{
    BOARD_COLS, BOARD_ROWS, CellContent, Color, CreateMessageRequest, Grid, HAlign,
    MessageTemplate, VAlign,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Message composer with live grid preview.
#[component]
pub fn MessageComposer() -> impl IntoView {
    let text = RwSignal::new(String::new());
    let align = RwSignal::new("center".to_string());
    let template = RwSignal::new(String::new());
    let expires = RwSignal::new(String::new());
    let display_at = RwSignal::new(String::new());
    let toast = RwSignal::new(Option::<(String, bool)>::None);
    let is_submitting = RwSignal::new(false);

    let preview_grid = Memo::new(move |_| {
        let t = text.get();
        let h = parse_align(&align.get());
        let tmpl = parse_template(&template.get());
        if t.is_empty() {
            Grid::blank()
        } else if let Some(tmpl) = tmpl {
            Grid::from_template(tmpl, &t).unwrap_or_else(|_| Grid::blank())
        } else {
            Grid::from_text(&t, h, VAlign::default()).unwrap_or_else(|_| Grid::blank())
        }
    });

    // Auto-hide toast after 3 seconds
    Effect::new(move |_| {
        if toast.get().is_some() {
            set_timeout(move || toast.set(None), std::time::Duration::from_secs(3));
        }
    });

    let on_push = move |_| {
        if is_submitting.get_untracked() {
            return;
        }
        let t = text.get_untracked();
        if t.trim().is_empty() {
            toast.set(Some(("Message text cannot be empty".to_string(), false)));
            return;
        }

        is_submitting.set(true);

        let token_signal = expect_context::<RwSignal<Option<String>>>();
        let show_login = expect_context::<RwSignal<bool>>();

        let token = match token_signal.get_untracked() {
            Some(t) => t,
            None => {
                toast.set(Some(("Not authenticated".to_string(), false)));
                is_submitting.set(false);
                return;
            }
        };

        let h_align = parse_align(&align.get_untracked());
        let template_value = parse_template(&template.get_untracked());
        let expires_str = expires.get_untracked();
        let expires_at = if expires_str.is_empty() {
            None
        } else {
            match parse_expiry(&expires_str) {
                Some(dt) => Some(dt),
                None => {
                    toast.set(Some(("Invalid expiry date format".to_string(), false)));
                    is_submitting.set(false);
                    return;
                }
            }
        };

        let display_at_value = {
            let val = display_at.get_untracked();
            if val.is_empty() {
                None
            } else {
                chrono::NaiveDateTime::parse_from_str(&val, "%Y-%m-%dT%H:%M")
                    .ok()
                    .map(|ndt| ndt.and_utc())
            }
        };

        let request = CreateMessageRequest {
            text: Some(t),
            grid: None,
            h_align,
            v_align: VAlign::default(),
            queue_position: None,
            expires_at,
            template: template_value,
            display_at: display_at_value,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match post_message(&token, &request).await {
                Ok(()) => {
                    toast.set(Some(("Message pushed successfully!".to_string(), true)));
                    text.set(String::new());
                    template.set(String::new());
                    expires.set(String::new());
                    display_at.set(String::new());
                }
                Err(e) => {
                    if e.contains("401") {
                        super::auth::clear_token();
                        token_signal.set(None);
                        show_login.set(true);
                    }
                    toast.set(Some((format!("Error: {e}"), false)));
                }
            }
            is_submitting.set(false);
        });
    };

    view! {
        <div class="composer">
            <h2 class="composer-title">"Message Composer"</h2>

            {move || toast.get().map(|(msg, success)| {
                let class = if success { "toast toast-success" } else { "toast toast-error" };
                view! { <div class=class>{msg}</div> }
            })}

            <div class="composer-form">
                <div class="composer-input-group">
                    <label class="composer-label">"Message"</label>
                    <textarea
                        class="composer-textarea"
                        placeholder="Enter your message..."
                        rows="3"
                        prop:value=move || text.get()
                        on:input=move |ev| {
                            text.set(event_target_value(&ev));
                        }
                    />
                </div>

                <div class="composer-row">
                    <div class="composer-input-group">
                        <label class="composer-label">"Template"</label>
                        <select
                            class="composer-select"
                            on:change=move |ev| {
                                template.set(event_target_value(&ev));
                            }
                        >
                            <option value="" selected>"None (default)"</option>
                            <option value="announcement">"Announcement"</option>
                            <option value="greeting">"Greeting"</option>
                            <option value="countdown">"Countdown"</option>
                            <option value="ticker">"Ticker"</option>
                        </select>
                    </div>

                    <div class="composer-input-group">
                        <label class="composer-label">"Alignment"</label>
                        <select
                            class="composer-select"
                            on:change=move |ev| {
                                align.set(event_target_value(&ev));
                            }
                        >
                            <option value="left">"Left"</option>
                            <option value="center" selected>"Center"</option>
                            <option value="right">"Right"</option>
                        </select>
                    </div>

                    <div class="composer-input-group">
                        <label class="composer-label">"Expires (optional)"</label>
                        <input
                            type="datetime-local"
                            class="composer-datetime"
                            prop:value=move || expires.get()
                            on:input=move |ev| {
                                expires.set(event_target_value(&ev));
                            }
                        />
                    </div>
                </div>

                <div class="composer-input-group">
                    <label class="composer-label">"Schedule"</label>
                    <input
                        type="datetime-local"
                        class="composer-datetime"
                        prop:value=move || display_at.get()
                        on:input=move |ev| display_at.set(event_target_value(&ev))
                    />
                </div>

                <button
                    class="composer-push-btn"
                    on:click=on_push
                    disabled=move || is_submitting.get()
                >
                    {move || if is_submitting.get() { "Pushing..." } else { "Push Message" }}
                </button>
            </div>

            <div class="composer-preview">
                <h3 class="composer-preview-title">"Preview"</h3>
                <div class="preview-grid">
                    {move || {
                        let grid = preview_grid.get();
                        (0..BOARD_ROWS).map(|row| {
                            let row_data = grid.0[row].clone();
                            (0..BOARD_COLS).map(move |col| {
                                let (class, style, ch) = cell_to_preview(&row_data[col]);
                                view! {
                                    <div class=class style=style>
                                        <span class="flap-char">{ch}</span>
                                    </div>
                                }
                            }).collect_view()
                        }).collect_view()
                    }}
                </div>
            </div>
        </div>
    }
}

fn cell_to_preview(cell: &CellContent) -> (&'static str, String, String) {
    match cell {
        CellContent::Char(c) => ("preview-tile", String::new(), c.to_string()),
        CellContent::Color(color) => {
            let css_color = color_to_css(color);
            (
                "preview-tile preview-tile--color",
                format!("background: {css_color};"),
                String::new(),
            )
        }
        CellContent::Blank => (
            "preview-tile preview-tile--blank",
            String::new(),
            " ".to_string(),
        ),
    }
}

fn color_to_css(color: &Color) -> &'static str {
    match color {
        Color::Red => "var(--color-red)",
        Color::Orange => "var(--color-orange)",
        Color::Yellow => "var(--color-yellow)",
        Color::Green => "var(--color-green)",
        Color::Blue => "var(--color-blue)",
        Color::Violet => "var(--color-violet)",
        Color::White => "var(--color-white)",
        Color::Black => "var(--color-black)",
    }
}

fn parse_align(s: &str) -> HAlign {
    match s {
        "left" => HAlign::Left,
        "right" => HAlign::Right,
        _ => HAlign::Center,
    }
}

fn parse_template(s: &str) -> Option<MessageTemplate> {
    match s {
        "announcement" => Some(MessageTemplate::Announcement),
        "greeting" => Some(MessageTemplate::Greeting),
        "countdown" => Some(MessageTemplate::Countdown),
        "ticker" => Some(MessageTemplate::Ticker),
        _ => None,
    }
}

fn parse_expiry(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return Some(naive.and_utc());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc());
    }
    s.parse::<chrono::DateTime<chrono::Utc>>().ok()
}

/// POST a message to the server using the Fetch API.
async fn post_message(token: &str, request: &CreateMessageRequest) -> Result<(), String> {
    let body = serde_json::to_string(request).map_err(|e| e.to_string())?;

    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::SameOrigin);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request_obj =
        web_sys::Request::new_with_str_and_init(&format!("{base}/api/messages"), &opts)
            .map_err(|e| format!("{e:?}"))?;

    request_obj
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request_obj
        .headers()
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request_obj))
        .await
        .map_err(|e| format!("{e:?}"))?;

    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "not a Response")?;

    if resp.ok() {
        Ok(())
    } else {
        Err(format!("{}", resp.status()))
    }
}
