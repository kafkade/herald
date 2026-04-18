use herald_common::{Countdown, CreateCountdownRequest, ListResponse, ZeroBehavior};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Countdown manager panel — create, list, and delete countdowns.
#[component]
pub fn CountdownManager() -> impl IntoView {
    let countdowns = RwSignal::new(Vec::<Countdown>::new());
    let toast = RwSignal::new(Option::<(String, bool)>::None);
    let is_creating = RwSignal::new(false);
    let tick = RwSignal::new(0u64);

    // Form inputs
    let label = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let zero_behavior = RwSignal::new("show_zero".to_string());

    // Auto-hide toast after 3 seconds
    Effect::new(move |_| {
        if toast.get().is_some() {
            set_timeout(move || toast.set(None), std::time::Duration::from_secs(3));
        }
    });

    // Fetch countdowns on mount
    Effect::new(move |_| {
        let token_signal = expect_context::<RwSignal<Option<String>>>();
        if let Some(token) = token_signal.get() {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(list) = fetch_countdowns(&token).await {
                    countdowns.set(list);
                }
            });
        }
    });

    // Poll every 10 seconds
    set_interval(
        move || {
            let token_signal = expect_context::<RwSignal<Option<String>>>();
            if let Some(token) = token_signal.get_untracked() {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(list) = fetch_countdowns(&token).await {
                        countdowns.set(list);
                    }
                });
            }
        },
        std::time::Duration::from_secs(10),
    );

    // Tick every second for live remaining time
    set_interval(
        move || tick.update(|t| *t += 1),
        std::time::Duration::from_secs(1),
    );

    let on_create = move |_| {
        if is_creating.get_untracked() {
            return;
        }

        let lbl = label.get_untracked();
        if lbl.trim().is_empty() {
            toast.set(Some(("Label cannot be empty".to_string(), false)));
            return;
        }

        let tgt = target.get_untracked();
        if tgt.is_empty() {
            toast.set(Some(("Target date/time is required".to_string(), false)));
            return;
        }

        let parsed_target = match parse_datetime(&tgt) {
            Some(dt) => dt,
            None => {
                toast.set(Some(("Invalid date/time format".to_string(), false)));
                return;
            }
        };

        let token_signal = expect_context::<RwSignal<Option<String>>>();
        let show_login = expect_context::<RwSignal<bool>>();

        let token = match token_signal.get_untracked() {
            Some(t) => t,
            None => {
                toast.set(Some(("Not authenticated".to_string(), false)));
                return;
            }
        };

        let zb = parse_zero_behavior(&zero_behavior.get_untracked());

        let request = CreateCountdownRequest {
            label: lbl,
            target: parsed_target,
            format_template: String::new(),
            zero_behavior: zb,
            queue_position: None,
        };

        is_creating.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            match create_countdown(&token, &request).await {
                Ok(()) => {
                    toast.set(Some(("Countdown created!".to_string(), true)));
                    label.set(String::new());
                    target.set(String::new());
                    zero_behavior.set("show_zero".to_string());
                    // Refresh list
                    if let Ok(list) = fetch_countdowns(&token).await {
                        countdowns.set(list);
                    }
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
            is_creating.set(false);
        });
    };

    let handle_delete = move |id: uuid::Uuid| {
        let window = web_sys::window().unwrap();
        let confirmed = window
            .confirm_with_message("Delete this countdown?")
            .unwrap_or(false);
        if !confirmed {
            return;
        }

        let token_signal = expect_context::<RwSignal<Option<String>>>();
        let show_login = expect_context::<RwSignal<bool>>();

        let token = match token_signal.get_untracked() {
            Some(t) => t,
            None => {
                toast.set(Some(("Not authenticated".to_string(), false)));
                return;
            }
        };

        let id_str = id.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match delete_countdown(&token, &id_str).await {
                Ok(()) => {
                    toast.set(Some(("Countdown deleted".to_string(), true)));
                    if let Ok(list) = fetch_countdowns(&token).await {
                        countdowns.set(list);
                    }
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
        });
    };

    view! {
        <div class="countdown-mgr">
            <h2 class="countdown-mgr-title">"Countdown Manager"</h2>

            {move || toast.get().map(|(msg, success)| {
                let class = if success { "toast toast-success" } else { "toast toast-error" };
                view! { <div class=class>{msg}</div> }
            })}

            <div class="countdown-form">
                <div class="countdown-form-row">
                    <div class="composer-input-group">
                        <label class="composer-label">"Label"</label>
                        <input
                            type="text"
                            class="composer-datetime"
                            placeholder="e.g. Product Launch"
                            prop:value=move || label.get()
                            on:input=move |ev| {
                                label.set(event_target_value(&ev));
                            }
                        />
                    </div>
                    <div class="composer-input-group">
                        <label class="composer-label">"Target"</label>
                        <input
                            type="datetime-local"
                            class="composer-datetime"
                            prop:value=move || target.get()
                            on:input=move |ev| {
                                target.set(event_target_value(&ev));
                            }
                        />
                    </div>
                </div>
                <div class="countdown-form-row">
                    <div class="composer-input-group">
                        <label class="composer-label">"At Zero"</label>
                        <select
                            class="composer-select"
                            prop:value=move || zero_behavior.get()
                            on:change=move |ev| {
                                zero_behavior.set(event_target_value(&ev));
                            }
                        >
                            <option value="show_zero">"Show Zero"</option>
                            <option value="remove">"Remove"</option>
                            <option value="pause">"Pause"</option>
                        </select>
                    </div>
                </div>
                <button
                    class="countdown-create-btn"
                    on:click=on_create
                    disabled=move || is_creating.get()
                >
                    {move || if is_creating.get() { "Creating..." } else { "Create Countdown" }}
                </button>
            </div>

            <div class="countdown-list">
                {move || {
                    // Subscribe to tick so this re-renders every second
                    let _ = tick.get();
                    let items = countdowns.get();
                    if items.is_empty() {
                        view! { <div class="countdown-empty">"No countdowns yet"</div> }.into_any()
                    } else {
                        items.into_iter().map(|cd| {
                            let id = cd.id;
                            let remaining = format_remaining(&cd.target);
                            let is_expired = remaining == "Expired";
                            let remaining_class = if is_expired {
                                "countdown-item-remaining expired"
                            } else {
                                "countdown-item-remaining"
                            };
                            let formatted_target = cd.target.format("%Y-%m-%d %H:%M UTC").to_string();
                            view! {
                                <div class="countdown-item">
                                    <div class="countdown-item-info">
                                        <span class="countdown-item-label">{cd.label.clone()}</span>
                                        <span class="countdown-item-meta">{formatted_target}</span>
                                    </div>
                                    <span class=remaining_class>{remaining}</span>
                                    <button
                                        class="countdown-delete-btn"
                                        on:click=move |_| handle_delete(id)
                                    >"Delete"</button>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>
        </div>
    }
}

fn format_remaining(target: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let remaining = *target - now;
    if remaining <= chrono::Duration::zero() {
        "Expired".to_string()
    } else {
        let days = remaining.num_days();
        let hours = remaining.num_hours() % 24;
        let mins = remaining.num_minutes() % 60;
        let secs = remaining.num_seconds() % 60;
        if days > 0 {
            format!("{days}d {hours}h {mins}m")
        } else if hours > 0 {
            format!("{hours}h {mins}m {secs}s")
        } else if mins > 0 {
            format!("{mins}m {secs}s")
        } else {
            format!("{secs}s")
        }
    }
}

fn parse_zero_behavior(s: &str) -> ZeroBehavior {
    match s {
        "remove" => ZeroBehavior::Remove,
        "pause" => ZeroBehavior::Pause,
        _ => ZeroBehavior::ShowZero,
    }
}

fn parse_datetime(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return Some(naive.and_utc());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc());
    }
    s.parse::<chrono::DateTime<chrono::Utc>>().ok()
}

async fn fetch_countdowns(token: &str) -> Result<Vec<Countdown>, String> {
    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::SameOrigin);

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/countdowns"), &opts)
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

    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text_str = text.as_string().ok_or("not a string")?;
    let list: ListResponse<Countdown> =
        serde_json::from_str(&text_str).map_err(|e| e.to_string())?;
    Ok(list.items)
}

async fn create_countdown(token: &str, req: &CreateCountdownRequest) -> Result<(), String> {
    let body = serde_json::to_string(req).map_err(|e| e.to_string())?;

    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(web_sys::RequestMode::SameOrigin);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/countdowns"), &opts)
        .map_err(|e| format!("{e:?}"))?;

    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;

    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "not a Response")?;

    if resp.ok() {
        Ok(())
    } else {
        Err(format!("{}", resp.status()))
    }
}

async fn delete_countdown(token: &str, id: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(web_sys::RequestMode::SameOrigin);

    let request =
        web_sys::Request::new_with_str_and_init(&format!("{base}/api/countdowns/{id}"), &opts)
            .map_err(|e| format!("{e:?}"))?;

    request
        .headers()
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(|e| format!("{e:?}"))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;

    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "not a Response")?;

    if resp.ok() {
        Ok(())
    } else {
        Err(format!("{}", resp.status()))
    }
}
