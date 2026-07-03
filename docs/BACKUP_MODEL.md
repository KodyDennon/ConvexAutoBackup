# Backup Model

## Backup Contents

Backups are full Convex exports. File storage is included by default because a database-only export is not a complete app-state backup for projects that use Convex file storage.

Each job has an explicit `include_file_storage` setting. Turning it off is allowed for cost, speed, or special workflows, and the manifest records that choice.

## Backup Flow

1. Scheduler marks a job due.
2. Worker queue applies concurrency limits.
3. Worker resolves Convex target credentials from the configured environment-variable reference.
4. Worker runs the pinned Convex export command.
5. Worker writes the archive to a staging area.
6. Worker calculates checksum and size.
7. Worker writes archive to the destination.
8. Worker writes the manifest.
9. Worker verifies destination visibility.
10. Worker records run result and audit events.

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
- Encryption metadata when encryption is enabled.

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

The current implementation stores S3-compatible backups through the Rust `object_store` S3 backend. Credentials are stored as encrypted JSON secrets with `access_key_id` and `secret_access_key` fields.

## Retention

Retention supports:

- Keep last N backups.
- Keep daily backups for N days.
- Keep weekly backups.
- Keep monthly backups.

Retention deletes only backups that have a valid manifest and are not protected by a DR drill or explicit hold.

## Verification

Verification reads the archive and manifest from local filesystem or S3-compatible storage, recalculates checksum, compares size, and returns a verification result. Verification failure does not delete the archive.
