# convex-autobackup

`convex-autobackup` is the command-line entry point for ConvexAutoBackup. It starts the local web service, runs one-off backup and restore operations, manages configuration, and provides the binary most users install from crates.io.

## Install

```bash
cargo install convex-autobackup --locked
```

You can also use the native binaries, Windows MSI, or Docker images published from the GitHub Releases workflow.

## Common Commands

```bash
convex-autobackup serve
convex-autobackup backup run
convex-autobackup restore plan
convex-autobackup worker
```

## Links

- Repository: <https://github.com/KodyDennon/ConvexAutoBackup>
- Installation docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/INSTALLATION.md>
- Release channels: <https://github.com/KodyDennon/ConvexAutoBackup/wiki/Release-Channels>
- Docker Hub: <https://hub.docker.com/r/kodydoty/convex-autobackup>
