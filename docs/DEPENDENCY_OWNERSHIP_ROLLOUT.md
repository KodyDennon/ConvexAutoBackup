# Dependency Ownership Rollout

This rollout turns `docs/DEPENDENCY_AUDIT.md` into executable replacement work.

The rule is direct: ownable dependencies are removed, not hidden behind permanent wrappers. Dependencies that are too risky to replace safely, such as crypto primitives, TLS/HTTP, SQLite bindings, serde compatibility, and Tokio, may remain normal upstream dependencies with a written reason.

## Completed Wave 1

The first ownership wave replaces these direct dependencies with first-party code:

| Removed dependency | Replacement |
| --- | --- |
| `regex` | ASCII backup-path segment validation in `convex-autobackup-core`. |
| `dirs` | Explicit app data directory policy in `convex-autobackup-core`. |
| `cron` | Owned six-field cron-compatible evaluator in `convex-autobackup-core`. |
| `async-trait` | Explicit boxed-future exporter/importer traits. |
| `anyhow` | `firstparty-error`, a standalone publishable owned error crate. |
| `thiserror` | Manual `Display` and `Error` implementations. |
| `mime_guess` | Static first-party asset content-type mapping. |
| `rust-embed` | Build-generated `include_bytes!` asset manifest. |

`firstparty-error` is intentionally general-purpose and publishable. It has no default dependency tree and exposes optional integration features for common ecosystem errors.

## Next Waves

- Replace narrow encoding helpers where usage is fully scoped: `hex`, `percent-encoding`, and selected `base64` paths.
- Freeze the CLI grammar, then replace `clap` with an owned parser.
- Continue reducing middleware and telemetry dependencies where the product behavior is small enough to own.

## Normal Dependencies Allowed

The following may stay as upstream dependencies unless a separate security/design review chooses an owned fork or rewrite:

- `aes-gcm`, `argon2`, `getrandom`, `subtle`, `sha2`, and `hmac`.
- `reqwest`, `rustls`, `hyper`, and lower HTTP/TLS stack crates.
- `rusqlite` and SQLite native bindings.
- `serde`, `serde_json`, `tokio`, `axum`, `uuid`, and `chrono`.

These are not fake-owned by wrappers. They are accepted normal dependencies because replacing them from scratch would increase product and security risk.

## Acceptance Gates

Each replacement wave must pass:

```bash
make check
cargo tree -p convex-autobackup-core -e normal
cargo tree -p convex-autobackup-server -e normal
```

For removed direct dependencies, also run inverse checks such as:

```bash
cargo tree -i anyhow
cargo tree -i cron
cargo tree -i rust-embed
```
