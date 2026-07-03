# Contributing

## Development Setup

```bash
make setup
make check
```

## Quality Bar

- Add tests with behavior changes.
- Keep CLI/API/MCP contracts stable and documented.
- Do not store secrets in plain metadata.
- Do not expose destructive restore without explicit confirmation and audit logging.
- Keep docs synchronized with implemented behavior.

## Rust

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Web

```bash
npm --prefix web run build
npm --prefix web test -- --run
```

## Pull Requests

Every PR should include:

- Summary of behavior changed.
- Tests run.
- Security or restore-risk notes when relevant.

