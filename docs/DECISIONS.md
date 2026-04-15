# Herald — Architecture Decision Records

This document captures the key architectural decisions made for the Herald project. Each decision is recorded as an Architecture Decision Record (ADR) with context, options considered, rationale, and consequences.

For the full system architecture, see [ARCHITECTURE.md](./ARCHITECTURE.md). For functional requirements, see [SPEC.md](./SPEC.md).

---

## Table of Contents

- [ADR-001: Rust as the Primary Language](#adr-001-rust-as-the-primary-language)
- [ADR-002: Axum as the Web Framework](#adr-002-axum-as-the-web-framework)
- [ADR-003: SQLite for Persistence](#adr-003-sqlite-for-persistence)
- [ADR-004: Fixed 6×22 Grid Format](#adr-004-fixed-622-grid-format)
- [ADR-005: WebSocket for Real-Time Viewer Updates](#adr-005-websocket-for-real-time-viewer-updates)
- [ADR-006: Leptos + WebAssembly for the Web Frontend](#adr-006-leptos--webassembly-for-the-web-frontend)
- [ADR-007: ratatui for Terminal UI](#adr-007-ratatui-for-terminal-ui)
- [ADR-008: Single Admin with Bearer Token](#adr-008-single-admin-with-bearer-token)
- [ADR-009: Monorepo with Cargo Workspace](#adr-009-monorepo-with-cargo-workspace)
- [ADR-010: TOML for Configuration](#adr-010-toml-for-configuration)

---

## ADR-001: Rust as the Primary Language

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald is a three-component system — a backend server, a terminal UI client, and a browser-based WebAssembly frontend. We need a language that can target all three deployment surfaces: a performant server binary, a native terminal application, and a WebAssembly module for browsers. The choice of language determines the project's shared-type strategy, toolchain complexity, and long-term maintainability.

### Decision

Use Rust as the sole language for all three components, organized as a Cargo workspace monorepo. A shared crate (`herald-common`) defines types used across the backend, CLI, and web frontend — `Message`, `Countdown`, `BoardState`, `Color`, `CellContent`, and related structures.

### Options Considered

1. **TypeScript / Node.js (full stack)** — Use Node for the backend and a TypeScript framework (React, Svelte) for the frontend. Excellent developer velocity and vast ecosystem. However, no good story for a native terminal UI (would need a separate tool or ncurses binding), and no shared compiled types between server and client without manual duplication or codegen.

2. **Go backend + TypeScript frontend** — Go is fast to develop and deploys as a single binary. Frontend in TypeScript. However, this splits the stack across two languages, eliminating shared type definitions. The CLI would be Go (fine), but the web frontend would be TypeScript — no type sharing between them.

3. **Rust monorepo** — Single language for backend (Axum), CLI (ratatui), and web (Leptos → Wasm). Shared types compile into all three targets. One toolchain, one CI pipeline, atomic cross-component refactors.

4. **Python backend + TypeScript frontend** — Python is quick to prototype but has weaker performance characteristics for real-time WebSocket handling at scale. Again splits the stack, with no shared types. CLI in Python (curses or textual) is viable but less polished than Rust TUI libraries.

### Rationale

Rust is the only option that allows a single crate of shared types to compile natively into the server binary, the CLI binary, *and* the browser Wasm module. This eliminates an entire class of serialization/deserialization bugs and ensures the client and server can never disagree on message structure. The Rust ecosystem has mature, well-maintained crates for each component: Axum for HTTP/WebSocket, ratatui for terminal UI, and Leptos for reactive Wasm frontends.

### Consequences

- **Positive:** Type safety across all three components. A single `cargo build` compiles the entire project. No runtime type mismatches between server and clients. Excellent runtime performance with minimal memory footprint — important for self-hosted/Docker deployments.
- **Negative:** Steeper learning curve for contributors unfamiliar with Rust. Slower initial development velocity compared to TypeScript or Go — the borrow checker and type system require more upfront investment. Compile times for the full workspace are longer than equivalent Go or TypeScript builds.

---

## ADR-002: Axum as the Web Framework

**Status:** Accepted
**Date:** 2026-04-15

### Context

The Herald backend needs an HTTP framework that supports REST API endpoints for admin operations, WebSocket connections for real-time viewer updates, static file serving for the web frontend, and composable middleware for concerns like authentication, logging, and CORS.

### Decision

Use Axum as the backend web framework.

### Options Considered

1. **Actix-web** — Mature, battle-tested, high-performance Rust web framework. Uses its own actor runtime, which adds complexity and is not directly compatible with the broader tokio middleware ecosystem. Strong community but somewhat opinionated runtime model.

2. **Axum** — Built on top of `hyper` and `tower`, maintained by the tokio team. First-class async support, built-in WebSocket upgrade handling, and full access to the `tower` middleware ecosystem. Extractors for type-safe request parsing. Composable routing.

3. **Warp** — Filter-based composition model. Unique API design that's powerful but can be hard to reason about for complex routing. Less active maintenance compared to Axum.

4. **Rocket** — Developer-friendly with macro-heavy ergonomics. Historically required nightly Rust (stable support is now available). Less alignment with the tokio/tower ecosystem and fewer middleware options.

### Rationale

Axum is the natural choice for a tokio-based async application. It provides built-in WebSocket support via `axum::extract::ws`, which is critical for Herald's real-time push model. The `tower` middleware ecosystem gives us composable layers for authentication, logging, CORS, and rate limiting without reinventing the wheel. Being maintained by the tokio team ensures long-term alignment with the async runtime Herald already depends on.

### Consequences

- **Positive:** Native WebSocket support simplifies the real-time push architecture. Tower middleware is composable and reusable. Strong ecosystem alignment — tokio, hyper, tower, and axum all work together seamlessly. Active maintenance and community.
- **Negative:** Axum's type-heavy extractor model can produce complex compiler errors that are difficult to debug. The framework is younger than Actix-web, so some edge-case patterns have less community documentation.

---

## ADR-003: SQLite for Persistence

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald needs to persist messages, countdowns, board configuration, and rotation queue state across server restarts. The storage solution must align with Herald's self-hosted, single-instance deployment model — no cloud dependencies, minimal operational overhead.

### Decision

Use SQLite as the sole persistence layer, accessed via the `sqlx` crate with compile-time query verification.

### Options Considered

1. **PostgreSQL** — Industry-standard relational database. Excellent feature set, ACID compliance, and tooling. However, it requires a separate running service, adds significant operational complexity for a personal/self-hosted project, and is overkill for Herald's data volume.

2. **SQLite** — Embedded, serverless, zero-configuration relational database. The entire database is a single file on disk. Supports SQL, transactions, and is well-supported by `sqlx` in Rust. Perfect for single-instance applications.

3. **Embedded key-value store (sled / RocksDB)** — High-performance embedded storage. However, no SQL query support, less mature Rust bindings (sled is pre-1.0), and requires manual indexing/querying logic for anything beyond simple key lookups.

4. **Flat files (JSON/TOML on disk)** — Simplest possible storage. No dependencies at all. However, no transactional safety, no query capabilities, and concurrent read/write is error-prone. Doesn't scale even slightly.

### Rationale

SQLite hits the sweet spot for Herald: it's a real relational database with SQL support and ACID transactions, but it requires zero external dependencies — no separate service to install, configure, or monitor. The entire database is a single file, which makes backup trivial (copy the file) and Docker volume mounting straightforward. The `sqlx` crate provides compile-time SQL verification, catching query errors at build time rather than runtime.

### Consequences

- **Positive:** Zero operational overhead — no database service to manage. Single-file database simplifies backup and migration. Docker deployment only needs a volume mount for the `.db` file. `sqlx` compile-time checks prevent SQL errors from reaching production.
- **Negative:** No multi-instance or clustering support — Herald cannot run multiple server instances against the same database. Write throughput is limited compared to PostgreSQL (but Herald's write volume is negligible). No built-in replication or high-availability. These limitations are acceptable for a personal, single-admin message board.

---

## ADR-004: Fixed 6×22 Grid Format

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald emulates a physical split-flap display. We need to define the board's dimensions, which affect message composition, rendering logic, API payloads, and the visual layout of both the terminal and web viewers.

### Decision

The board is a fixed 6-row × 22-column grid (132 total cells). This is not configurable — it is a fundamental design constraint of the system.

### Options Considered

1. **Dynamic / configurable grid** — Allow the board size to be set in configuration (e.g., 4×20, 8×30, etc.). More flexible, but dramatically increases complexity: every renderer must handle arbitrary dimensions, message composition logic becomes more complex, and the visual aesthetic varies unpredictably across configurations.

2. **Fixed 6×22 (Vestaboard-compatible)** — Match the physical Vestaboard hardware dimensions exactly. All messages, countdowns, and board state are always 6×22. Rendering logic is predictable. The constraint forces messages to be concise and intentional.

3. **Variable per-message** — Each message defines its own grid size. Maximum flexibility but near-impossible to render consistently across terminal and web clients. Breaks the split-flap display metaphor entirely.

### Rationale

The 6×22 grid is the product's core design constraint, not a limitation. Like Twitter's character limit, the fixed format forces content to be concise and deliberate. Matching the Vestaboard physical format means Herald can serve as a digital twin of real hardware. Every renderer knows exactly what to expect, simplifying the terminal UI layout, the web CSS grid, and the WebSocket payload structure.

### Consequences

- **Positive:** Predictable rendering across all clients. Message composition is constrained, encouraging concise content. API payloads are fixed-size. Compatible with physical Vestaboard format. Simplifies testing — there's only one grid size to validate.
- **Negative:** Some messages won't fit the grid and will need to be truncated or split. There's no "expand" option for long-form content. Users who want different dimensions must fork the project. This is an intentional trade-off: the constraint *is* the feature.

---

## ADR-005: WebSocket for Real-Time Viewer Updates

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald viewers (both CLI and web) must receive board state updates in real time when messages rotate, new messages are pushed, or countdown timers tick. The update mechanism must work across both browser-based (Wasm) and terminal-based (native) clients.

### Decision

Use WebSocket connections for all real-time communication between the server and viewers.

### Options Considered

1. **Server-Sent Events (SSE)** — Unidirectional (server → client) streaming over HTTP. Simpler than WebSocket, good browser support, automatic reconnection built into the `EventSource` API. However, unidirectional only — no future path for client → server communication without a separate HTTP endpoint.

2. **WebSocket** — Bidirectional, full-duplex communication over a single TCP connection. Well-supported in browsers and Rust ecosystem (`tokio-tungstenite`, Axum's built-in WebSocket). Enables both server push and future client-to-server messaging.

3. **HTTP polling** — Client repeatedly hits a REST endpoint on a timer. Simplest to implement but wastes bandwidth, adds latency (up to the polling interval), and doesn't scale well with many connected viewers.

4. **gRPC streaming** — High-performance bidirectional streaming with strong typing (protobuf). However, adds a heavy dependency (tonic, protobuf compiler), poor native browser support (requires grpc-web proxy), and is overkill for Herald's simple update model.

### Rationale

WebSocket provides real-time push with low latency and is natively supported in both browser JavaScript/Wasm and Rust terminal clients. While SSE would be sufficient for the current unidirectional push model, WebSocket is future-proof — it enables bidirectional features like viewer acknowledgments, live viewer counts pushed to admin, or interactive features down the road. Axum has first-class WebSocket support, and the `tokio-tungstenite` crate handles the protocol cleanly for CLI clients.

### Consequences

- **Positive:** Real-time updates with minimal latency. Works in browsers (Wasm via `web-sys` WebSocket API) and terminal clients (`tokio-tungstenite`). Bidirectional — future features don't require a protocol change. Single connection per viewer.
- **Negative:** More complex than SSE — requires connection lifecycle management (heartbeat pings, reconnection logic, graceful shutdown). Stateful connections mean the server must track connected clients. Proxies and load balancers must be configured for WebSocket upgrade (`Upgrade` and `Connection` headers). These complexities are manageable and well-understood.

---

## ADR-006: Leptos + WebAssembly for the Web Frontend

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald's web interface needs to render a 6×22 split-flap board with 3D CSS flip animations, real-time WebSocket updates, and smooth transitions. The project is an intentional exploration of Rust-to-WebAssembly as a frontend technology. The web frontend is built via Trunk and served as static assets by the Herald backend.

### Decision

Use Leptos compiled to WebAssembly via Trunk for the web frontend.

### Options Considered

1. **Plain HTML / CSS / JavaScript (no framework)** — Minimal dependencies, fast load time, full control. However, managing reactive DOM updates for 132 cells with real-time WebSocket data without a framework leads to fragile, imperative spaghetti code. No component model.

2. **Yew (Rust → Wasm)** — Mature Rust Wasm framework with a virtual DOM (React-like). Large community. However, the virtual DOM approach is less efficient for fine-grained updates (diffing 132 cells every tick), and the component model is heavier than needed.

3. **Leptos (Rust → Wasm)** — Fine-grained reactivity without a virtual DOM. Signals update only the specific DOM nodes that depend on changed data. Active development, good documentation, and compiles to small Wasm bundles. Built-in support for SSR (not needed now, but available).

4. **Dioxus (Rust → Wasm)** — React-like API for Rust, supports web, desktop, and mobile. Younger ecosystem, smaller community, and the multi-platform focus adds abstraction layers Herald doesn't need.

5. **TypeScript (React / Svelte)** — Industry-standard frontend development. Excellent tooling and ecosystem. However, breaks the Rust-only monorepo model, eliminates shared types with the backend, and doesn't align with the project's goal of exploring Rust Wasm.

### Rationale

Leptos's fine-grained reactivity model is ideal for Herald's use case: when the board state updates, only the specific cells that changed need to re-render — not the entire 132-cell grid. This is more efficient than virtual DOM diffing (Yew, Dioxus) for a grid that updates in real time. Leptos compiles to small Wasm bundles, has an active and growing community, and shares Rust types with the backend and CLI via the `herald-common` crate. Trunk handles the Wasm build pipeline cleanly.

### Consequences

- **Positive:** Shared Rust types across all components — `BoardState`, `CellContent`, `Color`, etc. are identical in server, CLI, and web. Fine-grained reactivity means efficient per-cell DOM updates. Small Wasm bundle size. Active community and good documentation.
- **Negative:** WebAssembly debugging is harder than JavaScript debugging — browser devtools have limited Wasm source-mapping support. The Leptos ecosystem, while growing rapidly, has fewer third-party component libraries than React or Svelte. Developers must learn Leptos's signal/memo reactive model. Trunk adds a build step that can be slow for large projects.

---

## ADR-007: ratatui for Terminal UI

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald's CLI viewer (`herald watch`) renders the 6×22 split-flap board in the terminal with character-by-character flip animations, color support, and a responsive layout. We need a terminal UI framework that provides rendering primitives, a widget system, and cross-platform terminal handling.

### Decision

Use ratatui with a crossterm backend for the terminal UI.

### Options Considered

1. **ratatui** — The de facto standard Rust TUI library. Forked from and succeeded `tui-rs`. Rich widget set (blocks, paragraphs, tables, gauges, custom widgets), immediate-mode rendering, active maintenance, large community. Uses pluggable backends — crossterm (cross-platform) or termion (Unix-only).

2. **cursive** — Callback-based TUI framework. Higher-level API with built-in event handling and view stacking. However, the callback model is less flexible for custom rendering (split-flap animations), and the community is smaller.

3. **tui-rs** — The original Rust TUI library, now unmaintained. ratatui is its actively-maintained fork and successor. Using tui-rs directly would mean depending on abandoned software.

4. **Raw crossterm** — Use the terminal manipulation library directly without a widget framework. Maximum control but requires implementing all rendering, layout, and widget logic from scratch. Unnecessary when ratatui provides these primitives.

### Rationale

ratatui is the clear choice: it's the actively-maintained successor to tui-rs, has the largest community of any Rust TUI library, and provides the rendering primitives needed to build custom split-flap tile widgets. The crossterm backend ensures cross-platform support (Linux, macOS, Windows). The immediate-mode rendering model gives precise control over per-cell updates, which is essential for the flip animation effect.

### Consequences

- **Positive:** Rich widget system with the flexibility to build custom split-flap tile rendering. Cross-platform via crossterm. Large community means more examples, tutorials, and third-party extensions. Active maintenance ensures ongoing compatibility with Rust ecosystem changes.
- **Negative:** Immediate-mode rendering requires manual state management — the application must track what changed and redraw accordingly. The flip animation (character cycling before settling) requires careful frame timing logic built on top of ratatui's rendering primitives. These are implementation details, not framework limitations.

---

## ADR-008: Single Admin with Bearer Token

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald's admin operations — pushing messages, managing countdowns, configuring the board — must be protected from unauthorized access. We need an authentication mechanism that matches Herald's single-admin, self-hosted deployment model.

### Decision

Use a single bearer token for admin authentication. The token is set in the configuration file (`herald.toml`) or via environment variable (`HERALD_AUTH__ADMIN_TOKEN`). All admin API endpoints require the token in the `Authorization: Bearer <token>` header.

### Options Considered

1. **Full user authentication (accounts, sessions, OAuth)** — Database-backed user accounts with password hashing, session management, and optionally OAuth providers. Full-featured but massively over-engineered for a single-admin personal project. Adds user management UI, password reset flows, session storage, and CSRF protection.

2. **API key / bearer token** — A single shared secret configured at deploy time. Admin requests include the token in the `Authorization` header. Simple to implement, simple to configure, simple to rotate.

3. **No authentication (trust the network)** — Rely on network-level security (VPN, firewall, bind to localhost). Zero implementation effort but dangerous if the server is accidentally exposed. No defense in depth.

4. **mTLS (mutual TLS)** — Client certificate authentication. Very secure but extremely complex to configure, requires PKI infrastructure, and is overkill for a personal message board.

### Rationale

A bearer token is the simplest mechanism that provides meaningful security. It requires zero infrastructure (no database tables, no session management), can be set via a single environment variable (ideal for Docker), and is trivial to implement in Axum middleware. For a self-hosted, single-admin application, this is the right level of security complexity. The token can be rotated by changing the config and restarting the server.

### Consequences

- **Positive:** Minimal implementation complexity. Easy to configure in Docker (`HERALD_AUTH__ADMIN_TOKEN` env var). No user management overhead. Works with standard HTTP tooling (`curl -H "Authorization: Bearer ..."`, Postman, etc.).
- **Negative:** No user identity — all admin actions are attributed to "the admin," not a named user. No built-in token rotation without restart. If the token leaks, all admin access is compromised until the token is changed. No audit log of *who* performed an action (there's only one admin). If multi-user support is ever needed, a more robust auth system would need to be added — but this can be layered on later as an additive change.

---

## ADR-009: Monorepo with Cargo Workspace

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald consists of multiple crates — the backend server, the CLI tool, the web frontend, and a shared library of common types. These components need to be developed, tested, and released in coordination. We need to decide on the repository structure.

### Decision

Use a single Git repository with a Cargo workspace containing all crates.

### Options Considered

1. **Separate repositories per component** — Each crate (herald-server, herald-cli, herald-web, herald-common) lives in its own Git repository. Independent versioning, independent CI, independent release cycles. However, cross-crate changes require coordinated PRs across multiple repos. The shared `herald-common` crate must be published to a registry (or used as a Git dependency) before other crates can consume changes.

2. **Monorepo with Cargo workspace** — All crates in a single repository under a unified `Cargo.toml` workspace. Shared dependencies are deduplicated. A single `cargo build` compiles everything. Cross-crate changes are atomic — one PR, one CI run, one merge.

### Rationale

Herald's components are tightly coupled through the shared `herald-common` crate. Any change to `BoardState`, `Message`, or `CellContent` types affects all three components simultaneously. A monorepo ensures these changes are atomic: you can update the shared type, the server's serialization, the CLI's rendering, and the web's rendering in a single commit. CI validates the entire workspace together, catching integration issues immediately.

### Consequences

- **Positive:** Atomic cross-component changes. Single CI pipeline validates everything together. Shared types are always in sync — no version mismatch between `herald-common` consumers. Single `cargo build --workspace` builds the entire project. Simpler dependency management (shared `Cargo.lock`).
- **Negative:** All crates share a single Git history, which can make per-component changelogs harder to generate. CI builds the entire workspace even for changes isolated to one crate (mitigated by caching). Repository size grows with all components — but Herald is small enough that this is negligible.

---

## ADR-010: TOML for Configuration

**Status:** Accepted
**Date:** 2026-04-15

### Context

Herald needs a configuration mechanism for server settings (bind address, port), database path, authentication tokens, rotation intervals, and other operational parameters. The configuration must work for both bare-metal deployments (config file on disk) and containerized deployments (environment variables in Docker).

### Decision

Use a TOML configuration file (`herald.toml`) as the primary configuration source, with environment variable overrides using the `HERALD_` prefix and double-underscore nesting (e.g., `HERALD_SERVER__PORT=3000` maps to `[server] port = 3000`).

### Options Considered

1. **YAML** — Widely used for configuration. Good human readability. However, YAML's implicit type coercion is a source of bugs (e.g., `no` becomes `false`, `3.10` becomes `3.1`), and the Rust `serde_yaml` crate has had maintenance issues.

2. **JSON** — Universal data format, excellent serde support. However, no comments allowed (bad for config files), verbose syntax with required quoting, and not human-friendly for hand-editing.

3. **TOML** — Rust ecosystem standard (Cargo itself uses `Cargo.toml`). Explicit types, comments supported, clean syntax for nested tables. Excellent `serde` support via `toml` crate. Familiar to any Rust developer.

4. **Environment variables only** — No config file, all settings via env vars. Works well for Docker but is painful for bare-metal deployments with many settings. No comments, no documentation inline with the config.

5. **CLI flags only** — All settings passed as command-line arguments. Gets unwieldy with many options and doesn't persist between runs.

### Rationale

TOML is the idiomatic choice in the Rust ecosystem — every Rust developer is already familiar with the format from `Cargo.toml`. It supports comments (critical for a config file that serves as its own documentation), has explicit types (no YAML-style coercion bugs), and has excellent `serde` support via the `toml` crate. The layered approach (TOML file → env var overrides → CLI flags) covers all deployment scenarios: the file provides documented defaults, env vars handle Docker/container configuration, and CLI flags enable one-off overrides.

### Consequences

- **Positive:** Familiar format for Rust developers. Comments allow the config file to be self-documenting. Env var overrides work naturally with Docker and CI. The `config` crate (or manual implementation) handles the layered merging cleanly. Strong `serde` support means the config struct and the file format stay in sync automatically.
- **Negative:** TOML's nested table syntax can be verbose for deeply nested config (mitigated by keeping Herald's config structure shallow). Environment variable mapping requires a convention (double underscore for nesting) that must be documented. Some operators unfamiliar with TOML may need to look up the syntax — but it's simple enough to learn in minutes.
