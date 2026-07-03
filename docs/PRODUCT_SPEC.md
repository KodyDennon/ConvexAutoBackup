# Product Spec

## Goal

ConvexAutoBackup gives users a reliable, self-hosted way to schedule, store, verify, restore, and report on full Convex backups.

The product must feel like an operations tool, not a demo. The first screen is the backup control plane, not a marketing page. The UI should make current protection state, next scheduled run, latest backup, storage health, and DR readiness immediately visible.

## Audiences

- Solo developers backing up personal or client Convex projects.
- Agencies managing many deployments across clients.
- Teams that need role-based access, audit trails, and restore evidence.
- AI agents that need noninteractive commands, JSON output, HTTP APIs, and MCP tools.

## Core Capabilities

- Add Convex Cloud deployments using deploy keys.
- Add self-hosted Convex deployments using URL and admin-key style credentials.
- Store multiple credentials and link each secret to the correct target or destination.
- Create full backup jobs that include file storage by default.
- Schedule jobs by interval, daily time, weekly time, or guided cron.
- Store backups on local filesystem destinations and S3-compatible destinations.
- Optionally encrypt backup archives per destination.
- Verify backups with checksums and manifest metadata.
- Run restore drills and restore to alternate deployments.
- Gate destructive restore to the original deployment behind strong confirmations.
- Export DR evidence reports for teams, agencies, and clients.

## Non-Goals

- Operating a hosted SaaS.
- Code signing in the first release.
- Depending on a user's random global Convex CLI installation for correctness.
- Treating email, Slack, Discord, or vendor-specific notifications as required foundation features.

## Success Criteria

- A user can install the app with one command and reach a polished local/LAN web UI.
- A user can create an admin account before managing real secrets.
- A user can configure a Convex target, backup destination, schedule, and retention policy.
- The worker can run backups through a bounded queue with clear logs and retry behavior.
- The app can verify stored backups and produce a manifest.
- The CLI and API expose the same operational state that the UI uses.
- Agents can inspect health and invoke safe operations through MCP.
- Documentation explains architecture, security, backup, restore, deployment, and contribution workflows.

