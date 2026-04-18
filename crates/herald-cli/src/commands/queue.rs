use crate::api_client::ApiClient;
use herald_common::{QueueItemKind, QueueListResponse, ReorderQueueRequest};
use uuid::Uuid;

pub async fn list(server: String, token: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(&server, &token);
    let response: QueueListResponse = client.get("/api/queue").await?;

    if response.items.is_empty() {
        println!("Queue is empty");
        return Ok(());
    }

    let pos_w = 4;
    let type_w = 10;
    let label_w = 20;
    let expires_w = 20;

    println!(
        "{:<pos_w$}  {:<type_w$}  {:<label_w$}  {:<expires_w$}",
        "#", "TYPE", "LABEL", "EXPIRES"
    );
    println!(
        "{:<pos_w$}  {:<type_w$}  {:<label_w$}  {:<expires_w$}",
        "—".repeat(pos_w),
        "—".repeat(type_w),
        "—".repeat(label_w),
        "—".repeat(expires_w),
    );

    for (i, item) in response.items.iter().enumerate() {
        let prefix = if i as i64 == response.current_index {
            "▶"
        } else {
            " "
        };

        let kind_str = match item.kind {
            QueueItemKind::Message => "message",
            QueueItemKind::Countdown => "countdown",
        };

        let expires_str = item
            .expires_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "—".to_string());

        let label_display = if item.label.len() > label_w {
            format!("{}…", &item.label[..label_w - 1])
        } else {
            item.label.clone()
        };

        println!(
            "{prefix} {:<pos_w$}  {:<type_w$}  {:<label_w$}  {:<expires_w$}",
            item.queue_position, kind_str, label_display, expires_str
        );
    }

    Ok(())
}

pub async fn reorder(
    server: String,
    token: String,
    ids: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let order: Vec<Uuid> = ids
        .iter()
        .map(|s| {
            s.parse::<Uuid>().map_err(|_| {
                format!(
                    "invalid UUID '{s}': expected format like 550e8400-e29b-41d4-a716-446655440000"
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let request = ReorderQueueRequest { order };

    let client = ApiClient::new(&server, &token);
    let _: QueueListResponse = client.put("/api/queue/reorder", &request).await?;

    println!("Queue reordered");

    Ok(())
}
