# Disaster Recovery Model

## Goals

The DR system must prove that backups are restorable, not just that files exist.

It supports:

- Backup verification.
- Restore-to-alternate deployment.
- Restore drills.
- Guarded destructive restore.
- RPO and RTO reporting.
- Evidence exports for teams, agencies, and clients.

## Restore-To-Alternate

Restore-to-alternate is the preferred safe restore path. The current restore engine imports a selected, verified backup into any configured Convex target whose exact deployment string is supplied as confirmation. It records:

- Source backup.
- Destination deployment.
- Operator.
- Start and finish times.
- Convex import command version.
- Result.
- Logs.

## Destructive Restore

Convex import is a destructive replace operation. The current restore engine requires exact target deployment confirmation and verification success before invoking import.

Required safeguards:

- Owner or explicitly authorized admin role.
- Exact target deployment confirmation.
- Backup checksum verification before import.
- Confirmation token generated immediately before restore.
- Audit entry before and after the operation.
- Clear failure state if import fails.

## Restore Drills

Restore drills are planned recovery exercises. A drill can verify a backup, restore to an alternate deployment, record timing, and generate a report.

## RPO/RTO

RPO is calculated from the latest successful backup age for each deployment. RTO is calculated from restore drill and restore run durations.

Reports must show:

- Protected deployments.
- Latest successful backup.
- Latest verification.
- Latest restore drill.
- RPO status.
- RTO evidence.
- Open risks.

## Reports

Reports can be generated from the CLI and API. The current report summarizes total runs, successful runs, failed runs, latest success/failure timestamps, latest manifest path, and readiness state. Reports are read-only artifacts and do not expose secrets.
