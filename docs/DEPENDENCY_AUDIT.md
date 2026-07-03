# Dependency Ownership Audit

This audit tracks which code ConvexAutoBackup should own directly and which upstream dependencies are justified exceptions. The goal is product control: if behavior is important to the product and realistically ownable, the default answer is to own it.

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

The full Cargo metadata set currently includes 292 registry packages. Of those, 257 are under 20,000 Rust LOC. That threshold is the ownership trigger: every direct dependency under that line needs a written reason to remain external or a concrete path to become first-party code.

## Ownership Policy

Default stance: own the code.

Use a third-party crate only when at least one of these is true:

- The crate owns deep security-critical implementation details, such as encryption, password hashing, constant-time comparison, random-number generation, or TLS certificate validation.
- The crate owns a large wire protocol/runtime/database surface where partial compatibility would be worse than dependency ownership, such as HTTP, SQLite bindings, async runtime scheduling, or serde compatibility.
- Replacing it would distract from the product before the surrounding ConvexAutoBackup boundary is stable.

Everything else should move toward first-party ownership. Ownership can mean local code, an in-repo Convex-specific crate, or a separate general-purpose git-subtree crate that is published independently.

Do not treat a wrapper as ownership by itself. A dependency is either replaced with first-party code, accepted as a normal upstream dependency because replacement is too risky, or scheduled for later replacement with an explicit exit condition.

## Owned Crate Policy

Not every replacement should be private inline code.

Use local modules when the behavior is only meaningful inside ConvexAutoBackup, such as static asset content types or backup path segment validation.

Create a dedicated owned crate when the replacement is reusable across projects or likely to become a stable utility surface. Dedicated owned crates should have:

- Their own git subtree so history and ownership stay isolated and the crate can be reused elsewhere.
- A general-purpose package name when the crate is useful outside ConvexAutoBackup.
- A `convex-autobackup-*` package name only when the crate is specifically useful to ConvexAutoBackup and belongs in the main ConvexAutoBackup repository.
- A crate-level README, changelog entry, license metadata, docs, tests, and examples.
- CI checks that can run the crate independently.
- crates.io publishing through the release pipeline.
- A clear API boundary so ConvexAutoBackup consumes it like any external user would.

Candidate owned crates from this audit:

| Candidate | Replaces | Classification | Publishing/Repo Shape | Scope |
| --- | --- | --- | --- | --- |
| Safe backup paths | `regex` for safe path/name validation | ConvexAutoBackup-specific unless generalized beyond backup paths. | Keep in main repo as local code or publish as `convex-autobackup-paths`. | Backup-safe relative paths, safe object-key segments, path traversal rejection. |
| Static asset bundling | `mime_guess`, `rust-embed` | Local for now; general-purpose later if extracted. | First-party build-generated code in the server crate. | Generated static asset manifests and content type lookup for bundled web UIs. |
| Error model | `anyhow`, `thiserror` | General-purpose. | Standalone publishable `firstparty-error` crate. | Owned error/context primitives without derive macros or broad dependencies. |
| Schedule grammar | `cron` if advanced cron is narrowed | General-purpose if it becomes a small reusable scheduler; Convex-specific if tied to backup policy. | General-purpose git subtree if reusable; otherwise main repo under ConvexAutoBackup scope. | Interval, daily, weekly, missed-run policy, and scoped cron-subset evaluation. |
| CLI parser | `clap` later | ConvexAutoBackup-specific. | Main repo only unless a generic parser framework accidentally emerges, which is not the goal. | Stable ConvexAutoBackup command grammar after the public CLI settles. |
| Encoding helpers | `hex`, `percent-encoding`, maybe narrow `base64` usage | General-purpose only if complete and well-scoped; otherwise local. | Prefer local modules first; separate non-Convex crate only if the APIs are useful outside this project. | Object key encoding, SigV4 canonical encoding, manifest-safe display helpers. |
| Object storage boundary | Wrapper around `reqwest` S3 adapter | ConvexAutoBackup-specific at first. | Main repo under ConvexAutoBackup scope; not a general crate unless it becomes provider-neutral. | Store/get/prune backup archives without leaking HTTP client details into product code. |

