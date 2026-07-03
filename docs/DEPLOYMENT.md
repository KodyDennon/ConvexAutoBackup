# Deployment

For install commands and first-run setup, see [Installation](INSTALLATION.md). For ongoing backup, restore, and upgrade procedures, see [Operations](OPERATIONS.md).

## Local Native

Normal beta install:

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.3/install.sh | sh
```

Windows:

```powershell
iwr https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.3/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The installer provisions the pinned Convex CLI runner, generates `CONVEX_AUTOBACKUP_MASTER_KEY`, and installs an autostart service by default. Use `--no-autostart` or `-NoAutostart` to opt out.

Windows releases also include an MSI that installs the binaries. The PowerShell installer remains the full setup path because it provisions the runner, generated env file, and optional Windows Service.

Source development:

```bash
make setup
cargo run -p convex-autobackup -- supervise
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

Normal beta install:

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.3/docker-setup.sh | sh
```

Manual source build:

```bash
CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" \
docker compose up --build
```

The Compose file exposes port `8976` and persists `/data` in a named volume.
The container starts the supervised web service and scheduler worker in one process.

Published images:

```text
ghcr.io/kodydennon/convex-autobackup
kodydoty/convex-autobackup
```

## Cargo Install

Rust users can install the CLI from crates.io:

```bash
cargo install convex-autobackup --version 0.1.0-beta.3
convex-autobackup runner install --json
convex-autobackup supervise
```

This path does not install OS services or generate a managed env file. Use the native or Docker installer for the lowest-friction setup.

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

Minimal nginx shape:

```nginx
server {
  listen 443 ssl http2;
  server_name backups.example.com;

  location / {
    proxy_pass http://127.0.0.1:8976;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto https;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  }
}
```

Firewall direct access to port `8976` when using a reverse proxy.

## Database Modes

SQLite is the default and only active database mode in the beta. Postgres remains a roadmap item for larger server deployments.

## Storage Modes

Local filesystem destinations require durable mounted storage. S3-compatible destinations require bucket, endpoint, region when needed, prefix, and credential secret reference.

## Backups Of ConvexAutoBackup Itself

Operators should back up:

- App database.
- Secret backend recovery material.
- Local backup destination root, if used.
- Configuration file.

Losing the encrypted secret-store key can make encrypted destination credentials or encrypted backup archives unrecoverable.

## Install Integrity

Release installers verify `SHA256SUMS` before unpacking native bundles. GitHub Release artifacts also publish provenance attestations where GitHub Actions supports them. Artifacts are unsigned; paid platform code signing is intentionally not required.

## Release Channels

Native bundles are published to GitHub Releases. Container images are published to GHCR and Docker Hub. Rust crates are published to crates.io. There is no separate npm package for normal users because the web UI is embedded in the Rust server.
