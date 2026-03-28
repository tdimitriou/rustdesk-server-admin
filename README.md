# rustdesk-server-admin

Web admin dashboard for a **self-hosted** RustDesk signal/relay stack (`hbbs` / `hbbr`).  
This repo is separate from [rustdesk](https://github.com/rustdesk/rustdesk) and [rustdesk-server](https://github.com/rustdesk/rustdesk-server).

## Features (v1)

- Password login (`ADMIN_PASSWORD`), session cookie signed with `ADMIN_SESSION_SECRET` (or a derived key if unset — see below).
- Read-only access to the hbbs SQLite database: peer count from the `peer` table.
- Plain HTML UI (no SPA). Intended to sit **behind** Apache or nginx (TLS, access control, rate limits) in production.

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ADMIN_PASSWORD` | **Yes** | — | Password for the admin UI. |
| `ADMIN_HOST` | No | `127.0.0.1` | Address to bind (use `0.0.0.0` only if you know what you are doing). |
| `ADMIN_PORT` | No | `3030` | TCP port (change if `3030` is already in use). |
| `HBBS_DB_PATH` | No | — | Absolute path to hbbs `db_v2.sqlite3` (or your configured DB file). If unset, the dashboard still loads but peer statistics are omitted. |
| `ADMIN_SESSION_SECRET` | No | — | Secret key for signing session cookies (any long random string). **Set this in production.** If omitted, a weak key is derived from `ADMIN_PASSWORD` and a warning is logged. |

## Run (development)

```bash
set ADMIN_PASSWORD=your-secret
set HBBS_DB_PATH=C:\path\to\db_v2.sqlite3
cargo run
```

On Unix:

```bash
export ADMIN_PASSWORD=your-secret
export HBBS_DB_PATH=/var/lib/rustdesk-server/db_v2.sqlite3
cargo run
```

Open `http://127.0.0.1:3030/` — you are redirected to `/login`, then to `/dashboard` after sign-in.

## Production (reverse proxy)

Bind the app to loopback and choose a free port, for example:

```bash
export ADMIN_HOST=127.0.0.1
export ADMIN_PORT=3031
export ADMIN_PASSWORD=...
export ADMIN_SESSION_SECRET=...   # long random value
export HBBS_DB_PATH=/path/to/db_v2.sqlite3
./rustdesk-server-admin
```

Terminate TLS and route a vhost path or subdomain to `http://127.0.0.1:3031` with nginx or Apache. The app does not set the `Secure` cookie flag so HTTP on localhost works in development; over HTTPS the session cookie still works — for stricter cookie policy you can extend the code to set `Secure` when behind HTTPS.

## Security notes

- **SQLite locking:** hbbs keeps the database open. Opening the same file read-only from this process is usually fine on SQLite, but avoid heavy concurrent access; if you see intermittent errors, retry or point a replica/snapshot at the admin tool instead.
- **Password:** Use a strong `ADMIN_PASSWORD` and rely on the reverse proxy for TLS and optional IP allowlists.
- **Scope:** v1 is read-only; it does not modify peers or server configuration.

## Build

```bash
cargo build --release
```

Binary: `target/release/rustdesk-server-admin` (or `.exe` on Windows).

## GitHub Actions (Linux amd64 binary)

Workflow [`.github/workflows/manual-binaries.yml`](.github/workflows/manual-binaries.yml) follows the same approach as [rustdesk-server’s manual binaries workflow](https://github.com/rustdesk/rustdesk-server/blob/master/.github/workflows/manual-binaries.yml): **Ubuntu 22.04**, **`cross`**, and target **`x86_64-unknown-linux-musl`**. The artifact is **`rustdesk-server-admin-linux-amd64-musl.tar.gz`** (statically linked musl binary, suitable for Rocky/RHEL and most Linux amd64 hosts).

- **Manual run:** Actions → *Manual Server Admin Binary Build* → *Run workflow*.
- **Optional:** enable publishing to a GitHub release and set the tag (default `admin-nightly`).

SQLite is built via **`libsqlite3-sys` / `bundled`** so the musl cross build does not rely on the image’s system SQLite dev packages.
