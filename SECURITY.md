# Security Policy

## Reporting

Report security issues privately to the maintainers before publishing details.

## Supported Versions

The project is pre-1.0. Security fixes apply to the current mainline until tagged releases begin.

## Sensitive Areas

- Convex deploy keys and self-hosted admin keys.
- Storage credentials.
- Backup encryption keys.
- API tokens.
- Restore workflows.
- LAN/public exposure configuration.

## Security Expectations

- Secrets must be stored in an approved secret backend.
- Backups must include checksums.
- Destructive restore must be audited.
- Agent access must be scoped.
- Public deployments must use HTTPS through a reverse proxy.