## Direct Dependency LOC

These are the direct registry dependencies currently used by workspace crates. LOC is counted from `.rs` files in the local Cargo registry source.

| Dependency | Version | Rust LOC | Used By | Ownership Decision |
| --- | ---: | ---: | --- | --- |
| `dirs` | 6.0.0 | 445 | core, server, CLI, worker | Owned in Wave 1 |
| `hmac` | 0.12.1 | 609 | core | Temporary upstream; keep behind owned S3 signer boundary |
| `percent-encoding` | 2.3.2 | 696 | core | Own |
| `rust-embed` | 8.11.0 | 811 | server | Owned in Wave 1 |
| `hex` | 0.4.3 | 823 | core | Own |
| `subtle` | 2.6.1 | 1,442 | core | Temporary upstream; security exception |
| `getrandom` | 0.2.17/0.3.4/0.4.3 | 2,026-2,986 | core | Temporary upstream; OS RNG exception |
| `mime_guess` | 2.0.5 | 2,397 | server | Owned in Wave 1 |
| `sha2` | 0.10.9 | 2,433 | core | Temporary upstream; crypto primitive exception |
| `thiserror` | 2.0.18 | 2,817 | core | Owned in Wave 1 |
| `argon2` | 0.5.3 | 2,865 | core | Temporary upstream; password-hashing exception |
| `cron` | 0.17.0 | 3,276 | core | Owned in Wave 1 |
| `async-trait` | 0.1.89 | 3,327 | core | Owned in Wave 1 |
| `clap` | 4.6.1 | 4,265 | CLI, worker | Own after command grammar stabilizes |
| `anyhow` | 1.0.103 | 5,890 | all crates | Owned in Wave 1 |
| `base64` | 0.22.1 | 7,549 | core | Own if usage remains narrow |
| `aes-gcm` | 0.11.0 | 7,816 | core | Temporary upstream; encryption exception |
| `uuid` | 1.23.4 | 8,741 | core, server, CLI, worker | Own boundary first; consider owned ID type |
| `regex` | 1.12.4 | 11,995 | core | Owned in Wave 1 |
| `serde` | 1.0.228 | 17,331 | all crates | Temporary upstream; serialization compatibility exception |
| `axum` | 0.8.9 | 19,775 | server, CLI | Temporary upstream; HTTP server exception |
| `reqwest` | 0.13.4 | 20,133 | core | Temporary upstream; isolate behind owned storage boundary |
| `rusqlite` | 0.40.1 | 22,384 | core | Temporary upstream; database binding exception |
| `serde_json` | 1.0.150 | 23,185 | all crates | Temporary upstream; serialization compatibility exception |
| `tower-http` | 0.6.11/0.7.0 | 25,507-31,204 | server/reqwest | Temporary upstream; replace middleware usage where easy |
| `tracing-subscriber` | 0.3.23 | 28,699 | server, CLI, worker | Own logging policy, keep sink implementation temporarily |
| `chrono` | 0.4.45 | 34,285 | core | Own time boundary first |
| `tracing` | 0.1.44 | 72,356 | server, worker | Own telemetry boundary first |
| `tokio` | 1.52.3 | 137,250 | core, server, CLI, worker | Temporary upstream; async runtime exception |

## Own First

These safe, production-grade ownership candidates are implemented in Wave 1. See `docs/DEPENDENCY_OWNERSHIP_ROLLOUT.md` for the rollout record.

| Dependency | Current Use | Why Own | Ownership Plan | Risk |
| --- | --- | --- | --- | --- |
| `regex` | One static safe-name check in `crates/core/src/paths.rs`. | We only need `[A-Za-z0-9._-]+`. | Replace with `bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' \| b'_' \| b'-'))` and keep the current path safety tests. | Low |
| `mime_guess` | One content type lookup for embedded web assets. | Static asset extensions are known. | Replace with a local `content_type_for_path()` covering `.html`, `.css`, `.js`, `.json`, `.svg`, `.ico`, `.wasm`, `.txt`, defaulting to `application/octet-stream`. | Low |
| `dirs` | Default data directory resolution in core/server. | App-specific path policy should be explicit and documented. | Add `default_data_dir()` helper using `CONVEX_AUTOBACKUP_DATA_DIR`, then OS-specific env fallbacks: XDG on Unix, `APPDATA` on Windows, HOME fallback. | Low-Medium |
| `rust-embed` | Embeds Vite build output into the server crate. | We can own generated asset embedding for our exact Vite output. | Generate a small Rust module at build/release time with `include_bytes!` asset entries and a static lookup table. | Medium |

