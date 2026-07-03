# Dependency Ownership Audit

This audit tracks which dependencies are worth keeping upstream and which behavior ConvexAutoBackup should own directly. The goal is code ownership and production control, not dependency-count minimalism by itself.

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

The full Cargo metadata set currently includes 292 registry packages. Of those, 257 are under 20,000 Rust LOC. That threshold is useful for finding candidates, but it is not enough by itself: small crypto, password hashing, TLS, database, serialization, HTTP, and async-runtime crates can still be bad ownership targets.

## Ownership Policy

Own behavior in this repo, or in small `convex-autobackup-*` support crates, when all of these are true:

- The behavior is narrow and stable.
- The upstream crate is under roughly 20,000 Rust LOC.
- A correct project-specific implementation is realistic to fully test.
- The code is not cryptography, TLS, HTTP protocol parsing, SQL storage, async runtime behavior, or password hashing.
- Owning it reduces supply-chain exposure or gives us meaningful product control.

Keep upstream crates when the crate owns security, protocol correctness, portability, or large compatibility surfaces. "Under 20k LOC" does not make those safe to own.

Feature-gate dependencies when they are valuable but not needed by every install path.

## Direct Dependency LOC

These are the direct registry dependencies currently used by workspace crates. LOC is counted from `.rs` files in the local Cargo registry source.

| Dependency | Version | Rust LOC | Used By | Ownership Decision |
| --- | ---: | ---: | --- | --- |
| `dirs` | 6.0.0 | 445 | core, server, CLI, worker | Own now |
| `hmac` | 0.12.1 | 609 | core | Keep upstream |
| `percent-encoding` | 2.3.2 | 696 | core | Own later or keep with S3 |
| `rust-embed` | 8.11.0 | 811 | server | Own now |
| `hex` | 0.4.3 | 823 | core | Own later or keep with S3 |
| `subtle` | 2.6.1 | 1,442 | core | Keep upstream |
| `getrandom` | 0.2.17/0.3.4/0.4.3 | 2,026-2,986 | core | Keep upstream |
| `mime_guess` | 2.0.5 | 2,397 | server | Own now |
| `sha2` | 0.10.9 | 2,433 | core | Keep upstream |
| `thiserror` | 2.0.18 | 2,817 | core | Own later |
| `argon2` | 0.5.3 | 2,865 | core | Keep upstream |
| `cron` | 0.17.0 | 3,276 | core | Own scoped version or feature-gate |
| `async-trait` | 0.1.89 | 3,327 | core | Own via API refactor |
| `clap` | 4.6.1 | 4,265 | CLI, worker | Own later if CLI stabilizes |
| `anyhow` | 1.0.103 | 5,890 | all crates | Own later |
| `base64` | 0.22.1 | 7,549 | core | Keep upstream unless S3/auth encoding is narrowed |
| `aes-gcm` | 0.11.0 | 7,816 | core | Keep upstream |
| `uuid` | 1.23.4 | 8,741 | core, server, CLI, worker | Keep upstream |
| `regex` | 1.12.4 | 11,995 | core | Own now |
| `serde` | 1.0.228 | 17,331 | all crates | Keep upstream |
| `axum` | 0.8.9 | 19,775 | server, CLI | Keep upstream |
| `reqwest` | 0.13.4 | 20,133 | core | Keep upstream, isolate behind owned storage boundary |
| `rusqlite` | 0.40.1 | 22,384 | core | Keep upstream |
| `serde_json` | 1.0.150 | 23,185 | all crates | Keep upstream |
| `tower-http` | 0.6.11/0.7.0 | 25,507-31,204 | server/reqwest | Keep upstream or reduce server middleware |
| `tracing-subscriber` | 0.3.23 | 28,699 | server, CLI, worker | Keep upstream for now |
| `chrono` | 0.4.45 | 34,285 | core | Keep upstream for now |
| `tracing` | 0.1.44 | 72,356 | server, worker | Keep upstream |
| `tokio` | 1.52.3 | 137,250 | core, server, CLI, worker | Keep upstream |

