# Architecture

## System Shape

ConvexAutoBackup is a Rust service with a bundled web UI and separate automation surfaces.

```text
React UI -> Rust HTTP API -> App DB
                         -> Secret backend
                         -> Worker queue
                         -> Convex runner
                         -> Storage destinations

CLI -> HTTP API or local command execution
MCP -> HTTP API or local read/action tools
```

## Crates

- `convex-autobackup-core`: shared domain types, schedules, manifests, path safety, storage contracts, encryption contracts, and policy logic.
- `convex-autobackup-server`: HTTP API, GUI hosting, auth/session handling, OpenAPI surface, and app orchestration.
- `convex-autobackup-worker`: queued backup, verify, retention, restore, and DR job execution.
- `convex-autobackup`: user and agent CLI.
- `convex-autobackup-mcp`: MCP stdio server for agent integrations.

## Implemented Data Store

SQLite is the implemented app database. It is simple, local, portable, and easy to back up. The schema currently stores users, API tokens, encrypted secrets, teams, projects, targets, destinations, jobs, schedules, and backup runs.

Postgres remains a scale-out target. It is not active in the current code path.

## Current Secret Handling

Convex deploy keys can be stored as encrypted database secrets. The encrypted secret store uses `CONVEX_AUTOBACKUP_MASTER_KEY` to derive an AES-256-GCM key. During backup execution the worker decrypts the deploy key and passes it to the Convex export process as `CONVEX_DEPLOY_KEY`.

Environment-variable references are still supported for automation and migration. OS keychain storage remains a native-app target.

## Worker Queue

All backup and restore operations run through a queue. The queue enforces:

- Global concurrency limit.
- Per-target concurrency limit.
- Per-destination concurrency limit.
- Retry policy with backoff.
- Terminal states for succeeded, failed, canceled, partial, and verification-failed runs.

## Convex Runner

The Convex runner executes the configured Convex command. Normal installs provision a pinned `convex` npm package into the app-managed runner directory and execute that binary directly with the selected deployment, output path, and file-storage flag. `CONVEX_AUTOBACKUP_CONVEX_BIN` can override the runner path for controlled environments. Source checkouts can still fall back to `npx convex` for development.

## Implemented Storage

Local filesystem storage and S3-compatible storage are implemented. Local writes use a staging file and atomic rename. S3-compatible writes use the Rust `object_store` S3 backend and upload both archive and manifest objects.

The storage contract requires:

- Write archive atomically or with upload completion verification.
- Write manifest after archive success.
- Read archive and manifest for verification and restore.
- List backups for retention and DR discovery.
- Delete only through retention or explicit operator action.

## UI

The React UI is a local/LAN operations console. It consumes the same HTTP API used by agents and must not contain privileged business logic that bypasses server authorization. It implements first-run owner creation, login, resource setup, backup execution, verification, guarded restore, DR reporting, audit review, user management, and API token management.

## Agent Surfaces

The CLI, HTTP API, and bundled UI are wired to the same SQLite, auth, secret, scheduler, backup, verification, and restore logic. The MCP server currently exposes health and capabilities tools. API bearer-token auth is active for automation and browser-created tokens.
