use crate::api_client::ApiClient;
use herald_common::StatsResponse;

/// Format seconds into a human-readable duration string (e.g. "2h 15m 30s").
fn format_uptime(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    match (hours, minutes, seconds) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m {s}s"),
        (h, m, s) => format!("{h}h {m}m {s}s"),
    }
}

pub async fn run(server: String, token: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(&server, &token);
    let stats: StatsResponse = client.get("/api/stats").await?;

    println!("Connected viewers:  {}", stats.connected_viewers);
    println!("Uptime:             {}", format_uptime(stats.uptime_secs));
    println!("Total messages:     {}", stats.total_messages);
    println!("Total countdowns:   {}", stats.total_countdowns);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(45), "45s");
        assert_eq!(format_uptime(60), "1m 0s");
        assert_eq!(format_uptime(90), "1m 30s");
        assert_eq!(format_uptime(3600), "1h 0m 0s");
        assert_eq!(format_uptime(8130), "2h 15m 30s");
    }
}
