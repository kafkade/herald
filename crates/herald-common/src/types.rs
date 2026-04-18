use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{BOARD_COLS, BOARD_ROWS};

// ── Board primitives ──────────────────────────────────────────────

/// The 8 Vestaboard-compatible tile colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
    White,
    Black,
}

/// Content of a single board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum CellContent {
    Char(char),
    Color(Color),
    #[default]
    Blank,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VAlign {
    Top,
    #[default]
    Middle,
}

// ── Grid ──────────────────────────────────────────────────────────

/// A 6×22 board grid. Stored as a flat vector for serde simplicity,
/// but always exactly BOARD_ROWS × BOARD_COLS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grid(pub Vec<Vec<CellContent>>);

impl Grid {
    /// Create a blank grid (all cells `Blank`).
    pub fn blank() -> Self {
        Self(vec![vec![CellContent::Blank; BOARD_COLS]; BOARD_ROWS])
    }

    /// Validate that the grid is exactly 6×22.
    pub fn validate(&self) -> Result<(), String> {
        if self.0.len() != BOARD_ROWS {
            return Err(format!(
                "grid must have {BOARD_ROWS} rows, got {}",
                self.0.len()
            ));
        }
        for (i, row) in self.0.iter().enumerate() {
            if row.len() != BOARD_COLS {
                return Err(format!(
                    "row {i} must have {BOARD_COLS} columns, got {}",
                    row.len()
                ));
            }
        }
        Ok(())
    }

    /// Normalize a character to the split-flap character set.
    /// Returns `None` for unsupported characters (rendered as blank).
    fn normalize_char(ch: char) -> Option<char> {
        let upper = ch.to_uppercase().next().unwrap_or(ch);
        match upper {
            'A'..='Z' | '0'..='9' | ' ' => Some(upper),
            '!' | '@' | '#' | '$' | '%' | '&' | '(' | ')' | '-' | '+' | '=' | ';' | ':' | '\''
            | '"' | ',' | '.' | '/' | '?' | '*' => Some(upper),
            // Common typographic mappings
            '\u{2019}' | '\u{2018}' => Some('\''), // smart quotes
            '\u{201C}' | '\u{201D}' => Some('"'),  // smart double quotes
            '\u{2014}' | '\u{2013}' => Some('-'),  // em/en dash
            '\u{2026}' => Some('.'),               // ellipsis → period
            _ => None,                             // unsupported → blank
        }
    }

    /// Word-wrap text into lines that fit within BOARD_COLS.
    fn wrap_text(text: &str) -> Vec<Vec<char>> {
        let mut lines: Vec<Vec<char>> = Vec::new();

        for input_line in text.split('\n') {
            let normalized: Vec<char> = input_line
                .chars()
                .map(|c| Self::normalize_char(c).unwrap_or(' '))
                .collect();

            // Collapse multiple spaces
            let collapsed: Vec<char> = {
                let mut result = Vec::new();
                let mut prev_space = false;
                for &ch in &normalized {
                    if ch == ' ' {
                        if !prev_space && !result.is_empty() {
                            result.push(' ');
                        }
                        prev_space = true;
                    } else {
                        result.push(ch);
                        prev_space = false;
                    }
                }
                // Trim trailing space
                if result.last() == Some(&' ') {
                    result.pop();
                }
                result
            };

            if collapsed.is_empty() {
                lines.push(Vec::new());
                continue;
            }

            // Split into words
            let words: Vec<&[char]> = collapsed
                .split(|c| *c == ' ')
                .filter(|w| !w.is_empty())
                .collect();

            let mut current_line: Vec<char> = Vec::new();
            for word in words {
                if word.len() > BOARD_COLS {
                    // Hard-split long words
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = Vec::new();
                    }
                    for chunk in word.chunks(BOARD_COLS) {
                        lines.push(chunk.to_vec());
                    }
                } else if current_line.is_empty() {
                    current_line = word.to_vec();
                } else if current_line.len() + 1 + word.len() <= BOARD_COLS {
                    current_line.push(' ');
                    current_line.extend_from_slice(word);
                } else {
                    lines.push(current_line);
                    current_line = word.to_vec();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }

        lines
    }

    /// Align a line of characters within a row based on horizontal alignment.
    fn place_line(row: &mut [CellContent], chars: &[char], h_align: HAlign) {
        let len = chars.len().min(BOARD_COLS);
        let start = match h_align {
            HAlign::Left => 0,
            HAlign::Center => (BOARD_COLS - len) / 2,
            HAlign::Right => BOARD_COLS - len,
        };
        for (i, &ch) in chars[..len].iter().enumerate() {
            row[start + i] = CellContent::Char(ch);
        }
    }

    /// Build a grid from a text string, with word-wrapping, normalization, and alignment.
    ///
    /// Returns an error if the text requires more than BOARD_ROWS lines after wrapping.
    pub fn from_text(text: &str, h_align: HAlign, v_align: VAlign) -> Result<Self, String> {
        let lines = Self::wrap_text(text);

        if lines.len() > BOARD_ROWS {
            return Err(format!(
                "text requires {} lines but the board only has {BOARD_ROWS} rows",
                lines.len()
            ));
        }

        let mut grid = Self::blank();

        let start_row = match v_align {
            VAlign::Top => 0,
            VAlign::Middle => (BOARD_ROWS - lines.len()) / 2,
        };

        for (i, line) in lines.iter().enumerate() {
            let row_idx = start_row + i;
            if row_idx < BOARD_ROWS {
                Self::place_line(&mut grid.0[row_idx], line, h_align);
            }
        }

        Ok(grid)
    }

    /// Compare two grids and return the (row, col) positions of cells that differ.
    pub fn diff_grids(old: &Grid, new: &Grid) -> Vec<(usize, usize)> {
        let mut diffs = Vec::new();
        for (row_idx, (old_row, new_row)) in old.0.iter().zip(new.0.iter()).enumerate() {
            for (col_idx, (old_cell, new_cell)) in old_row.iter().zip(new_row.iter()).enumerate() {
                if old_cell != new_cell {
                    diffs.push((row_idx, col_idx));
                }
            }
        }
        diffs
    }

    /// Returns true if the two grids are identical (no cells differ).
    pub fn grids_are_identical(old: &Grid, new: &Grid) -> bool {
        Grid::diff_grids(old, new).is_empty()
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::blank()
    }
}

// ── Messages ──────────────────────────────────────────────────────

/// A message stored on the board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub grid: Grid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub queue_position: i64,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Countdowns ────────────────────────────────────────────────────

/// Behavior when a countdown reaches zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ZeroBehavior {
    #[default]
    ShowZero,
    ShowMessage {
        grid: Grid,
    },
    Remove,
    Pause,
}

