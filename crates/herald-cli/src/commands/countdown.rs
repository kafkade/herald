use crate::api_client::ApiClient;
use chrono::{DateTime, Utc};
use herald_common::{Countdown, CreateCountdownRequest, ListResponse, ZeroBehavior};

pub async fn create(
    server: String,
    token: String,
    label: String,
    target: String,
    zero_behavior: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let target: DateTime<Utc> = target.parse().map_err(|e| {
        format!(
            "invalid target timestamp '{target}': expected ISO-8601 format \
             (e.g. 2025-12-31T00:00:00Z): {e}"
        )
    })?;

    let zero_behavior = match zero_behavior.to_lowercase().as_str() {
        "show_zero" => ZeroBehavior::ShowZero,
        "remove" => ZeroBehavior::Remove,
        "pause" => ZeroBehavior::Pause,
        other => {
            return Err(format!(
                "invalid zero_behavior '{other}': expected 'show_zero', 'remove', or 'pause'"
            )
            .into());
        }
    };

    let request = CreateCountdownRequest {
        label: label.clone(),
        target,
        format_template: "{D} DAYS  {HH}:{MM}:{SS}".to_string(),
        zero_behavior,
        queue_position: None,
    };

    let client = ApiClient::new(&server, &token);
    let countdown: Countdown = client.post("/api/countdowns", &request).await?;

    println!("Created countdown {} — \"{label}\"", countdown.id);

    Ok(())
}

pub async fn list(server: String, token: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(&server, &token);
    let response: ListResponse<Countdown> = client.get("/api/countdowns").await?;

    if response.items.is_empty() {
        println!("No countdowns found");
        return Ok(());
    }

    // Column widths
    let id_w = 8;
    let label_w = 20;
    let target_w = 20;
    let remaining_w = 16;

    println!(
        "{:<id_w$}  {:<label_w$}  {:<target_w$}  {:<remaining_w$}",
        "ID", "LABEL", "TARGET", "REMAINING"
    );
    println!(
        "{:<id_w$}  {:<label_w$}  {:<target_w$}  {:<remaining_w$}",
        "—".repeat(id_w),
        "—".repeat(label_w),
        "—".repeat(target_w),
        "—".repeat(remaining_w),
    );

    for countdown in &response.items {
        let short_id = &countdown.id.to_string()[..8];
        let target_str = countdown.target.format("%Y-%m-%d %H:%M UTC").to_string();
        let remaining = format_remaining(&countdown.target);

        let label_display = if countdown.label.len() > label_w {
            format!("{}…", &countdown.label[..label_w - 1])
        } else {
            countdown.label.clone()
        };

        println!(
            "{:<id_w$}  {:<label_w$}  {:<target_w$}  {:<remaining_w$}",
            short_id, label_display, target_str, remaining
        );
    }

    Ok(())
}

pub async fn delete(
    server: String,
    token: String,
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate UUID format (8-4-4-4-12 hex digits)
    let is_valid_uuid = id.len() == 36
        && id.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        });

    if !is_valid_uuid {
        return Err(format!(
            "invalid countdown ID '{id}': expected a UUID \
             (e.g. 550e8400-e29b-41d4-a716-446655440000)"
        )
        .into());
    }

    let client = ApiClient::new(&server, &token);
    client.delete(&format!("/api/countdowns/{id}")).await?;

    println!("Deleted countdown {id}");

    Ok(())
}

fn format_remaining(target: &DateTime<Utc>) -> String {
    let remaining = *target - Utc::now();
    if remaining <= chrono::Duration::zero() {
        "expired".to_string()
    } else {
        let days = remaining.num_days();
        let hours = remaining.num_hours() % 24;
        let minutes = remaining.num_minutes() % 60;
        if days > 0 {
            format!("{days}d {hours}h {minutes}m")
        } else if hours > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{minutes}m")
        }
    }
}
