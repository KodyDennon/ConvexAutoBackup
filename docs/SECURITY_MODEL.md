# Security Model

## Default Exposure

The service defaults to `0.0.0.0:8976` so it can work on LAN and server installs. Real product actions require authentication. First-run setup creates the first admin password before targets, destinations, secrets, schedules, or restore workflows can be managed.

Production installs should use a reverse proxy with HTTPS. The app must support explicit bind host and port configuration.

## Authentication

The product supports:

- Browser sessions for humans.
- API tokens for automation and AI agents.
- Role-based permissions for owners, admins, operators, and viewers.

Initial owner creation is a first-run bootstrap action. After bootstrap, owner creation requires an authorized owner.
The HTTP bootstrap endpoint returns a one-time API token with the first owner so headless installs can continue setup without browser sessions.

## Authorization

Authorization is enforced server-side for UI, API, CLI, and MCP paths. The UI is never trusted as an authorization boundary.

Minimum roles:

- Owner: full control, user management, destructive restore configuration.
- Admin: configure projects, targets, destinations, schedules, and tokens.
- Operator: run backups, verification, restore drills, and reports.
- Viewer: read health, runs, logs, and reports.

## Secrets

Secrets include Convex deploy keys, self-hosted Convex admin keys, S3 credentials, encryption keys, webhook tokens, and API tokens.

Rules:

- Secrets are linked by stable references.
- Secrets are never returned by normal API responses.
- Secrets are never included in normal config exports.
- Secret use is recorded in audit logs by reference, not by value.
- Native installs use OS keychain where feasible.
- Docker/headless installs use encrypted database storage.

## Backup Encryption

Backup archive encryption is optional per destination and implemented as [age](https://age-encryption.org) passphrase (scrypt) encryption. Archive bytes are encrypted in memory before they are written locally or uploaded, and stored as `.zip.age`.

- The passphrase is stored in the encrypted secret vault, so scheduled backups, verification and restore need no interaction.
- Changing a passphrase creates a new vault secret; older archives keep pointing at the secret they were written with. Secrets referenced by a destination cannot be deleted.
- Without the app, any archive can be recovered with `age -d backup.zip.age > backup.zip` or `convex-autobackup decrypt backup.zip.age --out backup.zip` (passphrase from `CONVEX_AUTOBACKUP_PASSPHRASE` or stdin).
- Manifests stay unencrypted and record the encryption mode, key reference, plaintext SHA-256 and stored SHA-256.

Recovery requirements:

- The UI and CLI must make key loss consequences explicit.
- Encrypted backup manifests must record encryption mode and key reference.
- Restore flows must verify decryption before import.

## Audit Log

Audit entries record:

- Auth events.
- User and role changes.
- API token creation and revocation.
- Secret reference creation, rotation, and deletion.
- Project, target, destination, schedule, and retention changes.
- Backup, verification, restore, and report actions.
- Destructive restore confirmations.

The current implementation persists audit events for user creation, token creation, secret storage, project/target/destination/job/schedule creation, backup run start and completion, and restore completion.

## Agent Safety

MCP and API token access default to safe read and non-destructive action scopes. Destructive restore is disabled for MCP by default and must remain separately gated.
