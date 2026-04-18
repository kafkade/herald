use herald_common::UpdateConfigRequest;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Config key metadata for rendering appropriate input types.
struct ConfigField {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    input_type: InputType,
}

enum InputType {
    Number,
    Select(&'static [&'static str]),
}

const CONFIG_FIELDS: &[ConfigField] = &[
    ConfigField {
        key: "rotation_interval_seconds",
        label: "Rotation Interval",
        description: "Seconds between queue item rotation",
        input_type: InputType::Number,
    },
    ConfigField {
        key: "countdown_refresh_seconds",
        label: "Countdown Refresh",
        description: "Seconds between countdown display updates",
        input_type: InputType::Number,
    },
    ConfigField {
        key: "default_h_align",
        label: "Default H-Align",
        description: "Default horizontal alignment for new messages",
        input_type: InputType::Select(&["left", "center", "right"]),
    },
    ConfigField {
        key: "default_v_align",
        label: "Default V-Align",
        description: "Default vertical alignment for new messages",
        input_type: InputType::Select(&["top", "middle"]),
    },
    ConfigField {
        key: "default_color",
        label: "Default Color",
        description: "Default tile color",
        input_type: InputType::Select(&[
            "red", "orange", "yellow", "green", "blue", "violet", "white", "black",
        ]),
    },
    ConfigField {
        key: "countdown_zero_behavior",
        label: "Countdown Zero Behavior",
        description: "What happens when a countdown reaches zero",
        input_type: InputType::Select(&["show_zero", "remove", "pause"]),
    },
];

fn get_value_str(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

/// Admin config panel for server settings.
#[component]
pub fn ConfigPanel() -> impl IntoView {
    let original = RwSignal::new(serde_json::Map::new());
    let edited = RwSignal::new(serde_json::Map::new());
    let is_saving = RwSignal::new(false);
    let is_loading = RwSignal::new(true);
    let toast = RwSignal::new(Option::<(String, bool)>::None);

    let has_changes = Memo::new(move |_| original.get() != edited.get());

    let token_signal = expect_context::<RwSignal<Option<String>>>();
    let show_login = expect_context::<RwSignal<bool>>();

    // Auto-hide toast after 3 seconds
    Effect::new(move |_| {
        if toast.get().is_some() {
            set_timeout(move || toast.set(None), std::time::Duration::from_secs(3));
        }
    });

    // Load config on mount
    {
        wasm_bindgen_futures::spawn_local(async move {
            let token = match token_signal.get_untracked() {
                Some(t) => t,
                None => {
                    is_loading.set(false);
                    return;
                }
            };
            match fetch_config(&token).await {
                Ok(map) => {
                    original.set(map.clone());
                    edited.set(map);
                }
                Err(e) => {
                    if e.contains("401") {
                        super::auth::clear_token();
                        token_signal.set(None);
                        show_login.set(true);
                    }
                    toast.set(Some((format!("Failed to load config: {e}"), false)));
                }
            }
            is_loading.set(false);
        });
    }

    let on_save = move |_| {
        if is_saving.get_untracked() {
            return;
        }

        let token = match token_signal.get_untracked() {
            Some(t) => t,
            None => {
                toast.set(Some(("Not authenticated".to_string(), false)));
                return;
            }
        };

        let orig = original.get_untracked();
        let edit = edited.get_untracked();

        // Only send changed values
        let mut changed = serde_json::Map::new();
        for (k, v) in &edit {
            if orig.get(k) != Some(v) {
                changed.insert(k.clone(), v.clone());
            }
        }

        if changed.is_empty() {
            return;
        }

        is_saving.set(true);

        wasm_bindgen_futures::spawn_local(async move {
            match save_config(&token, changed).await {
                Ok(()) => {
                    // Update original to match edited
                    original.set(edited.get_untracked());
                    toast.set(Some(("Config saved successfully!".to_string(), true)));
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
            is_saving.set(false);
        });
    };

    view! {
        <div class="config-panel">
            <h2 class="config-panel-title">"Server Config"</h2>

            {move || toast.get().map(|(msg, success)| {
                let class = if success { "toast toast-success" } else { "toast toast-error" };
                view! { <div class=class>{msg}</div> }
            })}

            {move || {
                if is_loading.get() {
                    view! { <div class="config-loading">"Loading config…"</div> }.into_any()
                } else {
                    let field_views = CONFIG_FIELDS.iter().map(|field| {
                        let key = field.key;
                        let label = field.label;
                        let description = field.description;

                        let current_value = Memo::new(move |_| {
                            get_value_str(&edited.get(), key)
                        });

                        let is_modified = Memo::new(move |_| {
                            let orig = original.get();
                            let edit = edited.get();
                            orig.get(key) != edit.get(key)
                        });

                        let input_view = match &field.input_type {
                            InputType::Number => {
                                view! {
                                    <input
                                        type="number"
                                        class=move || if is_modified.get() { "config-input modified" } else { "config-input" }
                                        prop:value=move || current_value.get()
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            edited.update(|map| {
                                                if let Ok(n) = val.parse::<i64>() {
                                                    map.insert(key.to_string(), serde_json::Value::Number(n.into()));
                                                }
                                            });
                                        }
                                    />
                                }.into_any()
                            }
                            InputType::Select(options) => {
                                let options = *options;
                                view! {
                                    <select
                                        class=move || if is_modified.get() { "config-select modified" } else { "config-select" }
                                        on:change=move |ev| {
                                            let val = event_target_value(&ev);
                                            edited.update(|map| {
                                                map.insert(key.to_string(), serde_json::Value::String(val));
                                            });
                                        }
                                    >
                                        {options.iter().map(|opt| {
                                            let opt_val = *opt;
                                            view! {
                                                <option
                                                    value=opt_val
                                                    selected=move || current_value.get() == opt_val
                                                >
                                                    {opt_val}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                }.into_any()
                            }
                        };

                        view! {
                            <div class="config-field">
                                <div class="config-field-header">
                                    <span class="config-field-label">{label}</span>
                                    <span class="config-field-key">{key}</span>
                                </div>
                                <span class="config-field-desc">{description}</span>
                                {input_view}
                            </div>
                        }
                    }).collect_view();

                    view! {
                        <div class="config-form">
                            {field_views}
                            <button
                                class="config-save-btn"
                                on:click=on_save
                                disabled=move || !has_changes.get() || is_saving.get()
                            >
                                {move || if is_saving.get() { "Saving..." } else { "Save Changes" }}
                            </button>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

async fn fetch_config(token: &str) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::SameOrigin);

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/config"), &opts)
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
    serde_json::from_str(&text_str).map_err(|e| e.to_string())
}

async fn save_config(
    token: &str,
    values: serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let request_body = UpdateConfigRequest { values };
    let body = serde_json::to_string(&request_body).map_err(|e| e.to_string())?;

    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("PUT");
    opts.set_mode(web_sys::RequestMode::SameOrigin);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/config"), &opts)
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
