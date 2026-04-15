# Herald — Project Roadmap

> **Herald** is an open-source, split-flap / Vestaboard-style message board and countdown tracker built in Rust.
> This roadmap defines the phased development plan and maps directly to GitHub Milestones and Issues.

---

## Table of Contents

- [Overview](#overview)
- [Summary](#summary)
- [Phase 1: Foundation — Shared Types \& Backend REST API](#phase-1-foundation--shared-types--backend-rest-api)
- [Phase 2: Real-Time — WebSocket \& Basic CLI Viewer](#phase-2-real-time--websocket--basic-cli-viewer)
- [Phase 3: Rotation Engine \& Countdown Logic](#phase-3-rotation-engine--countdown-logic)
- [Phase 4: CLI Split-Flap Animation](#phase-4-cli-split-flap-animation)
- [Phase 5: Web Viewer with Wasm](#phase-5-web-viewer-with-wasm)
- [Phase 6: Admin Interfaces](#phase-6-admin-interfaces)
- [Phase 7: Docker \& Deployment Polish](#phase-7-docker--deployment-polish)
- [Phase 8: Stretch Goals](#phase-8-stretch-goals)
- [Future Considerations (Out of Scope for v1)](#future-considerations-out-of-scope-for-v1)

---

## Overview

Herald is developed in **eight phases**, each producing a working (if incomplete) system. Every phase builds on the previous one, delivering a demoable outcome that can be tested end-to-end. This incremental approach ensures that:

1. **Each milestone ships value.** Even Phase 1 gives you a functional REST API you can test with curl.
2. **Risk is front-loaded.** Core data types, persistence, and the server backbone come first.
3. **Viewers are additive.** The CLI and web viewers plug into the same WebSocket broadcast, developed independently.
4. **Polish comes last.** Animation, admin UI, Docker packaging, and stretch goals follow the working core.

Every phase corresponds to a **GitHub Milestone**. Every item within a phase corresponds to a **GitHub Issue**, numbered sequentially across the entire roadmap (#1–#76) so cross-phase dependency references are unambiguous.

For detailed specifications, see [SPEC.md](./SPEC.md). For architecture and crate layout, see [ARCHITECTURE.md](./ARCHITECTURE.md). For deployment instructions, see [DEPLOYMENT.md](./DEPLOYMENT.md).

---

## Summary

| Phase | Milestone | Issues | Key Deliverable |
|-------|-----------|--------|-----------------|
| 1 | `v0.1.0 - Foundation` | 10 (#1–#10) | Cargo workspace, shared types, REST API with SQLite |
| 2 | `v0.2.0 - Real-Time` | 9 (#11–#19) | WebSocket broadcast, basic CLI viewer |
| 3 | `v0.3.0 - Rotation & Countdowns` | 10 (#20–#29) | Queue rotation, countdown ticking, auto-cycling |
| 4 | `v0.4.0 - Terminal Animation` | 8 (#30–#37) | Split-flap flip animation in terminal |
| 5 | `v0.5.0 - Web Interface` | 12 (#38–#49) | Leptos/Wasm web viewer with 3D CSS flips |
| 6 | `v0.6.0 - Admin Tools` | 11 (#50–#60) | CLI admin subcommands, web admin panel |
| 7 | `v0.7.0 - Production Ready` | 8 (#61–#68) | Docker, CI/CD, deployment docs |
| 8 | `v1.0.0 - Polish & Extras` | 8 (#69–#76) | Sound, themes, scheduling, rate limiting |
| | | **76 total** | |

---

## Phase 1: Foundation — Shared Types & Backend REST API

**GitHub Milestone:** `v0.1.0 - Foundation`

**Description:** Bootstrap the Cargo workspace with `herald-common` and `herald-server` crates. Define the canonical shared types for the 6×22 board grid, messages, countdowns, colors, and queue items. Stand up the Axum-based REST API with full CRUD for messages, countdowns, queue management, and configuration — all persisted to SQLite. No WebSocket, no viewers yet.

**Demoable outcome:** An admin can push messages and create countdowns via curl. Data persists across server restarts in SQLite. Bearer token authentication protects all write endpoints.

---

#### Issue #1: Initialize Cargo workspace with herald-common and herald-server crates
**Labels:** `enhancement`, `infra`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** —

Create the root `Cargo.toml` workspace with two member crates: `herald-common` (library) and `herald-server` (binary). Configure shared workspace dependencies for `serde`, `serde_json`, `chrono`, and `uuid`. Add a `.gitignore` for Rust/target artifacts. The workspace should compile cleanly with `cargo build` from the root. Reference [ARCHITECTURE.md](./ARCHITECTURE.md) for the crate layout.

---

#### Issue #2: Define shared types in herald-common
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #1

Define the core domain types in `herald-common`: `BoardState` (6×22 grid of `CellContent`), `CellContent` (character + `Color`), `Color` enum (Red, Orange, Yellow, Green, Blue, Violet, White, Black), `GridPosition` (row/col), `Message` (id, text, alignment, color markup, created_at), `Countdown` (id, label, target datetime, zero behavior), and `QueueItem` (id, item type, display order, expiry). All types must derive `Serialize`/`Deserialize` for JSON and `Clone`/`Debug`. See [SPEC.md](./SPEC.md) for the full type specifications.

---

#### Issue #3: Implement SQLite schema and migrations for herald-server
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #1

Set up SQLite persistence in `herald-server` using `sqlx` with compile-time checked queries. Create migration files for the initial schema: `messages`, `countdowns`, `queue_items`, and `config` tables. The `config` table stores key-value pairs (e.g., `rotation_interval_secs`, `admin_token`). Include a migration runner that auto-applies pending migrations on server startup. Default rotation interval is 30 seconds.

---

#### Issue #4: Implement message CRUD REST endpoints
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #2, #3

Implement the following Axum routes for message management: `POST /api/messages` (create a new message), `GET /api/messages` (list all messages), `GET /api/messages/:id` (get a single message), `PUT /api/messages/:id` (update a message), `DELETE /api/messages/:id` (delete a message). Creating a message should also add it to the queue. Return appropriate HTTP status codes (201 Created, 404 Not Found, etc.) and JSON response bodies. All write endpoints require bearer token authentication.

---

#### Issue #5: Implement countdown CRUD REST endpoints
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #2, #3

Implement Axum routes for countdown management: `POST /api/countdowns` (create — accepts label and target ISO-8601 datetime), `GET /api/countdowns` (list all, include computed time-remaining), `GET /api/countdowns/:id` (get single), `DELETE /api/countdowns/:id` (delete and remove from queue). Creating a countdown should also add it to the queue. Validate that target datetime is in the future on creation. All write endpoints require bearer token authentication.

---

#### Issue #6: Implement queue management endpoints
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #4, #5

Implement `GET /api/queue` (return ordered list of queue items with their associated message or countdown data) and `PUT /api/queue/reorder` (accept an ordered array of queue item IDs and update display_order accordingly). The reorder endpoint must validate that all provided IDs exist and that no IDs are missing. Return 400 Bad Request with a descriptive error if the provided list doesn't match the current queue. All write endpoints require bearer token authentication.

---

#### Issue #7: Implement configuration endpoints
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #3

Implement `GET /api/config` (return all configuration key-value pairs) and `PUT /api/config` (accept a JSON object of key-value pairs to upsert). Supported configuration keys: `rotation_interval_secs` (integer, default 30), `default_color` (Color, default White), `countdown_zero_behavior` (enum: show_zero / show_message / remove / pause, default show_zero). Reject unknown keys with 400 Bad Request. All endpoints require bearer token authentication.

---

#### Issue #8: Implement health check endpoint
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #1

Implement `GET /api/health` — an unauthenticated endpoint that returns `200 OK` with a JSON body containing `status: "ok"`, `version` (from Cargo.toml), and `uptime_secs`. This endpoint is used by Docker health checks and monitoring. It should also verify SQLite connectivity and report database status.

---

#### Issue #9: Add bearer token auth middleware
**Labels:** `enhancement`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #3

Implement an Axum middleware/extractor that reads the `Authorization: Bearer <token>` header and validates it against the `admin_token` stored in the config table. If no token is configured yet, the server should generate a random token on first startup and print it to stdout. Return `401 Unauthorized` with a JSON error body for missing or invalid tokens. The health check endpoint (`/api/health`) and future WebSocket/viewer endpoints must be excluded from auth. See [SPEC.md](./SPEC.md) for the authentication specification.

---

#### Issue #10: Add integration tests for all REST endpoints
**Labels:** `testing`, `backend`
**Milestone:** v0.1.0 - Foundation
**Blocked by:** #4, #5, #6, #7, #8, #9

Write integration tests that spin up the Axum server with an in-memory or temp-file SQLite database and exercise every REST endpoint. Cover: message CRUD lifecycle, countdown CRUD lifecycle, queue listing and reorder, config get/set, health check response, auth rejection (missing token, wrong token), and error cases (404 for missing resources, 400 for invalid input). Use `reqwest` or Axum's built-in test utilities. Aim for >90% endpoint coverage.

---

## Phase 2: Real-Time — WebSocket & Basic CLI Viewer

**GitHub Milestone:** `v0.2.0 - Real-Time`

**Description:** Add a WebSocket upgrade endpoint to the server with a broadcast channel that pushes board state changes to all connected viewers. Introduce the `herald-cli` crate with a basic terminal viewer that connects via WebSocket and renders the current board state as a static 6×22 grid — no animation yet, just instant updates.

**Demoable outcome:** Start the server, connect the CLI viewer with `herald watch`, and see the current board state. Push a message via the REST API and see it appear in the CLI within a second.

---

#### Issue #11: Implement WebSocket upgrade endpoint (/ws)
**Labels:** `enhancement`, `backend`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #1

Add a `GET /ws` route to `herald-server` that upgrades HTTP connections to WebSocket using `axum::extract::ws`. The endpoint is unauthenticated (viewers don't need a token). Each connected client is registered in a shared viewer list. Messages are sent as JSON-serialized `BoardState` frames. Handle connection errors and clean up on disconnect. See [ARCHITECTURE.md](./ARCHITECTURE.md) for the WebSocket message protocol.

---

#### Issue #12: Implement broadcast channel for connected viewers
**Labels:** `enhancement`, `backend`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #11

Implement a `tokio::sync::broadcast` channel that the server uses to fan out board state updates to all connected WebSocket clients. When the board state changes (message pushed, countdown updated, rotation tick), serialize the new `BoardState` and send it to the broadcast channel. Each WebSocket handler task subscribes to the broadcast receiver and forwards messages to its client. Handle slow consumers by dropping lagged messages gracefully.

---

#### Issue #13: Send initial board state on WebSocket connect
**Labels:** `enhancement`, `backend`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #12

When a new WebSocket client connects, immediately send the current `BoardState` as the first message — before the client receives any broadcast updates. This ensures viewers always see the board immediately on connect rather than waiting for the next rotation tick or state change. The initial message should be identical in format to broadcast messages so the client needs only one deserialization path.

---

#### Issue #14: Implement heartbeat/keepalive for WebSocket connections
**Labels:** `enhancement`, `backend`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #11

Implement a ping/pong heartbeat mechanism on the server side: send a WebSocket Ping frame every 30 seconds to each connected client. If a client doesn't respond with a Pong within 10 seconds, consider the connection dead and close it. This prevents stale connections from accumulating and ensures firewalls/proxies don't drop idle connections. Log connection and disconnection events with client identifiers.

---

#### Issue #15: Add herald-cli crate to workspace
**Labels:** `enhancement`, `cli`, `infra`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #1

Add the `herald-cli` crate as a new workspace member. Configure it as a binary crate with dependencies on `herald-common`, `clap` (for argument parsing), `ratatui` (terminal UI), `crossterm` (terminal backend), and `tokio-tungstenite` (WebSocket client). Set up the `clap` command structure with subcommands: `serve`, `watch`, `push`, `countdown`, `queue`, `config`. Only `watch` will be implemented in this phase; other subcommands will print "not yet implemented" stubs.

---

#### Issue #16: Implement basic CLI WebSocket client
**Labels:** `enhancement`, `cli`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #15, #11

Implement a WebSocket client module in `herald-cli` that connects to the server's `/ws` endpoint, deserializes incoming `BoardState` JSON messages, and exposes them via a `tokio::sync::watch` channel for the UI layer to consume. Implement automatic reconnection with exponential backoff (1s, 2s, 4s, 8s, max 30s). Accept the server URL as a CLI argument (`--server`) with a default of `ws://localhost:3000/ws`.

---

#### Issue #17: Implement basic ratatui board rendering (static grid)
**Labels:** `enhancement`, `cli`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #16

Implement a ratatui widget that renders the 6×22 board grid. Each cell displays its character centered in a fixed-width tile. Color tiles are rendered using the terminal's background color (map `Color` enum to ANSI colors). The grid should be centered in the terminal viewport. No animation — when a new `BoardState` arrives, the grid updates instantly. Use box-drawing characters to outline each tile. Handle the case where the terminal is too small by displaying a minimum size warning.

---

#### Issue #18: Implement CLI `herald watch` subcommand
**Labels:** `enhancement`, `cli`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #16, #17

Wire up the `watch` subcommand: parse `--server` URL, initialize the ratatui terminal, start the WebSocket client, and run the main event loop. The event loop should: poll for keyboard input (q/Esc to quit, Ctrl+C to quit), poll for new `BoardState` from the WebSocket watch channel, and re-render the board when it changes. Ensure clean terminal restoration on exit (alternate screen, cursor visibility). Accept `--fps` flag with a default of 30.

---

#### Issue #19: Add connection status bar to CLI
**Labels:** `enhancement`, `cli`
**Milestone:** v0.2.0 - Real-Time
**Blocked by:** #17

Add a status bar at the bottom of the CLI viewer that displays: connection state (Connected / Connecting / Reconnecting / Disconnected), the server URL, and the time since last update. Use color coding: green for connected, yellow for connecting/reconnecting, red for disconnected. When reconnecting, show the retry count and next retry delay. The status bar should be a single row and not overlap with the board grid.

---

## Phase 3: Rotation Engine & Countdown Logic

**GitHub Milestone:** `v0.3.0 - Rotation & Countdowns`

**Description:** Implement the server-side rotation timer that cycles through queue items at the configured interval (default 30s). Add countdown time computation that renders remaining time onto the 6×22 grid and ticks down in real time. Handle countdown expiry, empty queues, and edge cases.

**Demoable outcome:** Push multiple messages and countdowns. They cycle automatically every 30 seconds. Countdowns tick down second-by-second in real time. When a countdown hits zero, its configured zero behavior takes effect.

---

#### Issue #20: Implement server-side rotation timer
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #12

Create a background `tokio::spawn` task in the server that fires a rotation tick at the configured `rotation_interval_secs` interval. On each tick, advance to the next queue item and broadcast the new board state. The timer should be resettable (e.g., when a new message is pushed, optionally restart the timer). Read the interval from the config table and support runtime reconfiguration without restart.

---

#### Issue #21: Implement queue cycling logic
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #20, #6

Implement the logic that determines the next queue item on each rotation tick. Cycle through items in display_order, wrapping from the last item back to the first. Skip items that are expired (past their expiry time) or deleted. Track the "current index" in server state. Handle edge cases: queue with one item (always show it), queue becomes empty mid-rotation, items added or removed while rotating. Reference [SPEC.md](./SPEC.md) for queue behavior specification.

---

#### Issue #22: Implement countdown time computation and board rendering
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #2, #21

When the current queue item is a countdown, compute the remaining time (target minus now) and render it onto the 6×22 board grid. The label goes on the top rows and the time (DD:HH:MM:SS or HH:MM:SS) goes on the lower rows, centered. Define a `render_countdown_to_board` function in `herald-common` that takes a `Countdown` and current time and returns a `BoardState`. Handle countdowns more than 99 days away gracefully.

---

#### Issue #23: Implement countdown-specific refresh (1s updates)
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #22, #12

When the currently displayed queue item is a countdown, the server must broadcast updated board states every 1 second (not just on rotation ticks) so the countdown ticks down in real time for all viewers. Implement a secondary interval task that activates only when a countdown is being displayed. Ensure it doesn't interfere with the rotation timer — when the rotation timer fires, the countdown refresh should seamlessly switch to the next item.

---

#### Issue #24: Implement countdown zero behavior
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #22, #7

When a countdown reaches zero, apply the configured `countdown_zero_behavior`: `show_zero` (display 00:00:00:00 until next rotation), `show_message` (display a custom "time's up" message from the countdown's label), `remove` (immediately remove the countdown from the queue and advance to the next item), `pause` (stop rotation and hold on the zero display until admin intervenes). The behavior is configurable per-countdown with a global default from the config table.

---

#### Issue #25: Implement expired message auto-cleanup
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #21

Messages and countdowns can optionally have an `expires_at` field. Implement a periodic cleanup task (runs every 60 seconds) that removes expired items from the queue. Expired items should be soft-deleted (marked as expired in the database) rather than hard-deleted, so they can be viewed in history. Log when items are expired. If the currently-displayed item expires, immediately advance to the next queue item.

---

#### Issue #26: Implement empty queue default display
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #21, #22

When the queue is empty (no messages or countdowns), display a default "HERALD" splash screen on the board. The text should be centered on the 6×22 grid with the default color (White on Black). This provides a visual indicator that the server is running and the viewer is connected, even when no content has been pushed. The splash board is generated by a function in `herald-common` for reuse by all viewers.

---

#### Issue #27: Broadcast board updates on rotation tick
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #20, #12

Ensure that every rotation tick triggers a broadcast of the new `BoardState` to all connected WebSocket viewers. The broadcast should include the rendered board (from the current queue item) and happen atomically with the queue advancement. If rendering the new board state fails for any reason, broadcast the previous state and log the error rather than sending an empty or corrupted board.

---

#### Issue #28: Add rotation metadata to WebSocket messages
**Labels:** `enhancement`, `backend`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #27

Extend the WebSocket message format to include rotation metadata alongside the `BoardState`: `current_index` (which queue item is displayed), `total_items` (queue length), `next_rotation_at` (ISO-8601 timestamp of the next rotation tick), and `current_item_type` (message or countdown). Wrap the payload in a `BoardUpdate` envelope type in `herald-common`. Update the WebSocket send logic and document the updated protocol in [ARCHITECTURE.md](./ARCHITECTURE.md).

---

#### Issue #29: Update CLI to handle rotation and countdown updates
**Labels:** `enhancement`, `cli`
**Milestone:** v0.3.0 - Rotation & Countdowns
**Blocked by:** #28, #17

Update the CLI viewer's WebSocket message deserialization to handle the new `BoardUpdate` envelope format. Display rotation metadata in the status bar: "Item 2/5 · Next in 18s". When a countdown is active, the status bar should show "Countdown active" and the board updates every second without visual glitches. Ensure the ratatui rendering is efficient enough to handle 1-second update intervals without flicker.

---

## Phase 4: CLI Split-Flap Animation

**GitHub Milestone:** `v0.4.0 - Terminal Animation`

**Description:** Implement the satisfying split-flap flip effect in the terminal. Each tile cycles through intermediate characters before landing on the target, with a left-to-right cascade stagger across columns. This is the core aesthetic feature of the CLI viewer.

**Demoable outcome:** When the board transitions between messages, each tile flips through characters with a cascade effect from left to right. The animation is smooth, visually satisfying, and completes within a few seconds.

---

#### Issue #30: Implement split-flap tile widget in ratatui
**Labels:** `enhancement`, `cli`, `design`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #17

Create a custom ratatui widget for a single split-flap tile. Each tile is 3 characters wide × 3 rows tall, using box-drawing characters (┌─┐, │X│, └─┘) to create the flap border. The center character is the displayed value. The widget accepts a `CellContent` (character + color) and renders it with the appropriate background color. Support a "flipping" state where the tile displays a mid-flip visual (e.g., the border changes or the character is partially obscured with a ─ divider line).

---

#### Issue #31: Implement character cycling animation
**Labels:** `enhancement`, `cli`, `design`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #30

Implement the split-flap character cycling logic: when a tile transitions from one character to another, it cycles through intermediate characters in the Herald character set (A–Z, 0–9, special characters, space) at approximately 50ms per step. Characters cycle forward through the set (e.g., A→B→C→...→Z→0→...→target). Calculate the shortest cycling path. Each cycling step updates the tile's displayed character. The animation state machine tracks: current char, target char, intermediate steps remaining.

---

#### Issue #32: Implement left-to-right cascade stagger
**Labels:** `enhancement`, `cli`, `design`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #31

Add a cascade stagger effect: columns start their flip animation with a ~20ms delay between each (column 0 starts immediately, column 1 starts at +20ms, column 2 at +40ms, etc.). This creates the characteristic split-flap wave effect moving left to right. The stagger applies per-row independently. The total animation duration for a full board flip is approximately: (22 columns × 20ms stagger) + (max character distance × 50ms) ≈ 1.5–2 seconds.

---

#### Issue #33: Implement terminal color support for color tiles
**Labels:** `enhancement`, `cli`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #30

Map the `Color` enum values to terminal colors: Red → ANSI 196, Orange → ANSI 208, Yellow → ANSI 226, Green → ANSI 46, Blue → ANSI 21, Violet → ANSI 93, White → ANSI 231 (foreground on black background), Black → ANSI 232 (with white foreground). Use ANSI 256-color mode for consistent rendering across terminals. During flip animation, the color should transition on the final frame (when the target character lands). Detect terminal color capability and fall back to basic 8-color mode if needed.

---

#### Issue #34: Implement board diff detection
**Labels:** `enhancement`, `cli`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #31

When a new `BoardState` arrives, diff it against the currently displayed board to determine which cells have changed. Only animate changed tiles — unchanged tiles remain static. This reduces visual noise and CPU usage. Implement a `diff_boards(old: &BoardState, new: &BoardState) -> Vec<GridPosition>` function in `herald-common` that returns the positions of changed cells. Edge case: if the boards are identical, skip the animation entirely.

---

#### Issue #35: Implement terminal resize handling
**Labels:** `enhancement`, `cli`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #30

Handle terminal resize events (SIGWINCH / crossterm resize event). On resize, re-center the board grid in the new viewport dimensions. Calculate the minimum terminal size required to display the full board (6 rows × 3 chars/tile + borders, 22 cols × 3 chars/tile + borders) and display a warning message if the terminal is too small: "Terminal too small. Minimum: NNxMM". If a flip animation is in progress during a resize, cancel it and render the target state immediately.

---

#### Issue #36: Optimize rendering for 30fps during animation
**Labels:** `enhancement`, `cli`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #31, #32

Profile and optimize the ratatui rendering loop to sustain 30fps during flip animations. Use ratatui's built-in diffing (only repaint changed cells) and double-buffering. Minimize allocations in the hot loop — pre-allocate animation state buffers. Measure frame times and skip frames if rendering falls behind. Target: full board flip animation (132 tiles changing simultaneously) should render smoothly at 30fps on a modern terminal emulator.

---

#### Issue #37: Add animation speed configuration option
**Labels:** `enhancement`, `cli`
**Milestone:** v0.4.0 - Terminal Animation
**Blocked by:** #31, #32

Add a `--animation-speed` CLI flag and a corresponding config key (`animation_speed`) that controls the flip animation timing. Accept values: `fast` (25ms/step, 10ms stagger), `normal` (50ms/step, 20ms stagger — the default), `slow` (100ms/step, 40ms stagger), or `off` (instant transitions, no animation). The speed setting should be changeable at runtime via the config API without restarting the CLI viewer.

---

## Phase 5: Web Viewer with Wasm

**GitHub Milestone:** `v0.5.0 - Web Interface`

**Description:** Build the browser-based viewer using Leptos compiled to WebAssembly. The web viewer connects to the same WebSocket endpoint as the CLI and renders the board with 3D CSS split-flap animations. The server serves the compiled Wasm and static assets.

**Demoable outcome:** Open a browser, see the Herald board with beautiful 3D flip animations. Push a message via the API and watch it flip into place in the browser.

---

#### Issue #38: Add herald-web crate to workspace, configure Leptos + Trunk
**Labels:** `enhancement`, `web`, `infra`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #1

Add `herald-web` as a new workspace member crate. Configure it with Leptos (CSR mode) and Trunk as the build tool. Set up `index.html` with the Trunk build hooks, wasm-bindgen configuration, and a basic "Herald loading..." placeholder. Add Trunk build steps to the workspace (e.g., `trunk build --release` in `herald-web/`). Ensure `cargo build` from the workspace root still works (exclude herald-web from default members if needed, since it requires Trunk).

---

#### Issue #39: Implement WebSocket client in Wasm
**Labels:** `enhancement`, `web`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #38, #11

Implement a WebSocket client for the browser environment using `gloo-net` (or `web-sys` WebSocket API directly). Connect to the server's `/ws` endpoint, deserialize incoming `BoardUpdate` JSON messages, and expose the current board state via a Leptos reactive signal. Implement reconnection with exponential backoff. Auto-detect the WebSocket URL from the page's `window.location` (replace `http` with `ws`). Handle connection lifecycle events (open, close, error).

---

#### Issue #40: Implement basic board grid layout (HTML/CSS)
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #38

Create the HTML/CSS structure for the 6×22 board grid. Use CSS Grid with fixed cell aspect ratios. Each tile is a container with a dark background, rounded corners, and a subtle inner shadow to create depth. The board has a slight 3D perspective (`perspective: 1200px` on the container). Style the board frame with a dark surround. The grid should be centered in the viewport with appropriate padding. Use CSS custom properties for all dimensional and color values.

---

#### Issue #41: Implement split-flap tile component (top/bottom half, 3D perspective)
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #40

Create a Leptos component for a single split-flap tile. Each tile consists of two halves (top and bottom) with a visible horizontal split line, mimicking real split-flap displays. Each half shows the character clipped to its respective half. Use `transform-style: preserve-3d` and `perspective` to enable 3D transformations. The tile component accepts a character, color, and animation state as props.

---

#### Issue #42: Implement CSS flip animation
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #41

Implement the 3D flip animation using CSS `@keyframes`. The animation sequence: (1) top half rotates backward around the X-axis from 0° to -90° revealing the bottom of the flap, (2) a new top half appears with the next character, (3) the bottom half of the new character drops down from +90° to 0°. Use `backface-visibility: hidden` to hide the back face during rotation. Each character transition plays this animation. Total flip duration: ~150ms per character step.

---

#### Issue #43: Implement cascade stagger for flip animations
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #42

Apply the left-to-right cascade stagger effect to the web flip animations. Each column starts its animation with an increasing `animation-delay` (column × 20ms). Use CSS custom properties (`--col-index`) set per tile to calculate the delay: `animation-delay: calc(var(--col-index) * 20ms)`. The stagger creates the wave effect matching the terminal viewer. Ensure the animation timing is consistent regardless of how many characters need to cycle.

---

#### Issue #44: Implement color tile rendering (solid background fills)
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #41

Map the `Color` enum to CSS colors for tile backgrounds: Red → `#D32F2F`, Orange → `#F57C00`, Yellow → `#FDD835`, Green → `#388E3C`, Blue → `#1565C0`, Violet → `#7B1FA2`, White → `#F5F5F5` (with dark text), Black → `#212121` (with light text). Apply the background color to both halves of the split-flap tile. During flip animation, the color transitions on the final character landing. Use CSS custom properties for all color values to enable future theming.

---

#### Issue #45: Implement responsive scaling
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #40

Make the board grid responsive across screen sizes. Use CSS `clamp()` and viewport units to scale tile size: tiles should fill the viewport width on mobile while maintaining aspect ratio, and cap at a maximum size on large screens. The board should remain centered with consistent padding. Use `@container` queries or media queries for breakpoints. Test on mobile viewports (375px), tablets (768px), and desktop (1440px+). The grid should never overflow or require scrolling.

---

#### Issue #46: Implement loading state
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #40

Display a loading state while the WebSocket connection is being established. Show the 6×22 board grid with blank/empty tiles and a CSS shimmer animation sweeping across the tiles. Display a centered "Connecting..." label below the board. The shimmer should use a `linear-gradient` animation moving left to right. Once the first `BoardUpdate` is received, transition from the loading state to the live board with a flip animation (all tiles flip to their initial characters simultaneously).

---

#### Issue #47: Implement reconnection handling with status indicator
**Labels:** `enhancement`, `web`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #39, #40

Show a non-intrusive connection status indicator in the web viewer. When connected, show a small green dot (or hide entirely). When disconnected/reconnecting, show a yellow/red indicator with "Reconnecting..." text. The indicator should be positioned in the bottom-right corner and not obscure the board. During reconnection, the board should retain the last known state (not revert to loading). On successful reconnection, update the board with the new state received from the server.

---

#### Issue #48: Serve web assets from herald-server
**Labels:** `enhancement`, `backend`, `web`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #38

Configure `herald-server` to serve the compiled `herald-web` static assets (Wasm, JS glue, CSS, index.html). Use Axum's `ServeDir` to serve files from a configurable `--web-dir` path (default: `./web-dist`). The web viewer should be accessible at `http://localhost:3000/`. API routes (`/api/*`) and the WebSocket endpoint (`/ws`) take precedence over static file serving. Add a build script or Makefile target that builds `herald-web` with Trunk and copies output to the expected directory.

---

#### Issue #49: Performance optimization (will-change, contain, batched updates)
**Labels:** `enhancement`, `web`
**Milestone:** v0.5.0 - Web Interface
**Blocked by:** #42, #43

Optimize web rendering performance for smooth 60fps animations. Apply `will-change: transform` to tiles during animation (remove after animation completes to free GPU memory). Use `contain: layout style paint` on tile containers to isolate layout recalculations. Batch DOM updates when processing incoming `BoardUpdate` messages — apply all tile changes in a single requestAnimationFrame callback. Profile with Chrome DevTools and ensure paint time stays under 8ms during full-board flip animations.

---

## Phase 6: Admin Interfaces

**GitHub Milestone:** `v0.6.0 - Admin Tools`

**Description:** Build the CLI admin subcommands (`herald push`, `herald countdown`, `herald queue`, `herald config`) and the web admin panel (`/admin`). These provide user-friendly interfaces for managing the board content, replacing direct curl usage.

**Demoable outcome:** Push messages and manage the board entirely from the CLI or the web admin panel. Preview messages before pushing. Drag-to-reorder queue items in the browser.

---

#### Issue #50: Implement `herald push` CLI subcommand
**Labels:** `enhancement`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #15, #4

Implement the `herald push "MESSAGE TEXT"` subcommand. It sends a `POST /api/messages` request to the server with the message text. Accept `--server` (server URL, default `http://localhost:3000`), `--token` (bearer token), `--align` (left/center/right, default center), and `--expires` (optional ISO-8601 expiry time) flags. Print the created message ID on success. Handle errors gracefully (connection refused, auth failure, server error) with user-friendly error messages.

---

#### Issue #51: Implement `herald countdown` CLI subcommands
**Labels:** `enhancement`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #15, #5

Implement `herald countdown create --label "EVENT" --target "2025-12-31T00:00:00Z"` (creates a countdown and adds to queue), `herald countdown list` (lists all countdowns with computed remaining time, formatted as a table), and `herald countdown delete <id>` (deletes a countdown and removes from queue). Accept `--zero-behavior` flag on create (show_zero/show_message/remove/pause). Use the REST API endpoints from Phase 1.

---

#### Issue #52: Implement `herald queue` CLI subcommands
**Labels:** `enhancement`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #15, #6

Implement `herald queue list` (displays the current queue in display order as a formatted table, showing item type, title/label, position, and expiry) and `herald queue reorder <id1> <id2> ...` (reorders the queue by calling `PUT /api/queue/reorder`). The list command should highlight the currently displayed item. Handle the case where the provided reorder list doesn't match the queue by showing a clear error.

---

#### Issue #53: Implement `herald config` CLI subcommands
**Labels:** `enhancement`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #15, #7

Implement `herald config get [key]` (get all config or a specific key, displayed as a key-value table) and `herald config set <key> <value>` (set a config value). Validate known keys client-side before sending to the server. Display the previous and new values after a set operation. Include help text for each known config key when running `herald config get` without arguments.

---

#### Issue #54: Implement color markup parser
**Labels:** `enhancement`, `backend`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #2

Implement a color markup parser in `herald-common` that processes inline color tags in message text. Syntax: `{red}text{/red}` or shorthand `{red}text{}` (close resets to default). Supported tags: `{red}`, `{orange}`, `{yellow}`, `{green}`, `{blue}`, `{violet}`, `{white}`, `{black}`. The parser converts marked-up text into a sequence of `CellContent` values with the appropriate colors. Handle nested tags by using the innermost color. Invalid tags are rendered literally.

---

#### Issue #55: Implement message preview in CLI
**Labels:** `enhancement`, `cli`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #50, #54

Add a `--preview` flag to `herald push` that renders the message as a 6×22 grid in the terminal *before* sending it to the server. The preview uses the same rendering logic as the board viewer (text layout, color markup, alignment). After displaying the preview, prompt "Push this message? [Y/n]" and wait for confirmation. If the terminal is not interactive (piped input), skip the preview and push directly.

---

#### Issue #56: Implement web admin route (/admin) with token auth
**Labels:** `enhancement`, `web`, `backend`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #48

Add a `/admin` route to the web viewer that displays the admin panel. Protect it with a client-side token prompt: on first visit, show a login dialog that asks for the bearer token, stores it in `localStorage`, and includes it in all subsequent API requests as an `Authorization` header. If a 401 response is received, clear the stored token and show the login dialog again. The admin panel is a single-page Leptos component that houses the sub-panels defined in subsequent issues.

---

#### Issue #57: Implement web message composer
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #56

Build a web-based message composer in the admin panel. Features: a text input area for the message, alignment selector (left/center/right), optional expiry datetime picker, and a live 6×22 grid preview that updates as the user types. The preview renders color markup in real time. Include a "Push" button that calls `POST /api/messages` and shows a success/error toast notification. The preview reuses the board tile styles from the viewer for visual consistency.

---

#### Issue #58: Implement web countdown manager
**Labels:** `enhancement`, `web`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #56

Build a countdown management panel in the web admin: a form to create new countdowns (label, target datetime picker, zero behavior selector) and a list of existing countdowns showing label, target time, remaining time (live-updating), and a delete button. The list auto-refreshes by polling `GET /api/countdowns` every 10 seconds or by subscribing to a future admin WebSocket channel. Confirm before deleting.

---

#### Issue #59: Implement web queue manager (drag-to-reorder)
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #56

Build a queue management panel in the web admin that displays queue items as a sortable list. Each item shows its type (message/countdown icon), title/label, position number, and expiry. Implement drag-to-reorder using HTML5 Drag and Drop API (or a lightweight JS library via wasm-bindgen interop). On drop, call `PUT /api/queue/reorder` with the new order. Highlight the currently displayed item. Show a "Saving..." indicator during the API call.

---

#### Issue #60: Implement web config panel
**Labels:** `enhancement`, `web`
**Milestone:** v0.6.0 - Admin Tools
**Blocked by:** #56

Build a configuration panel in the web admin that displays all config key-value pairs from `GET /api/config` in an editable form. Each known key has an appropriate input type: number input for `rotation_interval_secs`, color picker/dropdown for `default_color`, radio buttons for `countdown_zero_behavior`. Include a "Save" button that sends `PUT /api/config` with changed values. Show validation errors inline and a success toast on save.

---

## Phase 7: Docker & Deployment Polish

**GitHub Milestone:** `v0.7.0 - Production Ready`

**Description:** Package Herald for production deployment with a multi-stage Docker build, Docker Compose configuration, structured logging, graceful shutdown, CI/CD pipelines, and deployment documentation. After this phase, anyone can deploy Herald with `docker compose up`.

**Demoable outcome:** Clone the repo, run `docker compose up`, open the browser, and see Herald running. Push messages from the CLI on the host. Everything works out of the box.

---

#### Issue #61: Create multi-stage Dockerfile
**Labels:** `enhancement`, `infra`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #48

Create a multi-stage Dockerfile: **Stage 1** — Rust builder: compile `herald-server` in release mode. **Stage 2** — Wasm builder: install Trunk, build `herald-web` in release mode. **Stage 3** — Minimal runtime: use `debian:bookworm-slim` (or `distroless`), copy the server binary and web assets, expose port 3000, set entrypoint. The final image should be under 100MB. Use cargo-chef for dependency caching to speed up rebuilds. See [DEPLOYMENT.md](./DEPLOYMENT.md) for deployment specifications.

---

#### Issue #62: Create docker-compose.yml with volume and env config
**Labels:** `enhancement`, `infra`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #61

Create a `docker-compose.yml` that runs Herald with: a named volume for SQLite persistence (`herald-data:/data`), environment variables for configuration (`HERALD_PORT`, `HERALD_ADMIN_TOKEN`, `HERALD_DB_PATH`, `HERALD_LOG_LEVEL`), port mapping (default 3000:3000), health check using the `/api/health` endpoint, and restart policy (`unless-stopped`). Include a `.env.example` file documenting all available environment variables with sensible defaults.

---

#### Issue #63: Implement graceful shutdown
**Labels:** `enhancement`, `backend`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #12

Handle SIGTERM and SIGINT signals for graceful shutdown: stop accepting new connections, send a "server shutting down" close frame to all connected WebSocket clients, wait up to 10 seconds for in-flight requests to complete, flush any pending SQLite writes, then exit. Use `tokio::signal` for signal handling. Log the shutdown sequence. This is critical for Docker (which sends SIGTERM on `docker stop`) and for zero-downtime deployments.

---

#### Issue #64: Add structured logging
**Labels:** `enhancement`, `backend`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #1

Replace any existing `println!` logging with the `tracing` crate and `tracing-subscriber` with JSON output format. Add structured log fields: request method/path/status/duration for HTTP requests (using tower-http's trace layer), WebSocket connect/disconnect events with client count, rotation ticks with current item info, and startup/shutdown events. Support `HERALD_LOG_LEVEL` environment variable (default: `info`). In development mode, use pretty-printed human-readable output instead of JSON.

---

#### Issue #65: Set up GitHub Actions CI
**Labels:** `infra`, `testing`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #10

Create a `.github/workflows/ci.yml` that runs on every push and PR: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` (all workspace crates), and `trunk build` for the Wasm crate. Cache Cargo dependencies and build artifacts across runs. Run on `ubuntu-latest`. Add a status badge to README.md. The CI should fail fast on the first error to provide quick feedback.

---

#### Issue #66: Create release workflow
**Labels:** `infra`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #65, #61

Create a `.github/workflows/release.yml` triggered by pushing a version tag (`v*`). It should: build release binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64) using `cross` or matrix builds; build and push a Docker image to GitHub Container Registry (ghcr.io); create a GitHub Release with the binaries as assets and auto-generated changelog. Use `cargo-dist` or manual cross-compilation as appropriate.

---

#### Issue #67: Write/finalize all deployment documentation
**Labels:** `docs`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #62

Write comprehensive deployment documentation in [DEPLOYMENT.md](./DEPLOYMENT.md) covering: Docker Compose quickstart (5-minute guide), bare-metal installation (download binary, configure, run as systemd service), environment variable reference, SQLite database location and backup procedures, reverse proxy configuration, HTTPS/TLS setup guidance, upgrading between versions, and troubleshooting common issues. Include copy-pasteable command blocks for each deployment method.

---

#### Issue #68: Add example nginx and Caddy reverse proxy configs
**Labels:** `docs`, `infra`
**Milestone:** v0.7.0 - Production Ready
**Blocked by:** #67

Create example reverse proxy configurations in `examples/`: `examples/nginx.conf` and `examples/Caddyfile`. Both must: proxy HTTP traffic to Herald's port, upgrade `/ws` connections to WebSocket, set appropriate headers (`X-Forwarded-For`, `X-Real-IP`), enable compression, and include TLS/HTTPS configuration with Let's Encrypt. Add comments explaining each directive. Reference these files from [DEPLOYMENT.md](./DEPLOYMENT.md). Test that WebSocket connections work through the proxy.

---

## Phase 8: Stretch Goals

**GitHub Milestone:** `v1.0.0 - Polish & Extras`

**Description:** Quality-of-life improvements, sound effects, visual themes, and additional features that elevate Herald from functional to delightful. These are non-essential for core functionality but significantly enhance the user experience.

---

#### Issue #69: Implement optional "clack" sound effect on web flip
**Labels:** `enhancement`, `web`, `design`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #42

Add an optional mechanical "clack" sound effect that plays during each tile flip in the web viewer, using the Web Audio API. Generate the sound programmatically (short noise burst with band-pass filter to simulate the mechanical flap sound) rather than loading an audio file. Stagger the sound with the visual cascade for realism. The sound should be disabled by default and enabled via a speaker icon toggle in the UI corner. Respect the user's `prefers-reduced-motion` media query.

---

#### Issue #70: Implement mute/unmute toggle for sound
**Labels:** `enhancement`, `web`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #69

Add a persistent mute/unmute toggle button in the bottom-left corner of the web viewer. Use a speaker icon (🔊/🔇) that toggles the flip sound effect. Persist the preference in `localStorage`. The toggle should be accessible (keyboard-focusable, aria-label). When muted, no Web Audio processing should occur (not just volume=0) to save CPU. The toggle is only visible when sound support is enabled.

---

#### Issue #71: Add board theme support
**Labels:** `enhancement`, `web`, `cli`, `design`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #44, #49

Implement a theme system for the board appearance. Include three built-in themes: **Classic** (black background, warm yellow text — traditional split-flap), **Dark** (dark gray background, white text — modern minimal), and **Custom** (user-defined colors via config). Themes are selectable via the admin config panel and the `herald config set theme <name>` CLI command. In the web viewer, themes swap CSS custom properties. In the CLI viewer, themes map to different ANSI color palettes. Store the active theme in the server config.

---

#### Issue #72: Add message templates
**Labels:** `enhancement`, `backend`, `cli`, `web`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #54, #57

Add pre-formatted message templates that simplify common use cases: "Countdown" (label on top, large timer below), "Announcement" (centered text with optional color highlight), "Greeting" (top line small, middle line large — e.g., "HAPPY" / "BIRTHDAY"), and "Ticker" (scrolling text placeholder for future use). Templates are selectable in both the CLI (`herald push --template announcement "TEXT"`) and the web message composer (template dropdown). Store templates as JSON in the database or as a static set in `herald-common`.

---

#### Issue #73: Add API rate limiting
**Labels:** `enhancement`, `backend`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #9

Add configurable rate limiting to the REST API using `tower::limit` or a custom middleware. Default limits: 60 requests/minute for authenticated endpoints, 10 requests/minute for unauthenticated endpoints (health check). Return `429 Too Many Requests` with a `Retry-After` header when limits are exceeded. Rate limit configuration is stored in the config table (`rate_limit_per_minute`). Exempt the WebSocket endpoint from rate limiting. Log rate limit events.

---

#### Issue #74: Add connected viewers count to admin panel
**Labels:** `enhancement`, `web`, `backend`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #56, #12

Track the number of currently connected WebSocket viewers in server state (atomic counter incremented on connect, decremented on disconnect). Expose it via `GET /api/stats` (authenticated) returning `{ "connected_viewers": N, "uptime_secs": M, "total_messages": X, "total_countdowns": Y }`. Display the viewer count in the web admin panel header as a live-updating badge. Update the CLI admin with a `herald stats` subcommand.

---

#### Issue #75: Add message scheduling
**Labels:** `enhancement`, `backend`, `cli`, `web`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #4, #20

Add the ability to schedule a message for future display. Extend the `Message` type with an optional `display_at` field (ISO-8601 datetime). Scheduled messages are added to the queue but skipped by the rotation engine until their `display_at` time arrives. Add `--display-at` flag to `herald push` and a datetime picker in the web message composer. Add a server-side task that checks for messages reaching their display time and activates them. Show scheduled messages in a separate "Upcoming" section in the queue view.

---

#### Issue #76: Performance profiling and optimization pass
**Labels:** `enhancement`, `backend`, `cli`, `web`
**Milestone:** v1.0.0 - Polish & Extras
**Blocked by:** #49, #36

Perform a comprehensive performance profiling pass across all three components. **Server:** profile with `tokio-console`, optimize SQLite query patterns, measure WebSocket broadcast latency with 50+ concurrent viewers. **CLI:** profile frame rendering time, ensure <5ms per frame during animation. **Web:** audit with Lighthouse, optimize Wasm binary size (wasm-opt), measure time-to-first-flip. Document performance baselines in a `PERFORMANCE.md` file. Address any bottlenecks found.

---

## Future Considerations (Out of Scope for v1)

The following features are potential directions for Herald beyond v1.0. They are listed here for visibility and community discussion but are **not planned for the initial release**.

- **Multi-board support** — Run multiple independent boards on a single server, each with its own queue, configuration, and viewer URL. Useful for offices, events, or dashboards with different audiences.
- **Integrations** — Connect Herald to external data sources: RSS feeds (display headlines), calendar (show upcoming events), weather (current conditions), Spotify now-playing (display track info). Plugin architecture with a trait-based integration API.
- **Mobile app viewer** — Native iOS/Android viewer app or Progressive Web App (PWA) with offline support, push notifications for countdown completion, and haptic feedback on flip animations.
- **Multi-user / team admin with role-based access** — Replace single bearer token with user accounts, OAuth/SSO support, and roles (admin, editor, viewer). Audit log for all changes.
- **Public API with API keys** — Issue API keys for third-party integrations, with per-key rate limits and permissions. OpenAPI/Swagger documentation for the public API surface.
- **Board templates marketplace / sharing** — Community-contributed board layouts, themes, and message templates. Import/export board configurations as shareable JSON files.
- **Notification system** — Alert subscribers (email, webhook, push notification) when a countdown reaches zero or a scheduled message goes live. Configurable notification channels per countdown.

---

*This roadmap is a living document. As development progresses, issues may be added, refined, or reprioritized. Track progress on the [GitHub Milestones](../../milestones) page.*
