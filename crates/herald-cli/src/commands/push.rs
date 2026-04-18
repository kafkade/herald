use crate::api_client::ApiClient;
use chrono::{DateTime, Utc};
use herald_common::{CreateMessageRequest, HAlign, Message, VAlign};

pub async fn run(
    text: String,
    server: String,
    token: String,
    align: String,
    expires: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let h_align = match align.to_lowercase().as_str() {
        "left" => HAlign::Left,
        "center" => HAlign::Center,
        "right" => HAlign::Right,
        other => {
            return Err(format!(
                "invalid alignment '{other}': expected 'left', 'center', or 'right'"
            )
            .into());
        }
    };

    let expires_at = expires
        .map(|s| {
            s.parse::<DateTime<Utc>>().map_err(|e| {
                format!("invalid expires timestamp '{s}': expected ISO-8601 format (e.g. 2025-12-31T23:59:59Z): {e}")
            })
        })
        .transpose()?;

    let request = CreateMessageRequest {
        text: Some(text),
        grid: None,
        h_align,
        v_align: VAlign::default(),
        queue_position: None,
        expires_at,
    };

    let client = ApiClient::new(&server, &token);
    let message: Message = client.post("/api/messages", &request).await?;

    println!("Created message {}", message.id);

    Ok(())
}
