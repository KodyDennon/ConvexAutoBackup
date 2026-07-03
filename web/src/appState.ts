export type Role = "owner" | "admin" | "operator" | "viewer";
export type SecretKind = "convex_deploy_key" | "s3_credentials" | "webhook_token" | "encryption_key";
export type BackupStatus = "queued" | "running" | "succeeded" | "failed" | "canceled" | "partial";
export type Readiness = "ready" | "needs_setup" | "at_risk";

export interface HealthResponse {
  status: string;
  service: string;
  version: string;
  database_path: string;
  users_configured: boolean;
}

export interface User {
  id: string;
  email: string;
  role: Role;
  created_at: string;
}

export interface ApiToken {
  id: string;
  user_id: string;
  name: string;
  token?: string;
  created_at: string;
  revoked_at?: string | null;
}

export interface StoredSecret {
  id: string;
  label: string;
  kind: SecretKind;
  created_at: string;
  updated_at: string;
}

export interface Project {
  id: string;
  team_id: string;
  name: string;
  description?: string | null;
  created_at: string;
}

export interface ConvexTarget {
  id: string;
  project_id: string;
  name: string;
  kind: "cloud" | "self_hosted";
  deployment: string;
  url?: string | null;
  secret: { id: string; label: string };
}

export type StorageKind =
  | { type: "local_filesystem"; root: string }
  | {
      type: "s3_compatible";
      bucket: string;
      region?: string | null;
      endpoint?: string | null;
      prefix?: string | null;
      credentials: { id: string; label: string };
    };

export interface RetentionPolicy {
  keep_last?: number | null;
  keep_days?: number | null;
  keep_weeklies?: number | null;
  keep_monthlies?: number | null;
}

export interface StorageDestination {
  id: string;
  team_id: string;
  name: string;
  kind: StorageKind;
  retention: RetentionPolicy;
}

export interface BackupJob {
  id: string;
  project_id: string;
  target_id: string;
  destination_id: string;
  name: string;
  include_file_storage: boolean;
  schedule_enabled: boolean;
}

export type Schedule =
  | { type: "interval_minutes"; every: number }
  | { type: "daily"; time: string }
  | { type: "weekly"; weekday: string; time: string }
  | { type: "cron"; expression: string };

export interface JobSchedule {
  id: string;
  job_id: string;
  schedule: Schedule;
  missed_run_policy: "run_once_on_resume" | "skip";
  next_due_at: string;
}

export interface BackupRun {
  id: string;
  job_id: string;
  status: BackupStatus;
  started_at: string;
  finished_at?: string | null;
  manifest_path?: string | null;
  error?: string | null;
}

export interface RunRecord {
  run: BackupRun;
  manifest_json?: string | null;
}

export interface AuditEvent {
  id: string;
  actor: string;
  action: string;
  resource_type: string;
  resource_id?: string | null;
  message: string;
  created_at: string;
}

export interface DrReport {
  generated_at: string;
  readiness: Readiness;
  latest_successful_run?: BackupRun | null;
  successful_run_count: number;
  failed_run_count: number;
  configured_job_count: number;
  findings: string[];
}

export interface ServiceState {
  health: HealthResponse | null;
  users: User[];
  tokens: ApiToken[];
  secrets: StoredSecret[];
  projects: Project[];
  targets: ConvexTarget[];
  destinations: StorageDestination[];
  jobs: BackupJob[];
  schedules: JobSchedule[];
  runs: RunRecord[];
  auditEvents: AuditEvent[];
  drReport: DrReport | null;
}

export interface DashboardInput {
  projects: Project[];
  targets: ConvexTarget[];
  destinations: StorageDestination[];
  jobs: BackupJob[];
  schedules: JobSchedule[];
  runs: RunRecord[];
  drReport?: DrReport | null;
}

export interface DashboardStat {
  label: string;
  value: string;
  detail: string;
  readiness: Readiness;
}