/// A countdown timer entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Countdown {
    pub id: Uuid,
    pub label: String,
    pub target: DateTime<Utc>,
    pub format_template: String,
    pub zero_behavior: ZeroBehavior,
    pub queue_position: i64,
    pub created_at: DateTime<Utc>,
}

// ── Queue ─────────────────────────────────────────────────────────

/// The kind of item in the display queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueItemKind {
    Message,
    Countdown,
}

/// A unified queue entry (for GET /api/queue responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub kind: QueueItemKind,
    pub label: String,
    pub queue_position: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Abbreviated info about the currently displayed queue item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemInfo {
    pub id: Uuid,
    pub kind: QueueItemKind,
    pub label: String,
}

// ── Board state ───────────────────────────────────────────────────

/// The full board state broadcast to viewers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
    pub grid: Grid,
    pub previous_grid: Grid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_item: Option<QueueItemInfo>,
    pub timestamp: DateTime<Utc>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            grid: crate::splash_grid(),
            previous_grid: Grid::blank(),
            current_item: None,
            timestamp: Utc::now(),
        }
    }
}

// ── WebSocket protocol ───────────────────────────────────────────

/// Messages sent from the server to connected viewers over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Full board state — sent on initial connection and on every board change.
    #[serde(rename = "board_update")]
    BoardUpdate(BoardState),

    /// Heartbeat — server sends periodically to keep the connection alive.
    #[serde(rename = "heartbeat")]
    Heartbeat { server_time: DateTime<Utc> },

    /// Queue metadata — sent alongside board_update for status bar rendering.
    #[serde(rename = "queue_info")]
    QueueInfo {
        current_index: usize,
        total_items: usize,
        next_rotation_seconds: u32,
        is_countdown_active: bool,
    },

    /// Server is shutting down — clients should expect disconnection.
    #[serde(rename = "shutdown")]
    Shutdown { reason: String },

    /// Error notification.
    #[serde(rename = "error")]
    Error { message: String },
}

/// Messages sent from a viewer client to the server over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Response to a heartbeat.
    #[serde(rename = "pong")]
    Pong,
}

// ── API request/response DTOs ─────────────────────────────────────

/// Request body for POST /api/messages.
/// Accepts either `text` (auto-rendered) or `grid` (raw 6×22), but not both.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub text: Option<String>,
    pub grid: Option<Grid>,
    #[serde(default)]
    pub h_align: HAlign,
    #[serde(default)]
    pub v_align: VAlign,
    pub queue_position: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request body for PUT /api/messages/:id.
/// Accepts either `text` or `grid` to replace the content.
#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub text: Option<String>,
    pub grid: Option<Grid>,
    pub h_align: Option<HAlign>,
    pub v_align: Option<VAlign>,
    pub queue_position: Option<i64>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Request body for POST /api/countdowns.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCountdownRequest {
    pub label: String,
    pub target: DateTime<Utc>,
    #[serde(default = "default_format_template")]
    pub format_template: String,
    #[serde(default)]
    pub zero_behavior: ZeroBehavior,
    pub queue_position: Option<i64>,
}

fn default_format_template() -> String {
    "{D} DAYS  {HH}:{MM}:{SS}".to_string()
}

/// Request body for PUT /api/countdowns/:id.
#[derive(Debug, Deserialize)]
pub struct UpdateCountdownRequest {
    pub label: Option<String>,
    pub target: Option<DateTime<Utc>>,
    pub format_template: Option<String>,
    pub zero_behavior: Option<ZeroBehavior>,
    pub queue_position: Option<i64>,
}

/// Request body for PUT /api/queue/reorder.
#[derive(Debug, Deserialize)]
pub struct ReorderQueueRequest {
    pub order: Vec<Uuid>,
}

/// Request body for PUT /api/config.
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    #[serde(flatten)]
    pub values: serde_json::Map<String, serde_json::Value>,
}

// ── Standard API response wrappers ────────────────────────────────

/// Standard list response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
}

/// Queue list response with rotation info.
#[derive(Debug, Serialize)]
pub struct QueueListResponse {
    pub items: Vec<QueueItem>,
    pub total: usize,
    pub current_index: i64,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub queue_size: usize,
}

/// Standard error response body.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}
