# Open Source Release Checklist

Use this checklist before making a repository, release, or tag public.

## Repository Metadata

- Repository visibility is public.
- Description: `Self-hosted Convex backup and disaster recovery control plane`.
- Homepage points to the README or project site.
- Topics include `convex`, `backup`, `disaster-recovery`, `rust`, `react`, `self-hosted`, `cli`, and `mcp`.
- Issues and GitHub Security Advisories are enabled.

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
```

If the repository does not exist yet:

```bash
gh repo create KodyDennon/ConvexAutoBackup --public --source=. --remote=origin --push
```
