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

## Deploy Key Check Fails In The Wizard

- `Invalid Convex deploy key`: copy the key again from Convex dashboard → Settings → Deploy keys; it looks like `prod:name-123|…`.
- `timed out`: the server cannot reach `api.convex.dev`; check outbound network and DNS.
- The wizard warns when the key is not a production key; that is allowed but only backs up that deployment.

## Destination Test Or Upload Fails

The error now includes the cause. Common ones:

- `403`/`SignatureDoesNotMatch`: wrong access key or secret, or the token is not scoped to that bucket.
- `NoSuchBucket`: bucket name or Cloudflare account ID is wrong (R2 endpoint is `https://<account-id>.r2.cloudflarestorage.com`, region `auto`).
- Connection errors: the server cannot reach the endpoint.

A run whose offsite copy failed is marked `partial`; the local copy is still valid. Fix the destination and use **Test** on its card.

## Wrong Or Lost Backup Passphrase

- Verification or restore failing with `wrong passphrase?` means the archive was written with a different passphrase than the one stored for it. Archives remember which stored passphrase they used, so this only happens if the app's secrets were replaced.
- A lost passphrase cannot be recovered: encrypted archives are unreadable without it. Set a new passphrase so future backups are recoverable, and run a new backup immediately.

## Remote URL Shows Cloudflare Login Or Errors

- A Cloudflare Access login page is expected; sign in with an allowed email.
- `502`/`1033`: the tunnel container is not connected. Check `docker logs convex-autobackup-tunnel` and that `TUNNEL_TOKEN` is set in the env file.

## Cargo Install Fails

```bash
rustup update stable
cargo install convex-autobackup --version 0.1.0-beta.7
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
