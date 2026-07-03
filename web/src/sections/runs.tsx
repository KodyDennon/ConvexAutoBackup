import { useEffect, useState } from "react";
import { CheckCircle2, Clock3, Play, RotateCcw } from "lucide-react";
import { ApiClient, formatDateTime, type ServiceState } from "../appState";
import { EmptyRow, Field, PanelHeader, RunList, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

export function RunsSection({
  client,
  state,
  actionLoading,
  perform
}: {
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
}) {
  const [restoreRunId, setRestoreRunId] = useState("");
  const [restoreTargetId, setRestoreTargetId] = useState("");
  const [confirmDeployment, setConfirmDeployment] = useState("");

  useEffect(() => {
    if (!restoreRunId && state.runs[0]) setRestoreRunId(state.runs[0].run.id);
    if (!restoreTargetId && state.targets[0]) setRestoreTargetId(state.targets[0].id);
  }, [restoreRunId, restoreTargetId, state.runs, state.targets]);

  const restoreTarget = state.targets.find((target) => target.id === restoreTargetId);

  return (
    <div className="page-stack">
      <section className="panel">
        <PanelHeader icon={<Play size={18} />} title="Backup jobs" detail={`${state.jobs.length} configured jobs`} />
        <div className="table">
          <div className="table-row table-head">
            <span>Name</span>
            <span>Target</span>
            <span>Destination</span>
            <span>Files</span>
            <span>Action</span>
          </div>
          {state.jobs.map((job) => (
            <div className="table-row" key={job.id}>
              <span>{job.name}</span>
              <span>{state.targets.find((target) => target.id === job.target_id)?.deployment ?? job.target_id}</span>
              <span>{state.destinations.find((destination) => destination.id === job.destination_id)?.name ?? job.destination_id}</span>
              <span>{job.include_file_storage ? "Included" : "Database only"}</span>
              <button
                className="small-button"
                type="button"
                disabled={actionLoading === `run-${job.id}`}
                onClick={() =>
                  void perform(`run-${job.id}`, async () => {
                    await client.request(`/api/v1/jobs/${job.id}/run`, { method: "POST" });
                    return "Backup run finished.";
                  })
                }
              >
                <Play size={14} /> Run now
              </button>
            </div>
          ))}
          {state.jobs.length === 0 && <EmptyRow message="Create a backup job before running backups." />}
        </div>
      </section>

      <section className="split">
        <div className="panel">
          <PanelHeader icon={<Clock3 size={18} />} title="Runs" detail={`${state.runs.length} recorded`} />
          <RunList runs={state.runs} jobs={state.jobs} />
        </div>
        <div className="panel">
          <PanelHeader icon={<RotateCcw size={18} />} title="Restore and verification" detail="Verification is required before a restore proceeds." />
          <div className="stack compact">
            <Field label="Run">
              <Select value={restoreRunId} onChange={setRestoreRunId} items={state.runs.map((record) => [record.run.id, `${record.run.status} · ${formatDateTime(record.run.started_at)}`])} required />
            </Field>
            <Field label="Target">
              <Select value={restoreTargetId} onChange={setRestoreTargetId} items={state.targets.map((target) => [target.id, `${target.name} · ${target.deployment}`])} required />
            </Field>
            <button
              className="secondary-button"
              type="button"
              disabled={!restoreRunId || actionLoading === "verify"}
              onClick={() =>
                void perform("verify", async () => {
                  await client.request(`/api/v1/runs/${restoreRunId}/verify`, { method: "POST" });
                  return "Backup verified against manifest.";
                })
              }
            >
              <CheckCircle2 size={16} /> Verify selected run
            </button>
            <Field label={`Type deployment to restore${restoreTarget ? ` (${restoreTarget.deployment})` : ""}`}>
              <input value={confirmDeployment} onChange={(event) => setConfirmDeployment(event.target.value)} />
            </Field>
            <button
              className="danger-button"
              type="button"
              disabled={!restoreRunId || !restoreTargetId || !confirmDeployment || actionLoading === "restore"}
              onClick={() =>
                void perform("restore", async () => {
                  await client.request("/api/v1/restore", {
                    method: "POST",
                    body: JSON.stringify({
                      run_id: restoreRunId,
                      target_id: restoreTargetId,
                      confirm_deployment: confirmDeployment
                    })
                  });
                  return "Restore completed.";
                })
              }
            >
              <RotateCcw size={16} /> Restore verified backup
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
