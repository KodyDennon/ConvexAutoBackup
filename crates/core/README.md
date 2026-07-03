# convex-autobackup-core

`convex-autobackup-core` is the shared Rust library behind ConvexAutoBackup. It contains the backup model, restore planning, schedule evaluation, metadata database access, encrypted secret handling, verification helpers, and local/S3-compatible storage adapters used by the CLI, web server, and worker.

Most users should install the top-level `convex-autobackup` binary or run the Docker image. Use this crate directly when you are embedding ConvexAutoBackup behavior into another Rust service or building a custom operator around the same backup primitives.

## What It Provides

- Convex deployment and backup target configuration types.
- Backup run metadata, status tracking, retention records, and verification results.
- Scheduled backup planning with cron-based policy evaluation.
- Local filesystem and S3-compatible object storage support.
- Encrypted secret storage using a master key supplied by the host application.
- Restore/export helpers shared by the service binaries.

## Install

```toml
[dependencies]
convex-autobackup-core = "0.1.0-beta.3"
```

## Stability

This is a beta crate. Public APIs can still change before `1.0.0`, but releases are intended to remain usable for real self-hosted deployments.

## Links

- Repository: <https://github.com/KodyDennon/ConvexAutoBackup>
- Installation docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/INSTALLATION.md>
- Operations docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/OPERATIONS.md>
- Security policy: <https://github.com/KodyDennon/ConvexAutoBackup/security/policy>
