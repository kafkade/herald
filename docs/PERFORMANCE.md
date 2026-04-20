# Performance

Herald is designed to be lightweight and efficient for single-instance deployments. This document covers the performance characteristics of each component, the architectural decisions behind them, and guidance for profiling and optimization.

## Server Performance

### Architecture Choices

**SQLite with WAL mode** — Write-Ahead Logging eliminates write contention and allows concurrent readers. WAL mode is set at connection time via `SqliteConnectOptions::journal_mode(Wal)`.

**`tokio::sync::broadcast` for WebSocket fanout** — The broadcast channel provides lock-free O(1) send to all connected viewers. Channel capacity is 16 messages, which is sufficient for Herald's update frequency.

**Pre-serialized JSON broadcasts (`Arc<str>`)** — Board state is serialized to JSON once, wrapped in an `Arc<str>`, and sent to N viewers without per-client serialization overhead. Each viewer's WebSocket handler receives a clone of the `Arc` pointer, not a copy of the data.

**Connection pool** — `sqlx::SqlitePool` with 5 max connections handles concurrent API requests without contention.

**Atomic viewer tracking** — Connected viewer count uses `AtomicUsize` with relaxed ordering, avoiding mutex locks on the hot path of WebSocket connect/disconnect.

### SQLite Indexes

Herald uses targeted indexes to keep query performance predictable:

| Index | Table | Purpose |
|-------|-------|---------|
| `idx_messages_queue_position` | `messages` | Fast ORDER BY for queue retrieval |
| `idx_countdowns_queue_position` | `countdowns` | Fast ORDER BY for queue retrieval |
| `idx_messages_expires_at` | `messages` | Partial index (non-NULL only) for expiration queries |
| `idx_messages_display_at` | `messages` | Partial index (non-NULL only) for scheduled display queries |
| `idx_messages_deleted_at` | `messages` | Soft-delete filter queries |
| `idx_countdowns_deleted_at` | `countdowns` | Soft-delete filter queries |

Partial indexes on `expires_at` and `display_at` only index rows where the column is non-NULL, keeping the index small and writes fast for the common case.

### Expected Baselines

| Operation | Expected Latency |
|-----------|-----------------|
| Health check response | <1ms |
| Board state build + broadcast | <5ms with 100 queue items |
| WebSocket broadcast to 50 viewers | <1ms (pre-serialized, no per-client work) |
| Memory per WebSocket connection | ~2KB (broadcast receiver + socket state) |

## CLI Viewer Performance

### Architecture Choices

**Pre-allocated `DisplayGrid` buffer** — The `DisplayGrid` struct holds a `Vec<Vec<CellDisplayState>>` that is allocated once and reused across frames. No per-frame heap allocation occurs during animation.

**Time-based animation with frame-skip** — The animation engine samples at the current wall-clock time. If rendering falls behind (e.g., due to a slow terminal), intermediate frames are skipped and the display jumps to the correct state for the current time. This prevents animation from "catching up" and causing visual stutter.

**20ms animation tick (50fps)** — During transitions, the tick interval is `min(ANIMATION_TICK, normal_tick)` where `ANIMATION_TICK` is 20ms. The normal tick rate is configurable via the `--fps` flag (default: 30fps).

**Board diff detection** — When a new board state arrives over WebSocket, only cells that actually changed trigger animation. If the board is identical to the current display, no animation is started.

### Expected Baselines

| Operation | Expected |
|-----------|----------|
| Frame render time | <2ms for full 6×22 grid |
| Animation sample (per cell) | <1μs |
| Memory footprint | ~1MB (terminal buffer + display state + animation state) |

## Web Viewer Performance

### Architecture Choices

**`requestAnimationFrame` batching** — WebSocket messages that arrive between paint frames are coalesced. The RAF callback processes the latest board state, ensuring at most one DOM update per frame regardless of WebSocket message rate.

**Fine-grained reactive signals** — Each cell in the 6×22 grid has its own `RwSignal<CellContent>` (132 signals total, plus 132 for previous state). When the board updates, only signals for changed cells are written, and only those cells re-render. Unchanged cells incur zero rendering cost.