export class ApiClient {
  constructor(
    private readonly token: string | null,
    private readonly baseUrl = ""
  ) {}

  async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const headers = new Headers(options.headers);
    if (!headers.has("Content-Type") && options.body) {
      headers.set("Content-Type", "application/json");
    }
    if (this.token) {
      headers.set("Authorization", `Bearer ${this.token}`);
    }

    const response = await fetch(`${this.baseUrl}${path}`, { ...options, headers });
    const text = await response.text();
    const body = text ? JSON.parse(text) : {};
    if (!response.ok) {
      throw new Error(body.error ?? `Request failed with ${response.status}`);
    }
    return body as T;
  }
}

export function buildDashboardStats(input: DashboardInput): DashboardStat[] {
  const latestRun = input.runs[0]?.run;
  const latestRunText = latestRun ? `${sentenceCase(latestRun.status)} ${relativeTime(latestRun.started_at)}` : "No runs yet";
  const latestRunReady: Readiness =
    latestRun?.status === "succeeded" ? "ready" : latestRun ? "at_risk" : "needs_setup";

  return [
    {
      label: "Protected deployments",
      value: input.targets.length.toLocaleString("en-US"),
      detail: `${input.projects.length} projects, ${input.jobs.length} jobs`,
      readiness: input.targets.length > 0 && input.jobs.length > 0 ? "ready" : "needs_setup"
    },
    {
      label: "Destinations",
      value: input.destinations.length.toLocaleString("en-US"),
      detail: input.destinations.some((destination) => destination.kind.type === "s3_compatible")
        ? "Local and offsite options configured"
        : "Add S3-compatible offsite storage for stronger DR",
      readiness: input.destinations.length > 0 ? "ready" : "needs_setup"
    },
    {
      label: "Next scheduled run",
      value: input.schedules[0] ? formatDateTime(input.schedules[0].next_due_at) : "Not scheduled",
      detail: input.schedules[0] ? describeSchedule(input.schedules[0].schedule) : "Create an interval, daily, weekly, or cron schedule",
      readiness: input.schedules.length > 0 ? "ready" : "needs_setup"
    },
    {
      label: "Latest backup",
      value: latestRunText,
      detail: latestRun?.manifest_path ?? latestRun?.error ?? "Run a job to produce the first manifest",
      readiness: latestRunReady
    },
    {
      label: "DR readiness",
      value: input.drReport ? sentenceCase(input.drReport.readiness) : "Unknown",
      detail: input.drReport?.findings[0] ?? "DR report will evaluate jobs, runs, and failures",
      readiness: input.drReport?.readiness ?? "needs_setup"
    }
  ];
}

export function describeSchedule(schedule: Schedule): string {
  if (schedule.type === "interval_minutes") {
    return `Every ${schedule.every} minutes`;
  }
  if (schedule.type === "daily") {
    return `Daily at ${schedule.time}`;
  }
  if (schedule.type === "weekly") {
    return `Weekly on ${sentenceCase(schedule.weekday)} at ${schedule.time}`;
  }
  return schedule.expression;
}

export function destinationLabel(destination: StorageDestination): string {
  if (destination.kind.type === "local_filesystem") {
    return destination.kind.root;
  }
  return [destination.kind.bucket, destination.kind.endpoint].filter(Boolean).join(" @ ");
}

export function formatDateTime(value?: string | null): string {
  if (!value) {
    return "Never";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(value));
}

export function relativeTime(value: string): string {
  const diffMs = Date.now() - new Date(value).getTime();
  const minutes = Math.round(diffMs / 60000);
  if (Math.abs(minutes) < 1) {
    return "just now";
  }
  if (Math.abs(minutes) < 60) {
    return `${minutes}m ago`;
  }
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 48) {
    return `${hours}h ago`;
  }
  return formatDateTime(value);
}

export function sentenceCase(value: string): string {
  return value.replaceAll("_", " ").replace(/^\w/, (match) => match.toUpperCase());
}
