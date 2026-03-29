# Changelog

All notable changes to **rustdesk-server-admin** are summarized here. For configuration and deployment, see [README.md](README.md).

## [Unreleased]

Planned / follow-up work can be tracked here or in GitHub Issues.

## [0.1.0] — 2026-03-29

### Added

- Axum HTTP server with password login (`ADMIN_PASSWORD`), HMAC-signed session cookie (`ADMIN_SESSION_SECRET`), configurable `ADMIN_HOST` / `ADMIN_PORT` (default `127.0.0.1:3030`).
- Dashboard: peer count from hbbs SQLite (`HBBS_DB_PATH`).
- Peer list (`/peers`): full `peer` table columns; search on id, uuid (hex), note, info; `rustdesk://` connect links with optional `RUSTDESK_CONNECT_RENDEZVOUS` for `/r@host` URLs.
- Peer **delete** and **rename ID** (SQLite write; `busy_timeout` for coexistence with hbbs).
- CI: [`.github/workflows/manual-binaries.yml`](.github/workflows/manual-binaries.yml) — `cross` build for `x86_64-unknown-linux-musl` (same pattern as rustdesk-server manual binaries).
- Linux startup: [`scripts/rustdesk-server-admin.sh`](scripts/rustdesk-server-admin.sh), [`scripts/rustdesk-server-admin.env.example`](scripts/rustdesk-server-admin.env.example), [`scripts/rustdesk-server-admin.service`](scripts/rustdesk-server-admin.service).

### Notes

- **Online / last seen:** Stock hbbs does not persist presence in SQLite; the UI documents this limitation.
- **Dependencies:** `libsqlite3-sys` with `bundled` for portable musl binaries; `urlencoding` for connect URLs.

[0.1.0]: https://github.com/tdimitriou/rustdesk-server-admin/commits/master
[Unreleased]: https://github.com/tdimitriou/rustdesk-server-admin/compare/master...HEAD
