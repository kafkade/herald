# Herald — Architecture Overview

> Architectural foundation for the Herald split-flap message board system.
> This document describes the system's components, how they communicate, and how the codebase is organized.

---

## Table of Contents

- [1. High-Level System Diagram](#1-high-level-system-diagram)
- [2. Component Overview](#2-component-overview)
  - [2.1 Backend Service (herald-server)](#21-backend-service-herald-server)
  - [2.2 CLI Viewer / Admin Tool (herald-cli)](#22-cli-viewer--admin-tool-herald-cli)
  - [2.3 Web Interface (herald-web)](#23-web-interface-herald-web)
  - [2.4 Shared Library (herald-common)](#24-shared-library-herald-common)
- [3. Cargo Workspace & Crate Structure](#3-cargo-workspace--crate-structure)
  - [3.1 Workspace Layout](#31-workspace-layout)
  - [3.2 Workspace Cargo.toml](#32-workspace-cargotoml)
  - [3.3 Crate Dependency Graph](#33-crate-dependency-graph)
  - [3.4 herald-common — Shared Types](#34-herald-common--shared-types)
  - [3.5 herald-server — Backend Service](#35-herald-server--backend-service)
  - [3.6 herald-cli — Terminal Viewer & Admin](#36-herald-cli--terminal-viewer--admin)
  - [3.7 herald-web — Leptos WebAssembly Frontend](#37-herald-web--leptos-webassembly-frontend)
- [4. Data Flow Diagrams](#4-data-flow-diagrams)
  - [4.1 Admin Pushes a Message](#41-admin-pushes-a-message)
  - [4.2 Rotation Timer Fires](#42-rotation-timer-fires)
  - [4.3 Countdown Tick](#43-countdown-tick)
  - [4.4 New Viewer Connects](#44-new-viewer-connects)
- [5. Concurrency Model](#5-concurrency-model)
  - [5.1 Tokio Task Architecture](#51-tokio-task-architecture)
  - [5.2 WebSocket Connection Management](#52-websocket-connection-management)
  - [5.3 Rotation Timer](#53-rotation-timer)
  - [5.4 Admin API Concurrency](#54-admin-api-concurrency)
  - [5.5 Graceful Shutdown](#55-graceful-shutdown)
- [6. Communication Protocols](#6-communication-protocols)
  - [6.1 REST API (Admin)](#61-rest-api-admin)
  - [6.2 WebSocket Protocol (Viewers)](#62-websocket-protocol-viewers)
    - [WebSocket Message Types](#websocket-message-types)
    - [Empty Queue Behavior](#empty-queue-behavior)
    - [Error Resilience](#error-resilience)
    - [Expired Item Handling](#expired-item-handling)
  - [6.3 Cross-References](#63-cross-references)

---

## 1. High-Level System Diagram

```
                       ┌──────────────────────────────────────────────────────────┐
                       │                      HERALD SERVER                       │
                       │                   (herald-server crate)                  │
                       │                                                          │
  ┌──────────┐         │  ┌─────────────┐     ┌───────────────┐                   │
  │  Admin   │  REST   │  │             │     │   Rotation    │                   │
  │  (curl / │ ──────► │  │  Axum       │     │   Timer       │                   │
  │  CLI)    │  HTTP   │  │  Router     │     │   (tokio      │                   │
  └──────────┘         │  │             │     │    interval)  │                   │
       │               │  │  ┌────────┐ │     └──────┬────────┘                   │
       │ Bearer        │  │  │ REST   │ │            │                            │
       │ Token         │  │  │ API    │ │            │ next item                  │
       │ Auth          │  │  │ Hdlrs  │ │            ▼                            │
       ▼               │  │  └───┬────┘ │     ┌───────────────┐                   │
  ┌──────────┐         │  │      │      │     │  Board State  │                   │
  │  Auth    │         │  │      │      │     │  Manager      │◄──── Computes     │
  │  Layer   │         │  │      ▼      │     │  (AppState)   │      6×22 grid    │
  └──────────┘         │  │  ┌────────┐ │     └──────┬────────┘                   │
                       │  │  │ WS     │ │            │                            │
                       │  │  │ Hdlr   │ │            │ broadcast                  │
                       │  │  └───┬────┘ │            ▼                            │
                       │  └──────┼──────┘     ┌───────────────┐                   │
                       │         │            │  Broadcast    │                   │
                       │         │            │  Channel      │                   │
                       │         │            │  (tokio)      │                   │
                       │         │            └──────┬────────┘                   │
                       │         │                   │                            │
                       │  ┌──────┴──────┐            │                            │
                       │  │  Static     │            │                            │
                       │  │  File       │            │                            │
                       │  │  Server     │            │                            │
                       │  │  (Wasm/CSS) │            │                            │
                       │  └─────────────┘            │                            │
                       │                             │                            │
                       │  ┌─────────────┐            │                            │
                       │  │  SQLite     │◄───────────┤                            │
                       │  │  (sqlx)     │  persist   │                            │
                       │  └─────────────┘            │                            │
                       └──────────────┬──────────────┼────────────────────────────┘
                                      │              │
                           ┌──────────┴───┐    ┌─────┴─────────────┐
                           │  WebSocket   │    │  WebSocket        │
                           │  Connection  │    │  Connection       │
                           ▼              │    │                   ▼
                  ┌──────────────────┐    │    │    ┌──────────────────────┐
                  │   CLI Viewer     │    │    │    │   Web Viewer         │
                  │  (herald-cli)    │    │    │    │  (herald-web)        │
                  │                  │    │    │    │                      │
                  │  ┌────────────┐  │    │    │    │  ┌────────────────┐  │
                  │  │ ratatui    │  │    │    │    │  │ Leptos/Wasm    │  │
                  │  │ split-flap │  │    │    │    │  │ 3D split-flap  │  │
                  │  │ renderer   │  │    │    │    │  │ CSS animation  │  │
                  │  └────────────┘  │    │    │    │  └────────────────┘  │
                  │                  │    │    │    │                      │
                  │  Terminal (TUI)  │    │    │    │  Browser (Wasm)      │
                  └──────────────────┘    │    │    └──────────────────────┘
                                          │    │
                                  ... more viewers ...
```

**Data flow summary:**

1. **Admin → Server:** REST API calls (HTTP + Bearer token) create/update messages and countdowns.
2. **Server → SQLite:** All messages, countdowns, queue state, and configuration are persisted.
3. **Server → Viewers:** Board state changes are pushed to all connected viewers via WebSocket broadcast.
4. **Static assets:** The server serves the compiled Wasm web frontend as static files (HTML/CSS/JS/Wasm).

---

## 2. Component Overview

### 2.1 Backend Service (herald-server)

**Responsibility:** Central hub for all Herald operations. The single source of truth for board state.

| Concern | Detail |
|---|---|
| REST API | Admin endpoints for pushing messages, managing countdowns, reordering the queue, and updating configuration. All endpoints require bearer token authentication. |
| WebSocket | Accepts viewer connections, delivers the current board state on connect, then pushes real-time updates as the board changes. |
| Rotation engine | A background Tokio task that fires on a configurable interval (default 30 s), advances the queue, computes the new 6×22 grid, and triggers a broadcast. |
| Countdown engine | Evaluates active countdowns, computes remaining time, formats the countdown display onto the grid, and triggers re-broadcast when countdown digits change. |
| Persistence | SQLite database (via `sqlx`) stores messages, countdowns, queue order, and configuration. |
| Static file server | Serves the compiled `herald-web` Wasm bundle so the web viewer is accessible at the server's root URL. |

**Communicates with:**
- Admin clients (REST over HTTP)
- CLI and Web viewers (WebSocket)
- SQLite database (local file)

**Boundaries:** The server never renders the board visually — it only computes and distributes the abstract `BoardState` (a 6×22 grid of `CellContent` values). Rendering is the viewers' responsibility.

### 2.2 CLI Viewer / Admin Tool (herald-cli)

**Responsibility:** Dual-purpose terminal application — both a viewer and an admin tool.

| Mode | Detail |
|---|---|
| **Viewer mode** (`herald watch`) | Connects to the server via WebSocket, receives `BoardState` updates, and renders the split-flap board in the terminal using ratatui. Includes flip animations (character cycling) and color tile support via terminal background colors. |
| **Admin mode** (`herald push`, `herald countdown`, etc.) | Sends admin commands to the server via REST API. Supports message composition, countdown management, and queue manipulation from the command line. |

**Communicates with:**
- Herald server (WebSocket for viewing, REST for admin commands)

**Boundaries:** The CLI is a pure client. It holds no persistent state of its own — the server is always authoritative. If the WebSocket connection drops, the CLI reconnects and re-syncs from the server's current state.

### 2.3 Web Interface (herald-web)

**Responsibility:** Premium browser-based viewer with 3D split-flap animations.

| Concern | Detail |
|---|---|
| Rendering | Full visual fidelity: each flap tile has a top half and bottom half, 3D CSS `perspective` + `rotateX` animations on character transitions, shadows, and highlights. |
| WebSocket | Connects to the server's WebSocket endpoint, receives `BoardState` updates, and triggers re-render with flip animations. |
| Color tiles | Rendered as solid-color flap tiles distinct from character tiles. |
| Responsive | Scales to fit viewport on desktop and mobile. |
| Optional sound | Toggleable "clack" sound effect on flip. |

**Communicates with:**
- Herald server (WebSocket only — no REST calls from the web viewer)

**Boundaries:** The web interface is a view-only client (no admin capabilities in V1's viewer path). It is compiled to WebAssembly via Trunk and served as static assets by the backend. The optional `/admin` panel (if enabled) is a separate route within the same Leptos app that uses REST for admin operations.

### 2.4 Shared Library (herald-common)

**Responsibility:** The canonical definition of every data type that crosses a component boundary.

| Concern | Detail |
|---|---|
| Board types | `BoardState`, `CellContent`, `GridPosition`, `Color`, alignment enums |
| Domain types | `Message`, `Countdown`, `QueueItem` |
| Protocol types | `WsMessage` (server→client and client→server WebSocket frames) |
| Serialization | All types derive `serde::Serialize` and `serde::Deserialize` for JSON wire format |
| Feature flags | `wasm` feature enables `#[derive(Clone)]` patterns and any Wasm-specific trait impls |

**Communicates with:** Nothing directly — it is a library crate with no I/O.

**Boundaries:** `herald-common` must have **zero** runtime dependencies (no `tokio`, no `axum`, no `ratatui`, no `leptos`). Its only dependencies are `serde`, `serde_json`, and `chrono` (for countdown timestamps). This ensures it compiles cleanly for all targets: native (server, CLI) and `wasm32-unknown-unknown` (web).

---

## 3. Cargo Workspace & Crate Structure

### 3.1 Workspace Layout

```
herald/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── config/
│   └── herald.toml         # Default configuration file
├── crates/
│   ├── herald-common/      # Shared types & protocol definitions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── board.rs    # BoardState, CellContent, GridPosition, Color
│   │       ├── message.rs  # Message, Countdown, QueueItem
│   │       └── protocol.rs # WebSocket message types
│   ├── herald-server/      # Axum backend service
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api/        # REST endpoint handlers
│   │       ├── ws/         # WebSocket connection management
│   │       ├── state.rs    # AppState, broadcast channel, board manager
│   │       ├── rotation.rs # Rotation timer & queue logic
│   │       ├── countdown.rs# Countdown engine
│   │       ├── db.rs       # SQLite queries (sqlx)
│   │       └── config.rs   # Configuration loading (TOML + env)
│   ├── herald-cli/         # Terminal viewer & admin tool
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── viewer/     # TUI rendering, flip animation
│   │       ├── admin/      # CLI subcommands (push, countdown, etc.)
│   │       └── ws.rs       # WebSocket client connection
│   └── herald-web/         # Leptos Wasm frontend
│       ├── Cargo.toml
│       ├── Trunk.toml      # Trunk build configuration
│       ├── index.html       # HTML shell
│       └── src/
│           ├── main.rs
│           ├── app.rs      # Root Leptos component
│           ├── board.rs    # Board grid component
│           ├── flap.rs     # Individual flap tile component (3D CSS)
│           ├── ws.rs       # WebSocket client (browser API)
│           └── sound.rs    # Optional clack sound effect
├── assets/                 # Static assets (fonts, sounds)
├── migrations/             # SQLite migrations (sqlx)
├── docs/
│   ├── ARCHITECTURE.md     # ← You are here
│   ├── SPEC.md
│   ├── DECISIONS.md
│   ├── DEPLOYMENT.md
│   ├── CONTRIBUTING.md
│   └── ROADMAP.md
├── Dockerfile
├── docker-compose.yml
├── README.md
└── LICENSE
```

### 3.2 Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/herald-common",
    "crates/herald-server",
    "crates/herald-cli",
    "crates/herald-web",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/kafkade/herald"

[workspace.dependencies]
# Shared across multiple crates — pinned at workspace level
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 3.3 Crate Dependency Graph

```
herald-common  (no workspace deps)
     ▲
     │
     ├──────────────┬────────────────┐
     │              │                │
herald-server  herald-cli      herald-web
     │              │                │
     │              │                │
   axum           ratatui          leptos
   sqlx           crossterm        trunk
   tokio          tokio            wasm-bindgen
   tower          clap             web-sys
```

All three application crates depend on `herald-common`. No application crate depends on another application crate.

### 3.4 herald-common — Shared Types

These types are the contract between server and all clients. They must compile for both native and `wasm32-unknown-unknown` targets.

**`Cargo.toml`:**

```toml
[package]
name = "herald-common"
version.workspace = true
edition.workspace = true

[features]
default = []
wasm = []  # Enables Wasm-specific derives or conditional code

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
```

**Board types (`board.rs`):**

```rust
use serde::{Deserialize, Serialize};

/// Board dimensions — fixed at 6 rows × 22 columns (Vestaboard format).
pub const BOARD_ROWS: usize = 6;
pub const BOARD_COLS: usize = 22;
pub const BOARD_CELLS: usize = BOARD_ROWS * BOARD_COLS; // 132

/// A position on the board grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPosition {
    /// Row index (0–5, top to bottom).
    pub row: u8,
    /// Column index (0–21, left to right).
    pub col: u8,
}

impl GridPosition {
    pub fn new(row: u8, col: u8) -> Self {
        debug_assert!(row < BOARD_ROWS as u8, "row out of bounds");
        debug_assert!(col < BOARD_COLS as u8, "col out of bounds");
        Self { row, col }
    }
}

/// Color tiles supported by the board (matching Vestaboard physical colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

/// The content of a single cell (flap) on the board.
///
/// Each cell is either a displayable character or a solid color tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CellContent {
    /// A character from the supported set (A–Z, 0–9, special chars).
    /// Stored as a single `char`, always uppercase.
    Char(char),
    /// A solid color tile (no character).
    Color(Color),
    /// A blank/empty flap (equivalent to a space character).
    Blank,
}

impl Default for CellContent {
    fn default() -> Self {
        CellContent::Blank
    }
}

/// Horizontal alignment for text within a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HAlign {
    Left,
    Center,
    Right,
}

impl Default for HAlign {
    fn default() -> Self {
        HAlign::Left
    }
}

/// Vertical alignment for content within the 6-row grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VAlign {
    Top,
    Center,
}

impl Default for VAlign {
    fn default() -> Self {
        VAlign::Top
    }
}

/// The complete board state: a 6×22 grid of cell contents.
///
/// This is the primary payload sent to viewers over WebSocket.
/// Cells are stored in row-major order: `cells[row][col]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardState {
    /// The 6×22 grid of cell contents.
    pub cells: [[CellContent; BOARD_COLS]; BOARD_ROWS],
    /// Identifier of the queue item currently being displayed.
    pub active_item_id: Option<String>,
    /// Seconds remaining before the next rotation.
    pub seconds_until_rotation: u32,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            cells: [[CellContent::Blank; BOARD_COLS]; BOARD_ROWS],
            active_item_id: None,
            seconds_until_rotation: 0,
        }
    }
}
```

**Domain types (`message.rs`):**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::board::{BoardState, CellContent, HAlign, VAlign, BOARD_COLS, BOARD_ROWS};

/// A message to display on the board.
///
/// Messages are composed as a 6×22 grid of cell contents. The grid is
/// pre-composed by the admin (or composed from text with alignment) before
/// being stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable label (for admin reference, not displayed on board).
    pub label: Option<String>,
    /// The pre-composed 6×22 grid content for this message.
    pub grid: [[CellContent; BOARD_COLS]; BOARD_ROWS],
    /// When this message was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry time — message is auto-removed from the queue after this.
    pub expires_at: Option<DateTime<Utc>>,
}

/// A countdown to a target date/time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Countdown {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Label displayed on the board (e.g., "DAYS UNTIL LAUNCH").
    pub label: String,
    /// The target date/time being counted down to.
    pub target: DateTime<Utc>,
    /// Format template controlling how the countdown is rendered.
    /// See SPEC.md for template syntax.
    pub format: CountdownFormat,
    /// What happens when the countdown reaches zero.
    pub zero_behavior: CountdownZeroBehavior,
    /// Optional message to display when the countdown reaches zero.
    pub zero_message: Option<String>,
    /// When this countdown was created.
    pub created_at: DateTime<Utc>,
}

/// How a countdown is rendered on the board grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountdownFormat {
    /// "DDD DAYS HH HRS MM MIN" — large format across multiple rows.
    DaysHoursMinutes,
    /// "HH:MM:SS" — compact single-row format.
    HoursMinutesSeconds,
    /// Custom format string (see SPEC.md for syntax).
    Custom(String),
}

/// What happens when a countdown reaches zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountdownZeroBehavior {
    /// Display the `zero_message` (or "EVENT NOW!" by default).
    ShowMessage,
    /// Remove the countdown from the rotation queue automatically.
    Remove,
    /// Hold at 00:00:00 indefinitely.
    Hold,
}

/// An item in the rotation queue — either a message or a countdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QueueItem {
    /// A static message.
    #[serde(rename = "message")]
    Message(Message),
    /// A live countdown.
    #[serde(rename = "countdown")]
    Countdown(Countdown),
}

impl QueueItem {
    /// Returns the unique identifier of the queue item.
    pub fn id(&self) -> &str {
        match self {
            QueueItem::Message(m) => &m.id,
            QueueItem::Countdown(c) => &c.id,
        }
    }
}

/// Summary of a queue item (for list responses — omits full grid data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemSummary {
    pub id: String,
    pub item_type: String,
    pub label: Option<String>,
    pub position: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
```

**WebSocket protocol types (`protocol.rs`):**

```rust
use serde::{Deserialize, Serialize};

use crate::board::BoardState;

/// Messages sent from the server to connected viewers over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Full board state — sent on initial connection and on every board change.
    #[serde(rename = "board_state")]
    BoardState {
        state: BoardState,
    },

    /// Heartbeat ping — server sends periodically to keep the connection alive.
    #[serde(rename = "ping")]
    Ping {
        timestamp: u64,
    },

    /// Server is shutting down — clients should expect disconnection.
    #[serde(rename = "shutdown")]
    Shutdown {
        reason: String,
    },

    /// Error notification (e.g., invalid client message).
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// Messages sent from a viewer client to the server over WebSocket.
///
/// Viewers have very limited interaction — mostly heartbeat responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Heartbeat pong — client responds to server pings.
    #[serde(rename = "pong")]
    Pong {
        timestamp: u64,
    },
}
```

**Feature flags:**

The `wasm` feature in `herald-common` is reserved for any Wasm-specific conditional compilation. Currently, the primary types compile on all targets without feature gating. The `wasm` feature may be used for:

- Enabling `#[cfg(feature = "wasm")]` blocks that provide `wasm_bindgen` trait implementations.
- Replacing `chrono` types with `js-sys` `Date` conversions in Wasm contexts.
- Gating test utilities that depend on a native runtime.

In `herald-web/Cargo.toml`:

```toml
[dependencies]
herald-common = { path = "../herald-common", features = ["wasm"] }
```

### 3.5 herald-server — Backend Service

```toml
[package]
name = "herald-server"
version.workspace = true
edition.workspace = true

[[bin]]
name = "herald-server"
path = "src/main.rs"

[dependencies]
herald-common = { path = "../herald-common" }
axum = { version = "0.8", features = ["ws"] }
tokio = { workspace = true }
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "cors", "trace"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
toml = "0.8"
uuid = { version = "1", features = ["v4"] }
```

**Key modules:**

| Module | Purpose |
|---|---|
| `api/` | Axum route handlers for all REST endpoints. Auth middleware extracts and validates bearer tokens. |
| `ws/` | WebSocket upgrade handler, connection lifecycle, per-connection read/write tasks. |
| `state.rs` | `AppState` — the shared application state held in `Arc`. Contains the `tokio::sync::broadcast::Sender`, current `BoardState`, database pool, and configuration. |
| `rotation.rs` | Background Tokio task: runs on `tokio::time::interval`, advances the queue, recomputes board state, and sends to broadcast channel. |
| `countdown.rs` | Evaluates countdowns, computes remaining time strings, formats them onto the grid. Called by `rotation.rs` and by a separate tick task for live countdowns. |
| `db.rs` | All SQLite queries via `sqlx`. Handles messages, countdowns, queue, and config CRUD. |
| `config.rs` | Loads `herald.toml`, merges environment variable overrides (`HERALD_*`). |

### 3.6 herald-cli — Terminal Viewer & Admin

```toml
[package]
name = "herald-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "herald"
path = "src/main.rs"

[dependencies]
herald-common = { path = "../herald-common" }
ratatui = "0.29"
crossterm = "0.28"
tokio = { workspace = true }
tokio-tungstenite = "0.26"
clap = { version = "4", features = ["derive"] }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
reqwest = { version = "0.12", features = ["json"] }
url = "2"
```

**Key modules:**

| Module | Purpose |
|---|---|
| `viewer/` | ratatui rendering loop: draws the 6×22 grid, handles flip animations (character cycling between frames), maps `Color` tiles to terminal background colors. |
| `admin/` | Clap subcommand handlers: `push`, `countdown`, `queue`, `config`. Each sends REST requests via `reqwest`. |
| `ws.rs` | WebSocket client: connects to the server, deserializes `ServerMessage`, feeds board state updates to the viewer. Handles reconnection on disconnect. |

### 3.7 herald-web — Leptos WebAssembly Frontend

```toml
[package]
name = "herald-web"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
herald-common = { path = "../herald-common", features = ["wasm"] }
leptos = { version = "0.7", features = ["csr"] }
serde = { workspace = true }
serde_json = { workspace = true }
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "WebSocket", "MessageEvent", "ErrorEvent", "CloseEvent",
    "HtmlAudioElement", "Window", "Document",
] }
js-sys = "0.3"
gloo-timers = "0.3"
console_error_panic_hook = "0.1"
```

**Key modules:**

| Module | Purpose |
|---|---|
| `app.rs` | Root Leptos component, manages WebSocket lifecycle as a reactive resource. |
| `board.rs` | Board grid component: renders 6 rows × 22 columns of `<Flap>` child components. |
| `flap.rs` | Individual flap tile: CSS 3D transform animation (`rotateX` flip), handles character-to-character transition with intermediate flip states. |
| `ws.rs` | WebSocket client using browser `WebSocket` API (via `web-sys`), parses `ServerMessage`, feeds Leptos signals. |
| `sound.rs` | Optional "clack" sound via `HtmlAudioElement`. Toggled by user preference (localStorage). |

---

## 4. Data Flow Diagrams

### 4.1 Admin Pushes a Message

This is the primary write path — an admin creates a new message and it appears on all viewers.

```
  Admin (curl/CLI)                Herald Server                       Viewers
       │                               │                                  │
       │  POST /api/messages            │                                 │
       │  Authorization: Bearer <token> │                                 │
       │  { grid: [[...]], label: ... } │                                 │
       │──────────────────────────────►│                                  │
       │                               │                                  │
       │                    ┌──────────┴────────────┐                     │
       │                    │ 1. Validate auth      │                     │
       │                    │ 2. Validate grid      │                     │
       │                    │    (6×22, valid chars)│                     │
       │                    │ 3. Generate UUID      │                     │
       │                    │ 4. INSERT into SQLite │                     │
       │                    │ 5. Add to queue       │                     │
       │                    └──────────┬────────────┘                     │
       │                               │                                  │
       │  201 Created                  │                                  │
       │  { id: "...", position: 3 }   │                                  │
       │◄──────────────────────────────│                                  │
       │                               │                                  │
       │                    ┌──────────┴───────────┐                      │
       │                    │ 6. If message has    │                      │
       │                    │    priority          │                      │
       │                    │    "immediate"       │                      │
       │                    │    → recompute board │                      │
       │                    │    → broadcast       │                      │
       │                    │ 7. Else: queued for  │                      │
       │                    │    next rotation slot│                      │
       │                    └──────────┬───────────┘                      │
       │                               │                                  │
       │                               │  ServerMessage::BoardState       │
       │                               │  { cells: [[...]] }              │
       │                               │───────────────────────────────►  │
       │                               │        (to ALL connected         │
       │                               │         WebSocket clients)       │
```

### 4.2 Rotation Timer Fires

The rotation engine advances through the queue automatically.

```
  Rotation Task (tokio::interval)     AppState              Broadcast Channel
       │                                  │                                 │
       │  timer tick (every N seconds)    │                                 │
       │─────────────────────────────────►│                                 │
       │                                  │                                 │
       │              ┌───────────────────┴───────────┐                     │
       │              │ 1. Load next queue item from  │                     │
       │              │    SQLite (advance index)     │                     │
       │              │ 2. If QueueItem::Message:     │                     │
       │              │    → use stored grid as-is    │                     │
       │              │ 3. If QueueItem::Countdown:   │                     │
       │              │    → compute remaining time   │                     │
       │              │    → format onto 6×22 grid    │                     │
       │              │ 4. Build new BoardState       │                     │
       │              │ 5. Update active_item_id      │                     │
       │              │ 6. Reset seconds_until_rot.   │                     │
       │              └───────────────────┬───────────┘                     │
       │                                  │                                 │
       │                                  │  broadcast.send()               │
       │                                  │───────────────────────►         │
       │                                  │                                 │
       │                                  │           ┌────────────┴────────┐
       │                                  │           │ Fan out to all      │
       │                                  │           │ subscriber Rx       │
       │                                  │           │ channels            │
       │                                  │           └────────────┬────────┘
       │                                  │                                 │
       │                                  │                 CLI ◄──┤
       │                                  │                 Web ◄──┤
       │                                  │                 ... ◄──┘
```

### 4.3 Countdown Tick

When the active queue item is a countdown, the board must update more frequently (e.g., every second for `HH:MM:SS` format or every minute for `DaysHoursMinutes` format) to keep the displayed time accurate.

```
  Countdown Tick Task                AppState                  Viewers
       │                                │                           │
       │  tick (1s or 60s depending     │                           │
       │        on active countdown     │                           │
       │        format)                 │                           │
       │───────────────────────────────►│                           │
       │                                │                           │
       │            ┌───────────────────┴────────────┐              │
       │            │ 1. Check if active item is a   │              │
       │            │    Countdown (else skip)       │              │
       │            │ 2. Recompute remaining time    │              │
       │            │ 3. Re-render countdown grid    │              │
       │            │ 4. Diff against BoardState     │              │
       │            │ 5. If changed → build new      │              │
       │            │    state and broadcast         │              │
       │            │ 6. If countdown hit zero:      │              │
       │            │    → apply zero_behavior       │              │
       │            │    (ShowMessage/Remove/Hold)   │              │
       │            └───────────────────┬────────────┘              │
       │                                │                           │
       │                                │  BoardState (if           │
       │                                │  digits changed)          │
       │                                │────────────────────────►  │
       │                                │                           │
```

### 4.4 New Viewer Connects

A newly connected viewer must immediately see the current board — no waiting for the next rotation.

```
  New Viewer (CLI or Web)           Herald Server
       │                                │
       │  GET /ws (WebSocket upgrade)   │
       │───────────────────────────────►│
       │                                │
       │  101 Switching Protocols       │
       │◄───────────────────────────────│
       │                                │
       │                 ┌──────────────┴───────────┐
       │                 │ 1. Accept WS connection  │
       │                 │ 2. Subscribe to broadcast│
       │                 │    channel (new Rx)      │
       │                 │ 3. Read current          │
       │                 │    BoardState from       │
       │                 │    AppState              │
       │                 └──────────────┬───────────┘
       │                                │
       │  ServerMessage::BoardState     │
       │  (current board - immediate)   │
       │◄───────────────────────────────│
       │                                │
       │  ... viewer renders board ...  │
       │                                │
       │  (subsequent updates arrive    │
       │   via broadcast channel)       │
       │                                │
       │  ServerMessage::Ping           │
       │◄───────────────────────────────│  (every 30s)
       │                                │
       │  ClientMessage::Pong           │
       │───────────────────────────────►│
       │                                │
```

---

## 5. Concurrency Model

Herald's backend runs on the Tokio async runtime. The concurrency design ensures that admin writes, viewer connections, and background timers operate without blocking each other.

### 5.1 Tokio Task Architecture

The server spawns the following long-lived tasks at startup:

```
tokio::main
 │
 ├── Axum server (HTTP listener)
 │    ├── REST API handlers (spawned per-request by Axum)
 │    └── WebSocket upgrade handler
 │         └── Per-connection task (spawned on each WS upgrade)
 │              ├── Read loop (client → server)
 │              └── Write loop (broadcast Rx → client)
 │
 ├── Rotation timer task (tokio::spawn)
 │    └── tokio::time::interval(rotation_duration)
 │
 ├── Countdown tick task (tokio::spawn)
 │    └── tokio::time::interval(1s) — only active when a countdown is displayed
 │
 └── Shutdown signal listener (tokio::signal::ctrl_c / SIGTERM)
```

All tasks share access to the application state via `Arc<AppState>`:

```rust
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

pub struct AppState {
    /// Current board state — read by new WS connections, written by rotation/countdown tasks.
    pub board: RwLock<BoardState>,

    /// Broadcast channel sender — all viewer connections subscribe to this.
    pub broadcast_tx: broadcast::Sender<ServerMessage>,

    /// Database connection pool.
    pub db: sqlx::SqlitePool,

    /// Server configuration (read-only after startup, or behind RwLock if runtime-mutable).
    pub config: RwLock<Config>,

    /// Shutdown signal — notifies all tasks to wind down.
    pub shutdown: tokio::sync::watch::Sender<bool>,
}
```

### 5.2 WebSocket Connection Management

Each WebSocket connection is managed by a dedicated Tokio task that is split into two halves:

```
                   ┌───────────────────────────────────┐
                   │      Per-Connection Task          │
                   │                                   │
  broadcast::Rx ──►│  Write half:                      │──► WebSocket sink
                   │   loop { msg = rx.recv() }        │    (send to client)
                   │                                   │
  WebSocket     ──►│  Read half:                       │──► (process pongs,
  stream           │   loop { msg = ws.recv() }        │     detect disconnect)
                   │                                   │
                   └───────────────────────────────────┘
```

**Broadcast channel semantics:**

- The server holds one `broadcast::Sender<ServerMessage>`.
- Each new WebSocket connection calls `broadcast_tx.subscribe()` to get a `broadcast::Receiver`.
- When the rotation timer or admin API sends a new `BoardState` via `broadcast_tx.send()`, all receivers wake up and forward the message to their respective WebSocket client.
- If a receiver falls behind (slow client), `broadcast` automatically drops old messages (`RecvError::Lagged`). The write loop detects this and sends the latest state, skipping intermediate frames.
- Channel capacity is set to `16` — more than enough for board updates that arrive at most once per second.

**Connection cleanup:**

- When a viewer disconnects (WebSocket close frame or TCP drop), the read half detects it and the task exits.
- The `broadcast::Receiver` is dropped automatically when the task exits — no explicit unsubscription needed.
- No connection registry or tracking is required for basic operation. An optional connection counter (via `AtomicUsize`) can be used for metrics.

### 5.3 Rotation Timer

The rotation timer runs as a standalone Tokio task:

```rust
async fn rotation_task(state: Arc<AppState>, mut shutdown_rx: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(state.config.read().await.rotation_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // 1. Advance queue index
                // 2. Load next item from DB
                // 3. Compute new BoardState
                // 4. Update state.board (write lock)
                // 5. Broadcast to all viewers
            }
            _ = shutdown_rx.changed() => {
                break; // Graceful shutdown
            }
        }
    }
}
```

**Key behaviors:**

- `MissedTickBehavior::Skip` ensures that if the server is under load and misses a tick, it doesn't try to "catch up" with multiple rapid rotations.
- The interval duration can be updated at runtime: the admin API writes the new duration to config and resets the interval.
- When the queue is empty, the rotation task still ticks but broadcasts a blank board (all `CellContent::Blank`).

### 5.4 Admin API Concurrency

Admin REST handlers and the rotation timer both need to read/write shared state. The `RwLock<BoardState>` ensures:

- **Multiple readers** (new WS connections reading current state) can proceed concurrently.
- **Single writer** (rotation task or admin API updating the board) gets exclusive access.
- Write locks are held for the minimum duration — only while swapping the `BoardState` value, not during database I/O or broadcast.

The pattern:

```
Admin API handler:
  1. Validate request (no lock needed)
  2. Write to SQLite (database pool handles concurrency internally)
  3. If immediate display: acquire write lock on board → update → release → broadcast
  4. Else: return success (next rotation will pick up the new item)
```

Database concurrency is handled by `sqlx::SqlitePool` with WAL (Write-Ahead Logging) mode enabled, which allows concurrent reads with a single writer — well-suited for Herald's workload (many reads, infrequent writes).

### 5.5 Graceful Shutdown

On receiving SIGTERM or Ctrl+C:

```
1. Shutdown signal received
        │
        ▼
2. Set shutdown watch channel → true
        │
        ├──► Rotation task exits its select! loop
        ├──► Countdown tick task exits its select! loop
        │
        ▼
3. Broadcast ServerMessage::Shutdown { reason: "server shutting down" }
        │
        ├──► All WebSocket write loops send shutdown message to clients
        ├──► All WebSocket write loops exit
        │
        ▼
4. Axum server stops accepting new connections
        │
        ▼
5. Wait for in-flight HTTP requests to complete (Axum graceful shutdown)
        │
        ▼
6. Close SQLite connection pool
        │
        ▼
7. Process exits
```

**Client-side behavior on shutdown:**

- CLI: Displays "Server disconnected" and enters reconnection loop (exponential backoff).
- Web: Shows a "Reconnecting..." overlay and retries the WebSocket connection.

---

## 6. Communication Protocols

### 6.1 REST API (Admin)

The REST API is the admin's interface to Herald. All endpoints live under `/api/` and require a bearer token in the `Authorization` header.

**Endpoint overview:**

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/messages` | Push a new message to the queue |
| `GET` | `/api/messages` | List all messages in the queue |
| `DELETE` | `/api/messages/{id}` | Remove a message from the queue |
| `POST` | `/api/countdowns` | Create a new countdown |
| `GET` | `/api/countdowns` | List all countdowns |
| `PUT` | `/api/countdowns/{id}` | Update a countdown |
| `DELETE` | `/api/countdowns/{id}` | Remove a countdown |
| `GET` | `/api/queue` | Get the full rotation queue (ordered) |
| `PUT` | `/api/queue/reorder` | Reorder the rotation queue |
| `GET` | `/api/board` | Get the current board state |
| `GET` | `/api/config` | Get current configuration |
| `PUT` | `/api/config` | Update configuration |
| `GET` | `/api/health` | Health check (no auth required) |

> **Full API specification** — request/response schemas, example payloads, error codes, and auth details — is documented in [SPEC.md](./SPEC.md).

### 6.2 WebSocket Protocol (Viewers)

**Connection:**

- **Endpoint:** `ws://<host>:<port>/ws` (or `wss://` behind TLS-terminating reverse proxy)
- **No authentication required** — the WebSocket endpoint is public (view-only).
- **Subprotocol:** None required.

**Message format:**

All WebSocket frames are JSON text. Messages use a tagged union format with a `"type"` discriminant field.

**Server → Client messages:**

```json
// Board update (sent on connect, rotation tick, countdown refresh, admin push/update/delete)
{
  "type": "board_update",
  "grid": [[/* 6 rows of 22 CellContent cells */]],
  "previous_grid": [[/* 6 rows of 22 CellContent cells */]],
  "current_item": { "id": "...", "kind": "message", "label": "HELLO" },
  "timestamp": "2025-07-14T10:30:45Z"
}

// Queue info (sent alongside every board_update as a separate message)
{
  "type": "queue_info",
  "current_index": 2,
  "total_items": 7,
  "next_rotation_seconds": 18,
  "is_countdown_active": false
}

// Heartbeat ping (every 30 seconds)
{
  "type": "ping",
  "timestamp": 1700000000
}

// Shutdown notice
{
  "type": "shutdown",
  "reason": "server shutting down"
}
```

When the queue is empty, `current_item` is `null` and `grid` contains the "HERALD" splash screen (see [Empty Queue Behavior](#empty-queue-behavior)).

**Client → Server messages:**

```json
// Heartbeat pong (response to ping)
{
  "type": "pong",
  "timestamp": 1700000000
}
```

**Connection lifecycle:**

1. Client opens WebSocket to `/ws`.
2. Server immediately sends a `board_update` with the current board, followed by a `queue_info` message.
3. Server sends `ping` every 30 seconds.
4. Client must respond with `pong` within 10 seconds (echoing the timestamp).
5. If the server receives no pong after 2 consecutive pings, it closes the connection.
6. On board changes, server sends a new `board_update` followed by `queue_info`.
7. When the active item is a countdown, the server sends `board_update` + `queue_info` every 1 second (countdown refresh).
8. On server shutdown, server sends `shutdown` then closes the connection.

**Client reconnection strategy:**

| Attempt | Delay |
|---|---|
| 1st | 1 second |
| 2nd | 2 seconds |
| 3rd | 4 seconds |
| 4th | 8 seconds |
| 5th+ | 15 seconds (capped) |

On successful reconnect, the client receives the full current `board_update` + `queue_info` and resumes normal operation. No state reconciliation is needed — the server always sends the complete board, never deltas.

#### WebSocket Message Types

The server sends JSON messages over WebSocket. Each message has a `type` field discriminator:

| Type | Trigger | Description |
|---|---|---|
| `board_update` | Connection, rotation tick, countdown refresh (1s), admin mutation | Full board grid, previous grid (for animation diffing), current item metadata, and timestamp |
| `queue_info` | Sent alongside every `board_update` | Rotation metadata: `current_index` (0-based), `total_items`, `next_rotation_seconds`, `is_countdown_active` |
| `ping` | Every 30 seconds | Heartbeat; client must reply with `pong` |
| `shutdown` | Server shutdown | Advance notice before connection close |

**`queue_info` fields:**

- `current_index` — 0-based index into the sorted queue
- `total_items` — total number of active (non-expired) items in the queue
- `next_rotation_seconds` — seconds until the next rotation tick
- `is_countdown_active` — `true` when the current item is a countdown (board updates every 1s)

#### Empty Queue Behavior

When no messages or countdowns are in the queue, the board displays a default splash screen:

```
                      
                      
     H E R A L D     
                      
                      
                      
```

The splash is generated by `herald_common::splash_grid()` and returned by `BoardState::default()`. It persists until the admin pushes the first message. The `current_item` field in the `board_update` is `null` when the splash is displayed.

#### Error Resilience

If the server fails to build the new board state during a broadcast cycle, it re-broadcasts the previous known-good state and logs the error at `error` level. Clients never receive an empty or corrupted board. This applies to all broadcast triggers: rotation ticks, countdown refreshes, and admin-initiated updates.

#### Expired Item Handling

Items with an `expires_at` timestamp are soft-deleted when they expire — the `deleted_at` column is set rather than removing the row. This preserves history for auditing. A background cleanup task runs every 60 seconds to expire stale items. All active queries filter on `WHERE deleted_at IS NULL`.

### 6.3 Cross-References

| Topic | Document |
|---|---|
| Full REST API specification (schemas, examples, errors) | [SPEC.md](./SPEC.md) |
| Data models and SQLite schema | [SPEC.md](./SPEC.md) |
| Split-flap rendering (terminal) | [SPEC.md](./SPEC.md) |
| Split-flap rendering (web/Wasm) | [SPEC.md](./SPEC.md) |
| Configuration reference | [SPEC.md](./SPEC.md) |
| Architecture decisions and rationale | [DECISIONS.md](./DECISIONS.md) |
| Docker and deployment | [DEPLOYMENT.md](./DEPLOYMENT.md) |
| Build from source and contributing | [CONTRIBUTING.md](./CONTRIBUTING.md) |
| Development phases and GitHub issues | [ROADMAP.md](./ROADMAP.md) |

---

*This document is the architectural foundation for Herald. For implementation-level detail, start with [SPEC.md](./SPEC.md). For the rationale behind key decisions, see [DECISIONS.md](./DECISIONS.md).*
