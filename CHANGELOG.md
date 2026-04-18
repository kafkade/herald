# Changelog

All notable changes to Herald will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Web viewer development proxy configuration in `Trunk.toml` (auto-proxies `/api` and `/ws` to the server)
- Web viewer section in README with setup instructions for both production and development modes
- `requestAnimationFrame` batching for WebSocket board updates — coalesces rapid messages into a single paint frame for smoother 60fps animations
- `herald push "TEXT"` command to post messages to the board via REST API, with `--align`, `--expires`, `--server`, and `--token` flags
- `herald countdown create`, `list`, and `delete` subcommands for managing countdowns via REST API, with formatted table output showing remaining time
- `herald queue list` and `herald queue reorder` subcommands for viewing and reordering the display queue
- `herald config get` and `herald config set` subcommands for viewing and updating server configuration
- Color markup parser in `herald-common` supporting `{red}text{/red}` syntax for all 8 Vestaboard colors, with nested tag and shorthand `{}` close support

### Changed

- Countdown animations in `herald watch` now use instant card-flip instead of cycling through intermediate characters, preventing animation overlap with 1-second countdown ticks

### Fixed

- Trunk dev server proxy error (`UnsupportedUrlScheme`) when connecting to the WebSocket backend

## [0.5.0] - 2026-04-16

### Added

- Cargo workspace with `herald-common` and `herald-server` crates
- Shared types: Grid (6×22), CellContent, Color, Message, Countdown, QueueItem, BoardState
- SQLite persistence with sqlx migrations (WAL mode)
- REST API: full CRUD for messages, countdowns, queue management, and configuration
- Bearer token authentication for admin endpoints
- Health check endpoint (unauthenticated) returning status, version, and uptime
- 17 integration tests covering all API endpoints and error cases
- CLI binary (`herald`) with subcommands: serve, watch, push, countdown, queue, config
- Real-time board update broadcasts to WebSocket viewers on every content mutation
- Immediate board state delivery to WebSocket clients on connect (no waiting for next update)
- CLI WebSocket client with automatic reconnection and exponential backoff (1s–30s)
- `--server` option on `herald watch` to specify the WebSocket server URL
- Terminal board viewer (`herald watch`) with live-updating 6×22 grid, color tile rendering, and centered viewport
- Connection status bar showing state (Connected/Connecting/Reconnecting/Disconnected), server URL, and time since last update
- `--fps` option on `herald watch` to control UI refresh rate (default 30)
- Server-side WebSocket ping/pong heartbeat (30s interval, 10s timeout) to detect and close stale connections
- Background queue rotation task with configurable interval (`rotation_interval_secs` in config)
- Automatic expired message cleanup during rotation (per spec §5.7)
- Countdown timer rendering on the board grid with label, formatted time remaining, and template placeholders
- Real-time countdown refresh (1-second broadcasts) when a countdown is the active display item
- Per-countdown zero behavior: `ShowZero`, `Remove`, `Pause`, and `ShowMessage` when countdown reaches zero
- Soft-delete for expired queue items (preserved in history with `deleted_at` timestamp)
- Periodic cleanup task (60s interval) that auto-expires messages past their `expires_at` time
- Default "HERALD" splash screen displayed when the queue is empty
- Rotation metadata (`queue_info`) broadcast alongside every board update for viewer status bars
- Rotation progress display in CLI status bar ("Item 2/5 · Next in 18s" or "Countdown active")
- Text-based message creation via `POST /api/messages` with `{"text": "..."}` — auto-renders to 6×22 grid with word-wrapping, alignment, and character normalization
- Dual-mode message API: accepts either `text` (auto-rendered) or `grid` (raw 6×22) for both create and update
- Source text preservation on messages for round-trip editing and alignment reflow
- Split-flap flip animation in `herald watch` — tiles cycle through intermediate characters with a "─X─" mid-flip visual when the board transitions between messages
- Left-to-right cascade stagger effect: columns start their flip animation with a 20ms delay between each, creating the characteristic split-flap wave
- Time-based animation engine with configurable step duration (50ms/char) and column stagger (20ms), smooth mid-animation restarts when new board updates arrive
- Board diff detection: only changed cells animate on board transitions, identical boards skip animation entirely
- Terminal resize handling: board re-centers on resize, in-progress animations cancel and snap to target state
- PowerShell helper scripts (`scripts/`) for common operations: `add-message`, `remove-message`, `add-countdown`, `set-rotation-interval`
- ANSI 256-color support for color tiles with automatic fallback to basic 8-color mode on older terminals
- `--animation-speed` flag on `herald watch`: `fast`, `normal` (default), `slow`, or `off` to control flip animation timing
- `list-queue.ps1` script to display all messages and countdowns currently in the rotation queue
- All PowerShell scripts auto-detect `$env:HERALD_ADMIN_TOKEN`, removing the need to pass `-Token` every time
- `add-color-message.ps1` script to push messages with colored tile fills (`-Color`, `-FillRows`, `-BgColor`)
- `remove-countdown.ps1` script to remove countdowns by ID or all at once
- Web viewer crate (`herald-web`) with Leptos CSR and Trunk build tooling
- WebSocket client for the browser with auto-detected server URL and exponential backoff reconnection
- 6×22 split-flap board grid rendered with CSS Grid, dark theme, and 3D perspective tilt
- Split-flap tile component with top/bottom half rendering and visible horizontal split line
- 3D CSS flip animation on board transitions (150ms per flap, top-half rotates away then bottom-half drops in)
- Left-to-right cascade stagger effect on web flip animations (20ms column delay, ~615ms full board)
- Color tile rendering with 8 Vestaboard-compatible colors mapped to CSS custom properties for theming
- Connection status bar with live connected/reconnecting indicator
- Responsive board layout with mobile horizontal scroll and large-screen centering
- Loading shimmer animation for tiles before WebSocket connection is established
- Fine-grained reactive signals (one per cell) for optimal re-render performance — only changed cells update
- Responsive board scaling: tiles auto-size to fit the viewport at any screen width (375px–1440px+) without scrolling
- Loading state with gradient shimmer animation while the WebSocket connection is being established
- Flip animation on initial connect: all tiles animate from blank to their first board state with cascade stagger
- Connection status indicator (bottom-right pill): green when connected (auto-hides after 3s), yellow while connecting, red when reconnecting
- Static web asset serving from `herald-server` via `HERALD_WEB_DIR` env var (default `./web-dist`), with SPA fallback routing
- `build-web.ps1` script to compile `herald-web` with Trunk and copy output to `web-dist/`

### Changed

- Optimized animation rendering: pre-allocated display buffers and frame-skip logic for smooth 30fps playback
- Color tiles now animate with a split-flap color cycling effect on board transitions, flipping through intermediate colors (e.g., Red → Orange → Yellow → Green → Blue) instead of snapping instantly
- Web board now scales responsively to always fit the viewport without scrolling (replaced mobile horizontal scroll with viewport-based tile sizing)
- Web loading state now uses a gradient shimmer sweep and triggers flip animation on all tiles when the first board update arrives
- Web connection status indicator repositioned to a fixed bottom-right pill that auto-hides 3 seconds after connecting

### Fixed

- `herald watch` now correctly exits on `q`, `Esc`, or `Ctrl+C` (previously these keypresses were silently ignored)
