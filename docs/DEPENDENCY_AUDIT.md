# Dependency Audit

This audit tracks which dependencies are worth keeping, replacing, or feature-gating as ConvexAutoBackup moves from beta packaging toward a smaller public crate surface.

Audit date: 2026-07-03

## Current Workspace Size

The Rust workspace is intentionally small:

| Crate | Rust LOC | Production LOC | Test LOC |
| --- | ---: | ---: | ---: |
| `convex-autobackup-core` | 4,062 | 3,963 | 99 |
| `convex-autobackup-server` | 825 | 636 | 189 |
| `convex-autobackup` | 846 | 846 | 0 |
| `convex-autobackup-worker` | 178 | 178 | 0 |
| `convex-autobackup-mcp` | 149 | 149 | 0 |
| **Total** | **6,060** | **5,772** | **288** |

The normal dependency graph currently resolves to about 257 crates. Because the first-party code is small, dependency choices should be judged by whether they avoid meaningful production risk, not only by convenience.

## Replacement Policy

Use local code when all of these are true:

- The behavior is narrow and stable.
- A correct implementation is under roughly 100-200 lines including tests.
- The code is not cryptography, TLS, HTTP protocol parsing, SQL storage, async runtime behavior, or password hashing.
- Tests can cover the full behavior.

Keep upstream crates when the crate owns security, protocol correctness, portability, or large compatibility surfaces.

Feature-gate dependencies when they are valuable but not needed by every install path.

## Replace First

These are safe, production-grade replacement candidates.

| Dependency | Current Use | Why Replace | Replacement Plan | Risk |
| --- | --- | --- | --- | --- |
| `regex` | One static safe-name check in `crates/core/src/paths.rs`. | Pulls a regex engine for `[A-Za-z0-9._-]+`. | Replace with `bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' \| b'_' \| b'-'))` and keep the current path safety tests. | Low |
| `mime_guess` | One content type lookup for embedded web assets. | Static asset extensions are known. | Replace with a local `content_type_for_path()` covering `.html`, `.css`, `.js`, `.json`, `.svg`, `.ico`, `.wasm`, `.txt`, defaulting to `application/octet-stream`. | Low |
| `dirs` | Default data directory resolution in core/server. | App-specific path policy should be explicit and documented. | Add `default_data_dir()` helper using `CONVEX_AUTOBACKUP_DATA_DIR`, then OS-specific env fallbacks: XDG on Unix, `APPDATA` on Windows, HOME fallback. | Low-Medium |

## Replace With Care

These can be replaced, but only if the behavior is deliberately scoped.

| Dependency | Current Use | Recommendation | Reason |
| --- | --- | --- | --- |
| `rust-embed` | Embeds Vite build output into the server crate. | Replace with generated static asset module or runtime `web-dist` serving. | Removes proc-macro/build-time asset machinery, but the replacement must preserve single-binary releases. |
| `cron` | Supports arbitrary six-field cron expressions. | Feature-gate first; replace later only if we accept a smaller schedule language. | Interval/daily/weekly are easy. Full cron correctness is a larger compatibility surface. |
| `async-trait` | Object-safe async `ConvexExporter`/`ConvexImporter` traits used by backup/restore/scheduler. | Replace only after converting trait calls to generic parameters or boxed futures. | Native async traits are not a drop-in for current `&dyn ConvexExporter` usage. |
| `clap` | CLI and worker argument parsing. | Keep for now; possible future manual parser. | The CLI has many subcommands. Manual parsing is feasible but would be more code and more UX risk. |

## Feature-Gate First

These dependencies are legitimate, but they should not be forced onto every consumer.

| Dependency Surface | Current Pull | Recommendation |
| --- | ---: | --- |
| S3-compatible storage via `reqwest` + TLS | About 116 crates from `reqwest` alone. | Move S3 support behind an `s3` feature. Local-only core installs should not compile HTTP/TLS. |
| Web server via `axum`/`hyper`/`tower` | About 70 crates from `axum`. | Keep in `convex-autobackup-server`, but remove direct `axum` from the CLI crate if CLI can call server helpers without importing server protocol types. |
| Tracing subscriber stack | About 27 crates. | Keep for binaries, avoid in library crates unless needed. |
| Cron support | About 25 crates. | Put `Schedule::Cron` evaluation behind a `cron` feature or keep the enum and return a clear unavailable error when disabled. |

## Keep

These should not be rebuilt locally.

| Dependency | Reason |
| --- | --- |
| `aes-gcm` | Authenticated encryption is security-critical. |
| `argon2` | Password hashing is security-critical and parameter-sensitive. |
| `rustls`/TLS stack through `reqwest` | TLS implementation and certificate verification should stay upstream. |
| `reqwest`/`hyper` HTTP stack | HTTP/HTTPS correctness is a large protocol surface; feature-gate instead of rewriting. |
| `rusqlite`/`libsqlite3-sys` | SQLite bindings and bundled native builds are not worth owning. |
| `tokio` | Async runtime behavior is foundational and not a project-specific concern. |
| `serde`/`serde_json` | Serialization compatibility and derive support are worth the dependency. |
| `uuid` | UUID generation/parsing is small to use but not worth custom compatibility risk. |
| `hmac`, `sha2`, `subtle`, `base64`, `percent-encoding`, `hex` | Low-level crypto/signing/encoding primitives used by S3 signing and manifests; keep or feature-gate with S3. |
| `chrono` | Date/time handling and serde support are broad enough to keep unless a larger time-model redesign happens. |

## Duplicate/Churn Notes

- `tower-http` appears twice: `0.7.x` directly in the server and `0.6.x` through `reqwest`. This is not directly fixable except by waiting on upstream alignment or feature-gating `reqwest`.
- Crypto ecosystem crates have some duplicate families due to current RustCrypto transitions. Do not force these manually unless RustSec or `cargo deny` indicates a real issue.
- `rust-embed` adds build-time/proc-macro dependencies that are avoidable if we generate static asset code ourselves.

## Recommended Order

1. Replace `regex`, `mime_guess`, and `dirs` with tested local helpers.
2. Add workspace features: `s3`, `cron`, `server`, and possibly `embedded-web`.
3. Move `reqwest`, `hmac`, `sha2`, `hex`, and `percent-encoding` under the `s3` feature where possible.
4. Replace `rust-embed` with a generated static asset module so release binaries still include the web UI.
5. Remove direct `axum` use from the CLI if the CLI can delegate serving to `convex-autobackup-server` without importing Axum directly.
6. Revisit `async-trait` after backup/restore/scheduler APIs are converted away from `&dyn` async traits.
7. Reconsider `clap` only after the public CLI stabilizes.

## Acceptance Gates

Every dependency-reduction PR should pass:

```bash
make check
cargo audit
cargo deny check advisories
npm --prefix web audit --audit-level=moderate
cargo tree -e normal --duplicates
```

For feature work, also check package surfaces:

```bash
cargo package -p convex-autobackup-core --list
cargo package -p convex-autobackup --list
cargo tree -e normal --no-default-features
```
