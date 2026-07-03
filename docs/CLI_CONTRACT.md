# CLI Contract

Binary name:

```bash
convex-autobackup
```

## Principles

- Every read/list/status command supports `--json`.
- JSON output is stable within a major version.
- Mutating commands return nonzero exit codes on partial or failed work.
- Destructive commands require explicit confirmation flags.
- Commands must be usable by AI agents without interactive prompts when all required flags are provided.

## Implemented Command Groups

- `serve`: start the web/API service.
- `supervise`: start the web/API service and scheduler worker in one supervised process.
- `init`: initialize SQLite state and print paths.
- `health`: print local CLI/service health.
- `doctor`: validate install, service, worker, database, master-key, and managed runner readiness.
- `runner`: install or inspect the pinned Convex CLI runner.
- `user`: create users.
- `token`: create API tokens.
- `secret`: store and list encrypted secrets.
- `project`: manage project records.
- `target`: manage Convex targets.
- `destination`: manage local and S3-compatible destinations.
- `job`: manage backup jobs.
- `schedule`: create/list schedules and run due jobs.
- `backup`: trigger backup runs.
- `verify`: verify backup archives and manifests.
- `restore`: restore a verified backup to a confirmed target deployment.
- `dr-report`: generate disaster recovery evidence from run history.
- `audit`: inspect audit events.
- `runs`: inspect run history.

## Future Command Groups

- `dr`: run drills and DR readiness checks.
- `reports`: export DR evidence.
- `logs`: inspect structured logs.
- `config`: inspect and validate app configuration.

## Exit Codes

- `0`: success.
- `1`: user input or validation error.
- `2`: authentication or authorization failure.
- `3`: service unavailable.
- `4`: backup, restore, or verification failed.
- `5`: partial success requiring operator review.
