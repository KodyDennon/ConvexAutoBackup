# Milestones

## Milestone 0: Foundation

Acceptance:

- Repo initializes cleanly.
- `make setup` installs dependencies.
- `make check` passes.
- Docker image builds.
- Docs cover product, architecture, security, backup, DR, CLI, API, MCP, deployment, roadmap, and contribution.
- SQLite-backed projects, targets, local destinations, jobs, backup runs, and manifests are implemented.
- CLI and HTTP API use the same real backup engine.

## Milestone 1: Local Backup MVP

Acceptance:

- First admin can be created.
- Convex Cloud target can be added with a deploy key reference.
- Local filesystem destination can be added.
- Manual full backup can run with file storage included.
- Manifest and run logs are persisted.
- Backup archive checksum can be verified.

## Milestone 2: Scheduling

Acceptance:

- Interval, daily, weekly, and guided cron schedules can be created.
- Due jobs enter the queue.
- Missed runs are recorded.
- Catch-up behavior is configurable.
- Queue limits prevent parallel runs against the same target or destination.

## Milestone 3: Teams And Secrets

Acceptance:

- Users and roles enforce server-side authorization.
- API tokens can be created, listed, authenticated, and revoked.
- OS keychain backend works where available.
- Encrypted database secret backend works in Docker/headless mode.
- Audit log records security-relevant actions.

## Milestone 4: Agent Interfaces

Acceptance:

- CLI exposes operational commands with JSON output.
- HTTP API covers the same state used by the UI.
- MCP exposes safe tools for health, status, listing, backup trigger, verification, and reports.
- Destructive restore is not available through MCP by default.

## Milestone 5: Storage And Encryption

Acceptance:

- S3-compatible destination works against MinIO and at least one cloud provider.
- Per-destination encryption can be enabled.
- Encrypted backups can be restored when key material is available.
- Retention works for local and S3-compatible destinations.

## Milestone 6: DR Suite

Acceptance:

- Restore-to-alternate works.
- Restore drills produce evidence.
- Destructive restore requires strong confirmation.
- RPO/RTO reports are exportable.

## Milestone 7: Release

Acceptance:

- Docker image builds and runs.
- Native unsigned binaries build for supported platforms.
- Fresh install smoke tests pass.
- Upgrade tests protect app database migrations.
