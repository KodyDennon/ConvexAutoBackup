# Changelog

## 0.1.0-beta.7

- Added passphrase encryption for backups (age/scrypt). Archives are stored as standard `.zip.age` files that `age -d` or `convex-autobackup decrypt` can open without the app. Manifests (schema v2) record the encryption key reference plus plaintext and stored checksums.
- Backup jobs can write one export to several destinations (e.g. local + Cloudflare R2). Each copy is tracked per run; a failed copy marks the run `partial` instead of losing the others.
- S3-compatible storage is now complete: restore and verify read from S3/R2, retention prunes S3/R2, destinations have a write/read/delete connection test, and object keys use RFC 3986 encoding (fixes signature mismatches for keys containing `.`, `-`, `_`).
- New setup wizard (“Add project”): system checks, live read-only deploy key validation, storage selection (including a server-provided Cloudflare R2 preset), encryption, schedule, one-step creation with resumable retries, and a verified first backup.
- Target connection test now really contacts Convex (read-only table listing) instead of only echoing configuration.
- Fixed restore: `convex import` now receives the archive positionally with `--yes` (the previous `--path` flag does not exist for import).
- Convex CLI pinned to 1.46.0.
- Docker: hardened image (non-root, read-only root filesystem, tini, healthcheck, baked-in Convex CLI, `--locked` builds) and a compose file with an optional Cloudflare Tunnel sidecar (`--profile tunnel`). Added `scripts/docker-update.sh` and `scripts/cloudflare-provision.py` (R2 bucket + bucket-scoped token, tunnel, DNS, Access).
- Self-update and factory-reset no longer use hard-coded host paths; factory reset only removes backup archives and manifests in configured local destinations.
- User-facing errors now include their underlying cause.
- UI polish: fixed invisible setup step labels, collapsed inline code blocks, overlapping panels, oversized checkboxes and missing navigation/inventory styles; improved mobile layout.

## 0.1.0-beta.6

- Complete Web UI/UX overhaul with interactive 6-step progress navigator, sub-tab navigation bar, and resource cards.
- Added full resource CRUD deletion & update handlers (`DELETE` & `PUT` endpoints for Projects, Targets, Vaults, Jobs, Schedules, Secrets).
- Added target connectivity testing (`POST /api/v1/targets/{id}/test`).
- Added interactive JSON manifest viewer modal in Run History.
- Reinforced AES-256-GCM secret encryption with SHA-256 key derivation and random 12-byte nonces.
- Rebuilt embedded web bundle and release binaries.

## 0.1.0-beta.5

- Fixed the web console release check for beta-only GitHub releases by reading the releases list instead of `/releases/latest`, which returns 404 when only prereleases exist.
- Hardened dashboard state rendering against partial API responses so missing arrays or DR findings do not crash the React app.
- Added regression coverage for prerelease update detection and partial runtime API data.
- Rebuilt the embedded web bundle served by the Rust server.

## 0.1.0-beta.4

- Reworked the web console setup page into a guided first-run flow that focuses on the next required backup task instead of showing every form at once.
- Added a compact setup inventory and kept advanced manual configuration available behind an explicit disclosure.
- Improved existing-install sign-in guidance with a local owner recovery command for LAN/server installs.
- Rebuilt the embedded web bundle served by the Rust server.

## 0.1.0-beta.3

- Released the first dependency ownership wave from the current mainline commit.
- Added the publishable `firstparty-error` crate and release publishing order for it.
- Replaced ownable runtime dependencies with first-party code for error handling, scheduling, path validation, data directory selection, and bundled asset embedding.
- Added crate-specific README files and package metadata for all published crates.

## 0.1.0-beta.2

- Added the publishable `firstparty-error` crate and release publishing order for it.
- Replaced ownable runtime dependencies with first-party code for error handling, scheduling, path validation, data directory selection, and bundled asset embedding.
- Added crate-specific README files and package metadata for all published crates.
- Fixed prerelease internal dependency pins for crates.io publishing.
- Filtered release artifacts so Docker build records are not uploaded to GitHub Releases.

## 0.1.0

- Initialized ConvexAutoBackup as a Rust and React self-hosted backup platform.
- Added core domain models, scheduling logic, path safety checks, backup manifests, worker policy, service health API, CLI, MCP stdio foundation, Docker, CI, and project documentation.
