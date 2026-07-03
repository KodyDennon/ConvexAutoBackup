# ConvexAutoBackup

ConvexAutoBackup is an open-source, self-hosted backup and disaster-recovery control plane for Convex projects.

It is designed for developers, teams, agencies, and AI agents that need reliable full exports of Convex deployments on a schedule, with local filesystem or S3-compatible storage, clear restore workflows, and audit-friendly DR evidence.

## Product Direction

- Self-hosted only. This project does not operate a hosted SaaS.
- Rust core for service logic, workers, CLI, MCP, scheduling, storage, and DR workflows.
- React/TypeScript web UI served by the Rust service.
- Local/LAN browser UI as the canonical GUI.
- Docker and unsigned native binaries.
- SQLite is implemented now; Postgres is part of the scale-out roadmap.
- Local filesystem and S3-compatible storage are both first-class.
- Convex Cloud and self-hosted Convex targets are both first-class.
- Full backups include Convex file storage by default.
- CLI, HTTP API, and MCP surfaces are designed for automation and AI agents.

## Implemented Now

The current implementation includes:

- Rust workspace with `core`, `server`, `cli`, `worker`, and `mcp` crates.
- SQLite app database with migrations for users, API tokens, encrypted secrets, teams, projects, targets, destinations, jobs, schedules, and runs.
- CLI commands to initialize state, create users/tokens/secrets/projects/targets/destinations/jobs/schedules, run due schedules, run backups, verify backups, restore backups, and list runs.
- HTTP API endpoints for bootstrap owner creation, email/password login, user management, token listing/revocation, encrypted secrets, projects, Convex Cloud targets, local and S3-compatible destinations, jobs, schedules, backup execution, verification, restore, DR reports, audit, and runs.
- Backup engine that resolves encrypted deploy-key secrets or env references, runs the Convex export command, writes local archives atomically or uploads S3-compatible objects, writes manifests, and records success/failure runs.
- Guarded restore engine that verifies the backup and requires exact deployment confirmation before invoking Convex import.
- DR evidence report generation from persisted run history.
- Audit log records for users, tokens, secrets, projects, targets, destinations, jobs, schedules, backup runs, and restore operations.
- Local retention pruning by `keep_last`.
- Dedicated scheduler worker process for persisted schedules.
- Tested auth, encrypted secrets, scheduling, manifest, path-safety, local storage, SQLite, backup, verification, restore, server health/auth, and worker policy logic.
- React web console served by the Rust service for onboarding, login, setup, backup runs, verification, guarded restore, DR reports, audit review, user management, and API token management.
- Dockerfile, Compose file, CI, Dependabot, Makefile, editor config, and environment example.
- Full project documentation in `docs/`.
- Public open-source metadata, issue templates, PR template, CODEOWNERS, security policy, and release checklist.

## Quick Start

```bash
make setup
make check
cargo run -p convex-autobackup -- serve
```

Open:

```text
http://localhost:8976
```

The default bind is `0.0.0.0:8976` so LAN access works in server installs. Normal product access must require first-run admin setup before real deployments and secrets can be managed.

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

Deploy keys can be stored as encrypted secrets. Environment-variable references remain supported for local automation.

The HTTP bootstrap endpoint creates the first owner and returns a one-time bootstrap API token so headless setups can continue configuration through the API.

## Docker

```bash
CONVEX_AUTOBACKUP_MASTER_KEY="$(openssl rand -base64 32)" docker compose up --build
```

The service listens on:

```text
http://localhost:8976
```

Persist application data by mounting the `/data` volume. Production deployments should run behind a reverse proxy with HTTPS.

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

## Development Commands

```bash
make setup       # Install web deps and fetch Rust deps
make build       # Build web and Rust workspace
make test        # Run Rust and web tests
make check       # Source-size guard, web build, fmt, clippy, and tests
make serve       # Start the local service
```

## Public Repository

Canonical repository:

```text
https://github.com/KodyDennon/ConvexAutoBackup
```

Before publishing a release or changing visibility, run the checklist in `docs/OPEN_SOURCE_RELEASE.md`.

## License

Apache-2.0. See `LICENSE`.
