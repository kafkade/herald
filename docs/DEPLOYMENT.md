# Herald — Deployment Guide

This guide covers deploying Herald from a 5-minute Docker quickstart to production bare-metal installs with reverse proxies and TLS.

For architecture details, see [ARCHITECTURE.md](./ARCHITECTURE.md). For the full system spec, see [SPEC.md](./SPEC.md).

---

## Table of Contents

- [1. Docker Compose Quickstart](#1-docker-compose-quickstart)
- [2. Bare-Metal Installation](#2-bare-metal-installation)
  - [2.1 Download from GitHub Releases](#21-download-from-github-releases)
  - [2.2 Build from Source](#22-build-from-source)
  - [2.3 Systemd Service](#23-systemd-service)
- [3. Environment Variable Reference](#3-environment-variable-reference)
- [4. SQLite Database](#4-sqlite-database)
  - [4.1 Location & Creation](#41-location--creation)
  - [4.2 Backup](#42-backup)
  - [4.3 Restore](#43-restore)
- [5. Reverse Proxy Configuration](#5-reverse-proxy-configuration)
- [6. HTTPS / TLS](#6-https--tls)
- [7. Upgrading](#7-upgrading)
- [8. Troubleshooting](#8-troubleshooting)

---

## 1. Docker Compose Quickstart

Get Herald running in under 5 minutes.

### Prerequisites

- Docker Engine 20.10+ and Docker Compose v2+

### Steps

**1. Clone the repository:**

```bash
git clone https://github.com/kafkade/herald.git
cd herald
```

**2. Create a `.env` file** (optional — sane defaults are provided):

```bash
cp .env.example .env
```

Edit `.env` and set a strong admin token:

```bash
# Generate a secure token
openssl rand -hex 32

# Paste it into .env
HERALD_ADMIN_TOKEN=<your-generated-token>
```

**3. Start Herald:**

```bash
docker compose up -d
```

> **Using the pre-built image:** To skip building from source, edit `docker-compose.yml` and replace `build: .` with:
> ```yaml
> image: ghcr.io/kafkade/herald:latest
> ```

**4. Verify it's running:**

```bash
curl http://localhost:3000/api/health
```

Expected response:

```json
{"status":"ok"}
```

**5. Open the web UI** at [http://localhost:3000](http://localhost:3000).

### Managing the container

```bash
docker compose logs -f herald     # Stream logs
docker compose restart herald     # Restart
docker compose down               # Stop and remove container (data persists in volume)
docker compose up -d --build      # Rebuild after code changes
```

The `docker-compose.yml` in the repository root defines a `herald_data` named volume mounted at `/data` for SQLite persistence. Your data survives container restarts and rebuilds.

---

## 2. Bare-Metal Installation

### 2.1 Download from GitHub Releases

Download a pre-built binary from [GitHub Releases](https://github.com/kafkade/herald/releases):

```bash
# Example for Linux x86_64 — adjust the URL for your platform/version
curl -Lo herald-server https://github.com/kafkade/herald/releases/latest/download/herald-server-linux-amd64
chmod +x herald-server
sudo mv herald-server /usr/local/bin/
```

### 2.2 Build from Source

If no pre-built binary is available for your platform:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/kafkade/herald.git
cd herald
cargo build --release -p herald-server
```

The binary is at `./target/release/herald-server`.

### Run manually

```bash
# Set environment and run
export HERALD_ADMIN_TOKEN=$(openssl rand -hex 32)
export HERALD_DB_PATH=/var/lib/herald/herald.db
export HERALD_WEB_DIR=/opt/herald/static
herald-server
```

Herald listens on port 3000 by default and creates the SQLite database automatically on first run.

### 2.3 Systemd Service

Create a dedicated user and directories:

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin herald
sudo mkdir -p /opt/herald/data /opt/herald/static
sudo chown -R herald:herald /opt/herald
```

Copy the binary and web assets into place:

```bash
sudo cp target/release/herald-server /usr/local/bin/
# If you built the web frontend:
sudo cp -r crates/herald-web/dist/* /opt/herald/static/
```

Create `/etc/systemd/system/herald.service`:

```ini
[Unit]
Description=Herald Split-Flap Message Board
After=network.target

[Service]
Type=simple
User=herald
Group=herald
ExecStart=/usr/local/bin/herald-server
Restart=on-failure
RestartSec=5

# Environment
Environment=HERALD_ADMIN_TOKEN=<your-secret-token>
Environment=HERALD_DB_PATH=/opt/herald/data/herald.db
Environment=HERALD_WEB_DIR=/opt/herald/static
Environment=HERALD_PORT=3000
Environment=HERALD_LOG_LEVEL=info
Environment=HERALD_LOG_FORMAT=json

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/herald/data

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now herald

# Verify
sudo systemctl status herald
curl http://localhost:3000/api/health
```

View logs:

```bash
sudo journalctl -u herald -f
```

---

## 3. Environment Variable Reference

All configuration is via environment variables. No config file is required.

| Variable | Description | Default |
|---|---|---|
| `HERALD_ADMIN_TOKEN` | Bearer token for admin API authentication. If unset, a random UUID is generated and logged at startup. | *(auto-generated)* |
| `HERALD_PORT` | HTTP listen port. | `3000` |
| `HERALD_DB_PATH` | Path to the SQLite database file. Created automatically if it doesn't exist. | `herald.db` |
| `HERALD_WEB_DIR` | Directory containing built web frontend assets (index.html, WASM, etc.). Set to empty to disable web UI. | `./web-dist` |
| `HERALD_LOG_LEVEL` | Log verbosity: `trace`, `debug`, `info`, `warn`, `error`. | `info` |
| `HERALD_LOG_FORMAT` | Log output format: `json` (structured, for production) or `pretty` (human-readable). | `pretty` |

**Example** — override port and log level:

```bash
HERALD_PORT=8080 HERALD_LOG_LEVEL=debug herald-server
```

See `.env.example` in the repository root for a ready-to-copy template.

---

## 4. SQLite Database

Herald stores all data (messages, countdowns, board state) in a single SQLite file using WAL (Write-Ahead Logging) journal mode for concurrent read performance.

### 4.1 Location & Creation

- Configured via `HERALD_DB_PATH` (default: `herald.db` in the working directory).
- The file and any parent directories are created automatically on first startup.
- Database migrations run automatically — no manual setup required.
- In Docker, the database lives at `/data/herald.db` inside the `herald_data` volume.

### 4.2 Backup

**Option A — Checkpoint and copy** (safe while Herald is running):

```bash
# Force a WAL checkpoint, then copy the database
sqlite3 /path/to/herald.db "PRAGMA wal_checkpoint(TRUNCATE);"
cp /path/to/herald.db /path/to/backup/herald-$(date +%Y%m%d).db
```

**Option B — SQLite `.backup` command** (online backup):

```bash
sqlite3 /path/to/herald.db ".backup /path/to/backup/herald-$(date +%Y%m%d).db"
```

**Option C — Docker volume backup:**

```bash
# Copy database out of the container
docker cp $(docker compose ps -q herald):/data/herald.db ./herald-backup.db
```

> **Tip:** Schedule daily backups with cron. The WAL checkpoint in Option A ensures the backup file is self-contained (no `-wal` or `-shm` files needed).

### 4.3 Restore

```bash
# Stop Herald
sudo systemctl stop herald   # or: docker compose down

# Replace the database
cp /path/to/backup/herald-20250101.db /path/to/herald.db

# Start Herald
sudo systemctl start herald  # or: docker compose up -d
```

Herald will pick up the restored database and apply any pending migrations automatically.

---

## 5. Reverse Proxy Configuration

For production, run Herald behind a reverse proxy to handle TLS termination and WebSocket upgrades. Herald does not terminate TLS itself.

Ready-to-use example configs are in [`examples/nginx.conf`](../examples/nginx.conf) and [`examples/Caddyfile`](../examples/Caddyfile). Copy the example config, replace `herald.example.com` with your domain, and adjust the upstream port if needed.

Key points for any proxy:
- Forward HTTP traffic to `localhost:3000` (or your configured port)
- Enable WebSocket upgrades for the `/ws` path (`Upgrade` and `Connection` headers)
- Pass `X-Forwarded-For` and `X-Forwarded-Proto` headers

### Nginx

See [`examples/nginx.conf`](../examples/nginx.conf) for a complete configuration including:
- HTTP → HTTPS redirect
- WebSocket upgrade handling for `/ws`
- Static asset caching
- Let's Encrypt TLS certificate paths

Quick setup:

```bash
sudo cp examples/nginx.conf /etc/nginx/sites-available/herald
sudo ln -s /etc/nginx/sites-available/herald /etc/nginx/sites-enabled/
# Edit server_name and certificate paths
sudo nginx -t && sudo systemctl reload nginx
```

### Caddy

See [`examples/Caddyfile`](../examples/Caddyfile) for a complete configuration. Caddy is the simplest option — it automatically obtains and renews Let's Encrypt certificates and handles WebSocket upgrades with no extra configuration:

```caddyfile
herald.example.com {
    reverse_proxy localhost:3000
}
```

```bash
sudo cp examples/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

---

## 6. HTTPS / TLS

Herald does not handle TLS directly. Use a reverse proxy with [Let's Encrypt](https://letsencrypt.org/) for free, automated certificates.

**Caddy** (recommended — zero-config TLS):

Caddy automatically provisions and renews certificates. Just point your domain at the server and use the Caddyfile from [Section 5](#5-reverse-proxy-configuration).

**Nginx + Certbot:**

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Obtain certificate (follow interactive prompts)
sudo certbot --nginx -d herald.example.com

# Auto-renewal is configured automatically — verify:
sudo certbot renew --dry-run
```

Update the Nginx config to reference the generated certificate paths. See [`examples/nginx.conf`](../examples/nginx.conf) for the full TLS configuration.

---

## 7. Upgrading

### Before you upgrade

1. **Back up the database** (see [Section 4.2](#42-backup)):
   ```bash
   sqlite3 /path/to/herald.db ".backup herald-pre-upgrade.db"
   ```
2. **Check the [release notes](https://github.com/kafkade/herald/releases)** for breaking changes or required migration steps.

### Docker

```bash
cd herald

# If using the pre-built image:
docker compose pull
docker compose up -d

# If building from source:
git pull
docker compose up -d --build
```

### Bare metal

```bash
# Download or build the new binary
# Option A: Download from releases
curl -Lo herald-server https://github.com/kafkade/herald/releases/latest/download/herald-server-linux-amd64
chmod +x herald-server
sudo mv herald-server /usr/local/bin/

# Option B: Build from source
git pull
cargo build --release -p herald-server
sudo cp target/release/herald-server /usr/local/bin/

# Restart the service
sudo systemctl restart herald
```

### After upgrading

Verify the new version is running:

```bash
curl http://localhost:3000/api/health
```

Database migrations run automatically on startup — no manual steps required.

---

## 8. Troubleshooting

### Connection refused on port 3000

- **Is Herald running?**
  ```bash
  sudo systemctl status herald
  # or
  docker compose ps
  ```
- **Is the port correct?** Check `HERALD_PORT` in your environment or `.env` file.
- **Firewall?** Ensure port 3000 (or your custom port) is open:
  ```bash
  sudo ufw allow 3000/tcp
  ```

### 401 Unauthorized on admin endpoints

- Verify you're sending the correct token:
  ```bash
  curl -H "Authorization: Bearer <your-token>" http://localhost:3000/api/admin/messages
  ```
- If `HERALD_ADMIN_TOKEN` is unset, Herald auto-generates a token and logs it at startup. Check the logs:
  ```bash
  docker compose logs herald | grep -i token
  # or
  sudo journalctl -u herald | grep -i token
  ```
- The `Authorization` header must use the format `Bearer <token>` (note the space after "Bearer").

### WebSocket connections failing

- **Behind a proxy?** Ensure WebSocket upgrade headers are forwarded. The proxy must pass `Upgrade` and `Connection` headers for the `/ws` path. See [Section 5](#5-reverse-proxy-configuration).
- **Timeouts?** Set a long read timeout on the proxy (e.g., `proxy_read_timeout 86400s` for Nginx).
- **Test directly** (bypass the proxy) to isolate the issue:
  ```bash
  # Using websocat (install: cargo install websocat)
  websocat ws://localhost:3000/ws
  ```

### Database locked errors

- SQLite allows only one writer at a time. Herald uses WAL mode, which handles this well under normal load.
- If you see `database is locked` errors:
  - Ensure no other process has the database file open (e.g., a backup script with a long-held lock).
  - Check that the `-wal` and `-shm` files exist alongside `herald.db` — don't delete them while Herald is running.
  - Ensure the database directory has correct write permissions for the Herald user.

### Port already in use

- Find what's using the port:
  ```bash
  sudo ss -tlnp | grep 3000
  ```
- Either stop the conflicting process or change Herald's port:
  ```bash
  HERALD_PORT=3001 herald-server
  ```
- In Docker, update both the container and host port mapping in `docker-compose.yml` or `.env`.

### High memory or CPU usage

- Set `HERALD_LOG_LEVEL=warn` to reduce log output in production.
- Ensure `HERALD_LOG_FORMAT=json` for efficient log processing.
- Herald is lightweight by design — sustained high resource usage usually indicates an external issue (e.g., a misbehaving WebSocket client reconnecting in a tight loop).
