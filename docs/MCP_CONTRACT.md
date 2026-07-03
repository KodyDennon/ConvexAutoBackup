# MCP Contract

The MCP server runs over stdio:

```bash
convex-autobackup-mcp
```

## Principles

- MCP tools use the same authorization model as the HTTP API.
- Tools are safe by default.
- Destructive restore is disabled by default.
- Tool output must be concise, structured, and suitable for agents.

## Foundation Tools

- `health`: reports MCP server health.
- `capabilities`: reports supported backup, storage, schedule, and agent surfaces.

## Planned Tools

- `list_projects`
- `list_targets`
- `list_destinations`
- `list_schedules`
- `list_backup_runs`
- `get_run_logs`
- `trigger_backup`
- `verify_backup`
- `generate_dr_report`
- `start_restore_drill`

## Destructive Restore Policy

MCP destructive restore tools are unavailable unless the server has an explicit configuration gate and the token has a destructive-restore scope. Even then, destructive restore must require the same confirmation token flow used by the UI and CLI.

