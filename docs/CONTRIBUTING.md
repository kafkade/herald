# Contributing to Herald

Welcome, and thank you for considering a contribution to Herald! Whether it's a bug fix, a new feature, improved documentation, or just a question — we're glad you're here.

Before diving in, please take a moment to read through this guide. It will help you get your development environment set up, understand how the project is organized, and know what to expect from the review process.

If you haven't already, check out the [README](../README.md) for an overview of the project.

---

## Development Setup

### Prerequisites

| Tool | Minimum Version | Notes |
|---|---|---|
| **Rust** | 1.75+ | Install via [rustup](https://rustup.rs/) |
| **wasm32-unknown-unknown target** | — | `rustup target add wasm32-unknown-unknown` |
| **Trunk** | 0.17+ | `cargo install trunk` (for Wasm/Leptos builds) |
| **SQLite dev libraries** | 3.35+ | `libsqlite3-dev` (Debian/Ubuntu) or `sqlite-devel` (Fedora) |

> **Tip:** On macOS, SQLite ships with the system. On Windows, sqlx can use a bundled version — no extra install needed.

### Clone and Build

```bash
git clone https://github.com/your-org/herald.git
cd herald

# Build all crates
cargo build

# Build the Wasm frontend separately
cargo build -p herald-web --target wasm32-unknown-unknown
```

### Run in Development Mode

**Start the server:**

```bash
export HERALD_ADMIN_TOKEN="dev-token"
cargo run -p herald-server -- serve
```

The server starts on <http://localhost:3000> by default.

**Run the CLI viewer:**

```bash
# In a separate terminal
cargo run -p herald-cli -- watch
```

**Build and serve the web frontend with Trunk:**

```bash
cd herald-web
trunk serve --open
```

Trunk will compile the Leptos app to Wasm, serve it with hot-reload, and open your browser.

---

## Project Structure

Herald is organized as a Cargo workspace:

```
herald/
├── Cargo.toml              # Workspace root
├── herald-common/          # Shared types, message formats, protocol definitions
│   ├── Cargo.toml
│   └── src/
├── herald-server/          # Axum backend — REST API, WebSocket, static file serving
│   ├── Cargo.toml
│   └── src/
├── herald-cli/             # ratatui terminal viewer — split-flap TUI
│   ├── Cargo.toml
│   └── src/
├── herald-web/             # Leptos WebAssembly browser viewer
│   ├── Cargo.toml
│   ├── Trunk.toml
│   └── src/
└── docs/                   # Documentation
    ├── ARCHITECTURE.md
    ├── CONTRIBUTING.md
    ├── DEPLOYMENT.md
    └── SPEC.md
```

### Crate Dependency Graph

```
herald-server ──┐
herald-cli   ───┼──▶ herald-common
herald-web   ───┘
```

All three application crates depend on `herald-common` for shared types. They do not depend on each other.

---

## Running Tests

```bash
# Run all tests across the workspace
cargo test

# Run tests for a specific crate
cargo test -p herald-server
cargo test -p herald-common
cargo test -p herald-cli

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test -p herald-server -- test_message_push
```

If you're adding a new feature or fixing a bug, please include tests that cover the change.

---

## Code Style

We use standard Rust tooling to keep the codebase consistent:

```bash
# Format code
cargo fmt

# Run lints
cargo clippy -- -D warnings
```

### Conventions

- **Error handling** — Use `thiserror` for defining error types. Avoid `.unwrap()` in library/server code; prefer `?` and meaningful error variants.
- **Async** — All server-side I/O uses `tokio`. Follow standard async/await patterns; avoid blocking calls on the async runtime.
- **Naming** — Follow Rust API guidelines: `snake_case` for functions and variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- **Documentation** — Public APIs should have doc comments (`///`). If a function's purpose isn't obvious from its name, add a brief explanation.
- **Dependencies** — Be conservative. Discuss new dependencies in the issue or PR before adding them.

---

## Pull Request Process

1. **Fork** the repository and create a branch from `main`.
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Implement** your changes. Keep commits focused and well-described.

3. **Test** your changes locally:
   ```bash
   cargo fmt -- --check
   cargo clippy -- -D warnings
   cargo test
   cargo build -p herald-web --target wasm32-unknown-unknown
   ```

4. **Push** your branch and open a pull request against `main`.

5. **Fill out the PR description** — explain what the change does and why. Reference any related issues (e.g., `Closes #42`).

### PR Guidelines

- **One PR, one concern** — keep pull requests focused on a single feature, fix, or improvement. If you find an unrelated issue, file a separate PR.
- **Reference an issue** — PRs should reference an existing issue. If there isn't one, create it first so the change can be discussed.
- **CI must pass** — every PR runs through CI which checks:
  - `cargo fmt -- --check`
  - `cargo clippy -- -D warnings`
  - `cargo test`
  - `cargo build -p herald-web --target wasm32-unknown-unknown`
- **Be responsive** — if a reviewer requests changes, please address them in a timely manner or let us know if you need help.

---

## Issue Guidelines

### Bug Reports

A good bug report helps us fix the problem quickly. Please include:

- **Summary** — one-line description of the issue
- **Steps to reproduce** — minimal sequence of actions to trigger the bug
- **Expected behavior** — what you expected to happen
- **Actual behavior** — what actually happened (include error messages, logs, or screenshots)
- **Environment** — OS, Rust version (`rustc --version`), Herald version or commit hash
- **Additional context** — anything else that might help (config file, network setup, etc.)

### Feature Requests

We'd love to hear your ideas. When opening a feature request:

- **Describe the problem** — what pain point or use case motivates this?
- **Describe the solution** — what would the ideal behavior look like?
- **Alternatives considered** — have you looked at workarounds?
- **Scope** — is this a small tweak or a larger effort?

---

## Architecture Guide

Not sure where to start? Here's a quick orientation:

| I want to work on… | Look at… |
|---|---|
| Message format, board types, protocol | `herald-common/` |
| REST API, admin endpoints | `herald-server/src/api/` |
| WebSocket handling, real-time push | `herald-server/src/ws/` |
| Message queue, rotation logic | `herald-server/src/queue/` |
| Database schema, migrations | `herald-server/src/db/` |
| Terminal rendering, flip animations | `herald-cli/src/` |
| Browser UI, Leptos components | `herald-web/src/` |
| CSS split-flap animations | `herald-web/style/` |
| Countdown timer logic | `herald-common/src/countdown.rs` |

For a deeper understanding of how the pieces fit together, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Thank You

Every contribution makes Herald better — whether it's code, docs, design feedback, or a thoughtful bug report. We appreciate your time and effort. 🧡
