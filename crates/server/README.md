# convex-autobackup-server

`convex-autobackup-server` is the Axum HTTP server and embedded dashboard package for ConvexAutoBackup. It serves the web UI, exposes the local API used by the dashboard, and wires the core backup primitives into a self-hosted service process.

Most users should run the top-level `convex-autobackup` binary or the Docker image. Use this crate directly when building a custom Rust host around the ConvexAutoBackup server.

## What It Provides

- Axum routes for deployments, backup runs, restore workflows, settings, and health checks.
- Embedded static assets for the React dashboard.
- Server state construction around the shared `convex-autobackup-core` database and storage layer.
- HTTP middleware and file serving configuration used by the released binary.

## Install

```toml
[dependencies]
convex-autobackup-server = "0.1.0-beta.5"
```

## Links

- Repository: <https://github.com/KodyDennon/ConvexAutoBackup>
- Deployment docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/DEPLOYMENT.md>
- Troubleshooting: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/TROUBLESHOOTING.md>
