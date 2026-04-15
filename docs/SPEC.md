# Herald — Technical Specification

> The deep technical reference for Herald: board specifications, data models, APIs, rendering, configuration, and error handling.

---

## Table of Contents

- [1. Board Specification](#1-board-specification)
  - [1.1 Grid Dimensions](#11-grid-dimensions)
  - [1.2 Character Set](#12-character-set)
  - [1.3 Color Tiles](#13-color-tiles)
  - [1.4 Coordinate System](#14-coordinate-system)
  - [1.5 Alignment Rules](#15-alignment-rules)
  - [1.6 Unsupported Character Mapping](#16-unsupported-character-mapping)
- [2. Data Models & Persistence](#2-data-models--persistence)
  - [2.1 Shared Rust Types](#21-shared-rust-types)
  - [2.2 SQLite Schema](#22-sqlite-schema)
  - [2.3 Storage Format Justification](#23-storage-format-justification)
  - [2.4 Migration Strategy](#24-migration-strategy)
- [3. REST API (Admin Operations)](#3-rest-api-admin-operations)
  - [3.1 Authentication](#31-authentication)
  - [3.2 Error Format](#32-error-format)
  - [3.3 Endpoints](#33-endpoints)
- [4. WebSocket API (Viewer Connection)](#4-websocket-api-viewer-connection)
  - [4.1 Connection Lifecycle](#41-connection-lifecycle)
  - [4.2 Server → Client Messages](#42-server--client-messages)
  - [4.3 Client → Server Messages](#43-client--server-messages)
  - [4.4 Reconnection Strategy](#44-reconnection-strategy)
- [5. Rotation & Queue Logic](#5-rotation--queue-logic)
  - [5.1 Queue Ordering](#51-queue-ordering)
  - [5.2 Rotation Timer](#52-rotation-timer)
  - [5.3 Empty Queue Behavior](#53-empty-queue-behavior)
  - [5.4 Countdown Rendering](#54-countdown-rendering)
  - [5.5 Countdown at Zero](#55-countdown-at-zero)
  - [5.6 Countdown Live Refresh](#56-countdown-live-refresh)
  - [5.7 Expired Message Handling](#57-expired-message-handling)
- [6. Split-Flap Rendering: Terminal (ratatui)](#6-split-flap-rendering-terminal-ratatui)
  - [6.1 Grid Cell Rendering](#61-grid-cell-rendering)
  - [6.2 Flip Animation](#62-flip-animation)
  - [6.3 Color Tile Rendering](#63-color-tile-rendering)
  - [6.4 TUI Layout](#64-tui-layout)
  - [6.5 Terminal Resize Handling](#65-terminal-resize-handling)
  - [6.6 Performance Targets](#66-performance-targets)
  - [6.7 ASCII Mockup](#67-ascii-mockup)
- [7. Split-Flap Rendering: Web (Leptos + Wasm)](#7-split-flap-rendering-web-leptos--wasm)
  - [7.1 Flap Tile HTML Structure](#71-flap-tile-html-structure)
  - [7.2 CSS Flip Animation](#72-css-flip-animation)
  - [7.3 Shadow & Depth](#73-shadow--depth)
  - [7.4 Color Tiles (Web)](#74-color-tiles-web)
  - [7.5 Responsive Design](#75-responsive-design)
  - [7.6 Sound Effects](#76-sound-effects)
  - [7.7 Loading State](#77-loading-state)
  - [7.8 Performance](#78-performance)
- [8. Admin Interface & Workflow](#8-admin-interface--workflow)
  - [8.1 CLI Subcommands](#81-cli-subcommands)
  - [8.2 Web Admin Panel](#82-web-admin-panel)
  - [8.3 Color Markup for CLI Push](#83-color-markup-for-cli-push)
- [9. Configuration Reference](#9-configuration-reference)
  - [9.1 Parameter Table](#91-parameter-table)
  - [9.2 Example herald.toml](#92-example-heraldtoml)
- [10. Error Handling & Resilience](#10-error-handling--resilience)
  - [10.1 WebSocket Disconnection](#101-websocket-disconnection)
  - [10.2 Database Failures](#102-database-failures)
  - [10.3 Malformed Requests](#103-malformed-requests)
  - [10.4 Rate Limiting](#104-rate-limiting)
  - [10.5 Graceful Shutdown](#105-graceful-shutdown)

---

## 1. Board Specification

### 1.1 Grid Dimensions

The board is a fixed **6 rows × 22 columns** grid, yielding **132 flap positions** in total. This matches the physical Vestaboard format exactly. The grid is never resized dynamically — all messages and countdowns must be composed to fit within these bounds.

### 1.2 Character Set

Every flap position displays exactly one of the supported characters or a color tile. The character set uses an internal index for serialization and for driving the flip animation (flaps cycle through indices sequentially).

| Index | Character | Description            |
|------:|-----------|------------------------|
|     0 | ` `       | Blank / space          |
|     1 | `A`       | Latin uppercase A      |
|     2 | `B`       | Latin uppercase B      |
|     3 | `C`       | Latin uppercase C      |
|     4 | `D`       | Latin uppercase D      |
|     5 | `E`       | Latin uppercase E      |
|     6 | `F`       | Latin uppercase F      |
|     7 | `G`       | Latin uppercase G      |
|     8 | `H`       | Latin uppercase H      |
|     9 | `I`       | Latin uppercase I      |
|    10 | `J`       | Latin uppercase J      |
|    11 | `K`       | Latin uppercase K      |
|    12 | `L`       | Latin uppercase L      |
|    13 | `M`       | Latin uppercase M      |
|    14 | `N`       | Latin uppercase N      |
|    15 | `O`       | Latin uppercase O      |
|    16 | `P`       | Latin uppercase P      |
|    17 | `Q`       | Latin uppercase Q      |
|    18 | `R`       | Latin uppercase R      |
|    19 | `S`       | Latin uppercase S      |
|    20 | `T`       | Latin uppercase T      |
|    21 | `U`       | Latin uppercase U      |
|    22 | `V`       | Latin uppercase V      |
|    23 | `W`       | Latin uppercase W      |
|    24 | `X`       | Latin uppercase X      |
|    25 | `Y`       | Latin uppercase Y      |
|    26 | `Z`       | Latin uppercase Z      |
|    27 | `0`       | Digit zero             |
|    28 | `1`       | Digit one              |
|    29 | `2`       | Digit two              |
|    30 | `3`       | Digit three            |
|    31 | `4`       | Digit four             |
|    32 | `5`       | Digit five             |
|    33 | `6`       | Digit six              |
|    34 | `7`       | Digit seven            |
|    35 | `8`       | Digit eight            |
|    36 | `9`       | Digit nine             |
|    37 | `.`       | Period / full stop     |
|    38 | `,`       | Comma                  |
|    39 | `!`       | Exclamation mark       |
|    40 | `?`       | Question mark          |
|    41 | `:`       | Colon                  |
|    42 | `-`       | Hyphen / dash          |
|    43 | `/`       | Forward slash          |
|    44 | `@`       | At sign                |
|    45 | `#`       | Pound / hash           |
|    46 | `$`       | Dollar sign            |
|    47 | `°`       | Degree symbol          |

**Total printable characters:** 48 (indices 0–47).

```rust
/// Internal character index for flap serialization and animation cycling.
/// Values 0–47 map to supported characters. Values 100+ map to color tiles.
pub type CharIndex = u8;

pub const CHAR_BLANK: CharIndex = 0;
pub const CHAR_A: CharIndex = 1;
// ... through CHAR_Z = 26
pub const CHAR_0: CharIndex = 27;
// ... through CHAR_9 = 36
pub const CHAR_PERIOD: CharIndex = 37;
pub const CHAR_COMMA: CharIndex = 38;
pub const CHAR_EXCLAMATION: CharIndex = 39;
pub const CHAR_QUESTION: CharIndex = 40;
pub const CHAR_COLON: CharIndex = 41;
pub const CHAR_HYPHEN: CharIndex = 42;
pub const CHAR_SLASH: CharIndex = 43;
pub const CHAR_AT: CharIndex = 44;
pub const CHAR_HASH: CharIndex = 45;
pub const CHAR_DOLLAR: CharIndex = 46;
pub const CHAR_DEGREE: CharIndex = 47;

pub const MAX_CHAR_INDEX: CharIndex = 47;
```

During a flip animation, a flap cycles through indices from its current value to its target value (wrapping from 47 → 0 if the target is lower), stepping one index at a time. This simulates the physical drum rotation of a split-flap display.

### 1.3 Color Tiles

Color tiles occupy a flap position and display a solid color fill with no character. They use index values starting at 100 to keep them distinct from the character range.

| Index | Enum Value | Display Name | Web Hex   | Terminal (ANSI 256) | RGB             |
|------:|------------|--------------|-----------|---------------------|-----------------|
|   100 | `Red`      | Red          | `#D32F2F` | 196                 | (211, 47, 47)   |
|   101 | `Orange`   | Orange       | `#F57C00` | 208                 | (245, 124, 0)   |
|   102 | `Yellow`   | Yellow       | `#FDD835` | 220                 | (253, 216, 53)  |
|   103 | `Green`    | Green        | `#388E3C` | 34                  | (56, 142, 60)   |
|   104 | `Blue`     | Blue         | `#1976D2` | 33                  | (25, 118, 210)  |
|   105 | `Violet`   | Violet       | `#7B1FA2` | 128                 | (123, 31, 162)  |
|   106 | `White`    | White        | `#FAFAFA` | 231                 | (250, 250, 250) |
|   107 | `Black`    | Black        | `#212121` | 234                 | (33, 33, 33)    |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl Color {
    /// Returns the internal index used for serialization.
    pub const fn index(self) -> u8 {
        match self {
            Color::Red    => 100,
            Color::Orange => 101,
            Color::Yellow => 102,
            Color::Green  => 103,
            Color::Blue   => 104,
            Color::Violet => 105,
            Color::White  => 106,
            Color::Black  => 107,
        }
    }

    /// Returns the CSS hex color string for web rendering.
    pub const fn hex(self) -> &'static str {
        match self {
            Color::Red    => "#D32F2F",
            Color::Orange => "#F57C00",
            Color::Yellow => "#FDD835",
            Color::Green  => "#388E3C",
            Color::Blue   => "#1976D2",
            Color::Violet => "#7B1FA2",
            Color::White  => "#FAFAFA",
            Color::Black  => "#212121",
        }
    }

    /// Returns the ANSI 256-color code for terminal rendering.
    pub const fn ansi256(self) -> u8 {
        match self {
            Color::Red    => 196,
            Color::Orange => 208,
            Color::Yellow => 220,
            Color::Green  => 34,
            Color::Blue   => 33,
            Color::Violet => 128,
            Color::White  => 231,
            Color::Black  => 234,
        }
    }
}
```

### 1.4 Coordinate System

Grid positions use zero-indexed row and column values:

- **Rows:** 0 (top) through 5 (bottom)
- **Columns:** 0 (left) through 21 (right)

```
        Col 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21
Row 0:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
Row 1:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
Row 2:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
Row 3:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
Row 4:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
Row 5:  [ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ]
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPosition {
    /// Row index, 0–5 (top to bottom).
    pub row: u8,
    /// Column index, 0–21 (left to right).
    pub col: u8,
}

impl GridPosition {
    pub const ROWS: u8 = 6;
    pub const COLS: u8 = 22;

    pub fn new(row: u8, col: u8) -> Option<Self> {
        if row < Self::ROWS && col < Self::COLS {
            Some(Self { row, col })
        } else {
            None
        }
    }
}
```

### 1.5 Alignment Rules

Alignment is specified per-message and controls how content shorter than the full grid is positioned.

**Horizontal alignment** (applied per row):

| Value    | Behavior                                                                                      |
|----------|-----------------------------------------------------------------------------------------------|
| `Left`   | Content starts at column 0. Remaining columns padded with blanks.                             |
| `Center` | Content centered horizontally. Odd remainder puts the extra blank on the right.               |
| `Right`  | Content ends at column 21. Leading columns padded with blanks.                                |

**Vertical alignment** (applied to the overall message when it uses fewer than 6 rows):

| Value    | Behavior                                                                                      |
|----------|-----------------------------------------------------------------------------------------------|
| `Top`    | Content starts at row 0. Remaining rows filled with blanks.                                   |
| `Middle` | Content centered vertically. Odd remainder puts the extra blank row at the bottom.            |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VAlign {
    Top,
    Middle,
}
```

**Default alignment:** `HAlign::Center`, `VAlign::Middle`. Overridable per-message and via global config.

### 1.6 Unsupported Character Mapping

When input text contains characters outside the supported set, the following mapping rules apply in order:

1. **Lowercase → Uppercase:** `a`–`z` → `A`–`Z`
2. **Typographic equivalents:** `'` `'` `"` `"` → blank (no direct equivalent), `–` `—` → `-`, `…` → `...` (three cells)
3. **Accented Latin → Base:** `É` → `E`, `ñ` → `N`, `ü` → `U`, etc. (strip combining marks / use Unicode NFKD decomposition, keep the base letter if it is A–Z)
4. **Everything else → Blank:** Any character with no mapping becomes index 0 (blank/space)

This mapping is applied automatically by the backend when processing CLI `push` commands that use text input. When the admin provides a raw 6×22 grid via the API, no mapping is applied — invalid indices are rejected with a 400 error.

---

## 2. Data Models & Persistence

### 2.1 Shared Rust Types

All types below live in the `herald-common` crate and are shared across backend, CLI, and web.

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The content of a single flap cell: either a character or a color tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CellContent {
    /// A character from the supported set, stored as its index (0–47).
    Char(u8),
    /// A solid color tile.
    Color(Color),
}

impl Default for CellContent {
    fn default() -> Self {
        CellContent::Char(0) // blank
    }
}

/// A complete 6×22 board grid.
/// Stored as a row-major array: grid[row][col].
pub type Grid = [[CellContent; 22]; 6];

/// Full board state as transmitted over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
    /// The current 6×22 grid of cell contents.
    pub grid: Grid,
    /// The previous grid state, for animation diffing.
    /// `None` on initial connection (no previous state).
    pub previous_grid: Option<Grid>,
    /// Metadata about the currently displayed queue item.
    pub current_item: Option<QueueItemInfo>,
    /// Server timestamp when this state was generated.
    pub timestamp: DateTime<Utc>,
}

/// Lightweight metadata about a queue item (sent with board state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemInfo {
    pub id: Uuid,
    pub kind: QueueItemKind,
    /// Human-readable label (message preview or countdown label).
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueItemKind {
    Message,
    Countdown,
}

/// A message in the rotation queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    /// The full 6×22 grid content for this message.
    pub grid: Grid,
    /// Horizontal alignment used when this message was composed.
    pub h_align: HAlign,
    /// Vertical alignment used when this message was composed.
    pub v_align: VAlign,
    /// Position in the rotation queue (lower = earlier).
    pub queue_position: i32,
    /// When this message was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry time. Message is auto-removed after this.
    pub expires_at: Option<DateTime<Utc>>,
}

/// A countdown timer in the rotation queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Countdown {
    pub id: Uuid,
    /// Display label (e.g., "DAYS UNTIL LAUNCH").
    pub label: String,
    /// The target date/time for the countdown.
    pub target: DateTime<Utc>,
    /// Template string for formatting the remaining time.
    /// See §5.4 for the template syntax.
    pub format_template: String,
    /// What to do when the countdown reaches zero.
    pub zero_behavior: ZeroBehavior,
    /// Position in the rotation queue.
    pub queue_position: i32,
    /// When this countdown was created.
    pub created_at: DateTime<Utc>,
}

/// Behavior when a countdown reaches zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ZeroBehavior {
    /// Display "00:00:00" (or equivalent per format template).
    ShowZero,
    /// Display a custom celebration message grid.
    ShowMessage { grid: Grid },
    /// Automatically remove the countdown from the queue.
    Remove,
    /// Stop at zero and keep in rotation indefinitely.
    Pause,
}

/// A unified queue item (either a message or a countdown).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum QueueItem {
    Message(Message),
    Countdown(Countdown),
}
```

### 2.2 SQLite Schema

Herald uses SQLite for persistence. All tables are created on first startup via embedded migrations.

```sql
-- Messages in the rotation queue.
-- Grid content is stored as a JSON blob: a 6-element array of 22-element arrays,
-- where each element is a CellContent JSON object.
CREATE TABLE messages (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4 as text
    grid            TEXT NOT NULL,               -- JSON: [[CellContent; 22]; 6]
    h_align         TEXT NOT NULL DEFAULT 'center',  -- 'left' | 'center' | 'right'
    v_align         TEXT NOT NULL DEFAULT 'middle',  -- 'top' | 'middle'
    queue_position  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,               -- ISO 8601 UTC timestamp
    expires_at      TEXT,                        -- ISO 8601 UTC timestamp, nullable
    CONSTRAINT valid_h_align CHECK (h_align IN ('left', 'center', 'right')),
    CONSTRAINT valid_v_align CHECK (v_align IN ('top', 'middle'))
);

CREATE INDEX idx_messages_queue_position ON messages(queue_position);
CREATE INDEX idx_messages_expires_at ON messages(expires_at) WHERE expires_at IS NOT NULL;

-- Countdowns in the rotation queue.
CREATE TABLE countdowns (
    id              TEXT PRIMARY KEY NOT NULL,   -- UUID v4 as text
    label           TEXT NOT NULL,               -- Display label, max 44 chars (2 rows × 22)
    target          TEXT NOT NULL,               -- ISO 8601 UTC target datetime
    format_template TEXT NOT NULL DEFAULT '{D} DAYS  {HH}:{MM}:{SS}',
    zero_behavior   TEXT NOT NULL DEFAULT '{"action":"show_zero"}',  -- JSON ZeroBehavior
    queue_position  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,               -- ISO 8601 UTC timestamp
    CONSTRAINT label_length CHECK (length(label) <= 44)
);

CREATE INDEX idx_countdowns_queue_position ON countdowns(queue_position);

-- Key-value configuration store.
CREATE TABLE configuration (
    key             TEXT PRIMARY KEY NOT NULL,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL                -- ISO 8601 UTC timestamp
);

-- Rotation state (singleton row, always id = 1).
CREATE TABLE rotation_state (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    current_index   INTEGER NOT NULL DEFAULT 0,  -- Index into the merged, sorted queue
    last_rotation   TEXT NOT NULL                 -- ISO 8601 UTC timestamp of last rotation
);

-- Seed the rotation state row.
INSERT INTO rotation_state (id, current_index, last_rotation)
VALUES (1, 0, '1970-01-01T00:00:00Z');

-- Seed default configuration.
INSERT INTO configuration (key, value, updated_at) VALUES
    ('rotation_interval_seconds', '30', '1970-01-01T00:00:00Z'),
    ('countdown_refresh_seconds', '1', '1970-01-01T00:00:00Z'),
    ('default_h_align', 'center', '1970-01-01T00:00:00Z'),
    ('default_v_align', 'middle', '1970-01-01T00:00:00Z'),
    ('admin_enabled', 'true', '1970-01-01T00:00:00Z');
```

### 2.3 Storage Format Justification

**Decision: Store board content as a serialized 6×22 JSON grid (blob) rather than 132 individual cell rows.**

Rationale:

1. **Atomicity.** A message is always a complete board snapshot. Storing 132 rows per message adds transactional complexity with no benefit — we never query or update individual cells independently.
2. **Read performance.** Loading a board state is a single row fetch + JSON deserialize, not a 132-row join. At Herald's scale (dozens to hundreds of messages), this is a single-digit-millisecond operation.
3. **Write simplicity.** Inserting or updating a message is one `INSERT`/`UPDATE` statement. No need to manage 132 child rows.
4. **Schema stability.** The grid dimensions are fixed at 6×22. If they ever changed (unlikely — it's a fundamental design constraint), the JSON blob format adapts without schema migration, whereas a cell-per-row design would require restructuring.
5. **Trade-off acknowledged.** We lose the ability to do SQL queries like "find all messages containing the letter A in position (2, 5)." This is acceptable — Herald has no need for cell-level querying. If analytics were ever needed, they could be computed application-side from the deserialized grid.

See [DECISIONS.md](./DECISIONS.md) for the full ADR.

### 2.4 Migration Strategy

Migrations are embedded in the backend binary using `sqlx::migrate!()` (the `migrations/` directory is compiled into the binary at build time). On every startup, the backend:

1. Opens (or creates) the SQLite database file at the configured path.
2. Runs all pending migrations in order. `sqlx` tracks applied migrations in its internal `_sqlx_migrations` table.
3. If a migration fails, the backend logs the error and exits with a non-zero status code. It does **not** attempt to roll back or skip the failed migration.

Migration files follow the naming convention:

```
migrations/
├── 20250101000000_initial_schema.sql
├── 20250201000000_add_countdown_table.sql
└── ...
```

This approach means:
- No external migration tool is required.
- The binary is always self-sufficient — it carries its own schema.
- Downgrade is not supported (intentional simplicity). To roll back, restore from a database backup.

---

## 3. REST API (Admin Operations)

### 3.1 Authentication

All endpoints **except** `GET /api/health` require authentication via bearer token:

```
Authorization: Bearer <admin_token>
```

The token is configured via `auth.admin_token` in `herald.toml` or the `HERALD_ADMIN_TOKEN` environment variable. If the header is missing or the token does not match, the server responds:

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Missing or invalid Authorization header"
  }
}
```

**HTTP Status:** `401 Unauthorized`

### 3.2 Error Format

All error responses use a consistent JSON structure:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable description of what went wrong"
  }
}
```

Standard error codes:

| Code                | HTTP Status | Description                              |
|---------------------|-------------|------------------------------------------|
| `UNAUTHORIZED`      | 401         | Missing or invalid auth token            |
| `NOT_FOUND`         | 404         | Resource does not exist                  |
| `VALIDATION_ERROR`  | 400         | Request body fails validation            |
| `CONFLICT`          | 409         | Resource conflict (e.g., duplicate ID)   |
| `INTERNAL_ERROR`    | 500         | Unexpected server error                  |
| `SERVICE_UNAVAILABLE` | 503       | Database or dependency unavailable       |
| `RATE_LIMITED`      | 429         | Too many requests                        |

### 3.3 Endpoints

---

#### `POST /api/messages` — Create a message

Push a new message to the rotation queue.

**Request:**

```json
{
  "grid": [
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":8},{"type":"char","value":5},{"type":"char","value":12},{"type":"char","value":12},{"type":"char","value":15},{"type":"char","value":0},{"type":"char","value":23},{"type":"char","value":15},{"type":"char","value":18},{"type":"char","value":12},{"type":"char","value":4},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}]
  ],
  "h_align": "center",
  "v_align": "middle",
  "queue_position": null,
  "expires_at": "2025-12-31T23:59:59Z"
}
```

> Above example: "HELLO WORLD" centered on row 2. Most cells are blank (char index 0).

**Validation rules:**
- `grid` must be exactly 6 arrays of exactly 22 `CellContent` objects.
- Each `CellContent` must be `{"type":"char","value":N}` where N is 0–47, or `{"type":"color","value":"red"|"orange"|...}`.
- `h_align` must be `"left"`, `"center"`, or `"right"`. Optional; defaults to server config.
- `v_align` must be `"top"` or `"middle"`. Optional; defaults to server config.
- `queue_position` is optional. If null, appended to end of queue (max position + 1).
- `expires_at` is optional. If provided, must be a valid ISO 8601 UTC timestamp in the future.

**Response (201 Created):**

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "grid": [[ "..." ]],
  "h_align": "center",
  "v_align": "middle",
  "queue_position": 3,
  "created_at": "2025-07-14T10:30:00Z",
  "expires_at": "2025-12-31T23:59:59Z"
}
```

**Error responses:**

| Status | Code               | Condition                                      |
|--------|--------------------|-------------------------------------------------|
| 400    | `VALIDATION_ERROR` | Invalid grid dimensions, unknown char index, etc. |
| 401    | `UNAUTHORIZED`     | Missing or invalid token                        |
| 500    | `INTERNAL_ERROR`   | Database write failure                          |

---

#### `GET /api/messages` — List all messages

Returns all messages in the rotation queue, sorted by `queue_position` ascending.

**Request:** No body. No query parameters.

**Response (200 OK):**

```json
{
  "messages": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "grid": [[ "..." ]],
      "h_align": "center",
      "v_align": "middle",
      "queue_position": 0,
      "created_at": "2025-07-14T10:30:00Z",
      "expires_at": null
    },
    {
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "grid": [[ "..." ]],
      "h_align": "left",
      "v_align": "top",
      "queue_position": 1,
      "created_at": "2025-07-14T11:00:00Z",
      "expires_at": "2025-12-31T23:59:59Z"
    }
  ],
  "total": 2
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `GET /api/messages/:id` — Get a specific message

**Request:** No body. `:id` is a UUID path parameter.

**Response (200 OK):**

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "grid": [[ "..." ]],
  "h_align": "center",
  "v_align": "middle",
  "queue_position": 0,
  "created_at": "2025-07-14T10:30:00Z",
  "expires_at": null
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 404    | `NOT_FOUND`      | No message with this ID  |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `PUT /api/messages/:id` — Update a message

**Request:**

```json
{
  "grid": [[ "..." ]],
  "h_align": "right",
  "v_align": "top",
  "queue_position": 1,
  "expires_at": "2026-01-01T00:00:00Z"
}
```

All fields are optional. Only provided fields are updated; omitted fields retain their current values.

**Response (200 OK):**

```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "grid": [[ "..." ]],
  "h_align": "right",
  "v_align": "top",
  "queue_position": 1,
  "created_at": "2025-07-14T10:30:00Z",
  "expires_at": "2026-01-01T00:00:00Z"
}
```

**Error responses:**

| Status | Code               | Condition                          |
|--------|--------------------|------------------------------------|
| 400    | `VALIDATION_ERROR` | Invalid field values               |
| 401    | `UNAUTHORIZED`     | Missing or invalid token           |
| 404    | `NOT_FOUND`        | No message with this ID            |
| 500    | `INTERNAL_ERROR`   | Database write failure             |

---

#### `DELETE /api/messages/:id` — Delete a message

**Request:** No body.

**Response (204 No Content):** Empty body.

If the deleted message is currently displayed, the server immediately advances to the next queue item and broadcasts the new board state.

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 404    | `NOT_FOUND`      | No message with this ID  |
| 500    | `INTERNAL_ERROR` | Database write failure   |

---

#### `PUT /api/queue/reorder` — Reorder the rotation queue

Sets new positions for all queue items (messages and countdowns).

**Request:**

```json
{
  "order": [
    "b2c3d4e5-f6a7-8901-bcde-f12345678901",
    "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "c3d4e5f6-a7b8-9012-cdef-123456789012"
  ]
}
```

The `order` array must contain the IDs of **every** item currently in the queue (messages and countdowns combined). The position of each ID in the array becomes its new `queue_position` (0-indexed).

**Response (200 OK):**

```json
{
  "reordered": 3
}
```

**Error responses:**

| Status | Code               | Condition                                        |
|--------|--------------------|--------------------------------------------------|
| 400    | `VALIDATION_ERROR` | Missing IDs, extra IDs, or duplicate IDs         |
| 401    | `UNAUTHORIZED`     | Missing or invalid token                         |
| 500    | `INTERNAL_ERROR`   | Database write failure                           |

---

#### `POST /api/countdowns` — Create a countdown

**Request:**

```json
{
  "label": "DAYS UNTIL LAUNCH",
  "target": "2025-12-31T00:00:00Z",
  "format_template": "{D} DAYS  {HH}:{MM}:{SS}",
  "zero_behavior": {
    "action": "show_message",
    "data": {
      "grid": [[ "...celebration grid..." ]]
    }
  },
  "queue_position": null
}
```

**Validation rules:**
- `label`: required, max 44 characters (must fit in 2 rows × 22 cols).
- `target`: required, valid ISO 8601 UTC datetime. May be in the past (countdown immediately at zero).
- `format_template`: optional (defaults to `"{D} DAYS  {HH}:{MM}:{SS}"`). See [§5.4](#54-countdown-rendering) for template syntax.
- `zero_behavior`: optional (defaults to `{"action":"show_zero"}`).
- `queue_position`: optional (defaults to end of queue).

**Response (201 Created):**

```json
{
  "id": "c3d4e5f6-a7b8-9012-cdef-123456789012",
  "label": "DAYS UNTIL LAUNCH",
  "target": "2025-12-31T00:00:00Z",
  "format_template": "{D} DAYS  {HH}:{MM}:{SS}",
  "zero_behavior": { "action": "show_message", "data": { "grid": [[ "..." ]] } },
  "queue_position": 4,
  "created_at": "2025-07-14T12:00:00Z"
}
```

**Error responses:**

| Status | Code               | Condition                           |
|--------|--------------------|-------------------------------------|
| 400    | `VALIDATION_ERROR` | Invalid label, target, or template  |
| 401    | `UNAUTHORIZED`     | Missing or invalid token            |
| 500    | `INTERNAL_ERROR`   | Database write failure              |

---

#### `GET /api/countdowns` — List all countdowns

**Response (200 OK):**

```json
{
  "countdowns": [
    {
      "id": "c3d4e5f6-a7b8-9012-cdef-123456789012",
      "label": "DAYS UNTIL LAUNCH",
      "target": "2025-12-31T00:00:00Z",
      "format_template": "{D} DAYS  {HH}:{MM}:{SS}",
      "zero_behavior": { "action": "show_zero" },
      "queue_position": 2,
      "created_at": "2025-07-14T12:00:00Z"
    }
  ],
  "total": 1
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `PUT /api/countdowns/:id` — Update a countdown

**Request:**

```json
{
  "label": "LAUNCH DAY",
  "target": "2026-01-15T00:00:00Z",
  "zero_behavior": { "action": "remove" }
}
```

All fields are optional. Only provided fields are updated.

**Response (200 OK):** Full updated countdown object (same shape as create response).

**Error responses:**

| Status | Code               | Condition                |
|--------|--------------------|--------------------------|
| 400    | `VALIDATION_ERROR` | Invalid field values     |
| 401    | `UNAUTHORIZED`     | Missing or invalid token |
| 404    | `NOT_FOUND`        | No countdown with this ID |
| 500    | `INTERNAL_ERROR`   | Database write failure   |

---

#### `DELETE /api/countdowns/:id` — Delete a countdown

**Request:** No body.

**Response (204 No Content):** Empty body.

Same behavior as message deletion: if the countdown is currently displayed, advance immediately.

**Error responses:**

| Status | Code             | Condition                  |
|--------|------------------|----------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token   |
| 404    | `NOT_FOUND`      | No countdown with this ID  |
| 500    | `INTERNAL_ERROR` | Database write failure     |

---

#### `GET /api/board` — Get current board state

Returns what is currently displayed on the board. No auth required for this endpoint as an alternative to WebSocket for simple polling clients, but auth is still enforced for consistency with other admin endpoints.

**Response (200 OK):**

```json
{
  "grid": [
    [{"type":"char","value":0}, "...21 more cells..."],
    ["...row 1..."],
    ["...row 2..."],
    ["...row 3..."],
    ["...row 4..."],
    ["...row 5..."]
  ],
  "current_item": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "kind": "message",
    "label": "HELLO WORLD"
  },
  "timestamp": "2025-07-14T10:30:45Z"
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `GET /api/queue` — Get full rotation queue

Returns messages and countdowns interleaved, sorted by `queue_position`.

**Response (200 OK):**

```json
{
  "items": [
    {
      "kind": "Message",
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "queue_position": 0,
      "label": "HELLO WORLD",
      "expires_at": null
    },
    {
      "kind": "Countdown",
      "id": "c3d4e5f6-a7b8-9012-cdef-123456789012",
      "queue_position": 1,
      "label": "DAYS UNTIL LAUNCH",
      "target": "2025-12-31T00:00:00Z"
    },
    {
      "kind": "Message",
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "queue_position": 2,
      "label": "WELCOME HOME",
      "expires_at": "2025-12-31T23:59:59Z"
    }
  ],
  "total": 3,
  "current_index": 0
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `PUT /api/config` — Update configuration

**Request:**

```json
{
  "rotation_interval_seconds": 45,
  "countdown_refresh_seconds": 1,
  "default_h_align": "left",
  "default_v_align": "top"
}
```

All fields are optional. Only provided fields are updated. Changes take effect immediately (runtime-changeable parameters).

**Response (200 OK):**

```json
{
  "rotation_interval_seconds": 45,
  "countdown_refresh_seconds": 1,
  "default_h_align": "left",
  "default_v_align": "top",
  "admin_enabled": true
}
```

**Error responses:**

| Status | Code               | Condition                |
|--------|--------------------|--------------------------|
| 400    | `VALIDATION_ERROR` | Invalid parameter value  |
| 401    | `UNAUTHORIZED`     | Missing or invalid token |
| 500    | `INTERNAL_ERROR`   | Database write failure   |

---

#### `GET /api/config` — Get current configuration

**Response (200 OK):**

```json
{
  "rotation_interval_seconds": 30,
  "countdown_refresh_seconds": 1,
  "default_h_align": "center",
  "default_v_align": "middle",
  "admin_enabled": true
}
```

**Error responses:**

| Status | Code             | Condition                |
|--------|------------------|--------------------------|
| 401    | `UNAUTHORIZED`   | Missing or invalid token |
| 500    | `INTERNAL_ERROR` | Database read failure    |

---

#### `GET /api/health` — Health check

**No authentication required.**

**Response (200 OK):**

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3621,
  "connected_viewers": 4,
  "queue_size": 7
}
```

**Error responses:**

| Status | Code                  | Condition                |
|--------|-----------------------|--------------------------|
| 503    | `SERVICE_UNAVAILABLE` | Database not reachable   |

---

## 4. WebSocket API (Viewer Connection)

### 4.1 Connection Lifecycle

**Endpoint:** `GET /ws`

The WebSocket endpoint requires no authentication — it is the public viewer connection. Any client can connect and receive board updates.

1. Client sends HTTP upgrade request to `/ws`.
2. Server upgrades to WebSocket.
3. Server immediately sends a `board_update` message with the current board state (`previous_grid` is `null` since there is no prior state for this client).
4. Server adds the connection to the broadcast pool.
5. Server sends `heartbeat` messages every 30 seconds.
6. Client responds with `pong` messages to keep the connection alive.
7. When the board state changes (rotation, admin push, countdown tick), the server sends a `board_update` to all connected clients.
8. On disconnect, the server removes the client from the broadcast pool and cleans up resources.

### 4.2 Server → Client Messages

All messages are JSON objects with a `type` field discriminator.

---

#### `board_update`

Sent whenever the displayed board state changes: on rotation, admin push, countdown tick, or initial connection.

```json
{
  "type": "board_update",
  "grid": [
    [{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":8},{"type":"char","value":5},{"type":"char","value":12},{"type":"char","value":12},{"type":"char","value":15},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0},{"type":"char","value":0}],
    ["...row 1..."],
    ["...row 2..."],
    ["...row 3..."],
    ["...row 4..."],
    ["...row 5..."]
  ],
  "previous_grid": [
    ["...previous row 0 (or null on first connect)..."],
    ["..."],
    ["..."],
    ["..."],
    ["..."],
    ["..."]
  ],
  "current_item": {
    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "kind": "message",
    "label": "HELLO"
  },
  "timestamp": "2025-07-14T10:30:45Z"
}
```

The `previous_grid` field enables clients to compute per-cell diffs for animation. Clients compare `previous_grid[r][c]` to `grid[r][c]` — only cells that differ need to play the flip animation. If `previous_grid` is `null` (initial connect), the client should animate all cells from blank to their target value.

---

#### `heartbeat`

Sent every 30 seconds to keep the connection alive and detect stale clients.

```json
{
  "type": "heartbeat",
  "server_time": "2025-07-14T10:31:15Z"
}
```

---

#### `queue_info`

Sent alongside every `board_update` as a separate message, providing rotation metadata for status bar rendering.

```json
{
  "type": "queue_info",
  "current_index": 2,
  "total_items": 7,
  "next_rotation_seconds": 18,
  "is_countdown_active": false
}
```

When `is_countdown_active` is `true`, the client knows the board is currently showing a countdown that refreshes every second (the `next_rotation_seconds` still reflects the overall rotation timer).

### 4.3 Client → Server Messages

---

#### `pong`

Response to a `heartbeat`. If the server does not receive a `pong` within 60 seconds of sending a `heartbeat`, it closes the connection.

```json
{
  "type": "pong"
}
```

### 4.4 Reconnection Strategy

Clients (both CLI and web) must implement reconnection with exponential backoff:

| Attempt | Delay   |
|---------|---------|
| 1       | 1s      |
| 2       | 2s      |
| 3       | 4s      |
| 4       | 8s      |
| 5       | 16s     |
| 6+      | 30s max |

On successful reconnection, the backoff resets. During reconnection attempts, the client should:
- Display a "Reconnecting..." indicator in the status bar.
- Keep the last-known board state displayed (do not clear the grid).
- On reconnect, the server sends a fresh `board_update` which the client renders with animation.

---

## 5. Rotation & Queue Logic

### 5.1 Queue Ordering

All messages and countdowns share a single rotation queue. Each item has an integer `queue_position` field:

- Items are sorted by `queue_position` ascending. Lower values display first.
- When a new item is created without an explicit `queue_position`, it receives `MAX(queue_position) + 1` (appended to end).
- When reordering via `PUT /api/queue/reorder`, items receive positions 0, 1, 2, ... in the order specified.
- Ties in `queue_position` are broken by `created_at` ascending (older first).

The unified queue is computed at runtime by querying both the `messages` and `countdowns` tables, sorting by `queue_position`, and merging into a single ordered list.

### 5.2 Rotation Timer

The rotation timer is implemented server-side using `tokio::time::interval`:

```rust
// Pseudocode for the rotation loop
let mut interval = tokio::time::interval(Duration::from_secs(rotation_interval));
loop {
    interval.tick().await;
    let next_state = advance_to_next_queue_item().await;
    broadcast_to_all_viewers(next_state).await;
    persist_rotation_state().await;
}
```

When configuration changes the rotation interval at runtime, the interval is reset:

```rust
interval = tokio::time::interval(Duration::from_secs(new_interval));
```

### 5.3 Empty Queue Behavior

When the rotation queue is empty (no messages or countdowns), the board displays a default splash screen:

```
                      
                      
       H E R A L D   
                      
                      
                      
```

This is the word "HERALD" centered on row 2 using `HAlign::Center`, `VAlign::Middle`. The splash board is generated programmatically (not stored in the database). It persists until the admin pushes the first message.

### 5.4 Countdown Rendering

When a countdown is the active queue item, the server renders it onto the 6×22 grid using the countdown's `label` and `format_template`.

**Grid layout for countdowns:**

```
Row 0:  [        label row 1 (first 22 chars)        ]
Row 1:  [        label row 2 (chars 23–44, if any)    ]
Row 2:  [                    blank                     ]
Row 3:  [        formatted time remaining row 1        ]
Row 4:  [        formatted time remaining row 2        ]
Row 5:  [                    blank                     ]
```

- **Rows 0–1:** The countdown `label`, split at 22 characters. Aligned according to the server's default `h_align`.
- **Row 2:** Blank separator.
- **Rows 3–4:** The formatted time remaining, computed from `format_template`. If the result fits in one row (≤22 chars), it goes on row 3; row 4 is blank. If it exceeds 22 chars, it wraps to row 4.
- **Row 5:** Blank.

**Format template syntax:**

Templates use `{PLACEHOLDER}` tokens that are replaced with computed values:

| Token   | Description                                  | Example Output |
|---------|----------------------------------------------|----------------|
| `{D}`   | Total days remaining (no leading zeros)      | `42`           |
| `{DD}`  | Days with leading zero (2 digits min)        | `04`           |
| `{DDD}` | Days with leading zeros (3 digits min)       | `042`          |
| `{H}`   | Hours component (0–23, no leading zero)      | `7`            |
| `{HH}`  | Hours component (00–23, leading zero)        | `07`           |
| `{M}`   | Minutes component (0–59, no leading zero)    | `5`            |
| `{MM}`  | Minutes component (00–59, leading zero)      | `05`           |
| `{S}`   | Seconds component (0–59, no leading zero)    | `9`            |
| `{SS}`  | Seconds component (00–59, leading zero)      | `09`           |

All literal text in the template is rendered as-is (subject to the character set mapping from [§1.6](#16-unsupported-character-mapping)).

**Example:**

Template: `{DDD} DAYS  {HH}:{MM}:{SS}`
With 42 days, 7 hours, 31 minutes, 15 seconds remaining:

```
Row 3:  042 DAYS  07:31:15  
```

Template: `{D} DAYS  {H} HRS` / `{M} MIN   {S} SEC` (two-row):

```
Row 3:  42 DAYS  7 HRS     
Row 4:  31 MIN   15 SEC    
```

For two-row time displays, the template uses a `/` delimiter to split into rows 3 and 4:

```
"{DDD} DAYS  {HH} HRS/{MM} MIN   {SS} SEC"
```

### 5.5 Countdown at Zero

When `target` is in the past (the countdown has reached zero), behavior depends on `zero_behavior`:

| Variant        | Behavior                                                                                       |
|----------------|------------------------------------------------------------------------------------------------|
| `ShowZero`     | Format template renders with all zeros: `000 DAYS  00:00:00`. Countdown stays in the queue.   |
| `ShowMessage`  | The provided `grid` replaces the countdown rendering. Stays in queue until manually removed.    |
| `Remove`       | The countdown is automatically deleted from the database and removed from the queue.            |
| `Pause`        | Same visual as `ShowZero`, but the countdown is flagged so the admin knows it's completed.      |

For `Remove`: deletion happens on the next rotation tick that encounters the expired countdown. If the countdown is currently displayed when it expires, it finishes its display duration normally, then is removed when the rotation advances past it.

### 5.6 Countdown Live Refresh

When a countdown is the **active** display item, the normal rotation timer pauses and the server enters **countdown refresh mode**:

1. The server spawns a secondary `tokio::time::interval` at the configured `countdown_refresh_seconds` rate (default: 1 second).
2. Every tick, the server recomputes the countdown's grid (updating the time remaining) and broadcasts a `board_update` to all viewers.
3. The `previous_grid` in each update is the grid from 1 second ago, enabling flip animations on the changing digits.
4. The overall rotation timer continues counting down in parallel. When it fires, the server exits countdown refresh mode, advances to the next queue item, and resumes normal rotation.

This means a countdown displayed for 30 seconds will generate ~30 `board_update` messages (one per second), whereas a static message generates exactly 1.

### 5.7 Expired Message Handling

When the rotation timer advances to a message whose `expires_at` is in the past:

1. The message is **skipped** (not displayed).
2. The message is **deleted** from the database.
3. The rotation advances to the next queue item.
4. If all remaining items are expired, the queue is effectively empty and the splash screen is displayed (see [§5.3](#53-empty-queue-behavior)).

Expiry checking happens at rotation time, not as a background sweep. This keeps the implementation simple and avoids race conditions.

---

## 6. Split-Flap Rendering: Terminal (ratatui)

### 6.1 Grid Cell Rendering

Each flap is rendered as a **3-character-wide × 3-row-tall** cell using Unicode box-drawing characters to create the tile feeling. The displayed character sits centered in the tile.

**Single flap cell (character `A`):**

```
┌─┐
│A│
└─┘
```

**Single flap cell (blank):**

```
┌─┐
│ │
└─┘
```

Adjacent cells share borders to avoid double-thickness lines:

```
┌─┬─┬─┬─┬─┐
│H│E│L│L│O│
└─┴─┴─┴─┴─┘
```

The full board is therefore **68 terminal columns wide** (22 cells × 3 chars + 1 shared border) and **13 terminal rows tall** (6 cells × 2 rows + 1 shared bottom border). With the outer border, the minimum rendering area is **68 × 13** characters.

### 6.2 Flip Animation

When the board state changes, each flap that has a different target value plays a flip animation:

1. **Diff computation.** Compare `previous_grid[r][c]` with `grid[r][c]` for all 132 cells. Only cells that differ are animated.
2. **Character cycling.** For character-to-character transitions, the flap cycles through intermediate character indices from the current value toward the target value (wrapping at index 47 → 0). Each intermediate step is displayed for **~50ms**.
   - Example: transitioning from `A` (index 1) to `D` (index 4) shows: `A` → `B` → `C` → `D` (3 steps × 50ms = 150ms).
   - Transitioning from `Y` (index 25) to `B` (index 2) wraps: `Y` → `Z` → `0` → `1` → ... → `$` → `°` → ` ` → `A` → `B` (24 steps × 50ms = 1200ms). To avoid excessively long animations for large jumps, **cap at 12 intermediate steps** (skip evenly through the character set if the distance exceeds 12).
3. **Color tile transitions.** When transitioning to/from a color tile, the flap shows 3 rapid blank-blink frames (50ms each) before settling on the target. No character cycling through the drum.
4. **Cascade stagger.** Flaps do **not** all start their animation simultaneously. Instead, animations cascade **left-to-right** with a **~20ms delay between columns**. Column 0 starts first; column 21 starts ~440ms later. Within a column, all 6 rows animate simultaneously.
5. **Frame rate.** During animation, the TUI renders at **30 FPS** (one frame every ~33ms). When the board is static (no animation in progress), rendering is idle — only redraws on terminal resize events.

### 6.3 Color Tile Rendering

Color tiles in the terminal use ANSI 256-color **background colors** on the cell. The cell character is a space so the background fill is visible:

```
┌─┐
│ │  ← background color set to the tile's ANSI 256-color code
└─┘
```

The border characters (`┌`, `─`, `┐`, etc.) use the default foreground color, regardless of the tile color. This keeps the grid lines visually consistent.

Color tile ANSI mapping (from [§1.3](#13-color-tiles)):

| Color  | ANSI 256 Code | Terminal Appearance |
|--------|---------------|---------------------|
| Red    | 196           | Bright red          |
| Orange | 208           | Orange              |
| Yellow | 220           | Yellow              |
| Green  | 34            | Green               |
| Blue   | 33            | Blue                |
| Violet | 128           | Purple/violet       |
| White  | 231           | Bright white        |
| Black  | 234           | Dark gray/black     |

Character cells use a dark background (ANSI 234 / black) with white or light-gray foreground text, simulating the dark flap appearance of a physical split-flap board.

### 6.4 TUI Layout

```
┌── Herald ─────────────────────────────────────────────────────────────┐
│                                                                       │
│   ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │H│E│L│L│O│ │W│O│R│L│D│ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘                  │
│                                                                       │
│  ● Connected  │  Message 3/7  │  Next in 12s  │  ↑↓ scroll queue     │
└───────────────────────────────────────────────────────────────────────┘
```

- **Outer border:** Single-line box around the entire terminal.
- **Title:** "Herald" centered at the top of the outer border.
- **Board grid:** Centered horizontally and vertically within the available space.
- **Status bar:** Bottom row, showing:
  - Connection status: `● Connected` (green dot) or `○ Reconnecting...` (red dot)
  - Current position: `Message 3/7` or `Countdown 3/7`
  - Next rotation: `Next in 12s`
  - Help hint: keyboard shortcuts

### 6.5 Terminal Resize Handling

On terminal resize (`SIGWINCH` / crossterm `Event::Resize`):

1. Re-calculate the center position of the board grid.
2. If the terminal is too small to fit the board (< 70 columns or < 16 rows), display a centered warning:
   ```
   Terminal too small.
   Minimum: 70×16
   Current: 60×12
   ```
3. If the terminal is large enough, re-render the full board immediately.

### 6.6 Performance Targets

| State              | Target FPS | CPU Usage |
|--------------------|------------|-----------|
| Static (no change) | 0 (idle)   | ~0%       |
| Animating          | 30 FPS     | Low       |
| Resize             | Immediate  | Burst     |

The TUI uses an event-driven loop (`crossterm::event::poll` with timeout). During animation, the timeout is set to ~33ms. When idle, the timeout is set to 1 second (only waking for WebSocket messages, resize events, or keypresses).

### 6.7 ASCII Mockup

A complete terminal mockup showing the board with "HELLO WORLD" on row 2 and a color tile strip on row 5:

```
┌── Herald ─────────────────────────────────────────────────────────────┐
│                                                                       │
│   ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │H│E│L│L│O│ │W│O│R│L│D│ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ │                  │
│   ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤                  │
│   │█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│█│                  │
│   └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘                  │
│                                                                       │
│  ● Connected  │  Message 1/3  │  Next in 24s  │  q: quit             │
└───────────────────────────────────────────────────────────────────────┘
```

*(Row 5 shows `█` characters representing color-filled tiles with background color applied.)*

---

## 7. Split-Flap Rendering: Web (Leptos + Wasm)

### 7.1 Flap Tile HTML Structure

Each flap tile consists of a container with two halves (top and bottom), enabling the 3D flip illusion.

```html
<!-- Single flap tile -->
<div class="flap-tile" data-row="2" data-col="5">
  <!-- Static bottom layer: shows the NEW character underneath -->
  <div class="flap-bottom">
    <span class="flap-char">H</span>
  </div>

  <!-- Animated top-half: flips down to reveal new char -->
  <div class="flap-top flap-top--flipping">
    <span class="flap-char">A</span> <!-- old character, flips away -->
  </div>

  <!-- Animated bottom-half: flips into place with new char -->
  <div class="flap-bottom-flip">
    <span class="flap-char">H</span> <!-- new character, flips in -->
  </div>

  <!-- Static top layer: shows the NEW character after animation -->
  <div class="flap-top flap-top--static">
    <span class="flap-char">H</span>
  </div>
</div>
```

**Board container:**

```html
<div class="herald-board" role="img" aria-label="Herald message board">
  <div class="board-grid">
    <!-- 6 rows × 22 cols = 132 flap-tile elements -->
    <!-- Rendered via Leptos #[component] with For loops -->
  </div>
  <div class="board-status">
    <span class="status-indicator connected">●</span>
    <span class="status-text">Message 3/7 • Next in 12s</span>
    <button class="sound-toggle" aria-label="Toggle sound">🔊</button>
  </div>
</div>
```

### 7.2 CSS Flip Animation

```css
/* ===== Board Container ===== */
.herald-board {
  perspective: 1200px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.board-grid {
  display: grid;
  grid-template-columns: repeat(22, 1fr);
  grid-template-rows: repeat(6, 1fr);
  gap: 2px;
  padding: 1rem;
  background: #1a1a1a;
  border-radius: 8px;
  box-shadow:
    0 4px 12px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

/* ===== Flap Tile ===== */
.flap-tile {
  position: relative;
  width: clamp(1.5rem, 3vw, 2.8rem);
  height: clamp(2rem, 4vw, 3.6rem);
  perspective: 300px;
  transform-style: preserve-3d;
}

.flap-tile .flap-char {
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: clamp(0.9rem, 1.8vw, 1.6rem);
  font-weight: 700;
  color: #f0f0f0;
  line-height: 1;
  user-select: none;
}

/* Top half of the flap (upper portion of the tile) */
.flap-top,
.flap-bottom {
  position: absolute;
  left: 0;
  right: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}

.flap-top {
  top: 0;
  height: 50%;
  background: #2c2c2c;
  border-radius: 4px 4px 0 0;
  border-bottom: 1px solid #111;
  /* Clip the character: show only upper half */
  clip-path: inset(0 0 0 0);
}

.flap-top .flap-char {
  transform: translateY(50%);
}

.flap-bottom {
  bottom: 0;
  height: 50%;
  background: #262626;
  border-radius: 0 0 4px 4px;
  clip-path: inset(0 0 0 0);
}

.flap-bottom .flap-char {
  transform: translateY(-50%);
}

/* ===== Flip Keyframes ===== */

/* Top half flips downward (old character disappears) */
@keyframes flap-top-flip {
  0% {
    transform: rotateX(0deg);
  }
  100% {
    transform: rotateX(-90deg);
  }
}

/* Bottom half flips into place (new character appears) */
@keyframes flap-bottom-flip {
  0% {
    transform: rotateX(90deg);
  }
  100% {
    transform: rotateX(0deg);
  }
}

.flap-top--flipping {
  animation: flap-top-flip 150ms ease-in forwards;
  transform-origin: bottom center;
}

.flap-bottom-flip {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 50%;
  background: #2c2c2c;
  border-radius: 0 0 4px 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
  transform-origin: top center;
  animation: flap-bottom-flip 150ms ease-out 150ms forwards;
  transform: rotateX(90deg);
}

.flap-bottom-flip .flap-char {
  transform: translateY(-50%);
}

/* ===== Cascade Stagger ===== */
/* Applied via inline style from Leptos: style="animation-delay: {col * 15}ms" */
/* Column 0: 0ms, Column 1: 15ms, ..., Column 21: 315ms */
/* Total cascade duration: 315ms + 300ms animation = ~615ms for full board */

/* ===== Idle State (no animation) ===== */
.flap-top--static {
  transform: rotateX(0deg);
}

/* ===== Color Tile Variant ===== */
.flap-tile--color .flap-top,
.flap-tile--color .flap-bottom {
  /* Background color set via inline style from Leptos signal */
}

.flap-tile--color .flap-char {
  display: none; /* No character on color tiles */
}
```

**Animation timing summary:**
- Each flap animation: ~300ms total (150ms top flip + 150ms bottom flip, overlapped).
- Cascade stagger: 15ms between columns → column 21 starts at 315ms offset.
- Full board transition: ~615ms from first flap to last flap settling.

### 7.3 Shadow & Depth

```css
/* Individual tile shadows for depth */
.flap-tile {
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.3),
    0 0 0 1px rgba(0, 0, 0, 0.15);
}

/* During flip, enhance shadow for "lifting" effect */
.flap-top--flipping,
.flap-bottom-flip {
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.5);
}

/* Board container perspective */
.board-grid {
  transform: perspective(1200px) rotateX(2deg);
  /* Subtle upward tilt gives "looking at a wall-mounted board" feel */
}
```

### 7.4 Color Tiles (Web)

Color tiles display a solid background color with no character. The hex values from [§1.3](#13-color-tiles) are applied directly as inline `background-color` styles via Leptos signals:

```css
/* Color tile: both halves same color, no split-line visible */
.flap-tile--color .flap-top,
.flap-tile--color .flap-bottom {
  border-bottom: none;
  border-radius: 4px;
}

.flap-tile--color .flap-top {
  /* background-color set inline: e.g., #D32F2F for red */
}

.flap-tile--color .flap-bottom {
  /* Same background-color as top half */
}
```

### 7.5 Responsive Design

The board scales to fit the viewport while maintaining readability:

```css
.board-grid {
  /* Each tile scales between 1.5rem (mobile) and 2.8rem (desktop) */
  width: clamp(33rem, 70vw, 65rem);
}

.flap-tile {
  width: clamp(1.5rem, 3vw, 2.8rem);
  height: clamp(2rem, 4vw, 3.6rem);
}

/* On very small screens, allow horizontal scroll */
@media (max-width: 480px) {
  .herald-board {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .board-grid {
    min-width: 33rem; /* minimum readable size */
  }
}

/* Large screens: cap max width, center */
@media (min-width: 1200px) {
  .herald-board {
    max-width: 65rem;
    margin: 0 auto;
  }
}
```

### 7.6 Sound Effects

Optional mechanical "clack" sound on each flap flip, implemented via the Web Audio API:

- **Audio sample:** A short (~50ms) pre-recorded mechanical click. Stored as a base64-encoded WAV in the Wasm binary or loaded from a static asset (`/static/clack.wav`).
- **Playback:** When a flap starts its animation, schedule the clack sound at the midpoint (150ms into the 300ms animation). Use `AudioContext.createBufferSource()` for low-latency playback.
- **Volume variation:** Randomize volume between 0.3–0.7 for each flap to simulate the imperfect mechanical nature of physical boards.
- **Rate limiting:** With 132 tiles potentially animating, cap simultaneous audio sources at ~10 and stagger playback with the cascade delay to avoid audio overload.
- **Toggle:** A mute button in the status bar. State stored in `localStorage`. Default: **muted** (opt-in sound).

### 7.7 Loading State

Before the WebSocket connection is established:

```css
/* Shimmer animation for loading tiles */
@keyframes tile-shimmer {
  0% { opacity: 0.3; }
  50% { opacity: 0.6; }
  100% { opacity: 0.3; }
}

.flap-tile--loading .flap-top,
.flap-tile--loading .flap-bottom {
  background: #1a1a1a;
  animation: tile-shimmer 1.5s ease-in-out infinite;
}

.board-status--connecting .status-text::after {
  content: '...';
  animation: ellipsis 1.5s steps(3, end) infinite;
}

@keyframes ellipsis {
  0% { content: '.'; }
  33% { content: '..'; }
  66% { content: '...'; }
}
```

Loading sequence:
1. Board renders with all tiles in `--loading` state (dark with shimmer).
2. Status bar shows "Connecting..." with animated ellipsis.
3. On WebSocket connect + first `board_update`, remove `--loading` class and animate all tiles from blank to their target values.

### 7.8 Performance

With 132 tiles potentially animating simultaneously, these optimizations are critical:

```css
/* Promote each tile to its own compositing layer */
.flap-tile {
  will-change: transform;
  contain: layout style paint;
}

/* Only apply will-change during animation, remove after */
.flap-tile--idle {
  will-change: auto;
}
```

**Leptos-side optimizations:**

- Use **fine-grained signals** — one `RwSignal<CellContent>` per cell (132 signals). When a `board_update` arrives, only update the signals for cells that changed. Leptos's reactivity system ensures only the affected DOM nodes re-render.
- **Batch signal updates** within a single `batch(|| { ... })` call to prevent intermediate renders during a board transition.
- **Cascade stagger** is purely CSS-driven (via `animation-delay`), not JavaScript-driven. This offloads timing to the browser's compositor thread.
- **Avoid layout thrashing:** The grid layout is fixed (CSS Grid), so no reflow occurs during animation — only composite-layer transforms.

---

## 8. Admin Interface & Workflow

### 8.1 CLI Subcommands

The `herald` binary serves double duty: server, viewer, and admin tool.

```bash
herald <COMMAND>

Commands:
  serve      Start the Herald backend server
  watch      Connect as a viewer (TUI split-flap display)
  push       Push a new message to the board
  countdown  Manage countdowns
  queue      Manage the rotation queue
  config     View or update server configuration
  help       Print help information

Global Options:
  --config <PATH>    Path to herald.toml (default: ./herald.toml)
  --token <TOKEN>    Admin API token (overrides HERALD_ADMIN_TOKEN env var)
  --server <URL>     Backend server URL (default: http://localhost:3000)
  -v, --verbose      Enable verbose output
  -h, --help         Print help
  -V, --version      Print version
```

**Subcommand details:**

```bash
# Start the backend server
herald serve [--bind 0.0.0.0] [--port 3000] [--db ./herald.db]

# Connect as a viewer (TUI mode)
herald watch [--server http://localhost:3000]
#   Keyboard: q = quit, m = toggle mute (if sound supported)

# Push a message (text mode — auto-layouts onto the 6×22 grid)
herald push "HELLO WORLD" [--align center] [--valign middle]
herald push "LINE ONE" "LINE TWO" "LINE THREE"  # multi-line
herald push --row 0 "HEADER" --row 2 "BODY" --row 5 "FOOTER"
herald push "HELLO {red}{red}{red} WORLD"  # inline color tiles
herald push --color red:0,0-0,4 "MESSAGE"  # positional colors
herald push --expires "2025-12-31T23:59:59Z" "TEMP MESSAGE"

# Countdown management
herald countdown create --label "DAYS UNTIL LAUNCH" \
  --target "2025-12-31T00:00:00Z" \
  --format "{DDD} DAYS  {HH}:{MM}:{SS}" \
  --zero-behavior show-message \
  --zero-grid-file celebration.json

herald countdown list
# Output:
#   ID                                    LABEL               TARGET                POSITION
#   c3d4e5f6-a7b8-9012-cdef-123456789012  DAYS UNTIL LAUNCH   2025-12-31T00:00:00Z  2

herald countdown delete c3d4e5f6-a7b8-9012-cdef-123456789012

# Queue management
herald queue list
# Output:
#   POS  TYPE       ID                                    LABEL
#   0    message    a1b2c3d4-e5f6-7890-abcd-ef1234567890  HELLO WORLD
#   1    countdown  c3d4e5f6-a7b8-9012-cdef-123456789012  DAYS UNTIL LAUNCH
#   2    message    b2c3d4e5-f6a7-8901-bcde-f12345678901  WELCOME HOME

herald queue reorder a1b2c3d4 c3d4e5f6 b2c3d4e5  # reorder by ID prefixes

# Configuration
herald config get
# Output:
#   rotation_interval_seconds = 30
#   countdown_refresh_seconds = 1
#   default_h_align = center
#   default_v_align = middle
#   admin_enabled = true

herald config set rotation_interval_seconds 45
```

**Authentication for CLI admin commands:**

Admin commands (`push`, `countdown`, `queue`, `config`) require a token. Resolution order:
1. `--token <TOKEN>` command-line flag (highest priority)
2. `HERALD_ADMIN_TOKEN` environment variable
3. If neither is set, the CLI prints an error: `Error: Admin token required. Use --token <TOKEN> or set HERALD_ADMIN_TOKEN.`

### 8.2 Web Admin Panel

The web admin panel is served at `/admin` and protected by the admin token.

**Access flow:**
1. Navigate to `http://host:port/admin`.
2. A login screen prompts for the admin token.
3. Token is validated via `GET /api/config` (a successful response means the token is valid).
4. Token is stored in `sessionStorage` (cleared when the browser tab closes).

**Admin panel pages:**

#### Message Composer (`/admin/compose`)
- **Visual grid editor:** 6×22 clickable grid. Click a cell to select it, then type a character. Arrow keys navigate between cells.
- **Color picker:** Toolbar with 8 color buttons. Click a color, then click cells to fill them with that color tile.
- **Alignment controls:** Dropdown for horizontal and vertical alignment.
- **Text input mode:** Type a message in a text input; see a live preview of how it will be laid out on the grid.
- **Expiry picker:** Optional date/time picker for `expires_at`.
- **Actions:** "Push to Board" (adds to end of queue), "Push & Display Now" (adds to queue and forces immediate display).

#### Countdown Manager (`/admin/countdowns`)
- List of all countdowns with label, target, position, and live preview of time remaining.
- "Create Countdown" form: label, target datetime picker, format template (with live preview), zero behavior selector.
- Edit / Delete buttons for each countdown.

#### Queue Manager (`/admin/queue`)
- Drag-to-reorder list of all queue items (messages and countdowns interleaved).
- Each item shows: position number, type icon (📋 message, ⏱ countdown), label/preview, and a delete button.
- "Current" indicator on the currently displayed item.
- Connected viewers count at the top.

#### Config Panel (`/admin/config`)
- Form with all runtime-changeable configuration parameters.
- "Save" button to apply changes.
- Read-only display of non-runtime-changeable parameters (bind address, port, db path).

### 8.3 Color Markup for CLI Push

Two methods for specifying color tiles in CLI `push` commands:

**Inline color tags:**

Insert `{color}` tokens in the message text. Each token occupies one cell position and renders as a solid color tile instead of a character.

```bash
herald push "HELLO {red}{red}{red} WORLD"
# Result: H E L L O [RED][RED][RED] _ W O R L D
```

Supported tags: `{red}`, `{orange}`, `{yellow}`, `{green}`, `{blue}`, `{violet}`, `{white}`, `{black}`.

**Positional color flag:**

Use `--color <color>:<row>,<start_col>-<row>,<end_col>` to fill a range of cells with a color. Multiple `--color` flags can be combined.

```bash
# Fill row 0, columns 0 through 4 with red
herald push --color red:0,0-0,4 "MY MESSAGE"

# Fill row 5 entirely with blue, and row 0 cols 0-2 with green
herald push --color blue:5,0-5,21 --color green:0,0-0,2 "HELLO"
```

Color flags are applied **after** the text is laid out on the grid, overwriting any characters in the specified positions.

---

## 9. Configuration Reference

### 9.1 Parameter Table

| Parameter | Type | Default | Env Var | Description | Runtime Changeable |
|---|---|---|---|---|---|
| `server.bind_address` | String | `"0.0.0.0"` | `HERALD_BIND_ADDRESS` | IP address to bind the HTTP server to | No |
| `server.port` | u16 | `3000` | `HERALD_PORT` | Port for the HTTP/WebSocket server | No |
| `database.path` | String | `"./herald.db"` | `HERALD_DB_PATH` | Path to the SQLite database file | No |
| `auth.admin_token` | String | *(required)* | `HERALD_ADMIN_TOKEN` | Bearer token for admin API authentication | No |
| `rotation.interval_seconds` | u32 | `30` | `HERALD_ROTATION_INTERVAL` | Seconds between rotation advances | Yes |
| `rotation.countdown_refresh_seconds` | u32 | `1` | `HERALD_COUNTDOWN_REFRESH` | Seconds between countdown display updates | Yes |
| `board.default_h_align` | String | `"center"` | `HERALD_DEFAULT_H_ALIGN` | Default horizontal alignment for messages | Yes |
| `board.default_v_align` | String | `"middle"` | `HERALD_DEFAULT_V_ALIGN` | Default vertical alignment for messages | Yes |
| `board.countdown_zero_behavior` | String | `"show_zero"` | `HERALD_COUNTDOWN_ZERO` | Default zero behavior for new countdowns | Yes |
| `web.admin_enabled` | bool | `true` | `HERALD_ADMIN_ENABLED` | Enable the `/admin` web panel | No |
| `web.static_dir` | String | `"./static"` | `HERALD_STATIC_DIR` | Directory for static web assets (Wasm/HTML/CSS/JS) | No |
| `websocket.heartbeat_seconds` | u32 | `30` | `HERALD_WS_HEARTBEAT` | Seconds between WebSocket heartbeat pings | Yes |
| `websocket.path` | String | `"/ws"` | `HERALD_WS_PATH` | WebSocket endpoint path | No |
| `rate_limit.enabled` | bool | `false` | `HERALD_RATE_LIMIT_ENABLED` | Enable rate limiting on admin endpoints | Yes |
| `rate_limit.requests_per_minute` | u32 | `60` | `HERALD_RATE_LIMIT_RPM` | Max requests per minute per IP for admin endpoints | Yes |

**"Runtime Changeable"** indicates whether the parameter can be updated via `PUT /api/config` or `herald config set` without restarting the server. Parameters marked "No" require a server restart to take effect.

### 9.2 Example herald.toml

```toml
# Herald Configuration
# =====================
# Copy this file to herald.toml and adjust as needed.
# Environment variables override file values (see HERALD_* prefixes).

[server]
# IP address to bind to. Use "0.0.0.0" for all interfaces.
bind_address = "0.0.0.0"

# HTTP and WebSocket port.
port = 3000

[database]
# Path to the SQLite database file.
# Created automatically on first run. Use a Docker volume mount for persistence.
path = "./herald.db"

[auth]
# Admin API bearer token. REQUIRED — the server will not start without this.
# Use a strong random value: `openssl rand -hex 32`
admin_token = "change-me-to-a-strong-random-token"

[rotation]
# Seconds each message/countdown is displayed before advancing.
interval_seconds = 30

# Seconds between countdown timer updates when a countdown is active.
# Set to 1 for second-by-second countdown display.
countdown_refresh_seconds = 1

[board]
# Default horizontal alignment for new messages: "left", "center", or "right".
default_h_align = "center"

# Default vertical alignment for new messages: "top" or "middle".
default_v_align = "middle"

# Default zero-behavior for new countdowns: "show_zero", "remove", or "pause".
# "show_message" requires per-countdown configuration via the API.
countdown_zero_behavior = "show_zero"

[web]
# Enable the /admin web panel. Set to false to disable admin UI entirely.
admin_enabled = true

# Directory containing the compiled Leptos/Wasm static assets.
# The server serves these at the root path (/).
static_dir = "./static"

[websocket]
# Seconds between heartbeat pings to connected viewers.
heartbeat_seconds = 30

# WebSocket endpoint path.
path = "/ws"

[rate_limit]
# Enable rate limiting on admin API endpoints.
enabled = false

# Maximum admin API requests per minute per IP address.
requests_per_minute = 60
```

**Environment variable override convention:**

All environment variables use the `HERALD_` prefix with underscores replacing dots and converting to uppercase. Nested keys use a single underscore separator:

- `server.bind_address` → `HERALD_BIND_ADDRESS`
- `rotation.interval_seconds` → `HERALD_ROTATION_INTERVAL`
- `auth.admin_token` → `HERALD_ADMIN_TOKEN`

Environment variables take precedence over `herald.toml` values. The resolution order is:

1. Environment variable (highest priority)
2. Config file (`herald.toml`)
3. Compiled default (lowest priority)

---

## 10. Error Handling & Resilience

### 10.1 WebSocket Disconnection

**Server-side:**
- When a WebSocket connection drops (TCP reset, client vanishes), the server detects this on the next attempted write to that connection.
- The connection is immediately removed from the broadcast pool. No retry from the server side.
- Connection count is decremented in the health endpoint.
- Resources (buffers, sender handle) are dropped.

**Client-side (CLI and Web):**
- Both clients implement exponential backoff reconnection as specified in [§4.4](#44-reconnection-strategy).
- During reconnection, the last-known board state remains displayed.
- On successful reconnect, the server sends a fresh `board_update` (initial state), and the client animates the transition if the board has changed.

### 10.2 Database Failures

**Startup:**
- If the database file cannot be opened or created, the server logs the error and exits with code 1.
- If migrations fail, the server logs the specific migration that failed and exits with code 1.

**Runtime:**
- If a database query fails (disk full, corruption, file locked), the affected API endpoint returns:

```json
{
  "error": {
    "code": "SERVICE_UNAVAILABLE",
    "message": "Database temporarily unavailable"
  }
}
```

**HTTP Status:** `503 Service Unavailable`

- The server does **not** crash. It continues serving WebSocket connections (with the last-known board state) and retries database operations on subsequent requests.
- The health endpoint (`GET /api/health`) returns 503 when the database is unreachable, enabling external monitoring tools to detect the issue.

### 10.3 Malformed Requests

All API request bodies are validated before processing. Validation failures return:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Descriptive message about what's wrong"
  }
}
```

**HTTP Status:** `400 Bad Request`

Specific validation scenarios:

| Scenario | Error Message |
|---|---|
| Grid is not 6×22 | `"Grid must be exactly 6 rows of 22 cells"` |
| Invalid char index (e.g., 50) | `"Invalid character index 50 at position (2, 5). Valid range: 0-47"` |
| Invalid color in CellContent | `"Unknown color 'purple' at position (0, 0). Valid colors: red, orange, yellow, green, blue, violet, white, black"` |
| Invalid h_align value | `"Invalid horizontal alignment 'justify'. Must be: left, center, right"` |
| expires_at in the past | `"expires_at must be a future timestamp"` |
| Countdown label > 44 chars | `"Label exceeds maximum length of 44 characters"` |
| Invalid format template token | `"Unknown template token '{W}' in format_template"` |
| Missing required field | `"Missing required field: target"` |
| Malformed JSON | `"Invalid JSON: expected '{' at line 1 column 1"` |
| Reorder missing IDs | `"Reorder list is missing items: [id1, id2]"` |

### 10.4 Rate Limiting

When `rate_limit.enabled` is `true`, admin endpoints (everything except `GET /api/health` and `GET /ws`) are rate-limited per IP address:

- **Algorithm:** Token bucket with per-minute refill.
- **Default:** 60 requests per minute per IP.
- **Response on limit exceeded:**

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 15 seconds."
  }
}
```

**HTTP Status:** `429 Too Many Requests`
**Headers:** `Retry-After: 15` (seconds until the next token is available)

Rate limiting is **disabled by default** since Herald is a single-admin system and rate limiting adds complexity. Enable it for deployments exposed to the public internet.

### 10.5 Graceful Shutdown

On receiving `SIGTERM` or `SIGINT` (Ctrl-C):

1. **Stop accepting new connections.** The HTTP listener stops accepting new TCP connections immediately.
2. **Notify WebSocket clients.** Send a WebSocket close frame (code 1001, "Server shutting down") to every connected viewer.
3. **Drain in-flight requests.** Wait up to **10 seconds** for any in-flight HTTP requests to complete.
4. **Persist state.** Flush any pending rotation state to the database.
5. **Exit.** Process exits with code 0.

If in-flight requests do not complete within the 10-second grace period, they are forcefully terminated and the server exits anyway.

```rust
// Pseudocode for shutdown handler
let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

tokio::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
    shutdown_tx.send(true).unwrap();
});

// In the server main loop:
axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .with_graceful_shutdown(async {
        shutdown_rx.changed().await.ok();
    })
    .await?;

// After server stops accepting:
broadcast_close_to_all_websockets().await;
flush_rotation_state().await;
```

---

## Cross-References

- [ARCHITECTURE.md](./ARCHITECTURE.md) — System architecture, crate structure, data flow, concurrency model
- [DEPLOYMENT.md](./DEPLOYMENT.md) — Docker, bare metal deployment, reverse proxy configuration
- [DECISIONS.md](./DECISIONS.md) — Architecture Decision Records for all major technical choices
- [CONTRIBUTING.md](./CONTRIBUTING.md) — Build from source, development workflow, PR process
- [ROADMAP.md](./ROADMAP.md) — Development phases, milestones, and GitHub issue plan
