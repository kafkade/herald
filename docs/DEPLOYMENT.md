# Herald — Deployment Guide

> **Note:** Docker deployment (Dockerfile, docker-compose.yml) is implemented and ready for use. Systemd service units and reverse proxy configurations are planned for a future release. See also the [README](../README.md) Quick Start section.

This guide covers everything you need to deploy Herald, from Docker containers to bare-metal installs. By the end, you should have a running Herald instance accessible via web browser or CLI viewer.

For architecture details, see [ARCHITECTURE.md](./ARCHITECTURE.md). For the full system spec, see [SPEC.md](./SPEC.md). For design decisions behind these choices, see [DECISIONS.md](./DECISIONS.md).

---

## Table of Contents

- [1. Prerequisites](#1-prerequisites)
- [2. Docker Deployment (Recommended)](#2-docker-deployment-recommended)
  - [2.1 Dockerfile](#21-dockerfile)
  - [2.2 Docker Compose](#22-docker-compose)
  - [2.3 Running with Docker Compose](#23-running-with-docker-compose)
- [3. Bare Metal Deployment](#3-bare-metal-deployment)
  - [3.1 Build from Source](#31-build-from-source)
  - [3.2 Run the Server](#32-run-the-server)
  - [3.3 Systemd Service (Linux)](#33-systemd-service-linux)
- [4. Configuration](#4-configuration)
  - [4.1 Configuration File (herald.toml)](#41-configuration-file-heraldtoml)
  - [4.2 Environment Variable Overrides](#42-environment-variable-overrides)
- [5. Reverse Proxy Configuration](#5-reverse-proxy-configuration)
  - [5.1 Nginx](#51-nginx)
  - [5.2 Caddy](#52-caddy)
- [6. Persistent Data](#6-persistent-data)
- [7. Updating](#7-updating)
- [8. Monitoring & Health](#8-monitoring--health)
- [9. Security Considerations](#9-security-considerations)

---

## 1. Prerequisites

Depending on your deployment method, you'll need:

### For Docker deployment (recommended)

- **Docker Engine** 20.10+ and **Docker Compose** v2+
- No Rust toolchain required — everything builds inside the container

### For bare-metal deployment

- **Rust toolchain** — install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **wasm32-unknown-unknown target** (for building the web frontend):
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Trunk** (Wasm build tool for the Leptos frontend):
  ```bash
  cargo install trunk
  ```
- **Build essentials** — a C linker and standard build tools (`build-essential` on Debian/Ubuntu, `base-devel` on Arch, Xcode Command Line Tools on macOS)

### Optional (recommended for production)

- **Reverse proxy** — Nginx or Caddy for TLS termination, static asset caching, and WebSocket proxying
- **A domain name** — for HTTPS via Let's Encrypt (Caddy handles this automatically)

---

## 2. Docker Deployment (Recommended)

Docker is the simplest way to deploy Herald. The multi-stage Dockerfile builds the server binary and web frontend inside containers, producing a minimal runtime image.

### 2.1 Dockerfile

Place this `Dockerfile` in the repository root:

```dockerfile
# =============================================================================
# Stage 1: Build the Herald server binary
# =============================================================================
FROM rust:latest AS server-builder

WORKDIR /app
COPY . .

RUN cargo build --release -p herald-server

# =============================================================================
# Stage 2: Build the Leptos/Wasm web frontend
# =============================================================================
FROM rust:latest AS web-builder

# Install the wasm32 target and Trunk
RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk

WORKDIR /app
COPY . .

RUN cd herald-web && trunk build --release

# =============================================================================
# Stage 3: Minimal runtime image
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (OpenSSL, CA certificates)
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd --create-home --shell /bin/bash herald

WORKDIR /home/herald

# Copy the server binary from stage 1
COPY --from=server-builder /app/target/release/herald /usr/local/bin/herald

# Copy the built web assets from stage 2
COPY --from=web-builder /app/herald-web/dist /home/herald/static

# Create the data directory for SQLite
RUN mkdir -p /data && chown herald:herald /data

USER herald

# Default configuration via environment variables
ENV HERALD_SERVER__BIND_ADDRESS="0.0.0.0" \
    HERALD_SERVER__PORT="3000" \
    HERALD_DATABASE__PATH="/data/herald.db" \
    HERALD_WEB__STATIC_DIR="/home/herald/static"

EXPOSE 3000

ENTRYPOINT ["herald", "serve"]
```

> **Note:** The example above uses the old double-underscore (`HERALD_SERVER__PORT`) env var format and a simplified build. See the actual `Dockerfile` in the repository root for the current implementation, which uses `cargo-chef` for cached dependency builds, flat env vars (`HERALD_PORT`, `HERALD_DB_PATH`, etc.), and `curl` for health checks.

### 2.2 Docker Compose

Create a `docker-compose.yml` in the repository root:

```yaml
services:
  herald:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - herald_data:/data
    environment:
      HERALD_AUTH__ADMIN_TOKEN: "your-secret-token-here"
      HERALD_DATABASE__PATH: "/data/herald.db"
      HERALD_SERVER__BIND_ADDRESS: "0.0.0.0"
      HERALD_SERVER__PORT: "3000"
    restart: unless-stopped

volumes:
  herald_data:
```

> **Note:** The example above uses the old double-underscore env var format (`HERALD_AUTH__ADMIN_TOKEN`, `HERALD_SERVER__PORT`, etc.). The actual `docker-compose.yml` in the repository root uses the flat format (`HERALD_ADMIN_TOKEN`, `HERALD_PORT`, `HERALD_DB_PATH`, `HERALD_WEB_DIR`, `HERALD_LOG_LEVEL`). See that file for the current implementation.

> **Important:** Replace `your-secret-token-here` with a strong, random token. Generate one with:
> ```bash
> openssl rand -hex 32
> ```

### 2.3 Running with Docker Compose

**Start Herald:**

```bash
docker compose up -d
```

**View logs:**

```bash
docker compose logs -f herald
```

**Stop Herald:**

```bash
docker compose down
```

**Rebuild after code changes:**

```bash
docker compose up -d --build
```

#### Volume Mount

The `herald_data` named volume persists the SQLite database at `/data/herald.db` inside the container. This ensures your messages, countdowns, and configuration survive container restarts and rebuilds.

To back up the database:

```bash
docker compose exec herald cp /data/herald.db /data/herald.db.bak
# Or copy to host:
docker cp $(docker compose ps -q herald):/data/herald.db ./herald-backup.db
```

#### Environment Variables

All Herald configuration can be set via environment variables in the `docker-compose.yml`. See [Section 4.2](#42-environment-variable-overrides) for the full list. Environment variables override any values in `herald.toml`.

---

## 3. Bare Metal Deployment

### 3.1 Build from Source

```bash
# Clone the repository
git clone https://github.com/your-org/herald.git
cd herald

# Build the server binary (release mode)
cargo build --release -p herald-server

# Build the web frontend (Wasm)
cd herald-web
trunk build --release
cd ..
```

After building:
- The server binary is at `./target/release/herald`
- The web assets are at `./herald-web/dist/`

### 3.2 Run the Server

```bash
# Copy the web assets to a known location
mkdir -p ./static
cp -r ./herald-web/dist/* ./static/

# Run with a config file
./target/release/herald serve --config herald.toml

# Or run with environment variables
HERALD_AUTH__ADMIN_TOKEN="your-secret-token" \
HERALD_WEB__STATIC_DIR="./static" \
  ./target/release/herald serve
```

The server needs to know where the built web assets are. Set the `static_dir` in `herald.toml` or via the `HERALD_WEB__STATIC_DIR` environment variable. Point it to the directory containing the Trunk build output (the `index.html`, `.wasm`, and `.js` files).

### 3.3 Systemd Service (Linux)

Create `/etc/systemd/system/herald.service`:

```ini
[Unit]
Description=Herald Split-Flap Message Board
After=network.target

[Service]
Type=simple
User=herald
Group=herald
WorkingDirectory=/opt/herald
ExecStart=/opt/herald/herald serve --config /opt/herald/herald.toml
Restart=on-failure
RestartSec=5

# Environment overrides (optional — can also use herald.toml)
Environment=HERALD_AUTH__ADMIN_TOKEN=your-secret-token-here
Environment=HERALD_DATABASE__PATH=/opt/herald/data/herald.db
Environment=HERALD_WEB__STATIC_DIR=/opt/herald/static

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/herald/data

[Install]
WantedBy=multi-user.target
```

Set up and start the service:

```bash
# Create the herald user
sudo useradd --system --create-home --home-dir /opt/herald herald

# Copy files into place
sudo cp target/release/herald /opt/herald/
sudo cp -r herald-web/dist /opt/herald/static
sudo cp herald.toml /opt/herald/
sudo mkdir -p /opt/herald/data
sudo chown -R herald:herald /opt/herald

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable herald
sudo systemctl start herald

# Check status
sudo systemctl status herald
sudo journalctl -u herald -f
```

---

## 4. Configuration

Herald uses a layered configuration system. Values are resolved in this order (later overrides earlier):

1. **Built-in defaults** — sensible defaults for all settings
2. **Configuration file** (`herald.toml`) — primary configuration source
3. **Environment variables** (`HERALD_*`) — overrides for deployment-specific values
4. **CLI flags** — one-off overrides for the current run

### 4.1 Configuration File (herald.toml)

Below is a complete example `herald.toml` with all available parameters. Uncomment and modify values as needed:

```toml
# =============================================================================
# Herald Configuration
# =============================================================================
# All values shown are the defaults. Uncomment and change as needed.
# Environment variables override these values (see DEPLOYMENT.md § 4.2).

# -----------------------------------------------------------------------------
# Server settings
# -----------------------------------------------------------------------------
[server]
# Address to bind the HTTP server to.
# Use "0.0.0.0" to listen on all interfaces (required for Docker).
# Use "127.0.0.1" to restrict to localhost only.
bind_address = "0.0.0.0"

# Port for the HTTP server.
port = 3000

# -----------------------------------------------------------------------------
# Database settings
# -----------------------------------------------------------------------------
[database]
# Path to the SQLite database file.
# The file is created automatically on first run.
# For Docker, mount a volume at this path for persistence.
path = "./herald.db"

# -----------------------------------------------------------------------------
# Authentication
# -----------------------------------------------------------------------------
[auth]
# Bearer token for admin API access.
# All admin endpoints require: Authorization: Bearer <this-token>
# CHANGE THIS to a strong, random value before deploying.
admin_token = "change-me-to-a-secure-token"

# -----------------------------------------------------------------------------
# Rotation settings
# -----------------------------------------------------------------------------
[rotation]
# Seconds each message/countdown is displayed before rotating to the next.
interval_seconds = 30

# How often countdown timers refresh their display (in seconds).
countdown_refresh_seconds = 1

# -----------------------------------------------------------------------------
# Board settings
# -----------------------------------------------------------------------------
[board]
# Default text alignment for messages: "left", "center", or "right".
default_alignment = "center"

# What happens when a countdown reaches zero:
#   "show_message" — display the countdown's associated message
#   "remove"       — remove the countdown from the rotation queue
countdown_zero_behavior = "show_message"

# -----------------------------------------------------------------------------
# Web frontend settings
# -----------------------------------------------------------------------------
[web]
# Enable the admin web interface.
admin_enabled = true

# Path to the directory containing the built web frontend assets
# (index.html, .wasm, .js files from `trunk build`).
static_dir = "./static"

# -----------------------------------------------------------------------------
# WebSocket settings
# -----------------------------------------------------------------------------
[websocket]
# Interval (in seconds) between server-sent heartbeat pings.
# Clients that don't respond within this interval may be disconnected.
heartbeat_seconds = 30

# URL path for the WebSocket endpoint.
path = "/ws"
```

### 4.2 Environment Variable Overrides

Environment variables follow the pattern `HERALD_<SECTION>__<KEY>` — note the **double underscore** (`__`) separating nested TOML table names from key names.

> **Note:** The current Herald server implementation uses **flat** environment variable names instead of the double-underscore nested format shown in the table below. The actual env vars are: `HERALD_PORT`, `HERALD_DB_PATH`, `HERALD_WEB_DIR`, `HERALD_ADMIN_TOKEN`, `HERALD_LOG_LEVEL`, and `HERALD_LOG_FORMAT`. See the `Dockerfile` and `.env.example` in the repository root for the canonical list.

| TOML Key                            | Environment Variable                      | Example Value             |
|-------------------------------------|-------------------------------------------|---------------------------|
| `server.bind_address`               | `HERALD_SERVER__BIND_ADDRESS`             | `0.0.0.0`                |
| `server.port`                       | `HERALD_SERVER__PORT`                     | `3000`                   |
| `database.path`                     | `HERALD_DATABASE__PATH`                   | `/data/herald.db`        |
| `auth.admin_token`                  | `HERALD_AUTH__ADMIN_TOKEN`                | `my-secret-token`        |
| `rotation.interval_seconds`         | `HERALD_ROTATION__INTERVAL_SECONDS`       | `30`                     |
| `rotation.countdown_refresh_seconds`| `HERALD_ROTATION__COUNTDOWN_REFRESH_SECONDS` | `1`                   |
| `board.default_alignment`           | `HERALD_BOARD__DEFAULT_ALIGNMENT`         | `center`                 |
| `board.countdown_zero_behavior`     | `HERALD_BOARD__COUNTDOWN_ZERO_BEHAVIOR`   | `show_message`           |
| `web.admin_enabled`                 | `HERALD_WEB__ADMIN_ENABLED`               | `true`                   |
| `web.static_dir`                    | `HERALD_WEB__STATIC_DIR`                  | `./static`               |
| `websocket.heartbeat_seconds`       | `HERALD_WEBSOCKET__HEARTBEAT_SECONDS`     | `30`                     |
| `websocket.path`                    | `HERALD_WEBSOCKET__PATH`                  | `/ws`                    |

**Example:** Override the port and admin token via environment variables:

```bash
HERALD_PORT=8080 HERALD_ADMIN_TOKEN="super-secret" herald-server
```

---

## 5. Reverse Proxy Configuration

For production deployments, run Herald behind a reverse proxy to provide TLS termination, caching, and WebSocket upgrade handling.

### 5.1 Nginx

Create `/etc/nginx/sites-available/herald`:

```nginx
upstream herald_backend {
    server 127.0.0.1:3000;
}

server {
    listen 80;
    server_name herald.example.com;

    # Redirect HTTP to HTTPS
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name herald.example.com;

    # --- TLS Configuration ---
    ssl_certificate     /etc/letsencrypt/live/herald.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/herald.example.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    # --- WebSocket endpoint ---
    location /ws {
        proxy_pass http://herald_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket timeout — keep connections alive
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }

    # --- API and static assets ---
    location / {
        proxy_pass http://herald_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # --- Static asset caching ---
    location ~* \.(js|wasm|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
        proxy_pass http://herald_backend;
        proxy_set_header Host $host;
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
}
```

Enable the site and reload Nginx:

```bash
sudo ln -s /etc/nginx/sites-available/herald /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### 5.2 Caddy

Caddy provides automatic HTTPS via Let's Encrypt with zero configuration. Create a `Caddyfile`:

```caddyfile
herald.example.com {
    # Reverse proxy all traffic to Herald
    reverse_proxy localhost:3000

    # Static asset caching
    @static path *.js *.wasm *.css *.png *.jpg *.ico *.svg *.woff *.woff2
    header @static Cache-Control "public, max-age=604800, immutable"
}
```

That's it. Caddy automatically:
- Obtains and renews TLS certificates from Let's Encrypt
- Handles WebSocket upgrade headers (no special configuration needed)
- Proxies all requests to Herald

Run Caddy:

```bash
sudo caddy start --config /etc/caddy/Caddyfile
```

---

## 6. Persistent Data

### SQLite Database

Herald stores all data in a single SQLite file. By default, this is `./herald.db` (relative to the working directory), configurable via `database.path` in `herald.toml` or the `HERALD_DB_PATH` environment variable.

**First run:** The database file is created automatically. Herald runs all pending migrations on startup — no manual setup required.

**Backup strategy:** SQLite databases are single files. To back up:

```bash
# Simple file copy (stop Herald first for consistency, or use SQLite's backup API)
cp herald.db herald.db.bak

# Or use sqlite3's .backup command (safe while Herald is running)
sqlite3 herald.db ".backup herald-backup.db"
```

### Docker Volume Best Practices

- **Always use a named volume** (as shown in the Docker Compose example) to persist data across container rebuilds.
- **Never store the database inside the container's writable layer** — it will be lost on `docker compose down`.
- The volume mount point should match `HERALD_DB_PATH` (default: `/data/herald.db`).

```yaml
volumes:
  - herald_data:/data    # Named volume — persists across rebuilds
  # - ./data:/data       # Bind mount — alternative, stores on host filesystem
```

---

## 7. Updating

### Docker

```bash
# Pull latest changes and rebuild
cd herald
git pull
docker compose up -d --build

# Or if using a pre-built image from a registry:
docker compose pull
docker compose up -d
```

### Bare Metal

```bash
cd herald
git pull

# Rebuild the server
cargo build --release -p herald-server

# Rebuild the web frontend
cd herald-web
trunk build --release
cd ..

# Copy updated assets
cp -r herald-web/dist/* /opt/herald/static/
cp target/release/herald /opt/herald/

# Restart the service
sudo systemctl restart herald
```

### Database Migrations

Herald runs database migrations automatically on server startup. When you update to a new version:

1. The server detects pending migrations
2. Migrations are applied in order before the server begins accepting requests
3. No manual `migrate` command is needed

> **Tip:** Back up your database before updating to a new version, in case a migration needs to be rolled back.

---

## 8. Monitoring & Health

### Health Endpoint

Herald exposes a health check endpoint:

```
GET /api/health
```

**Response (200 OK):**

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

Use this for:
- Docker `HEALTHCHECK` directives
- Load balancer health checks
- Uptime monitoring (e.g., UptimeRobot, Healthchecks.io)

**Docker Compose healthcheck example:**

```yaml
services:
  herald:
    # ... other config ...
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
```

### Logs

Herald logs to stdout in a structured format. Key things to watch for:

| Log Level | What It Means                                          |
|-----------|--------------------------------------------------------|
| `INFO`    | Server started, client connected/disconnected, rotation events |
| `WARN`    | Failed authentication attempts, WebSocket errors, config issues |
| `ERROR`   | Database errors, unrecoverable failures, bind failures |

Set the log level via the `RUST_LOG` environment variable:

```bash
# Show info and above (default)
RUST_LOG=info herald serve

# Verbose debugging
RUST_LOG=debug herald serve

# Herald-specific debug logs only
RUST_LOG=herald=debug herald serve
```

### Connected Viewers

The admin API provides information about connected viewers:

```bash
curl -H "Authorization: Bearer <token>" http://localhost:3000/api/admin/viewers
```

This returns the number of active WebSocket connections (both CLI and web viewers).

---

## 9. Security Considerations

### Admin Token

- **Always change the default token** before deploying. Use a cryptographically random value:
  ```bash
  openssl rand -hex 32
  ```
- Store the token in an environment variable, not in a config file committed to version control.
- Rotate the token by changing the env var and restarting Herald.

### TLS / HTTPS

- **Always run behind a reverse proxy with TLS** in production. Herald itself does not handle TLS — use Nginx or Caddy (see [Section 5](#5-reverse-proxy-configuration)).
- Caddy provides automatic HTTPS with zero configuration.
- For Nginx, use Let's Encrypt with Certbot for free certificates.

### Network Binding

- In production behind a reverse proxy, bind Herald to `127.0.0.1` so it's only accessible via the proxy:
  ```toml
  [server]
  bind_address = "127.0.0.1"
  ```
- In Docker, bind to `0.0.0.0` inside the container (the default) — Docker's network isolation provides the boundary.

### SQLite File Permissions

- The database file contains all messages and configuration. Restrict file permissions:
  ```bash
  chmod 600 herald.db
  chown herald:herald herald.db
  ```
- In Docker, the non-root `herald` user owns the data directory.

### General Recommendations

- Keep Herald updated to get security fixes.
- Use the systemd hardening options shown in [Section 3.3](#33-systemd-service-linux) (`NoNewPrivileges`, `ProtectSystem`, `ProtectHome`).
- Monitor failed authentication attempts in the logs (`WARN` level).
- If Herald is only accessed from your local machine, bind to `127.0.0.1` and skip the reverse proxy entirely.
