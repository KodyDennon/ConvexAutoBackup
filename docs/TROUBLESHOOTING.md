# Troubleshooting

Collect these first:

```bash
convex-autobackup --version
convex-autobackup doctor --json
curl http://localhost:8976/api/v1/health
```

## Docker Container Does Not Start

```bash
docker ps -a
docker logs convex-autobackup
docker inspect convex-autobackup
```

Common causes:

- Missing `CONVEX_AUTOBACKUP_MASTER_KEY`.
- `/data` is not writable.
- Port `8976` is already in use.
- Host cannot download or run the managed Convex runner.

## Web UI Is Not Reachable

```bash
curl http://localhost:8976/api/v1/health
```

For Docker:

```bash
docker ps
```

For remote servers, check firewall rules, reverse proxy rules, DNS, and TLS certificate state.

## Bootstrap Or Login Fails

Bootstrap only works before the first user exists. After owner creation, use normal login or owner/admin user management.

If owner access is lost, stop and back up the data directory before manual database repair.

## Convex Export Fails

Check:

- Deploy key is valid.
- Deployment name is exact.
- Deploy key can access the project.
- Host can reach Convex services.
- Managed runner is installed.

```bash
convex-autobackup runner install --json
convex-autobackup doctor --json
```

## Verification Fails

Possible causes:

- Archive upload was interrupted.
- Object storage returned stale or partial data.
- Local files were moved or edited manually.
- Manifest and archive no longer match.

Do not delete the failed artifact until you know whether it is the only copy for that recovery window.

## Restore Is Blocked

Restore requires exact target confirmation:

```bash
convex-autobackup restore \
  --run-id <run-id> \
  --target-id <target-id> \
  --confirm-deployment <deployment-name> \
  --json
```

Use the deployment name from the configured target.

## Cargo Install Fails

```bash
rustup update stable
cargo install convex-autobackup --version 0.1.0-beta.1
```

If the release was just published, crates.io indexing can lag for a few minutes.

## Release Artifact Missing

Check Actions:

```text
https://github.com/KodyDennon/ConvexAutoBackup/actions/workflows/release.yml
```

Check releases:

```text
https://github.com/KodyDennon/ConvexAutoBackup/releases/latest
```

If a release partially published, rerun after fixing the failed job. The crates.io publish job skips exact versions that are already published.
