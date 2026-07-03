# convex-autobackup-worker

`convex-autobackup-worker` is the scheduled worker binary for ConvexAutoBackup. It evaluates configured backup schedules, executes due backup jobs, applies retention policy, and records run status using the shared core library.

Most operators should use the top-level `convex-autobackup worker` command or Docker image. This package exists so the worker can also be installed, packaged, and composed independently.

## Install

```bash
cargo install convex-autobackup-worker --locked
```

## Links

- Repository: <https://github.com/KodyDennon/ConvexAutoBackup>
- Operations docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/OPERATIONS.md>
- Deployment docs: <https://github.com/KodyDennon/ConvexAutoBackup/blob/main/docs/DEPLOYMENT.md>
