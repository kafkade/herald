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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum CellContent {
    Char(char),
    Color(Color),
    Blank,
}

impl Default for CellContent {
    fn default() -> Self {
        Self::Blank
    }
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
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub queue_position: i64,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Countdowns ────────────────────────────────────────────────────

/// Behavior when a countdown reaches zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ZeroBehavior {
    ShowZero,
    ShowMessage { grid: Grid },
    Remove,
    Pause,
}

impl Default for ZeroBehavior {
    fn default() -> Self {
        Self::ShowZero
    }
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
            grid: Grid::blank(),
            previous_grid: Grid::blank(),
            current_item: None,
            timestamp: Utc::now(),
        }
    }
}

// ── API request/response DTOs ─────────────────────────────────────

/// Request body for POST /api/messages.
#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub grid: Grid,
    #[serde(default)]
    pub h_align: HAlign,
    #[serde(default)]
    pub v_align: VAlign,
    pub queue_position: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request body for PUT /api/messages/:id.
#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub grid: Option<Grid>,
    pub h_align: Option<HAlign>,
    pub v_align: Option<VAlign>,
    pub queue_position: Option<i64>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Request body for POST /api/countdowns.
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Serialize)]
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
