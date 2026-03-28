# rustdesk-server-admin

Web admin dashboard for a **self-hosted** RustDesk signal/relay stack (`hbbs` / `hbbr`).  
This repo is separate from [rustdesk](https://github.com/rustdesk/rustdesk) and [rustdesk-server](https://github.com/rustdesk/rustdesk-server).

## Run (dev)

```bash
cargo run
```

Optional bind address:

```bash
set BIND_ADDR=0.0.0.0:3030
cargo run
```

Then open `http://127.0.0.1:3030/` — you should see a short stub response.

## Status

Early scaffold: HTTP server only. Auth, DB/API integration, and UI layers come next.
