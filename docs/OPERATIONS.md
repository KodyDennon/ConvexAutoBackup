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

Local destinations currently enforce `keep_last` after successful backup completion. Choose a retention value that covers delayed discovery of application bugs, accidental deletes, and operator error.

Suggested starting points:

- Production: keep at least 14 to 30 successful backups.
- Client projects: keep at least 7 successful backups.
- Development: keep enough for your rollback window.

## Upgrade Procedure

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
