# rustdesk-server-admin

Web admin dashboard for a **self-hosted** RustDesk signal/relay stack (`hbbs` / `hbbr`).  
This repo is separate from [rustdesk](https://github.com/rustdesk/rustdesk) and [rustdesk-server](https://github.com/rustdesk/rustdesk-server).

## Features

- Password login (`ADMIN_PASSWORD`), session cookie signed with `ADMIN_SESSION_SECRET` (or a derived key if unset — see below).
- **Dashboard** with total peer count.
- **Peer list** (`/peers`): all columns from the hbbs `peer` table, **search** (ID, **UUID as hex**, note, info JSON), **`rustdesk://` connect links** (Windows / Android when the app registered the URL scheme).
- **Delete peer** and **rename registration ID** (SQL `UPDATE` on `id`; client identifies by `uuid` blob — same idea as changing ID on the device).
- Plain HTML UI (no SPA). Intended to sit **behind** Apache or nginx (TLS, access control, rate limits) in production.

### Online / last seen

Stock **hbbs does not store “who is online” or “last seen offline” in SQLite**. Online checks use **in-memory** registration (~30s window). The UI shows **`status`**, **`created_at`**, **`info`** (JSON, often last IP from hbbs), and **`note`** — treat them as hints, not authoritative presence.

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ADMIN_PASSWORD` | **Yes** | — | Password for the admin UI. |
| `ADMIN_HOST` | No | `127.0.0.1` | Address to bind (use `0.0.0.0` only if you know what you are doing). |
| `ADMIN_PORT` | No | `3030` | TCP port (change if `3030` is already in use). |
| `HBBS_DB_PATH` | No | — | Absolute path to hbbs `db_v2.sqlite3` (or your configured DB file). Required for peer list and DB writes. |
| `ADMIN_SESSION_SECRET` | No | — | Secret key for signing session cookies (any long random string). **Set this in production.** If omitted, a weak key is derived from `ADMIN_PASSWORD` and a warning is logged. |
| `RUSTDESK_CONNECT_RENDEZVOUS` | No | — | Public **hbbs / rendezvous host** for client links, e.g. `rustdesk.example.com` or `rustdesk.example.com:21116`. If set, connect URLs use `rustdesk://<id>/r@<host>` so clients use your server. If unset, links are `rustdesk://<id>` (default public relay behaviour in the client). |

## Run (development)

```bash
set ADMIN_PASSWORD=your-secret
set HBBS_DB_PATH=C:\path\to\db_v2.sqlite3
set RUSTDESK_CONNECT_RENDEZVOUS=your.hbbs.host
cargo run
```

On Unix:

```bash
export ADMIN_PASSWORD=your-secret
export HBBS_DB_PATH=/var/lib/rustdesk-server/db_v2.sqlite3
export RUSTDESK_CONNECT_RENDEZVOUS=your.hbbs.host
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
export RUSTDESK_CONNECT_RENDEZVOUS=your.hbbs.host
./rustdesk-server-admin
```

Terminate TLS and route a vhost path or subdomain to `http://127.0.0.1:3031` with nginx or Apache. The app does not set the `Secure` cookie flag so HTTP on localhost works in development; over HTTPS the session cookie still works — for stricter cookie policy you can extend the code to set `Secure` when behind HTTPS.

## Startup script (Linux)

The repo includes:

| File | Purpose |
|------|---------|
| [`scripts/rustdesk-server-admin.sh`](scripts/rustdesk-server-admin.sh) | Sources an env file, checks `ADMIN_PASSWORD`, finds the binary, `exec`s it. |
| [`scripts/rustdesk-server-admin.env.example`](scripts/rustdesk-server-admin.env.example) | Copy to `/etc/rustdesk-server-admin.env` or `scripts/rustdesk-server-admin.env` (`chmod 600`). Use `KEY=value` lines (no `export` needed). |
| [`scripts/rustdesk-server-admin.service`](scripts/rustdesk-server-admin.service) | Example **systemd** unit using `EnvironmentFile=` and the binary on `/opt/rustdesk/`. |

Quick manual start:

```bash
chmod +x scripts/rustdesk-server-admin.sh
cp scripts/rustdesk-server-admin.env.example /etc/rustdesk-server-admin.env
# edit /etc/rustdesk-server-admin.env — set ADMIN_PASSWORD, HBBS_DB_PATH, etc.
sudo install -m 755 rustdesk-server-admin /opt/rustdesk/   # your built binary
sudo RUSTDESK_SERVER_ADMIN_BIN=/opt/rustdesk/rustdesk-server-admin scripts/rustdesk-server-admin.sh
```

The shell script looks for an env file in order: `RUSTDESK_SERVER_ADMIN_ENV` (if set), then `/etc/rustdesk-server-admin.env`, then `scripts/rustdesk-server-admin.env`. It looks for the binary in `RUSTDESK_SERVER_ADMIN_BIN`, then next to the script, `target/release/`, `/opt/rustdesk/`, `/usr/local/bin/`.

## Security notes

- **SQLite locking:** hbbs keeps the database open. Reads usually work; **writes** (delete / rename) can return `SQLITE_BUSY`. Use **WAL** mode on the DB if needed, retry, or briefly stop hbbs for maintenance operations.
- **Password:** Use a strong `ADMIN_PASSWORD` and rely on the reverse proxy for TLS and optional IP allowlists.
- **Destructive actions:** Anyone who can log in can **delete peers** or **change IDs** — protect the admin URL accordingly.

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
