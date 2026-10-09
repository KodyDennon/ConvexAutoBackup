# Backup Model

## Backup Contents

Backups are full Convex exports. File storage is included by default because a database-only export is not a complete app-state backup for projects that use Convex file storage.

Each job has an explicit `include_file_storage` setting. Turning it off is allowed for cost, speed, or special workflows, and the manifest records that choice.

## Backup Flow

1. Scheduler marks a job due.
2. Worker queue applies concurrency limits.
3. Worker resolves the deploy key from the encrypted secret store (or an environment-variable reference) and checks it matches the target deployment.
4. Worker runs the pinned Convex export command into the staging area.
5. Worker calculates the plaintext checksum, size, and table inventory.
6. For each destination of the job (primary first):
   1. Encrypt the archive if the destination has a passphrase.
   2. Write the archive, then its manifest.
   3. Record the copy (`run_copies`) as succeeded or failed.
   4. Apply that destination's retention.
7. Worker records the run as `succeeded`, `partial` (some copies failed), or `failed` (no copy stored), plus audit events.

## Manifest

Every backup has a manifest with:

- Schema version.
- Project ID.
- Target ID.
- Run ID.
- Deployment identifier.
- Convex CLI version.
- Include-file-storage flag.
- Archive size.
- SHA-256 checksum.
- Start and finish timestamps.
- Duration.
- Storage URI.
- Destination ID (schema v2).
- Encryption metadata when encryption is enabled: mode (`age_scrypt`) and the secret ID of the passphrase used.
- Stored checksum and size (the encrypted bytes) next to the plaintext checksum and size.
- Table inventory (table names, document counts). It is kept in the app database but omitted from manifest files stored next to encrypted archives so they do not leak schema details.

Schema v1 manifests from older releases are still read.

## Encryption

Destinations can encrypt archives with an age passphrase (scrypt). Archives are written as `<timestamp>-<run>.zip.age` and can be decrypted with stock `age -d` or `convex-autobackup decrypt`. The passphrase is stored in the encrypted secret store so scheduled backups need no interaction; changing it creates a new secret, and existing archives keep referencing the passphrase they were written with.

## Local Filesystem Storage

Local destinations must:

- Validate paths.
- Prevent parent-directory escape.
- Check free space before large writes when size is known.
- Write through staging and atomic rename where the filesystem supports it.
- Store manifests next to archives or in a deterministic manifest prefix.
- Apply retention through the same manifest listing logic used by the UI.

The current implementation writes local archives through a temporary file and atomic rename, writes manifests next to the archive, records the manifest path in SQLite, and records failed runs when credential resolution or export execution fails.

## S3-Compatible Storage

S3-compatible destinations must:

- Support endpoint override for R2, B2, MinIO, and compatible providers.
- Use deterministic object keys.
- Verify upload completion.
- Store manifests as separate objects.
- Support retention listing by prefix.
- Avoid provider-specific behavior in the shared storage contract.

The current implementation stores S3-compatible backups with a built-in SigV4 client (path-style URLs, so Cloudflare R2 works with `endpoint = https://<account>.r2.cloudflarestorage.com` and region `auto`). Retention, verification and restore all work against S3/R2. Credentials are stored as encrypted JSON secrets with `access_key_id` and `secret_access_key` fields.

## Retention

Retention supports:

- Keep last N backups.
- Keep daily backups for N days.
- Keep weekly backups.
- Keep monthly backups.

`keep_last` is enforced for local folders and S3/R2 buckets after each successful copy: older archive/manifest pairs for the same project and deployment are deleted. Retention only runs after a new copy was stored, so a stopped server never erodes existing backups. Retention failures are recorded in the audit log and do not fail the backup. Avoid object-storage lifecycle rules that expire backups by age for the same reason.

## Verification

Verification reads the first readable copy (primary destination first, then the others), checks the stored checksum, decrypts encrypted copies, and compares the plaintext checksum and size with the manifest. Verification failure does not delete the archive. Restore uses the same path and stages the decrypted zip in a private temporary directory that is removed afterwards.
