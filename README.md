# ConvexAutoBackup

[![CI](https://github.com/KodyDennon/ConvexAutoBackup/actions/workflows/ci.yml/badge.svg)](https://github.com/KodyDennon/ConvexAutoBackup/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/KodyDennon/ConvexAutoBackup?include_prereleases&label=release)](https://github.com/KodyDennon/ConvexAutoBackup/releases)
[![crates.io](https://img.shields.io/badge/crates.io-v0.1.0--beta.7-orange)](https://crates.io/crates/convex-autobackup)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Docker](https://img.shields.io/badge/Docker-Hub%20%2B%20GHCR-2496ed)](https://hub.docker.com/r/kodydoty/convex-autobackup)
[![Rust](https://img.shields.io/badge/Rust-core-b7410e)](Cargo.toml)
[![React](https://img.shields.io/badge/React-console-226b52)](web/package.json)

ConvexAutoBackup is an open-source, self-hosted backup and disaster-recovery control plane for Convex projects.

It is designed for developers, teams, agencies, and AI agents that need reliable full exports of Convex deployments on a schedule, encrypted with a passphrase, copied to local storage and offsite object storage (Cloudflare R2, AWS S3, MinIO, …), with clear restore workflows and audit-friendly DR evidence.

Highlights:

- **Setup wizard** — “Add project” walks through server checks, a live deploy-key test, storage, encryption, schedule and a verified first backup.
- **Encrypted backups** — [age](https://age-encryption.org) passphrase encryption; archives are standard `.zip.age` files you can decrypt without this app.
- **Local + offsite in one run** — one export is written to every destination of a job (for example a local folder and Cloudflare R2), each tracked and verified separately.
- **Automatic retention** — keeps the newest N backups per destination, locally and in S3/R2.
- **Guarded restore** — verifies and decrypts before import, and requires typing the deployment name and a confirmation phrase.

## What You Get

| Install path | Status | Use it for |
| --- | --- | --- |
| Native macOS/Linux | Beta | Desktop or small server installs with autostart |
| Windows MSI/PowerShell | Beta | Windows users who want binaries plus guided setup |
| Docker Compose | Beta | VPS, NAS, homelab, and containerized installs |
| crates.io | Beta | Rust users who prefer `cargo install` |
| Source checkout | Developer | Local development and contribution |

Self-hosted only. There is no hosted SaaS. The native service binds to `0.0.0.0:8976` for LAN/server installs, so expose it to the public internet only behind HTTPS and an access gate. The bundled Docker Compose file publishes the app on host loopback only and can add a Cloudflare Tunnel (see [Deployment](docs/DEPLOYMENT.md)).

## Implemented Now

The current implementation includes:

- Rust workspace with `core`, `server`, `cli`, `worker`, and `mcp` crates.
- SQLite app database with migrations for users, API tokens, encrypted secrets, teams, projects, targets, destinations, jobs, schedules, and runs.
- CLI commands to initialize state, run the supervised service, install the pinned Convex runner, validate installs with `doctor`, create users/tokens/secrets/projects/targets/destinations/jobs/schedules, run due schedules, run backups, verify backups, restore backups, and list runs.
- HTTP API endpoints for bootstrap owner creation, email/password login, user management, token listing/revocation, encrypted secrets, projects, Convex Cloud targets, local and S3-compatible destinations, jobs, schedules, backup execution, verification, restore, DR reports, audit, and runs.
- Backup engine that resolves encrypted deploy-key secrets or env references, runs the Convex export command, writes local archives atomically or uploads S3-compatible objects, writes manifests, and records success/failure runs.
- Guarded restore engine that verifies the backup and requires exact deployment confirmation before invoking Convex import.
- DR evidence report generation from persisted run history.
- Audit log records for users, tokens, secrets, projects, targets, destinations, jobs, schedules, backup runs, and restore operations.
- Retention pruning by `keep_last` for local folders and S3/R2 buckets, applied after each successful copy.
- Passphrase encryption (age/scrypt) per destination, manifest schema v2 with encryption metadata and plaintext/stored checksums, and offline recovery with `convex-autobackup decrypt` or `age -d`.
- Multi-destination jobs: one export, one encrypted copy per destination, per-copy status (`partial` when a copy fails).
- Verify and restore from local or S3/R2 copies, with automatic fallback to the next readable copy.
- Setup wizard API: read-only deploy key validation, install checks, and a server-provided Cloudflare R2 preset.
- Supervised web service plus scheduler worker for persisted schedules.
- Managed pinned Convex CLI runner provisioned by normal installers.
- Tested auth, encrypted secrets, scheduling, manifest, path-safety, local storage, SQLite, backup, verification, restore, server health/auth, and worker policy logic.
- React web console served by the Rust service for onboarding, login, setup, backup runs, verification, guarded restore, DR reports, audit review, user management, and API token management.
- Hardened Dockerfile (non-root, read-only root filesystem, healthcheck, baked-in Convex CLI), Compose file with optional Cloudflare Tunnel sidecar, Cloudflare provisioning script, native install scripts, Windows MSI packaging, CI, release automation, Dependabot, Makefile, editor config, and environment example.
- Full project documentation in `docs/`.
- Public open-source metadata, issue templates, PR template, CODEOWNERS, security policy, and release checklist.

## Quick Start

### Native beta install

macOS and Linux:

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.7/install.sh | sh
```

Windows PowerShell:

```powershell
iwr https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.7/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The native installers download the matching release bundle, verify checksums, generate `CONVEX_AUTOBACKUP_MASTER_KEY`, install the pinned Convex CLI runner, and configure autostart by default. Use `--no-autostart` on macOS/Linux or `-NoAutostart` on Windows to opt out. Windows releases also include a basic MSI for users who prefer installing binaries through a package.

Open:

```text
http://localhost:8976
```

### Docker beta install

```bash
curl -fsSL https://github.com/KodyDennon/ConvexAutoBackup/releases/download/v0.1.0-beta.7/docker-setup.sh | sh
```

The Docker setup script writes a persistent install directory, generates a master key, starts the supervised service, and checks health. Images are published to:

```text
ghcr.io/kodydennon/convex-autobackup
kodydoty/convex-autobackup
```

### Cargo install

```bash
cargo install convex-autobackup --version 0.1.0-beta.7
convex-autobackup runner install --json
convex-autobackup supervise
```

The `cargo install` path is useful for Rust users. It does not create OS services, generate a master key, or write installer-managed config, so native/Docker installers remain the lower-friction path.

### Source development

```bash
make setup
make check
cargo run -p convex-autobackup -- supervise
```

Open:

```text
http://localhost:8976
```

The default bind is `0.0.0.0:8976` so LAN access works in server installs. Normal product access requires first-run admin setup before real deployments and secrets can be managed. Public internet deployments should run behind HTTPS.

## CLI Backup Flow

```bash
convex-autobackup init --json
convex-autobackup user create --email owner@example.com --password '<12+ chars>' --role owner --json
convex-autobackup project create --name "Client App" --json
convex-autobackup destination create-local --name "Local Vault" --root ./backups --json
CONVEX_AUTOBACKUP_MASTER_KEY=<master> convex-autobackup secret put \
  --label "Production deploy key" \
  --kind convex_deploy_key \
  --value "<deploy-key>" \
  --json
convex-autobackup target create-cloud \
  --project-id <project-id> \
  --name Production \
  --deployment <deployment-name> \
  --deploy-key-secret-id <secret-id> \
  --json
convex-autobackup job create \
  --project-id <project-id> \
  --target-id <target-id> \
  --destination-id <destination-id> \
  --also-destination-id <offsite-destination-id> \
  --name "Manual full backup" \
  --json
CONVEX_AUTOBACKUP_MASTER_KEY=<master> convex-autobackup backup run --job-id <job-id> --json
convex-autobackup verify --run-id <run-id> --json
CONVEX_AUTOBACKUP_MASTER_KEY=<master> convex-autobackup restore \
  --run-id <run-id> \
  --target-id <target-id> \
  --confirm-deployment <deployment-name> \
  --json
convex-autobackup runs --json
convex-autobackup dr-report --json
convex-autobackup audit --json
```

Deploy keys can be stored as encrypted secrets (`secret put` also reads the value from `CONVEX_AUTOBACKUP_SECRET_VALUE`, keeping it out of process listings). Environment-variable references remain supported for local automation.

Most people never need these commands: the web console's **Add project** wizard does all of the above, plus encryption and a schedule, and finishes with a verified backup.

### Recovering without the app

If the server is gone, download a `.zip.age` archive from your bucket and decrypt it with your passphrase:

```bash
age -d backup.zip.age > backup.zip
# or
CONVEX_AUTOBACKUP_PASSPHRASE='<passphrase>' convex-autobackup decrypt backup.zip.age --out backup.zip
npx convex import --replace backup.zip   # into the deployment you choose
```

The HTTP bootstrap endpoint creates the first owner and returns a one-time bootstrap API token so headless setups can continue configuration through the API.

## Docker

```bash
CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" docker compose up -d --build
```

The service listens on host loopback:

```text
http://127.0.0.1:8976
```

Application data lives in the `data` volume (`/data`). The image runs as a non-root user with a read-only root filesystem and ships the pinned Convex CLI. For remote access, add a Cloudflare Tunnel with `docker compose --profile tunnel up -d` and protect the hostname with Cloudflare Access; `scripts/cloudflare-provision.py` can create the tunnel, DNS record, Access app and an R2 bucket for you. Update with `scripts/docker-update.sh`. See [Deployment](docs/DEPLOYMENT.md).

## Repository Layout

```text
crates/core     Shared domain models, scheduling, manifests, storage-safe paths
crates/server   HTTP API and web UI host
crates/cli      Agent-friendly CLI
crates/worker   Backup queue and worker process logic
crates/mcp      MCP stdio server
web             React/TypeScript browser UI
docs            Product, architecture, security, DR, API, CLI, MCP, deployment docs
```

## Documentation

- [Installation](docs/INSTALLATION.md)
- [Deployment](docs/DEPLOYMENT.md)
- [Operations](docs/OPERATIONS.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Security model](docs/SECURITY_MODEL.md)
- [Backup model](docs/BACKUP_MODEL.md)
- [Disaster recovery model](docs/DR_MODEL.md)
- [CLI contract](docs/CLI_CONTRACT.md)
- [HTTP API contract](docs/API_CONTRACT.md)
- [MCP contract](docs/MCP_CONTRACT.md)
- [Testing](docs/TESTING.md)
- [Dependency audit](docs/DEPENDENCY_AUDIT.md)
- [GitHub repository setup](docs/GITHUB_REPOSITORY.md)
- [Open-source release checklist](docs/OPEN_SOURCE_RELEASE.md)

## Development Commands

```bash
make setup       # Install web deps and fetch Rust deps
make build       # Build web and Rust workspace
make test        # Run Rust and web tests
make check       # Source-size guard, web build, fmt, clippy, and tests
make serve       # Start the supervised local service
convex-autobackup doctor --json
convex-autobackup runner install --json
```

## Public Repository

Canonical repository:

```text
https://github.com/KodyDennon/ConvexAutoBackup
```

Before publishing a release or changing visibility, run the checklist in `docs/OPEN_SOURCE_RELEASE.md`.

## License

Apache-2.0. See `LICENSE`.
