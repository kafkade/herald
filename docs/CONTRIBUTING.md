# Contributing to Herald

Welcome, and thank you for considering a contribution to Herald! Whether it's a bug fix, a new feature, improved documentation, or just a question — we're glad you're here.

Before diving in, please take a moment to read through this guide. It will help you get your development environment set up, understand how the project is organized, and know what to expect from the review process.

If you haven't already, check out the [README](../README.md) for an overview of the project.

---

## Development Setup

### Prerequisites

| Tool | Minimum Version | Notes |
|---|---|---|
| **Rust** | 1.75+ | Install via [rustup](https://rustup.rs/). See `rust-toolchain.toml` for the exact version. |
| **SQLite dev libraries** | 3.35+ | `libsqlite3-dev` (Debian/Ubuntu) or `sqlite-devel` (Fedora) |

> **Tip:** On macOS, SQLite ships with the system. On Windows, sqlx can use a bundled version — no extra install needed.

### Clone and Build

```bash
git clone https://github.com/kafkade/herald.git
cd herald

# Build all crates
cargo build
```

### Run in Development Mode

**Start the server:**

```bash
# On Linux/macOS
export HERALD_ADMIN_TOKEN="dev-token"
cargo run -p herald-server

# On Windows (PowerShell)
$env:HERALD_ADMIN_TOKEN = "dev-token"
cargo run -p herald-server
```

The server starts on <http://localhost:3000> by default and prints the admin token to stdout.

**Run the CLI viewer:**

```bash
# In a separate terminal
cargo run -p herald-cli -- watch

# With animation speed control
cargo run -p herald-cli -- watch --animation-speed fast

# Disable animation
cargo run -p herald-cli -- watch --animation-speed off
```

**Push a message (PowerShell):**

```powershell
$env:HERALD_ADMIN_TOKEN = "dev-token"
.\scripts\add-message.ps1 -Text "HELLO WORLD"
.\scripts\add-color-message.ps1 -Text "GO TEAM" -Color green -FillRows all
.\scripts\list-queue.ps1
```

---

## Project Structure

Herald is organized as a Cargo workspace:

```
herald/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── herald-common/      # Shared types, message formats, Grid, CellContent
│   │   └── src/
│   ├── herald-server/      # Axum backend — REST API, WebSocket, SQLite persistence
│   │   ├── migrations/     # SQLite schema migrations
│   │   ├── src/
│   │   └── tests/
│   └── herald-cli/         # ratatui terminal viewer — split-flap TUI with animation
│       └── src/
├── scripts/                # PowerShell helper scripts for admin operations
└── docs/                   # Documentation
```

### Crate Dependency Graph

```
herald-server ───┐
herald-cli   ────┼──▶ herald-common
```

Both application crates depend on `herald-common` for shared types. A future `herald-web` crate will also depend on it.

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
| Message format, board types, grid rendering | `crates/herald-common/src/` |
| REST API, admin endpoints | `crates/herald-server/src/api/` |
| WebSocket handling, real-time push | `crates/herald-server/src/ws.rs` |
| Rotation engine, queue logic | `crates/herald-server/src/db.rs` |
| Database schema, migrations | `crates/herald-server/migrations/` |
| Terminal rendering, board widget | `crates/herald-cli/src/ui/` |
| Split-flap animation engine | `crates/herald-cli/src/ui/animation.rs` |
| CLI command structure | `crates/herald-cli/src/main.rs` |
| Countdown timer logic | `crates/herald-common/src/countdown.rs` |
| PowerShell admin scripts | `scripts/` |

For a deeper understanding of how the pieces fit together, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Thank You

Every contribution makes Herald better — whether it's code, docs, design feedback, or a thoughtful bug report. We appreciate your time and effort. 🧡
