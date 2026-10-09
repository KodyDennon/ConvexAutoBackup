# Operations

Backups only matter if they can be verified, restored, and explained during an incident. This document covers the operating routine for ConvexAutoBackup.

## Health Checks

API health:

```bash
curl http://localhost:8976/api/v1/health
```

CLI health:

```bash
convex-autobackup health --json
convex-autobackup doctor --json
```

Docker logs:

```bash
docker logs convex-autobackup
```

systemd logs:

```bash
journalctl -u convex-autobackup -f
```

## Backup Routine

Run a manual backup before relying on a schedule:

```bash
CONVEX_AUTOBACKUP_MASTER_KEY=<master> convex-autobackup backup run \
  --job-id <job-id> \
  --json
```

List runs:

```bash
convex-autobackup runs --json
```

Verify:

```bash
convex-autobackup verify --run-id <run-id> --json
```

## Restore Drills

Restore drills should target a non-production Convex deployment.

```bash
CONVEX_AUTOBACKUP_MASTER_KEY=<master> convex-autobackup restore \
  --run-id <run-id> \
  --target-id <target-id> \
  --confirm-deployment <deployment-name> \
  --json
```

The explicit `--confirm-deployment` guard is intentional. Restore is more dangerous than backup creation and should not be hidden behind a casual click or generic confirmation.

## Disaster Recovery Evidence

Generate a report:

```bash
convex-autobackup dr-report --json
```

Inspect audit entries:

```bash
convex-autobackup audit --json
```

Keep evidence of:

- Backup schedule.
- Retention settings.
- Recent successful backup runs.
- Recent verification results.
- Last restore drill result.
- Storage destination.
- Operator notes for failed jobs.

## Retention

Every destination (local folder or S3/R2 bucket) enforces `keep_last` after each successful backup, so old copies are cleaned up automatically. History length is `keep_last × schedule interval`; for example 28 backups every 6 hours keeps 7 days. Choose a value that covers delayed discovery of application bugs, accidental deletes, and operator error.

Suggested starting points:

- Production: keep at least 14 to 30 successful backups.
- Client projects: keep at least 7 successful backups.
- Development: keep enough for your rollback window.

## Recovering Without The App

Keep the backup passphrase in a password manager. If the server is lost, download a `.zip.age` archive and decrypt it anywhere:

```bash
age -d backup.zip.age > backup.zip
CONVEX_AUTOBACKUP_PASSPHRASE='<passphrase>' convex-autobackup decrypt backup.zip.age --out backup.zip
```

Then import it into the deployment you choose with `npx convex import --replace backup.zip`.

## Upgrade Procedure

Docker Compose installs: run `scripts/docker-update.sh` (pulls, rebuilds, restarts; the `data` volume is kept). Otherwise:


1. Record the currently running version.
2. Back up the ConvexAutoBackup data directory.
3. Confirm `CONVEX_AUTOBACKUP_MASTER_KEY` is recoverable.
4. Read release notes.
5. Pull the new Docker image or install the new native release.
6. Restart the service.
7. Run `convex-autobackup doctor --json`.
8. Run and verify a manual backup.

## Rollback Procedure

1. Stop the service.
2. Restore the previous binary or container image.
3. Restore the previous data directory if the new version migrated state incompatibly.
4. Start the service.
5. Confirm health, run history, and a manual backup.
