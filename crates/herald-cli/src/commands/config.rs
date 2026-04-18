use crate::api_client::ApiClient;
use herald_common::UpdateConfigRequest;

const KNOWN_KEYS: &[&str] = &[
    "rotation_interval_seconds",
    "countdown_refresh_seconds",
    "default_h_align",
    "default_v_align",
    "default_color",
    "countdown_zero_behavior",
];

fn config_description(key: &str) -> &'static str {
    match key {
        "rotation_interval_seconds" => "Seconds between queue item rotation",
        "countdown_refresh_seconds" => "How often countdown displays refresh",
        "default_h_align" => "Default horizontal alignment (left/center/right)",
        "default_v_align" => "Default vertical alignment (top/middle)",
        "default_color" => "Default tile color",
        "countdown_zero_behavior" => "What happens when countdown reaches zero",
        _ => "",
    }
}

/// Get one or all configuration values.
pub async fn get(
    server: String,
    token: String,
    key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(&server, &token);
    let config: serde_json::Map<String, serde_json::Value> = client.get("/api/config").await?;

    match key {
        Some(k) => {
            let value = config
                .get(&k)
                .ok_or_else(|| format!("Unknown config key: {k}"))?;
            println!("{}", format_value(value));
        }
        None => {
            // Determine column widths
            let key_width = config.keys().map(|k| k.len()).max().unwrap_or(3).max(3);
            let val_width = config
                .values()
                .map(|v| format_value(v).len())
                .max()
                .unwrap_or(5)
                .max(5);

            println!(
                "{:<key_width$}    {:<val_width$}    DESCRIPTION",
                "KEY", "VALUE"
            );
            let key_sep = "─".repeat(key_width);
            let val_sep = "─".repeat(val_width);
            println!("{key_sep}    {val_sep}    ───────────",);

            for (k, v) in &config {
                println!(
                    "{:<key_width$}    {:<val_width$}    {}",
                    k,
                    format_value(v),
                    config_description(k)
                );
            }
        }
    }

    Ok(())
}

/// Set a configuration value.
pub async fn set(
    server: String,
    token: String,
    key: String,
    value: String,
) -> Result<(), Box<dyn std::error::Error>> {
    if !KNOWN_KEYS.contains(&key.as_str()) {
        eprintln!("Warning: '{key}' is not a known config key");
    }

    let client = ApiClient::new(&server, &token);

    // Read current value
    let current: serde_json::Map<String, serde_json::Value> = client.get("/api/config").await?;
    let old_value = current
        .get(&key)
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // Parse value: try number first, fall back to string
    let json_value: serde_json::Value = if let Ok(n) = value.parse::<i64>() {
        serde_json::Value::Number(n.into())
    } else if let Ok(n) = value.parse::<f64>() {
        serde_json::Number::from_f64(n)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(value.clone()))
    } else {
        serde_json::Value::String(value.clone())
    };

    let mut values = serde_json::Map::new();
    values.insert(key.clone(), json_value);

    let _updated: serde_json::Map<String, serde_json::Value> = client
        .put("/api/config", &UpdateConfigRequest { values })
        .await?;

    println!("{key}: {} → {}", format_value(&old_value), value);

    Ok(())
}

/// Format a JSON value for display.
fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
