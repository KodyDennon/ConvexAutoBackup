import { useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, Clock3, FolderGit2, Play, RotateCcw, ShieldAlert } from "lucide-react";
import { ApiClient, formatBytes, formatDateTime, sentenceCase, type ServiceState } from "../appState";
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
      {/* Backup Jobs Table */}
      <section className="panel">
        <PanelHeader icon={<Play size={18} />} title="Backup Jobs & Target Deployments" detail={`${(state.jobs ?? []).length} configured job pipelines`} />
        <div className="table">
          <div className="table-row table-head">
            <span>Project</span>
            <span>Job Name</span>
            <span>Target Deployment</span>
            <span>Destination Vault</span>
            <span>File Storage</span>
            <span>Action</span>
          </div>
          {(state.jobs ?? []).map((job) => {
            const projObj = (state.projects ?? []).find((p) => p.id === job.project_id);
            const targetObj = (state.targets ?? []).find((t) => t.id === job.target_id);
            const destObj = (state.destinations ?? []).find((d) => d.id === job.destination_id);

            return (
              <div className="table-row" key={job.id}>
                <span>
                  <strong style={{ display: "inline-flex", alignItems: "center", gap: "0.3rem", color: "#0369a1" }}>
                    <FolderGit2 size={14} /> {projObj?.name ?? "Default Project"}
                  </strong>
                </span>
                <span><strong>{job.name}</strong></span>
                <span><code style={{ color: "#0284c7" }}>{targetObj?.deployment ?? job.target_id.slice(0, 8)}</code></span>
                <span>{destObj?.name ?? job.destination_id.slice(0, 8)}</span>
                <span>
                  <span className={`badge ${job.include_file_storage ? "success" : "info"}`}>
                    {job.include_file_storage ? "Included" : "DB Only"}
                  </span>
                </span>
                <span>
                  <button
                    className="primary-button small"
                    type="button"
                    disabled={actionLoading === `run-${job.id}`}
                    onClick={() =>
                      void perform(`run-${job.id}`, async () => {
                        await client.request(`/api/v1/jobs/${job.id}/run`, { method: "POST" });
                        return `Backup run started for job "${job.name}".`;
                      })
                    }
                  >
                    <Play size={14} /> Run Now
                  </button>
                </span>
              </div>
            );
          })}
          {(state.jobs ?? []).length === 0 && <EmptyRow message="Create a backup job from the Setup tab before running backups." />}
        </div>
      </section>

      {/* History & Extreme Protection Restore Split */}
      <section className="split">
        <div className="panel">
          <PanelHeader icon={<Clock3 size={18} />} title="Backup Execution History" detail={`${(state.runs ?? []).length} total runs recorded`} />
          <RunList runs={state.runs ?? []} jobs={state.jobs ?? []} />
        </div>

        <div className="panel" style={{ display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
          <div>
            <PanelHeader icon={<RotateCcw size={18} />} title="Database Restore (Extreme Protection)" detail="Mandatory 3-step verification protocol to prevent accidental data loss." />
            
            <div className="danger-zone-banner" style={{ background: "#fef2f2", border: "2px solid #ef4444", borderRadius: "10px", padding: "0.85rem 1rem", color: "#7f1d1d", margin: "0.75rem 0" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.35rem" }}>
                <AlertTriangle size={18} color="#dc2626" />
                <strong style={{ fontSize: "0.95rem", color: "#991b1b" }}>HIGH-RISK DESTRUCTIVE ACTION</strong>
              </div>
              <p style={{ margin: 0, fontSize: "0.82rem", lineHeight: "1.4" }}>
                Restoring a snapshot will <strong>OVERWRITE all tables, documents, and schemas</strong> in your Convex target. This action cannot be undone.
              </p>
            </div>

            <div className="stack gap-3" style={{ marginTop: "0.75rem" }}>
              <Field label="1. Select Backup Snapshot to Restore">
                <Select
                  value={restoreRunId}
                  onChange={setRestoreRunId}
                  items={(state.runs ?? []).map((record) => {
                    let bytes = 0;
                    let deploymentName = "";
                    if (record.manifest_json) {
                      try {
                        const parsed = JSON.parse(record.manifest_json);
                        bytes = parsed.archive_size_bytes ?? parsed.archive_size ?? parsed.bytes_exported ?? 0;
                        deploymentName = parsed.deployment ?? "";
                      } catch {}
                    }
                    const sizeStr = bytes > 0 ? ` (${formatBytes(bytes)})` : "";
                    const targetStr = deploymentName ? `[${deploymentName}] ` : "";
                    const label = `${targetStr}${sentenceCase(record.run.status)} · ${formatDateTime(record.run.started_at)}${sizeStr}`;
                    return [record.run.id, label];
                  })}
                  required
                />
              </Field>

              <Field label="2. Select Destination Target Deployment">
                <Select
                  value={restoreTargetId}
                  onChange={setRestoreTargetId}
                  items={(state.targets ?? []).map((target) => [target.id, `${target.name} (${target.deployment})`])}
                  required
                />
              </Field>

              <button
                className="secondary-button"
                type="button"
                style={{ width: "100%", justifyContent: "center" }}
                disabled={!restoreRunId || actionLoading === "verify"}
                onClick={() =>
                  void perform("verify", async () => {
                    await client.request(`/api/v1/runs/${restoreRunId}/verify`, { method: "POST" });
                    return "Backup archive and SHA-256 manifest verified successfully!";
                  })
                }
              >
                <CheckCircle2 size={16} /> Step 1: Mandatory SHA-256 Checksum Verification
              </button>

              <Field label={`Step 2: Confirm Target Deployment Name${restoreTarget ? ` (Must match: "${restoreTarget.deployment}")` : ""}`}>
                <input
                  value={confirmDeployment}
                  onChange={(event) => setConfirmDeployment(event.target.value.trim())}
                  placeholder={restoreTarget?.deployment ?? "e.g. zany-cheetah-864"}
                />
              </Field>

              <Field label="Step 3: Type Confirmation Phrase (RESTORE MY CONVEX DATABASE)">
                <input
                  value={confirmPhrase}
                  onChange={(event) => setConfirmPhrase(event.target.value)}
                  placeholder="RESTORE MY CONVEX DATABASE"
                />
              </Field>
            </div>
          </div>

          <div style={{ marginTop: "1.25rem", paddingTop: "1rem", borderTop: "1px solid #f1f5f9" }}>
            <button
              className="danger-button"
              type="button"
              style={{ width: "100%", minHeight: "44px", fontWeight: 700, fontSize: "0.95rem" }}
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
                  return `Database restore completed successfully for ${restoreTarget?.deployment}!`;
                })
              }
            >
              <ShieldAlert size={18} /> Confirm Restore & Overwrite Database
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
