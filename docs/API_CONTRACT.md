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
- `POST /api/v1/destinations/s3`
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
- `GET /api/v1/runs`

## Future Resource Groups

Scoped token permissions, logs, settings, and Postgres administration are roadmap resource groups.

`POST /api/v1/bootstrap` is available only before any user exists. It creates the first owner and returns a one-time bootstrap API token.

`POST /api/v1/login` validates email/password credentials and returns a revocable API token for browser or agent use.

## Rules

- API responses use JSON.
- Secrets are returned only as redacted metadata and stable references.
- Mutating endpoints require CSRF/session protection for browser sessions or bearer tokens for agents.
- OpenAPI must be generated or kept in sync with implemented routes.
- New endpoint behavior requires tests covering success, auth failure, validation failure, and permission failure.
