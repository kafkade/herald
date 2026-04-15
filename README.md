# Herald

**A split-flap message board for your terminal and browser.**

Herald is an open-source, split-flap / Vestaboard-style digital message board and countdown tracker built entirely in Rust. Inspired by the iconic departure boards in train stations — where letters and symbols mechanically flip into place — Herald brings that aesthetic to both the terminal and the web browser, backed by a real-time server.

<!-- TODO: Add hero image / demo GIF -->

---

## ✨ Features

- **6×22 character grid** — Vestaboard-compatible format
- **Full character set** — A–Z, 0–9, special characters, plus colored tiles (red, orange, yellow, green, blue, violet, white, black)
- **Message queue** with configurable rotation (default 30 s)
- **Countdown timers** mixed into the rotation
- **Real-time WebSocket push** to all connected viewers
- **Terminal viewer** — split-flap flip animations right in your terminal
- **Browser viewer** — 3D split-flap animations via CSS + WebAssembly
- **Single-binary server** — serves the web frontend as static assets
- **Simple auth** — single admin with bearer-token authentication
- **Self-hosted** — Docker or bare metal, SQLite for persistence
- **All Rust** — monorepo, one toolchain, no JavaScript runtime

---

## 📸 Screenshots

### Terminal (herald-cli)

<!-- TODO: Add terminal screenshot -->

```
┌──────────────────────────────────────────────┐
│  H E L L O   W O R L D · · · · · · · · · ·  │
│  · · · · · · · · · · · · · · · · · · · · · · │
│  · · · · · · · · · · · · · · · · · · · · · · │
│  · · · · · · · · · · · · · · · · · · · · · · │
│  · · · · · · · · · · · · · · · · · · · · · · │
│  · · · · · · · · · · · · · · · · · · · · · · │
└──────────────────────────────────────────────┘
```

### Browser (herald-web)

<!-- TODO: Add browser screenshot -->

---

## 🚀 Quick Start

### Docker

```bash
docker run -p 3000:3000 herald
```

Open <http://localhost:3000> to see the web viewer.

### From Source

```bash
git clone https://github.com/your-org/herald.git
cd herald
cargo build --release

# Start the server
./target/release/herald serve
```

### First Steps

```bash
# Set an admin token (or configure via TOML / env var)
export HERALD_ADMIN_TOKEN="my-secret-token"

# Start the server
herald serve

# Push your first message
herald push "HELLO WORLD"

# Open the TUI viewer in another terminal
herald watch

# Or open http://localhost:3000 in your browser
```

---

## 🏗 Architecture

Herald is structured as a Cargo workspace with three main crates and a shared library:

```
                 ┌──────────────────────┐
                 │    herald-server      │
                 │    (Axum + SQLite)    │
                 └───┬─────────────┬────┘
          REST API   │             │  WebSocket
         (admin ops) │             │  (real-time push)
                     │             │
             ┌───────┘             └───────┐
             │                             │
  ┌──────────▼───────────┐   ┌────────────▼───────────┐
  │     herald-cli       │   │      herald-web         │
  │  (ratatui terminal)  │   │    (Leptos → Wasm)      │
  └──────────┬───────────┘   └────────────┬───────────┘
             │                            │
             └──────────┬─────────────────┘
                        │
              ┌─────────▼──────────┐
              │   herald-common    │
              │   (shared types)   │
              └────────────────────┘
```

- **herald-server** — Axum-based backend. REST API for admin operations, WebSocket for viewer push. Serves the web frontend as static assets. SQLite (via sqlx) for persistence.
- **herald-cli** — Terminal TUI viewer built with ratatui + crossterm. Connects via WebSocket and renders split-flap board with flip animations and color support.
- **herald-web** — Browser viewer built with Leptos, compiled to WebAssembly. Connects via WebSocket and renders 3D split-flap animations with CSS.
- **herald-common** — Shared types, message formats, and protocol definitions used by all other crates.

For a deeper dive, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

---

## 💻 CLI Reference

| Command | Description |
|---|---|
| `herald serve` | Start the Herald server |
| `herald watch` | Open the terminal split-flap viewer (TUI) |
| `herald push "<MESSAGE>"` | Push a message to the board |
| `herald countdown create` | Create a countdown timer (`--label`, `--target`) |
| `herald queue list` | List messages in the rotation queue |
| `herald queue remove <ID>` | Remove a message from the queue |
| `herald config set <KEY> <VALUE>` | Update a configuration value |
| `herald config get <KEY>` | Read a configuration value |

**Examples:**

```bash
# Push a message
herald push "TRAIN TO PARIS"

# Create a countdown
herald countdown create --label "LAUNCH" --target "2025-12-31T00:00:00Z"

# Change rotation interval to 45 seconds
herald config set rotation_interval 45
```

For full CLI documentation, see [docs/CLI.md](docs/CLI.md).

---

## ⚙️ Configuration

Herald is configured via a TOML file, with environment variable overrides.

| Option | Default | Env Override | Description |
|---|---|---|---|
| `server.port` | `3000` | `HERALD_PORT` | HTTP / WebSocket listen port |
| `server.host` | `0.0.0.0` | `HERALD_HOST` | Bind address |
| `admin.token` | *(required)* | `HERALD_ADMIN_TOKEN` | Bearer token for admin API |
| `board.rotation_interval` | `30` | `HERALD_ROTATION_INTERVAL` | Seconds between message rotations |
| `board.rows` | `6` | — | Board row count |
| `board.cols` | `22` | — | Board column count |
| `database.path` | `herald.db` | `HERALD_DB_PATH` | SQLite database file path |

See [docs/SPEC.md](docs/SPEC.md) for the full configuration reference.

---

## 🐳 Deployment

### Docker

```bash
docker run -d \
  --name herald \
  -p 3000:3000 \
  -e HERALD_ADMIN_TOKEN="my-secret-token" \
  -v herald-data:/data \
  herald
```

### Bare Metal

```bash
cargo build --release
cp target/release/herald /usr/local/bin/

# Create a config file
cat > /etc/herald/config.toml <<EOF
[server]
port = 3000

[admin]
token = "my-secret-token"

[database]
path = "/var/lib/herald/herald.db"
EOF

herald serve --config /etc/herald/config.toml
```

For systemd units, reverse proxy setup, and production hardening, see [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

---

## 🤝 Contributing

We welcome contributions of all kinds — bug reports, feature requests, documentation improvements, and code. Whether you're fixing a typo or building a new feature, we'd love your help.

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) to get started.

---

## 📄 License

Herald is licensed under the [MIT License](LICENSE).

> **Note:** If you have a preference for Apache-2.0 or dual licensing (MIT OR Apache-2.0, common in the Rust ecosystem), open an issue and we can discuss.

---

## 🙏 Acknowledgments

- [Vestaboard](https://www.vestaboard.com/) — visual inspiration for the 6×22 character grid and colored tile system.
- [Solari di Udine](https://en.wikipedia.org/wiki/Solari_di_Udine) — creators of the original split-flap (Solari board) display mechanism that has graced train stations and airports worldwide since the 1950s.
- The Rust community and the maintainers of [Axum](https://github.com/tokio-rs/axum), [ratatui](https://github.com/ratatui/ratatui), [Leptos](https://github.com/leptos-rs/leptos), and [sqlx](https://github.com/launchbadge/sqlx) — Herald stands on your shoulders.
