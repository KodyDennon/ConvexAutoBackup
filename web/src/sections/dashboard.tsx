import { Activity, Clock3, ShieldCheck, Terminal } from "lucide-react";
import {
  buildDashboardStats,
  type ServiceState,
  type DashboardStat,
  sentenceCase
} from "../appState";
import { FindingList, PanelHeader, RunList } from "../components/common";

export function Dashboard({ stats, state }: { stats: DashboardStat[]; state: ServiceState }) {
  return (
    <div className="page-stack">
      <section className="grid metrics-grid">
        {stats.map((stat) => (
          <article className={`metric ${stat.readiness}`} key={stat.label}>
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
            <p>{stat.detail}</p>
          </article>
        ))}
      </section>

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

      <section className="panel">
        <PanelHeader icon={<Terminal size={18} />} title="Agent surface" detail="Use the same service state through CLI, JSON API, and MCP stdio." />
        <div className="command-grid">
          <code>convex-autobackup health --json</code>
          <code>convex-autobackup schedule run-due --json</code>
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