**CSS-driven 3D flip animation** — The split-flap flip effect uses CSS `transform: rotateX()` with `perspective`, which is GPU-accelerated. No JavaScript drives the animation frames — the browser's compositor handles it natively.

**Layout isolation** — Each tile uses `contain: layout style paint` to isolate layout recalculation. A style change on one tile cannot trigger reflow of neighboring tiles.

**Cascade stagger** — The left-to-right flip cascade uses CSS `animation-delay` driven by a `--col-index` custom property set per tile. The delay is computed as `col_index * 20ms + 350ms`, creating a natural wave effect without JavaScript timers.

**Compositor layer management** — Tiles use `will-change: transform` during flip animations for GPU compositing, but this is not applied to idle tiles, avoiding unnecessary memory consumption from compositor layers.

**Lazy AudioContext** — Web Audio resources are allocated only on the first user interaction that enables sound. This avoids browser autoplay restrictions and saves resources when sound is disabled.

### WASM Optimization

**`wasm-opt -Oz`** — Applied automatically via Trunk's build pipeline (`data-wasm-opt="z"` in `index.html`). This runs Binaryen's aggressive size optimizer on the output WASM.

**Release profile tuning:**

```toml
[profile.release]
lto = true           # Link-time optimization across all crates
codegen-units = 1    # Single codegen unit for maximum optimization
strip = true         # Strip debug symbols from binary

[profile.release.package.herald-web]
opt-level = "z"      # Optimize for size (smallest binary)
```

### Expected Baselines

| Metric | Target |
|--------|--------|
| Time to first flip | <500ms (WASM load + WebSocket connect + first board update) |
| Per-update render time | <2ms (signal updates + DOM diffing for changed cells only) |
| WASM binary size | <500KB gzipped |
| Memory | ~3MB (WASM heap + 132 cell signals + WebSocket state) |

## Profiling Guide

### Server

```bash
# Enable debug logging for request timing
HERALD_LOG_LEVEL=debug cargo run -p herald-server

# Use tokio-console for async task inspection (if compiled with support)
RUSTFLAGS="--cfg tokio_unstable" cargo run -p herald-server --features tokio-console

# SQLite query analysis — connect to the database and inspect query plans
sqlite3 herald.db "EXPLAIN QUERY PLAN SELECT * FROM messages WHERE deleted_at IS NULL ORDER BY queue_position;"
```

### CLI Viewer

```bash
# Profile with flamegraph (requires cargo-flamegraph)
cargo flamegraph -p herald-cli -- watch --server ws://localhost:3000/ws --fps 60

# Measure frame timing by increasing log verbosity
RUST_LOG=herald_cli=debug cargo run -p herald-cli -- watch --server ws://localhost:3000/ws
```

### Web Viewer

1. **Chrome DevTools → Performance tab** — Record during a board transition to see paint times, compositor layers, and JavaScript execution.
2. **Lighthouse audit** — Run a Lighthouse performance audit for general web vitals.
3. **WASM profiling** — Inspect the optimized binary:
   ```bash
   wasm-opt --print-profile dist/herald-web_bg.wasm
   ```
4. **Network tab** — Check the gzipped transfer size of the WASM binary against the <500KB target.

## Optimization Checklist

A checklist for contributors working on performance-sensitive changes:

- [ ] SQLite queries use appropriate indexes (verify with `EXPLAIN QUERY PLAN`)
- [ ] WebSocket broadcasts are pre-serialized (one JSON encode per update, not per viewer)
- [ ] Animation state is pre-allocated, not heap-allocated per frame
- [ ] CSS animations use `transform` and `opacity` only (GPU-accelerated properties)
- [ ] WASM binary is optimized with `wasm-opt -Oz`
- [ ] Idle UI elements release `will-change` compositor layers
- [ ] Reactive signals are fine-grained (per-cell, not per-row or per-grid)
- [ ] New SQLite columns that appear in WHERE clauses have appropriate indexes
- [ ] WebSocket message coalescing prevents redundant DOM updates
