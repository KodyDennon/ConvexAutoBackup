# Open Source Release Checklist

Use this checklist before making a repository, release, or tag public.

## Repository Metadata

- Repository visibility is public.
- Description: `Self-hosted Convex backup, restore, and disaster recovery control plane with web UI, CLI, Docker, native binaries, and MCP support.`
- Homepage points to the latest GitHub Release.
- Topics include `convex`, `convex-backup`, `backup`, `backups`, `disaster-recovery`, `rust`, `react`, `self-hosted`, `cli`, `mcp`, `docker`, `ghcr`, `crates-io`, and `github-actions`.
- Issues, Discussions, Wiki, and GitHub Security Advisories are enabled where available.

## Required Files

- `README.md`
- `LICENSE`
- `SECURITY.md`
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `.github/CODEOWNERS`
- Pull request template
- Bug and feature issue templates
- CI and release workflows
- Native install scripts
- Docker setup script
- SHA256 checksums and artifact provenance

## Validation

Run:

```bash
make check
```

The check includes:

- Source file line-limit guard.
- Web production build.
- Rust format check.
- Rust clippy with warnings denied.
- Rust workspace tests.
- Web tests.

Also run:

```bash
git diff --check
```

Before publishing, run a repository text scan for unfinished-work markers and resolve any hits.

## Security Review

- Do not commit `.env`, local databases, backup archives, deploy keys, API tokens, or S3 credentials.
- Confirm `.env.example` contains names only, not live values.
- Use a temporary data directory for smoke tests.
- Never run restore smoke tests against a production Convex deployment.
- Confirm backup and restore changes include tests.

## Publish Steps

```bash
git status --short
make check
git push -u origin main
gh repo edit KodyDennon/ConvexAutoBackup --visibility public
git -c tag.gpgSign=false tag -a v0.1.0-beta.2 -m "ConvexAutoBackup v0.1.0-beta.2"
git push origin v0.1.0-beta.2
```

If the repository does not exist yet:

```bash
gh repo create KodyDennon/ConvexAutoBackup --public --source=. --remote=origin --push
```

The release workflow publishes:

- Full macOS, Linux, and Windows bundles.
- `install.sh`, `install.ps1`, and `docker-setup.sh`.
- `SHA256SUMS`.
- GitHub Release provenance attestations.
- GHCR image `ghcr.io/kodydennon/convex-autobackup`.
- Docker Hub image `kodydoty/convex-autobackup`.
- crates.io packages for `firstparty-error`, `convex-autobackup-core`, `convex-autobackup-server`, `convex-autobackup-worker`, `convex-autobackup-mcp`, and `convex-autobackup`.

Docker Hub and crates.io publishing require repository secrets:

```text
DOCKERHUB_USERNAME
DOCKERHUB_TOKEN
CARGO_REGISTRY_TOKEN
```

Windows MSI packaging pins WiX Toolset `6.0.2` so CI does not pick up newer tool behavior unexpectedly.
