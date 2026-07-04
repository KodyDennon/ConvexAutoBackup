import { describe, expect, it, vi } from "vitest";
import {
  buildDashboardStats,
  describeSchedule,
  destinationLabel,
  type BackupJob,
  type ConvexTarget,
  type DashboardInput,
  type Project,
  type StorageDestination
} from "./appState";

const project: Project = {
  id: "project-1",
  team_id: "team-1",
  name: "Production",
  created_at: "2026-07-03T10:00:00Z"
};

const target: ConvexTarget = {
  id: "target-1",
  project_id: "project-1",
  name: "Prod",
  kind: "cloud",
  deployment: "prod:careful-otter-123",
  secret: { id: "secret-1", label: "prod deploy key" }
};

const destination: StorageDestination = {
  id: "destination-1",
  team_id: "team-1",
  name: "Vault",
  kind: { type: "local_filesystem", root: "/var/lib/convex-autobackup/backups" },
  retention: { keep_last: 20, keep_days: 30, keep_weeklies: 12, keep_monthlies: 12 }
};

const job: BackupJob = {
  id: "job-1",
  project_id: "project-1",
  target_id: "target-1",
  destination_id: "destination-1",
  name: "Nightly full backup",
  include_file_storage: true,
  schedule_enabled: true
};

function dashboardInput(overrides: Partial<DashboardInput> = {}): DashboardInput {
  return {
    projects: [],
    targets: [],
    destinations: [],
    jobs: [],
    schedules: [],
    runs: [],
    drReport: null,
    ...overrides
  };
}

describe("dashboard state helpers", () => {
  it("marks a configured service with a successful run as ready", () => {
    vi.setSystemTime(new Date("2026-07-03T12:00:00Z"));
    const stats = buildDashboardStats(
      dashboardInput({
        projects: [project],
        targets: [target],
        destinations: [destination],
        jobs: [job],
        schedules: [
          {
            id: "schedule-1",
            job_id: "job-1",
            schedule: { type: "interval_minutes", every: 60 },
            missed_run_policy: "run_once_on_resume",
            next_due_at: "2026-07-03T13:00:00Z"
          }
        ],
        runs: [
          {
            run: {
              id: "run-1",
              job_id: "job-1",
              status: "succeeded",
              started_at: "2026-07-03T11:30:00Z",
              finished_at: "2026-07-03T11:31:00Z",
              manifest_path: "/vault/manifest.json"
            }
          }
        ],
        drReport: {
          generated_at: "2026-07-03T12:00:00Z",
          readiness: "ready",
          latest_successful_run: null,
          successful_run_count: 1,
          failed_run_count: 0,
          configured_job_count: 1,
          findings: ["Latest successful backup is available."]
        }
      })
    );

    expect(stats.map((stat) => stat.readiness)).toEqual(["ready", "ready", "ready", "ready", "ready"]);
    expect(stats.find((stat) => stat.label === "Latest backup")?.value).toContain("Succeeded");
  });

  it("keeps an empty service in setup state", () => {
    const stats = buildDashboardStats(dashboardInput());
    expect(stats.every((stat) => stat.readiness === "needs_setup")).toBe(true);
  });

  it("tolerates partial runtime API data without crashing", () => {
    const stats = buildDashboardStats({
      ...dashboardInput(),
      runs: undefined,
      schedules: undefined,
      drReport: {
        generated_at: "2026-07-03T12:00:00Z",
        readiness: "needs_setup",
        latest_successful_run: null,
        successful_run_count: 0,
        failed_run_count: 0,
        configured_job_count: 0
      }
    } as unknown as DashboardInput);

    expect(stats.find((stat) => stat.label === "DR readiness")?.detail).toBe(
      "DR report will evaluate jobs, runs, and failures"
    );
  });

  it("describes schedules and destinations for operator-facing labels", () => {
    expect(describeSchedule({ type: "interval_minutes", every: 15 })).toBe("Every 15 minutes");
    expect(describeSchedule({ type: "cron", expression: "0 0 2 * * *" })).toBe("0 0 2 * * *");
    expect(destinationLabel(destination)).toBe("/var/lib/convex-autobackup/backups");
  });
});
