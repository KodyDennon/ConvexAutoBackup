# API Contract

The HTTP API is versioned under:

```text
/api/v1
```

## Current Endpoints

- `GET /api/v1/health`
- `GET /api/v1/capabilities`
- `GET /api/v1/openapi.json`
- `POST /api/v1/bootstrap`
- `POST /api/v1/login`
- `GET /api/v1/users`
- `POST /api/v1/users`
- `GET /api/v1/tokens`
- `POST /api/v1/tokens`
- `DELETE /api/v1/tokens/{token_id}`
- `GET /api/v1/secrets`
- `POST /api/v1/secrets`
- `GET /api/v1/projects`
- `POST /api/v1/projects`
- `GET /api/v1/targets`
- `POST /api/v1/targets/cloud`
- `GET /api/v1/destinations`
- `POST /api/v1/destinations/local`
- `POST /api/v1/destinations/s3` (accepts inline `access_key_id`/`secret_access_key` and `encryption_passphrase`)
- `PUT /api/v1/destinations/{destination_id}/encryption` (set, change, or disable the passphrase)
- `POST /api/v1/destinations/{destination_id}/test` (write, read and delete a probe object)
- `POST /api/v1/targets/{target_id}/test` (read-only Convex connection check)
- `POST /api/v1/setup/check-deploy-key` (validate a deploy key before saving it)
- `GET /api/v1/setup/presets` (wizard defaults and pre-provisioned storage)
- `POST /api/v1/setup/presets/r2` (create a destination from the pre-provisioned R2 bucket)
- `GET /api/v1/system/checks` (install health for the setup wizard)
- `GET /api/v1/jobs`
- `POST /api/v1/jobs`
- `GET /api/v1/schedules`
- `POST /api/v1/schedules`
- `POST /api/v1/schedules/run-due`
- `POST /api/v1/jobs/{job_id}/run`
- `POST /api/v1/runs/{run_id}/verify`
- `POST /api/v1/restore`
- `GET /api/v1/dr/report`
- `GET /api/v1/audit`
- `GET /api/v1/runs` (each run includes its per-destination `copies`)

## Future Resource Groups

Scoped token permissions, logs, settings, and Postgres administration are roadmap resource groups.

`POST /api/v1/bootstrap` is available only before any user exists. It creates the first owner and returns a one-time bootstrap API token.

`POST /api/v1/login` validates email/password credentials and returns a revocable API token for browser or agent use.

## Rules

- API responses use JSON.
- Secrets are returned only as redacted metadata and stable references. Backup passphrases and S3 credentials are never returned.
- The API does not send CORS headers; the web console is served from the same origin.
- Mutating endpoints require CSRF/session protection for browser sessions or bearer tokens for agents.
- OpenAPI must be generated or kept in sync with implemented routes.
- New endpoint behavior requires tests covering success, auth failure, validation failure, and permission failure.
