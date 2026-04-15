# Changelog

All notable changes to Herald will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with `herald-common` and `herald-server` crates
- Shared types: Grid (6×22), CellContent, Color, Message, Countdown, QueueItem, BoardState
- SQLite persistence with sqlx migrations (WAL mode)
- REST API: full CRUD for messages, countdowns, queue management, and configuration
- Bearer token authentication for admin endpoints
- Health check endpoint (unauthenticated) returning status, version, and uptime
- 17 integration tests covering all API endpoints and error cases
- CLI binary (`herald`) with subcommands: serve, watch, push, countdown, queue, config