## Own First

These are safe, production-grade ownership candidates.

| Dependency | Current Use | Why Own | Ownership Plan | Risk |
| --- | --- | --- | --- | --- |
| `regex` | One static safe-name check in `crates/core/src/paths.rs`. | We only need `[A-Za-z0-9._-]+`. | Replace with `bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' \| b'_' \| b'-'))` and keep the current path safety tests. | Low |
| `mime_guess` | One content type lookup for embedded web assets. | Static asset extensions are known. | Replace with a local `content_type_for_path()` covering `.html`, `.css`, `.js`, `.json`, `.svg`, `.ico`, `.wasm`, `.txt`, defaulting to `application/octet-stream`. | Low |
| `dirs` | Default data directory resolution in core/server. | App-specific path policy should be explicit and documented. | Add `default_data_dir()` helper using `CONVEX_AUTOBACKUP_DATA_DIR`, then OS-specific env fallbacks: XDG on Unix, `APPDATA` on Windows, HOME fallback. | Low-Medium |
| `rust-embed` | Embeds Vite build output into the server crate. | We can own generated asset embedding for our exact Vite output. | Generate a small Rust module at build/release time with `include_bytes!` asset entries and a static lookup table. | Medium |

## Own With Care

These can be owned, but only if the behavior is deliberately scoped.

| Dependency | Current Use | Recommendation | Reason |
| --- | --- | --- | --- |
| `cron` | Supports arbitrary six-field cron expressions. | Feature-gate first; replace later only if we accept a smaller schedule language. | Interval/daily/weekly are easy. Full cron correctness is a larger compatibility surface. |
| `async-trait` | Object-safe async `ConvexExporter`/`ConvexImporter` traits used by backup/restore/scheduler. | Replace only after converting trait calls to generic parameters or boxed futures. | Native async traits are not a drop-in for current `&dyn ConvexExporter` usage. |
| `clap` | CLI and worker argument parsing. | Keep for now; possible future manual parser. | The CLI has many subcommands. Manual parsing is feasible but would be more code and more UX risk. |
| `anyhow` | Application-wide fallible operations and context. | Replace after errors are grouped into explicit core/server/CLI error types. | Broad mechanical refactor; improves public API clarity. |
| `thiserror` | Error derive for a few enums. | Replace after `anyhow` cleanup with manual `Display` and `Error` implementations where useful. | Low technical risk, medium churn. |

## Own Boundaries, Keep Internals

Some dependencies should stay upstream, but we should own the boundary around them so they are swappable and isolated.

| Dependency Surface | Current Pull | Recommendation |
| --- | ---: | --- |
| S3-compatible storage via `reqwest` + TLS | About 116 crates from `reqwest` alone. | Keep upstream HTTP/TLS, but own the `ObjectStore` boundary and hide `reqwest` inside the S3 adapter. |
| Web server via `axum`/`hyper`/`tower` | About 70 crates from `axum`. | Keep upstream HTTP server stack, but own handler contracts and avoid leaking Axum types from public core APIs. |
| Tracing subscriber stack | About 27 crates. | Keep upstream logging, but keep setup in binaries only. |
| Cron support | About 25 crates. | Either own a narrow schedule parser or keep cron as optional advanced scheduling. |

## Keep

These should not be rebuilt locally even when they are under 20k LOC.

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

## Recommended Ownership Order

1. Own `regex`, `mime_guess`, and `dirs` as local production helpers.
2. Own web asset embedding instead of using `rust-embed`.
3. Own a typed error model and remove `anyhow` from public library APIs.
4. Remove `thiserror` by manually implementing small error enums.
5. Decide whether advanced cron is product-critical. If not, own a scoped schedule grammar.
6. Convert async backup/restore traits away from `async-trait`.
7. Keep `reqwest`, TLS, SQLite, crypto, `serde`, `axum`, and `tokio`, but own narrow boundaries around each.
8. Consider owning CLI parsing only after command shape stabilizes.

## Acceptance Gates

Every ownership PR should pass:

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
