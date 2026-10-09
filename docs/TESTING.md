# Testing

Run the full local verification suite:

```bash
make check
```

This runs:

- Web production build.
- Rust formatting check.
- Rust clippy with warnings denied.
- Rust workspace tests.
- Web unit tests.

Operational smoke tests should use temporary data directories and a Convex-compatible command wrapper unless real disposable Convex credentials are available. Never use production deploy keys for restore tests.

The server test suite covers bootstrap-token API flow, bearer-token protection, destination encryption and probe endpoints, and the absence of cross-origin access. Core tests cover auth, encrypted secrets, scheduling, backup, multi-destination fan-out and partial failure, age encryption round trips and wrong passphrases, local and S3 retention selection, SigV4 query signing and R2 URLs, manifest v1 compatibility and redaction, verification, restore, and DR reports.

For end-to-end checks without touching real data, point `CONVEX_AUTOBACKUP_CONVEX_BIN` at a stub script that answers `--version`, `data` and `export --path <file>`, and use a throwaway bucket prefix.
