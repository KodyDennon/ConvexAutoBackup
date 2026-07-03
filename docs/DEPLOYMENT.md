# Deployment

## Local Native

```bash
make setup
cargo run -p convex-autobackup -- serve
```

Default bind:

```text
0.0.0.0:8976
```

Open:

```text
http://localhost:8976
```

## Docker Compose

```bash
CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" \
docker compose up --build
```

The Compose file exposes port `8976` and persists `/data` in a named volume.
It also starts a dedicated scheduler worker that polls persisted schedules every 30 seconds.

Run a single scheduler pass manually:

```bash
convex-autobackup-worker --data-dir /data run-once --json
```

Run the scheduler loop directly:

```bash
convex-autobackup-worker --data-dir /data run --poll-seconds 30
```

## Reverse Proxy

Production installs should place the service behind a reverse proxy that provides HTTPS. The app must support forwarded headers and secure cookie settings before public internet exposure.

## Database Modes

SQLite is the default. Postgres is selected through configuration for multi-user or larger server deployments.

## Storage Modes

Local filesystem destinations require durable mounted storage. S3-compatible destinations require bucket, endpoint, region when needed, prefix, and credential secret reference.

## Backups Of ConvexAutoBackup Itself

Operators should back up:

- App database.
- Secret backend recovery material.
- Local backup destination root, if used.
- Configuration file.

Losing the encrypted secret-store key can make encrypted destination credentials or encrypted backup archives unrecoverable.
