use crate::api_client::ApiClient;
use chrono::{DateTime, Utc};
use herald_common::{
    BOARD_COLS, BOARD_ROWS, CellContent, Color, CreateMessageRequest, Grid, HAlign, Message,
    MessageTemplate, VAlign,
};
use std::io::{self, BufRead, IsTerminal, Write};

pub async fn run(
    text: String,
    server: String,
    token: String,
    align: String,
    expires: Option<String>,
    template: Option<String>,
    preview: bool,
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

    let template = match template.as_deref() {
        Some("announcement") => Some(MessageTemplate::Announcement),
        Some("greeting") => Some(MessageTemplate::Greeting),
        Some("countdown") => Some(MessageTemplate::Countdown),
        Some("ticker") => Some(MessageTemplate::Ticker),
        Some(other) => {
            return Err(format!(
                "Unknown template: {other}. Options: announcement, greeting, countdown, ticker"
            )
            .into());
        }
        None => None,
    };

    if preview {
        let grid = if let Some(tmpl) = template {
            Grid::from_template(tmpl, &text)?
        } else {
            Grid::from_text(&text, h_align, VAlign::default())?
        };
        println!();
        print_grid_preview(&grid);
        println!();

        if io::stdin().is_terminal() {
            print!("Push this message? [Y/n] ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().lock().read_line(&mut input)?;
            let answer = input.trim().to_lowercase();
            if !answer.is_empty() && answer != "y" {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    let request = CreateMessageRequest {
        text: Some(text),
        grid: None,
        h_align,
        v_align: VAlign::default(),
        queue_position: None,
        expires_at,
        template,
    };

    let client = ApiClient::new(&server, &token);
    let message: Message = client.post("/api/messages", &request).await?;

    println!("Created message {}", message.id);

    Ok(())
}

fn print_grid_preview(grid: &Grid) {
    let top = format!("┌{}┐", "───┬".repeat(BOARD_COLS - 1).to_string() + "───");
    let mid = format!("├{}┤", "───┼".repeat(BOARD_COLS - 1).to_string() + "───");
    let bot = format!("└{}┘", "───┴".repeat(BOARD_COLS - 1).to_string() + "───");

    println!("{top}");
    for (i, row) in grid.0.iter().enumerate() {
        let cells: String = row
            .iter()
            .map(|cell| match cell {
                CellContent::Char(c) => format!(" {} ", c),
                CellContent::Color(color) => {
                    let symbol = color_symbol(color);
                    format!(" {} ", symbol)
                }
                CellContent::Blank => "   ".to_string(),
            })
            .collect::<Vec<_>>()
            .join("│");
        println!("│{cells}│");
        if i < BOARD_ROWS - 1 {
            println!("{mid}");
        }
    }
    println!("{bot}");
}

fn color_symbol(color: &Color) -> char {
    match color {
        Color::Red => '■',
        Color::Orange => '■',
        Color::Yellow => '■',
        Color::Green => '■',
        Color::Blue => '■',
        Color::Violet => '■',
        Color::White => '□',
        Color::Black => '■',
    }
}
