use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::ws_client::ConnectionState;

pub struct StatusBar<'a> {
    connection_state: &'a ConnectionState,
    server_url: &'a str,
    last_update: Option<std::time::Instant>,
}

impl<'a> StatusBar<'a> {
    pub fn new(
        connection_state: &'a ConnectionState,
        server_url: &'a str,
        last_update: Option<std::time::Instant>,
    ) -> Self {
        Self {
            connection_state,
            server_url,
            last_update,
        }
    }

    fn format_elapsed(elapsed: std::time::Duration) -> String {
        let total_secs = elapsed.as_secs();
        if total_secs < 60 {
            format!("Updated {total_secs}s ago")
        } else if total_secs < 3600 {
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("Updated {mins}m {secs}s ago")
        } else {
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            format!("Updated {hours}h {mins}m ago")
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let y = area.y;

        // Left: connection state
        let (state_text, state_color) = match self.connection_state {
            ConnectionState::Connected => ("● Connected".to_string(), Color::Green),
            ConnectionState::Connecting => ("◌ Connecting...".to_string(), Color::Yellow),
            ConnectionState::Reconnecting {
                attempt,
                next_retry_secs,
            } => (
                format!("◌ Reconnecting (attempt {attempt}, retry in {next_retry_secs}s)..."),
                Color::Yellow,
            ),
            ConnectionState::Disconnected => ("✖ Disconnected".to_string(), Color::Red),
        };
        let left_x = area.x.saturating_add(1);
        buf.set_string(left_x, y, &state_text, Style::default().fg(state_color));

        // Center: server URL
        let url_len = self.server_url.len() as u16;
        let center_x = area.x + area.width.saturating_sub(url_len) / 2;
        buf.set_string(
            center_x,
            y,
            self.server_url,
            Style::default().fg(Color::DarkGray),
        );

        // Right: time since last update
        let time_text = match self.last_update {
            Some(instant) => Self::format_elapsed(instant.elapsed()),
            None => "No updates yet".to_string(),
        };
        let time_len = time_text.len() as u16;
        let right_x = area.x + area.width.saturating_sub(time_len + 1);
        buf.set_string(right_x, y, &time_text, Style::default().fg(Color::Gray));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Position, Rect};

    #[test]
    fn test_connected_status() {
        let state = ConnectionState::Connected;
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        let widget = StatusBar::new(&state, "ws://localhost:3000/ws", None);
        widget.render(area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| buf.cell(Position::new(x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(content.contains("● Connected"));

        // Verify green color on the connection indicator cells
        let left_x = 1; // 1 char padding
        let cell = buf.cell(Position::new(left_x, 0)).unwrap();
        assert_eq!(cell.fg, Color::Green);
    }

    #[test]
    fn test_reconnecting_shows_attempt() {
        let state = ConnectionState::Reconnecting {
            attempt: 3,
            next_retry_secs: 5,
        };
        let area = Rect::new(0, 0, 100, 1);
        let mut buf = Buffer::empty(area);

        let widget = StatusBar::new(&state, "ws://localhost:3000/ws", None);
        widget.render(area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| buf.cell(Position::new(x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(
            content.contains("attempt 3"),
            "Expected 'attempt 3' in: {content}"
        );
        assert!(
            content.contains("retry in 5s"),
            "Expected 'retry in 5s' in: {content}"
        );

        // Verify yellow color
        let left_x = 1;
        let cell = buf.cell(Position::new(left_x, 0)).unwrap();
        assert_eq!(cell.fg, Color::Yellow);
    }
}
