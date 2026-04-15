# Herald — Copilot Instructions

## Build & Test

```bash
cargo build                       # Debug build (full workspace)
cargo build --release             # Release build
cargo test                        # Run all tests
cargo test api_tests              # Run integration tests
cargo clippy                      # Lint
cargo fmt                         # Format
cargo run -p herald-server        # Launch the server (default port 3000)
```

### Environment variables

```bash
HERALD_ADMIN_TOKEN=secret         # Bearer token for admin API (auto-generated if unset)
HERALD_DB_PATH=herald.db          # SQLite database path
HERALD_PORT=3000                  # Server listen port
HERALD_LOG_LEVEL=info             # Log level (trace, debug, info, warn, error)
```

## Architecture

Herald is a Rust monorepo with a workspace layout:

```
crates/
  herald-common/    — Shared types (Grid, CellContent, Message, Countdown, etc.)
  herald-server/    — Axum REST API + SQLite persistence
  herald-cli/       — Terminal viewer (ratatui TUI) [future: Phase 2+]
  herald-web/       — Web viewer (Leptos + Wasm) [future: Phase 5+]
```

**Server** (`herald-server`): Axum-based HTTP server with REST API and SQLite via `sqlx`. Routes are split into admin (auth-protected) and public (health, future WebSocket). State is `AppState` wrapping `Arc<InnerState>` with a `SqlitePool`.

**Shared types** (`herald-common`): The 6×22 board grid, cell content (Char/Color/Blank), messages, countdowns, queue items, board state, and all API request/response DTOs. Used by every crate.

**Data flow**: HTTP request → Axum handler → `db` module → SQLite → JSON response. The board is a 6×22 grid of `CellContent` cells. Queue is derived from messages + countdowns sorted by `queue_position`.

## Conventions

- **Error handling**: `thiserror` for `ApiError` enum in the server, with `IntoResponse` for automatic HTTP status codes. Don't use `unwrap()` in library code.
- **Database**: `sqlx` with runtime queries. Migrations in `crates/herald-server/migrations/`. Use WAL journal mode.
- **Auth**: Bearer token middleware applied at the router level to admin routes. Health endpoint is always public.
- **Types**: All shared types derive `Serialize + Deserialize + Debug + Clone`. Grid is validated to be exactly 6×22 on API input.
- **API responses**: Use `ListResponse<T>` for collections, `ErrorResponse` for errors, standard HTTP status codes (201 created, 204 no content, 400/401/404/500 for errors).

## Git Policy

**Never execute Git commands that modify history or submit code.** This includes `git commit`, `git push`, `git rebase`, `git merge`, `git reset`, `git cherry-pick`, `git revert`, and `git tag`. Read-only commands like `git status`, `git diff`, `git log`, and `git branch` are fine. A human must always review and commit code themselves.

## Key References

- `docs/ROADMAP.md` — 76 issues across 8 milestones (v0.1.0 through v1.0.0)
- `docs/SPEC.md` — Full project specification: board model, REST API, WebSocket, TUI, web viewer
- `docs/ARCHITECTURE.md` — System architecture, crate structure, data flows
- `docs/DECISIONS.md` — Architecture Decision Records (Rust, Axum, SQLite, Leptos, ratatui)
- `docs/RELEASING.md` — Release process, changelog maintenance, versioning policy
