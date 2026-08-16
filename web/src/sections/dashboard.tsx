import { Activity, Clock3, DatabaseBackup, FolderGit2, HardDrive, Layers, Play, ShieldCheck, Terminal } from "lucide-react";
import {
  buildDashboardStats,
  formatBytes,
  relativeTime,
  sentenceCase,
  type DashboardStat,
  type Project,
  type ServiceState
} from "../appState";
import { FindingList, PanelHeader, RunList } from "../components/common";

export function Dashboard({
  stats,
  state,
  onSelectProject,
  onRunJob,
  actionLoading
}: {
  stats: DashboardStat[];
  state: ServiceState;
  onSelectProject?: (projectId: string) => void;
  onRunJob?: (jobId: string) => void;
  actionLoading?: string | null;
}) {
  const projects = state.projects ?? [];
  const targets = state.targets ?? [];
  const jobs = state.jobs ?? [];
  const runs = state.runs ?? [];

  return (
    <div className="page-stack">
      {/* Top Metrics Cards */}
      <section className="grid metrics-grid">
        {stats.map((stat) => (
          <article className={`metric ${stat.readiness}`} key={stat.label}>
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
            <p>{stat.detail}</p>
          </article>
        ))}
      </section>

      {/* Projects Fleet & Isolation Overview */}
      <section className="panel">
        <PanelHeader
          icon={<Layers size={18} />}
          title="Protected Convex Projects & Environments"
          detail={`${projects.length} project${projects.length === 1 ? "" : "s"} monitored`}
        />
        {projects.length === 0 ? (
          <p className="empty">No projects configured yet. Use the Setup tab to create your first project.</p>
        ) : (
          <div className="grid split-even" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", gap: "1rem" }}>
            {projects.map((proj) => {
              const projTargets = targets.filter((t) => t.project_id === proj.id);
              const projJobs = jobs.filter((j) => j.project_id === proj.id);
              const projJobIds = new Set(projJobs.map((j) => j.id));
              const projRuns = runs.filter((r) => projJobIds.has(r.run.job_id));

              let projStorageBytes = 0;
              let successfulCount = 0;
              for (const record of projRuns) {
                if (record.manifest_json) {
                  try {
                    const parsed = JSON.parse(record.manifest_json);
                    const bytes = parsed.archive_size_bytes ?? parsed.archive_size ?? parsed.bytes_exported ?? 0;
                    projStorageBytes += Number(bytes) || 0;
                    successfulCount++;
                  } catch {}
                }
              }

              const latestRun = projRuns[0]?.run;

              return (
                <div key={proj.id} style={{ background: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px", padding: "1.1rem", display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
                  <div>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "0.5rem" }}>
                      <div>
                        <h3 style={{ margin: 0, fontSize: "1.05rem", fontWeight: 700, color: "#0f172a" }}>{proj.name}</h3>
                        {proj.description && <p className="subtle" style={{ margin: "0.25rem 0 0", fontSize: "0.82rem" }}>{proj.description}</p>}
                      </div>
                      <span className="badge success" style={{ fontSize: "0.75rem" }}>Active</span>
                    </div>

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.5rem", margin: "0.85rem 0", background: "#f8fafc", padding: "0.75rem", borderRadius: "8px", border: "1px solid #f1f5f9" }}>
                      <div>
                        <span className="subtle" style={{ fontSize: "0.75rem", display: "block" }}>Target Deployments</span>
                        <strong style={{ fontSize: "0.92rem", color: "#0369a1" }}>
                          {projTargets.length > 0 ? projTargets.map(t => t.deployment).join(", ") : "None connected"}
                        </strong>
                      </div>
                      <div>
                        <span className="subtle" style={{ fontSize: "0.75rem", display: "block" }}>Vault Storage</span>
                        <strong style={{ fontSize: "0.92rem", color: "#0f172a" }}>{formatBytes(projStorageBytes)}</strong>
                      </div>
                    </div>

                    <div style={{ fontSize: "0.82rem", color: "#475569" }}>
                      <span>Latest Status: </span>
                      {latestRun ? (
                        <span className={`status-pill ${latestRun.status}`}>
                          {sentenceCase(latestRun.status)} ({relativeTime(latestRun.started_at)})
                        </span>
                      ) : (
                        <span className="subtle">No backups executed</span>
                      )}
                    </div>
                  </div>

                  <div style={{ display: "flex", gap: "0.5rem", marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid #f1f5f9" }}>
                    {onSelectProject && (
                      <button
                        type="button"
                        className="secondary-button small"
                        onClick={() => onSelectProject(proj.id)}
                      >
                        <FolderGit2 size={14} /> Filter Scope
                      </button>
                    )}
                    {projJobs[0] && onRunJob && (
                      <button
                        type="button"
                        className="primary-button small"
                        disabled={actionLoading === `run-${projJobs[0].id}`}
                        onClick={() => onRunJob(projJobs[0].id)}
                      >
                        <Play size={14} /> Run Backup
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>

      {/* Recent Runs & DR Posture Split */}
      <section className="split">
        <div className="panel">
          <PanelHeader icon={<Clock3 size={18} />} title="Recent backup runs" detail={`${state.runs.length} recorded runs`} />
          <RunList runs={state.runs.slice(0, 6)} jobs={state.jobs} compact />
        </div>
        <div className="panel">
          <PanelHeader icon={<ShieldCheck size={18} />} title="DR posture" detail={state.drReport ? sentenceCase(state.drReport.readiness) : "No report"} />
          <FindingList findings={state.drReport?.findings ?? ["Configure a job and run the first backup."]} />
        </div>
      </section>

      {/* CLI & MCP Agent Command Reference */}
      <section className="panel">
        <PanelHeader icon={<Terminal size={18} />} title="Agent & MCP Command Surface" detail="Operate state via CLI, JSON API, and MCP stdio." />
        <div className="command-grid">
          <code>convex-autobackup health --json</code>
          <code>convex-autobackup backup run --job-id &lt;id&gt; --json</code>
          <code>convex-autobackup verify --run-id &lt;id&gt; --json</code>
          <code>convex-autobackup dr-report --json</code>
        </div>
      </section>
    </div>
  );
}

export function dashboardStats(state: ServiceState) {
  return buildDashboardStats({
    projects: state.projects,
    targets: state.targets,
    destinations: state.destinations,
    jobs: state.jobs,
    schedules: state.schedules,
    runs: state.runs,
    drReport: state.drReport
  });
}
