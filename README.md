# Herald

[![CI](https://github.com/kafkade/herald/actions/workflows/ci.yml/badge.svg)](https://github.com/kafkade/herald/actions/workflows/ci.yml)

**A split-flap message board for your terminal and browser.**

Herald is an open-source, split-flap / Vestaboard-style digital message board and countdown tracker built entirely in Rust. Inspired by the iconic departure boards in train stations — where letters and symbols mechanically flip into place — Herald brings that aesthetic to both the terminal and the web browser, backed by a real-time server.

<!-- TODO: Add hero image / demo GIF -->

---

## ✨ Features

- **6×22 character grid** — Vestaboard-compatible format
- **Full character set** — A–Z, 0–9, special characters, plus colored tiles (red, orange, yellow, green, blue, violet, white, black)
- **Text-based message API** — just send `{"text": "HELLO WORLD"}` with automatic word-wrapping, alignment, and character normalization
- **Raw grid API** — full control over individual cells for color tiles and custom layouts
- **Message queue** with configurable rotation (default 10 s)
- **Countdown timers** mixed into the rotation with real-time ticking
- **Real-time WebSocket push** to all connected viewers
- **Terminal viewer** — split-flap flip animations with left-to-right cascade stagger
- **Web viewer** — Leptos + WebAssembly with 3D CSS flip animations, responsive scaling, and real-time WebSocket connection
- **ANSI 256-color support** — vibrant color tiles with automatic fallback for basic terminals
- **Animation speed control** — `fast`, `normal`, `slow`, or `off`
- **Simple auth** — single admin with bearer-token authentication
- **SQLite persistence** — data survives restarts
- **PowerShell helper scripts** — quick admin without remembering curl syntax
- **CLI admin tools** — `herald push`, `herald countdown`, `herald queue`, `herald config` subcommands for full board management from the terminal
- **Message preview** — `--preview` flag renders an ASCII grid preview before pushing
- **Color markup** — inline `{red}text{/red}` syntax for colored text in messages
- **Web admin panel** — `/admin` route with token auth, message composer with live preview, countdown manager, queue manager with drag-to-reorder, and server config editor
- **Docker deployment** — multi-stage Dockerfile with cargo-chef caching, Docker Compose with healthcheck and persistent volumes
- **Graceful shutdown** — clean SIGTERM/SIGINT handling with WebSocket client notification
- **Structured logging** — HTTP request/response tracing with optional JSON output format
- **All Rust** — monorepo, one toolchain

### Roadmap (not yet implemented)

- Sound effects, themes, scheduling, rate limiting

---

## 🚀 Quick Start

### Prerequisites

- Rust toolchain (see `rust-toolchain.toml` for the exact version)
- A terminal that supports ANSI colors (most modern terminals do)

### 1. Build

```bash
git clone https://github.com/kafkade/herald.git
cd herald
cargo build --release
```

### 2. Start the server

```powershell
cargo run -p herald-server
```

The server prints an admin token on first startup — **copy it**. You can also set it via environment variable:

```powershell
$env:HERALD_ADMIN_TOKEN = "my-secret-token"
cargo run -p herald-server
```

The server listens on port 3000 by default. Configure with `$env:HERALD_PORT`.

### 3. Connect the terminal viewer

In a second terminal:

```powershell
cargo run -p herald-cli -- watch
```

You'll see the HERALD splash screen. Options:

```powershell
# Custom server URL
cargo run -p herald-cli -- watch --server ws://myhost:3000/ws

# Adjust frame rate
cargo run -p herald-cli -- watch --fps 60

# Control animation speed: fast, normal, slow, or off
cargo run -p herald-cli -- watch --animation-speed fast
cargo run -p herald-cli -- watch --animation-speed off    # instant transitions
```

Press `q` or `Esc` to exit the viewer.

### 4. Push messages

Set your token once, then use the helper scripts:

```powershell
$env:HERALD_ADMIN_TOKEN = "your-token-here"

# Simple text message
.\scripts\add-message.ps1 -Text "HELLO WORLD"

# Left-aligned message
.\scripts\add-message.ps1 -Text "DEPARTURE 08:45" -HAlign left

# Message that expires after 5 minutes
.\scripts\add-message.ps1 -Text "FLASH SALE" -ExpiresIn 300

# Colored message with green background fill
.\scripts\add-color-message.ps1 -Text "GO TEAM" -Color green -FillRows all

# Colored text with decorative rows
.\scripts\add-color-message.ps1 -Text "ALERT" -Color red -FillRows top

# Create a countdown
.\scripts\add-countdown.ps1 -Label "LAUNCH" -Target "2026-12-31T00:00:00Z"
.\scripts\add-countdown.ps1 -Label "STANDUP" -InMinutes 15

# List everything in the queue
.\scripts\list-queue.ps1

# Remove a message by ID
.\scripts\remove-message.ps1 -Id "some-uuid"

# Remove all messages
.\scripts\remove-message.ps1 -All

# Change rotation speed
.\scripts\set-rotation-interval.ps1 -Seconds 15
```

### 4b. CLI admin commands

Herald includes built-in admin subcommands (no scripts needed):

```bash
# Push a message
herald push "HELLO WORLD" --token secret
herald push "LEFT ALIGNED" --align left --token secret
herald push "PREVIEW FIRST" --preview --token secret

# Manage countdowns
herald countdown create --label "LAUNCH" --target "2026-12-31T00:00:00Z" --token secret
herald countdown list --token secret
herald countdown delete <UUID> --token secret

# View and reorder the queue
herald queue list --token secret
herald queue reorder <id1> <id2> <id3> --token secret

# View and update config
herald config get --token secret
herald config set rotation_interval_seconds 15 --token secret
```

The `--token` flag can also be set via `HERALD_ADMIN_TOKEN` environment variable.

### 5. Open the web viewer

**Option A — Served from the server (production-style):**

```powershell
# Build the web assets (requires Trunk: cargo install trunk)
.\scripts\build-web.ps1

# Restart the server — it auto-detects web-dist/
cargo run -p herald-server
```

Open `http://localhost:3000` in your browser. The server serves both the API and the web viewer.

**Option B — Development with hot-reload:**

```powershell
# Terminal 1: start the server
cargo run -p herald-server

# Terminal 2: start the Trunk dev server
cd crates\herald-web
trunk serve
```

Open `http://localhost:8080` — Trunk serves the web viewer with live-reload and proxies API/WebSocket requests to the server.

### 5b. Web admin panel

Navigate to `http://localhost:3000/admin` (or `http://localhost:8080/admin` in dev mode) to access the admin panel. Features:
- **Message Composer** — type a message and see a live 6×22 grid preview, select alignment, set optional expiry, and push
- **Countdown Manager** — create, view (with live remaining time), and delete countdowns
- **Queue Manager** — view queue items and drag-to-reorder
- **Config Editor** — view and edit all server settings

You'll be prompted for your admin token on first visit. The token is stored in your browser's localStorage.

### 6. Docker deployment

```bash
# Quick start with Docker Compose
HERALD_ADMIN_TOKEN=your-secret docker compose up --build

# Or customize via .env file (see .env.example)
cp .env.example .env
# Edit .env with your settings
docker compose up -d
```

The Docker image uses a multi-stage build with cargo-chef for dependency caching. The runtime image is based on `debian:bookworm-slim` and includes a healthcheck on `/api/health`.

### Using curl instead

```bash
TOKEN="your-token"

# Push a text message
curl -X POST http://localhost:3000/api/messages \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"text": "HELLO WORLD"}'

# Create a countdown
curl -X POST http://localhost:3000/api/countdowns \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"label": "LAUNCH", "target": "2026-12-31T00:00:00Z"}'

# List the queue
curl http://localhost:3000/api/queue \
  -H "Authorization: Bearer $TOKEN"

# Check server health (no auth needed)
curl http://localhost:3000/api/health
```

---

## 📜 PowerShell Scripts

All scripts live in `scripts/` and auto-detect `$env:HERALD_ADMIN_TOKEN`. Pass `-Token` to override.

| Script | Description |
|---|---|
| `add-message.ps1` | Push a text message (`-Text`, `-HAlign`, `-VAlign`, `-ExpiresIn`) |
| `add-color-message.ps1` | Push a message with colored tile fills (`-Color`, `-FillRows`, `-BgColor`) |
| `add-countdown.ps1` | Create a countdown (`-Label`, `-Target` or `-InMinutes` / `-InHours`) |
| `remove-message.ps1` | Remove by `-Id`, `-All`, or list messages (no args) |
| `list-queue.ps1` | Show all messages and countdowns in the rotation |
| `set-rotation-interval.ps1` | Set rotation speed in `-Seconds` |
| `build-web.ps1` | Build `herald-web` with Trunk and copy output to `web-dist/` |

---

## 🏗 Architecture

Herald is structured as a Cargo workspace:

```
crates/
  herald-common/    — Shared types (Grid, CellContent, Message, Countdown, etc.)
  herald-server/    — Axum REST API + WebSocket + SQLite persistence + static file serving
  herald-cli/       — Terminal viewer (ratatui TUI) with split-flap animation
  herald-web/       — Browser viewer (Leptos + Wasm) with 3D CSS flip animations
```

```
             ┌───────────────────────┐
             │    herald-server      │
             │    (Axum + SQLite)    │
             └───┬─────────────┬─────┘
      REST API   │             │  WebSocket
     (admin ops) │             │  (real-time push)
                 │             │
         ┌───────┘             └───────┐
         │                             │
┌────────▼─────────┐        ┌─────────▼─────────┐
│   herald-cli     │        │   herald-web      │
│  (ratatui TUI)   │        │  (Leptos + Wasm)  │
└────────┬─────────┘        └─────────┬─────────┘
         │                            │
         └────────────┬───────────────┘
              ┌───────▼───────┐
              │ herald-common │
              │ (shared types)│
              └───────────────┘
```

**Data flow:** HTTP request → Axum handler → SQLite → JSON response. Board state broadcast via WebSocket to all connected viewers on every mutation.

For details, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## 🔧 REST API

All write endpoints require `Authorization: Bearer <token>`.

### Messages

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/messages` | Create a message (`text` or `grid`) |
| `GET` | `/api/messages` | List all messages |
| `GET` | `/api/messages/:id` | Get a message |
| `PUT` | `/api/messages/:id` | Update a message |
| `DELETE` | `/api/messages/:id` | Delete a message |

**Create with text** (auto-wrapped to 6×22):
```json
{"text": "HELLO WORLD", "h_align": "center", "v_align": "middle"}
```

**Create with raw grid** (for color tiles):
```json
{"grid": [[{"type":"char","value":"H"}, {"type":"color","value":"red"}, {"type":"blank"}, ...]]}
```

### Countdowns

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/countdowns` | Create a countdown |
| `GET` | `/api/countdowns` | List all countdowns |
| `GET` | `/api/countdowns/:id` | Get a countdown |
| `DELETE` | `/api/countdowns/:id` | Delete a countdown |

```json
{"label": "LAUNCH", "target": "2026-12-31T00:00:00Z", "zero_behavior": {"action": "show_zero"}}
```

Zero behaviors: `show_zero`, `remove`, `pause`, `show_message`.

### Queue & Config

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/queue` | List queue items in display order |
| `PUT` | `/api/queue/reorder` | Reorder queue items |
| `GET` | `/api/config` | Get all config values |
| `PUT` | `/api/config` | Update config values |
| `GET` | `/api/health` | Health check (no auth) |

---

## ⚙️ Environment Variables

| Variable | Default | Description |
|---|---|---|
| `HERALD_ADMIN_TOKEN` | *(auto-generated)* | Bearer token for admin API |
| `HERALD_DB_PATH` | `herald.db` | SQLite database file path |
| `HERALD_PORT` | `3000` | Server listen port |
| `HERALD_LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `HERALD_WEB_DIR` | `./web-dist` | Directory for compiled web viewer assets (auto-detected) |
| `HERALD_LOG_FORMAT` | *(pretty)* | Log output format: `json` for structured logs, default is human-readable |

---

## 🧪 Development

```bash
cargo build                       # Debug build (server + CLI)
cargo test                        # Run all tests (103+ tests)
cargo clippy                      # Lint
cargo fmt                         # Format
cargo run -p herald-server        # Start the server
cargo run -p herald-cli -- watch  # Start the terminal viewer
```

### Web viewer development

```bash
# Prerequisites (one-time)
rustup target add wasm32-unknown-unknown
cargo install trunk

# Check the web crate compiles
cargo check -p herald-web --target wasm32-unknown-unknown

# Build for production
.\scripts\build-web.ps1 -Release

# Dev server with hot-reload (in crates/herald-web/)
trunk serve
```

Note: `herald-web` is excluded from the default workspace members since it requires the `wasm32-unknown-unknown` target. Use `-p herald-web --target wasm32-unknown-unknown` when checking or building it directly.

---

## 📄 License

Herald is licensed under the [MIT License](LICENSE-MIT).

---

## 🙏 Acknowledgments

- [Vestaboard](https://www.vestaboard.com/) — visual inspiration for the 6×22 character grid and colored tile system.
- [Solari di Udine](https://en.wikipedia.org/wiki/Solari_di_Udine) — creators of the original split-flap display mechanism.
- The Rust community and the maintainers of [Axum](https://github.com/tokio-rs/axum), [ratatui](https://github.com/ratatui/ratatui), [Leptos](https://github.com/leptos-rs/leptos), and [sqlx](https://github.com/launchbadge/sqlx).
