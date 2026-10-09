# Roadmap

## Phase 0: Foundation

- Rust workspace.
- React web shell.
- CLI, worker, server, MCP entrypoints.
- CI, Docker, Compose, docs, and test infrastructure.
- Tested core schedule, manifest, path safety, and queue policy logic.

## Phase 1: Local Backup MVP

- First-run admin setup.
- SQLite metadata store.
- Convex Cloud target setup.
- Local filesystem destination setup.
- Manual backup run.
- Manifest persistence.
- Run logs.

## Phase 2: Scheduler And Queue

- Interval schedules.
- Daily and weekly schedules.
- Guided cron.
- Worker queue persistence.
- Retry and backoff.
- Missed-run handling.
- Retention execution.

## Phase 3: Teams, Secrets, And API

- Users and roles.
- API tokens.
- Secret references.
- OS keychain backend.
- Encrypted database secret backend.
- Audit log.
- Versioned HTTP API expansion.

## Phase 4: Agent Surfaces

- Full CLI command groups.
- Stable JSON responses.
- MCP tools for safe read/action workflows.
- OpenAPI contract coverage.

## Phase 5: Storage And Encryption (done in 0.1.0-beta.7)

- S3-compatible destination, including Cloudflare R2.
- Per-destination passphrase encryption (age).
- Checksum verification from destination, including decryption.
- Retention over object storage.
- Multi-destination jobs (local + offsite from one export).

## Phase 6: DR And Compliance

- Restore-to-alternate.
- Restore drills.
- Guarded destructive restore.
- RPO/RTO tracking.
- Evidence report exports.

## Phase 7: Packaging And Hardening

- Unsigned native release bundles.
- Docker release image.
- Native install scripts and Windows PowerShell installer.
- Supervised server plus worker runtime.
- Managed pinned Convex CLI runner.
- Install doctor checks.
- Upgrade and migration tests.
- Reverse proxy hardening.
- Load and concurrency tests.
