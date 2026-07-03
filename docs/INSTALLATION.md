# Installation

This guide is for people installing ConvexAutoBackup without needing to understand the repository internals.

## Pick An Install Path

| Path | Best for | Notes |
| --- | --- | --- |
| Native installer | Local desktop, small server, or VM installs | Downloads release bundle, verifies checksums, generates a master key, provisions the managed Convex runner, and can configure autostart. |
| Docker installer | VPS, NAS, homelab, and containerized server installs | Starts the supervised service with persistent `/data`. |
| Manual Docker | Operators who already manage Docker services | Requires you to provide env and volumes. |
| crates.io | Rust users and scripted CLI installs | Installs the binary, but does not create OS services or managed config. |
| Source checkout | Contributors and local development | Requires Rust, Node.js, npm, and make. |

## Native Beta Install

macOS and Linux:

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.4/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.4/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The native installers:

- Download the platform release bundle.
- Verify `SHA256SUMS`.
- Generate `CONVEX_AUTOBACKUP_MASTER_KEY`.
- Install the pinned Convex CLI runner.
- Configure autostart by default.

Opt out of autostart:

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.4/install.sh | sh -s -- --no-autostart
```

Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -NoAutostart
```

Open:

```text
http://localhost:8976
```

## Docker Install Script

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.4/docker-setup.sh | sh
```

The Docker setup script writes a persistent install directory, generates a master key, starts the supervised service, and checks health.

## Manual Docker

Docker Hub:

```bash
docker pull kodydoty/convex-autobackup:v0.1.0-beta.4
docker run -d \
  --name convex-autobackup \
  --restart unless-stopped \
  -p 8976:8976 \
  -v convex-autobackup-data:/data \
  -e CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" \
  kodydoty/convex-autobackup:v0.1.0-beta.4
```

GHCR:

```bash
docker pull ghcr.io/kodydennon/convex-autobackup:v0.1.0-beta.4
docker run -d \
  --name convex-autobackup \
  --restart unless-stopped \
  -p 8976:8976 \
  -v convex-autobackup-data:/data \
  -e CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" \
  ghcr.io/kodydennon/convex-autobackup:v0.1.0-beta.4
```

## Cargo Install

```bash
cargo install convex-autobackup --version 0.1.0-beta.4
convex-autobackup runner install --json
convex-autobackup supervise
```

Cargo is useful when you already manage service files and environment variables yourself.

## Source Install

```bash
git clone https://github.com/KodyDennon/ConvexAutoBackup.git
cd ConvexAutoBackup
make setup
make check
cargo run -p convex-autobackup -- supervise
```

## First Run Checklist

1. Open `http://localhost:8976`.
2. Create the first owner account.
3. Store or reference a Convex deploy key.
4. Create a project.
5. Create a Convex Cloud target.
6. Create a local or S3-compatible destination.
7. Create a backup job.
8. Run a manual backup.
9. Verify the backup.
10. Create a schedule after the first manual backup works.

## Public Exposure

The service defaults to `0.0.0.0:8976` for LAN/server installs. For production, put it behind HTTPS and firewall direct access to the app port.
