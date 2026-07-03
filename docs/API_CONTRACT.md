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
- `POST /api/v1/tokens`
- `GET /api/v1/secrets`
- `POST /api/v1/secrets`
- `GET /api/v1/projects`
- `POST /api/v1/projects`
- `POST /api/v1/targets/cloud`
- `POST /api/v1/destinations/local`
- `POST /api/v1/destinations/s3`
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

Browser sessions, scoped token permissions, logs, settings, and Postgres administration are roadmap resource groups.

`POST /api/v1/bootstrap` is available only before any user exists. It creates the first owner and returns a one-time bootstrap API token.

## Rules

- API responses use JSON.
- Secrets are returned only as redacted metadata and stable references.
- Mutating endpoints require CSRF/session protection for browser sessions or bearer tokens for agents.
- OpenAPI must be generated or kept in sync with implemented routes.
- New endpoint behavior requires tests covering success, auth failure, validation failure, and permission failure.
