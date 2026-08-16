import { useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, Clock3, Play, RotateCcw, ShieldAlert } from "lucide-react";
import { ApiClient, formatDateTime, type ServiceState } from "../appState";
import { EmptyRow, Field, PanelHeader, RunList, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

const REQUIRED_RESTORE_PHRASE = "RESTORE MY CONVEX DATABASE";

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
  const [confirmPhrase, setConfirmPhrase] = useState("");

  useEffect(() => {
    if (!restoreRunId && state.runs?.[0]?.run) setRestoreRunId(state.runs[0].run.id);
    if (!restoreTargetId && state.targets?.[0]) setRestoreTargetId(state.targets[0].id);
  }, [restoreRunId, restoreTargetId, state.runs, state.targets]);

  const restoreTarget = (state.targets ?? []).find((target) => target.id === restoreTargetId);

  return (
    <div className="page-stack">
      <section className="panel">
        <PanelHeader icon={<Play size={18} />} title="Backup jobs" detail={`${(state.jobs ?? []).length} configured jobs`} />
        <div className="table">
          <div className="table-row table-head">
            <span>Name</span>
            <span>Target</span>
            <span>Destination</span>
            <span>Files</span>
            <span>Action</span>
          </div>
          {(state.jobs ?? []).map((job) => (
            <div className="table-row" key={job.id}>
              <span>{job.name}</span>
              <span>{(state.targets ?? []).find((target) => target.id === job.target_id)?.deployment ?? job.target_id}</span>
              <span>{(state.destinations ?? []).find((destination) => destination.id === job.destination_id)?.name ?? job.destination_id}</span>
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
          {(state.jobs ?? []).length === 0 && <EmptyRow message="Create a backup job before running backups." />}
        </div>
      </section>

      <section className="split">
        <div className="panel">
          <PanelHeader icon={<Clock3 size={18} />} title="Runs" detail={`${(state.runs ?? []).length} recorded`} />
          <RunList runs={state.runs ?? []} jobs={state.jobs ?? []} />
        </div>
        <div className="panel">
          <PanelHeader icon={<RotateCcw size={18} />} title="Database Restore (Extreme Protection)" detail="Requires mandatory SHA-256 verification and double confirmation." />
          <div className="stack compact">
            <div className="danger-zone-banner" style={{ background: "#fef2f2", border: "2px solid #ef4444", borderRadius: "10px", padding: "0.85rem 1rem", color: "#7f1d1d", margin: "0.5rem 0" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.35rem" }}>
                <AlertTriangle size={18} color="#dc2626" />
                <strong style={{ fontSize: "0.95rem", color: "#991b1b" }}>HIGH-RISK DESTRUCTIVE ACTION</strong>
              </div>
              <p style={{ margin: 0, fontSize: "0.82rem", lineHeight: "1.4" }}>
                Restoring a backup snapshot will <strong>OVERWRITE all tables, documents, and schema</strong> in your Convex deployment. This cannot be undone.
              </p>
            </div>

            <Field label="1. Select Backup Run">
              <Select value={restoreRunId} onChange={setRestoreRunId} items={(state.runs ?? []).map((record) => [record.run.id, `${record.run.status} · ${formatDateTime(record.run.started_at)}`])} required />
            </Field>
            <Field label="2. Select Destination Target">
              <Select value={restoreTargetId} onChange={setRestoreTargetId} items={(state.targets ?? []).map((target) => [target.id, `${target.name} · ${target.deployment}`])} required />
            </Field>

            <button
              className="secondary-button"
              type="button"
              disabled={!restoreRunId || actionLoading === "verify"}
              onClick={() =>
                void perform("verify", async () => {
                  await client.request(`/api/v1/runs/${restoreRunId}/verify`, { method: "POST" });
                  return "Backup archive and SHA-256 manifest verified successfully!";
                })
              }
            >
              <CheckCircle2 size={16} /> 1. Mandatory SHA-256 Checksum Verification
            </button>

            <Field label={`2. Confirm Deployment Name${restoreTarget ? ` (Must match: ${restoreTarget.deployment})` : ""}`}>
              <input value={confirmDeployment} onChange={(event) => setConfirmDeployment(event.target.value.trim())} placeholder={restoreTarget?.deployment ?? "e.g. zany-cheetah-864"} />
            </Field>

            <Field label="3. Type Confirmation Phrase (RESTORE MY CONVEX DATABASE)">
              <input value={confirmPhrase} onChange={(event) => setConfirmPhrase(event.target.value)} placeholder="RESTORE MY CONVEX DATABASE" />
            </Field>

            <button
              className="danger-button"
              type="button"
              style={{ marginTop: "0.5rem", minHeight: "44px", fontWeight: 700 }}
              disabled={
                !restoreRunId ||
                !restoreTargetId ||
                confirmDeployment !== restoreTarget?.deployment ||
                confirmPhrase !== REQUIRED_RESTORE_PHRASE ||
                actionLoading === "restore"
              }
              onClick={() =>
                void perform("restore", async () => {
                  if (!confirm(`FINAL WARNING: Are you 100% sure you want to OVERWRITE deployment "${restoreTarget?.deployment}" with run #${restoreRunId.slice(0, 8)}?`)) return;
                  await client.request("/api/v1/restore", {
                    method: "POST",
                    body: JSON.stringify({
                      run_id: restoreRunId,
                      target_id: restoreTargetId,
                      confirm_deployment: confirmDeployment,
                      confirm_phrase: confirmPhrase
                    })
                  });
                  return `Database restore completed for ${restoreTarget?.deployment}!`;
                })
              }
            >
              <ShieldAlert size={18} /> Execute Dangerous Restore (Overwrite Database)
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
