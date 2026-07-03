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

The server test suite covers bootstrap-token API flow and bearer-token protection. Core tests cover auth, encrypted secrets, scheduling, backup, local retention, verification, restore, and DR reports.
