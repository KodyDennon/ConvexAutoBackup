# Changelog

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
