import { Clock3, RefreshCw, RotateCcw } from "lucide-react";
import {
  ApiClient,
  describeSchedule,
  formatDateTime,
  sentenceCase,
  type ServiceState
} from "../appState";
import { FindingList, PanelHeader, SimpleTable, StatusTile } from "../components/common";

export function DrSection({
  client,
  state,
  actionLoading,
  perform
}: {
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;
}) {
  return (
    <div className="page-stack">
      <section className="panel">
        <PanelHeader icon={<RotateCcw size={18} />} title="Disaster recovery report" detail={state.drReport ? `Generated ${formatDateTime(state.drReport.generated_at)}` : "No report"} />
        <div className="dr-grid">
          <StatusTile label="Readiness" value={state.drReport ? sentenceCase(state.drReport.readiness) : "Unknown"} />
          <StatusTile label="Configured jobs" value={String(state.drReport?.configured_job_count ?? state.jobs.length)} />
          <StatusTile label="Successful runs" value={String(state.drReport?.successful_run_count ?? 0)} />
          <StatusTile label="Failed runs" value={String(state.drReport?.failed_run_count ?? 0)} />
        </div>
        <FindingList findings={state.drReport?.findings ?? []} />
        <div className="button-row">
          <button
            className="secondary-button"
            type="button"
            disabled={actionLoading === "dr-refresh"}
            onClick={() =>
              void perform("dr-refresh", async () => {
                await client.request("/api/v1/dr/report");
                return "DR report refreshed.";
              })
            }
          >
            <RefreshCw size={16} /> Refresh report
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={actionLoading === "run-due"}
            onClick={() =>
              void perform("run-due", async () => {
                await client.request("/api/v1/schedules/run-due", { method: "POST" });
                return "Due schedules processed.";
              })
            }
          >
            <Clock3 size={16} /> Run due schedules
          </button>
        </div>
      </section>

      <section className="panel">
        <PanelHeader icon={<Clock3 size={18} />} title="Schedules" detail={`${state.schedules.length} active definitions`} />
        <SimpleTable
          headers={["Job", "Schedule", "Missed runs", "Next due"]}
          rows={state.schedules.map((schedule) => [
            state.jobs.find((job) => job.id === schedule.job_id)?.name ?? schedule.job_id,
            describeSchedule(schedule.schedule),
            sentenceCase(schedule.missed_run_policy),
            formatDateTime(schedule.next_due_at)
          ])}
          emptyMessage="Create schedules from Setup to automate backup runs."
        />
      </section>
    </div>
  );
}
