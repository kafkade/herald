use herald_common::{QueueItem, QueueItemKind, QueueListResponse, ReorderQueueRequest};
use leptos::prelude::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Queue manager with drag-to-reorder.
#[component]
pub fn QueueManager() -> impl IntoView {
    let items = RwSignal::new(Vec::<QueueItem>::new());
    let current_index = RwSignal::new(-1i64);
    let is_saving = RwSignal::new(false);
    let toast = RwSignal::new(Option::<(String, bool)>::None);
    let drag_over_index = RwSignal::new(Option::<usize>::None);

    // Fetch on mount
    Effect::new(move |_| {
        let token_signal = expect_context::<RwSignal<Option<String>>>();
        if let Some(token) = token_signal.get() {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(resp) = fetch_queue(&token).await {
                    items.set(resp.items);
                    current_index.set(resp.current_index);
                }
            });
        }
    });

    // Poll every 10s
    set_interval(
        move || {
            if is_saving.get_untracked() {
                return;
            }
            let token_signal = expect_context::<RwSignal<Option<String>>>();
            if let Some(token) = token_signal.get_untracked() {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(resp) = fetch_queue(&token).await {
                        items.set(resp.items);
                        current_index.set(resp.current_index);
                    }
                });
            }
        },
        std::time::Duration::from_secs(10),
    );

    // Toast auto-hide
    Effect::new(move |_| {
        if toast.get().is_some() {
            set_timeout(move || toast.set(None), std::time::Duration::from_secs(3));
        }
    });

    view! {
        <div class="queue-mgr">
            <h2 class="queue-mgr-title">
                "Queue"
                {move || is_saving.get().then(|| view! {
                    <span class="queue-saving">" Saving..."</span>
                })}
            </h2>

            {move || toast.get().map(|(msg, success)| {
                let class = if success { "toast toast-success" } else { "toast toast-error" };
                view! { <div class=class>{msg}</div> }
            })}

            {move || {
                let queue_items = items.get();
                let cur_idx = current_index.get();
                if queue_items.is_empty() {
                    view! { <p class="queue-empty">"Queue is empty"</p> }.into_any()
                } else {
                    view! {
                        <div class="queue-list">
                            {queue_items.iter().enumerate().map(|(idx, item)| {
                                let _id = item.id;
                                let is_current = idx as i64 == cur_idx;
                                let kind_icon = match item.kind {
                                    QueueItemKind::Message => "\u{1F4DD}",
                                    QueueItemKind::Countdown => "\u{23F1}",
                                };
                                let label = item.label.clone();
                                let expiry = item.expires_at
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                    .unwrap_or_else(|| "\u{2014}".to_string());
                                let item_class = if is_current {
                                    "queue-item queue-item--current"
                                } else {
                                    "queue-item"
                                };
                                let drag_class = move || {
                                    if drag_over_index.get() == Some(idx) {
                                        format!("{item_class} queue-item--drag-over")
                                    } else {
                                        item_class.to_string()
                                    }
                                };

                                view! {
                                    <div
                                        class=drag_class
                                        draggable="true"
                                        on:dragstart=move |ev| {
                                            if let Some(dt) = ev.data_transfer() {
                                                let _ = dt.set_data("text/plain", &idx.to_string());
                                            }
                                        }
                                        on:dragover=move |ev| {
                                            ev.prevent_default();
                                            drag_over_index.set(Some(idx));
                                        }
                                        on:dragleave=move |_| {
                                            if drag_over_index.get_untracked() == Some(idx) {
                                                drag_over_index.set(None);
                                            }
                                        }
                                        on:drop=move |ev| {
                                            ev.prevent_default();
                                            drag_over_index.set(None);
                                            let from_idx = ev.data_transfer()
                                                .and_then(|dt| dt.get_data("text/plain").ok())
                                                .and_then(|s| s.parse::<usize>().ok());
                                            if let Some(from) = from_idx {
                                                if from != idx {
                                                    let mut new_items = items.get_untracked();
                                                    let moved = new_items.remove(from);
                                                    new_items.insert(idx, moved);
                                                    items.set(new_items.clone());

                                                    let order: Vec<Uuid> = new_items.iter().map(|i| i.id).collect();
                                                    is_saving.set(true);
                                                    let token_signal = expect_context::<RwSignal<Option<String>>>();
                                                    if let Some(token) = token_signal.get_untracked() {
                                                        wasm_bindgen_futures::spawn_local(async move {
                                                            match reorder_queue(&token, order).await {
                                                                Ok(resp) => {
                                                                    items.set(resp.items);
                                                                    current_index.set(resp.current_index);
                                                                    toast.set(Some(("Queue reordered".to_string(), true)));
                                                                }
                                                                Err(e) => {
                                                                    toast.set(Some((format!("Error: {e}"), false)));
                                                                }
                                                            }
                                                            is_saving.set(false);
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    >
                                        <span class="queue-item-handle">{"\u{2261}"}</span>
                                        <span class="queue-item-kind">{kind_icon}</span>
                                        <span class="queue-item-label">{label}</span>
                                        <span class="queue-item-pos">{"#"}{idx + 1}</span>
                                        <span class="queue-item-expiry">{expiry}</span>
                                        {is_current.then(|| view! {
                                            <span class="queue-item-badge">{"\u{25B6} Now"}</span>
                                        })}
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// GET /api/queue
async fn fetch_queue(token: &str) -> Result<QueueListResponse, String> {
    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::SameOrigin);

    let request = web_sys::Request::new_with_str_and_init(&format!("{base}/api/queue"), &opts)
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

    let body = text.as_string().ok_or("response not a string")?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// PUT /api/queue/reorder
async fn reorder_queue(token: &str, order: Vec<Uuid>) -> Result<QueueListResponse, String> {
    let body = serde_json::to_string(&ReorderQueueRequest { order }).map_err(|e| e.to_string())?;

    let window = web_sys::window().ok_or("no window")?;
    let base = window
        .location()
        .origin()
        .map_err(|_| "no origin".to_string())?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("PUT");
    opts.set_mode(web_sys::RequestMode::SameOrigin);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let request =
        web_sys::Request::new_with_str_and_init(&format!("{base}/api/queue/reorder"), &opts)
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

    if !resp.ok() {
        return Err(format!("{}", resp.status()));
    }

    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;

    let body_str = text.as_string().ok_or("response not a string")?;
    serde_json::from_str(&body_str).map_err(|e| e.to_string())
}
