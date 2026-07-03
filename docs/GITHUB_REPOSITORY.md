# GitHub Repository Setup

This document records the intended public GitHub setup for ConvexAutoBackup.

## Repository Metadata

Repository:

```text
https://github.com/KodyDennon/ConvexAutoBackup
```

Description:

```text
Self-hosted Convex backup, restore, and disaster recovery control plane with web UI, CLI, Docker, native binaries, and MCP support.
```

Homepage:

```text
https://github.com/KodyDennon/ConvexAutoBackup/releases/latest
```

Topics:

```text
backup
backups
cli
convex
convex-backup
crates-io
disaster-recovery
docker
ghcr
github-actions
mcp
react
rust
self-hosted
```

Enabled features:

- Issues.
- Discussions.
- Wiki.
- Delete branch on merge.
- Pull request update branch.

## Public Links

- README: https://github.com/KodyDennon/ConvexAutoBackup#readme
- Wiki: https://github.com/KodyDennon/ConvexAutoBackup/wiki
- Releases: https://github.com/KodyDennon/ConvexAutoBackup/releases
- Docker Hub: https://hub.docker.com/r/kodydoty/convex-autobackup
- GHCR: https://github.com/KodyDennon/ConvexAutoBackup/pkgs/container/convex-autobackup
- crates.io: https://crates.io/crates/convex-autobackup
- Issues: https://github.com/KodyDennon/ConvexAutoBackup/issues
- Discussions: https://github.com/KodyDennon/ConvexAutoBackup/discussions

## Wiki Pages

The public wiki should include:

- Home.
- Installation.
- Configuration.
- Deployment.
- Operations.
- Release Channels.
- Security.
- Troubleshooting.
- Sidebar and footer navigation.

## Release Secrets

Required GitHub Actions secrets:

```text
DOCKERHUB_USERNAME
DOCKERHUB_TOKEN
CARGO_REGISTRY_TOKEN
```

Do not store token values in repository files, issues, discussions, wiki pages, or release notes.