## Own With Care

These should be owned, but the migration needs sequencing because they touch public API shape or user-facing behavior.

| Dependency | Current Use | Recommendation | Reason |
| --- | --- | --- | --- |
| `cron` | Supports arbitrary six-field cron expressions. | Build an owned schedule grammar and migration behavior. | Interval/daily/weekly are product-native. Cron can be narrowed or made an advanced compatibility mode. |
| `async-trait` | Object-safe async `ConvexExporter`/`ConvexImporter` traits used by backup/restore/scheduler. | Convert APIs to owned generic or boxed-future patterns. | This removes macro dependency and clarifies library boundaries. |
| `clap` | CLI and worker argument parsing. | Freeze command grammar, then write an owned parser. | The CLI is product surface; owning it is aligned once commands settle. |
| `anyhow` | Application-wide fallible operations and context. | Replace with typed product errors. | Broad mechanical refactor; improves public API clarity. |
| `thiserror` | Error derive for a few enums. | Replace with manual `Display` and `Error` implementations. | Low technical risk, medium churn. |

## Own Boundaries, Keep Internals

Some dependencies should stay upstream, but we should own the boundary around them so they are swappable and isolated.

| Dependency Surface | Current Pull | Recommendation |
| --- | ---: | --- |
| S3-compatible storage via `reqwest` + TLS | About 116 crates from `reqwest` alone. | Keep upstream HTTP/TLS, but own the `ObjectStore` boundary and hide `reqwest` inside the S3 adapter. |
| Web server via `axum`/`hyper`/`tower` | About 70 crates from `axum`. | Keep upstream HTTP server stack, but own handler contracts and avoid leaking Axum types from public core APIs. |
| Tracing subscriber stack | About 27 crates. | Keep upstream logging, but keep setup in binaries only. |
| Cron support | About 25 crates. | Either own a narrow schedule parser or keep cron as optional advanced scheduling. |

## Keep

These are exceptions, not permanent blind trust. Keep them upstream until owning them is realistic, and keep them behind first-party boundaries.

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
| `hmac`, `sha2`, `subtle` | Low-level crypto/signing primitives; keep behind owned signing APIs. |
| `base64`, `percent-encoding`, `hex` | Encoding helpers; own these once usage is fully scoped. |
| `chrono` | Date/time handling and serde support are broad enough to keep unless a larger time-model redesign happens. |

## Duplicate/Churn Notes

- `tower-http` appears twice: `0.7.x` directly in the server and `0.6.x` through `reqwest`. This is not directly fixable except by waiting on upstream alignment or feature-gating `reqwest`.
- Crypto ecosystem crates have some duplicate families due to current RustCrypto transitions. Do not force these manually unless RustSec or `cargo deny` indicates a real issue.
- `rust-embed` adds build-time/proc-macro dependencies that are avoidable if we generate static asset code ourselves.

## Recommended Ownership Order

1. Own `regex`, `mime_guess`, and `dirs` as local production helpers or small owned crates.
2. Own web asset embedding instead of using `rust-embed`; this is a strong candidate for a dedicated reusable crate.
3. Own a typed error model and remove `anyhow` from public library APIs.
4. Remove `thiserror` by manually implementing small error enums or moving shared error helpers into an owned crate.
5. Decide whether advanced cron is product-critical. If not, own a scoped schedule grammar as a dedicated crate.
6. Convert async backup/restore traits away from `async-trait`.
7. Keep `reqwest`, TLS, SQLite, crypto, `serde`, `axum`, and `tokio` only as justified exceptions, and own narrow boundaries around each.
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
