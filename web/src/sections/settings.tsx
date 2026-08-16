import { useState } from "react";
import { AlertTriangle, CheckCircle2, RefreshCw, Settings, ShieldAlert, Sparkles, Trash2 } from "lucide-react";
import { ApiClient, type ServiceState } from "../appState";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

export function SettingsSection({
  client,
  state,
  actionLoading,
  perform,
  onRefresh,
  onInstallUpdate
}: {
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
  onRefresh: () => Promise<void>;
  onInstallUpdate: () => void;
}) {
  const [showWipeModal, setShowWipeModal] = useState(false);
  const [confirmText, setConfirmText] = useState("");
  const [wipeFiles, setWipeFiles] = useState(true);

  const health = state.health;
  const projectCount = (state.projects ?? []).length;
  const targetCount = (state.targets ?? []).length;
  const secretCount = (state.secrets ?? []).length;
  const destCount = (state.destinations ?? []).length;
  const jobCount = (state.jobs ?? []).length;
  const runCount = (state.runs ?? []).length;

  return (
    <div className="page-stack">
      <div className="info-banner">
        <strong><Settings size={18} style={{ verticalAlign: "middle", marginRight: "0.4rem" }} /> System Settings & Maintenance</strong>
        <p>Manage system diagnostics, state synchronization, release updates, and factory reset actions.</p>
      </div>

      <div className="grid split-even">
        {/* System Diagnostics Panel */}
        <div className="panel stack">
          <div className="panel-header">
            <h3><CheckCircle2 size={18} /> System Diagnostics</h3>
            <span className="badge success">Active</span>
          </div>

          <div className="inventory-list">
            <div className="inventory-row">
              <span>Service Version</span>
              <strong>{health?.version ?? "0.1.0-beta.6"}</strong>
            </div>
            <div className="inventory-row">
              <span>Database Path</span>
              <code className="tiny-code">{health?.database_path ?? "/home/user/.local/share/convex-autobackup/convex-autobackup.sqlite3"}</code>
            </div>
            <div className="inventory-row">
              <span>Configured Projects</span>
              <strong>{projectCount}</strong>
            </div>
            <div className="inventory-row">
              <span>Connected Targets</span>
              <strong>{targetCount}</strong>
            </div>
            <div className="inventory-row">
              <span>Encrypted Secrets</span>
              <strong>{secretCount}</strong>
            </div>
            <div className="inventory-row">
              <span>Storage Vaults</span>
              <strong>{destCount}</strong>
            </div>
            <div className="inventory-row">
              <span>Backup Jobs</span>
              <strong>{jobCount}</strong>
            </div>
            <div className="inventory-row">
              <span>Recorded Runs</span>
              <strong>{runCount}</strong>
            </div>
          </div>

          <div className="button-row" style={{ marginTop: "1rem" }}>
            <button
              type="button"
              className="secondary-button"
              disabled={actionLoading === "manual-refresh"}
              onClick={() =>
                void perform("manual-refresh", async () => {
                  await onRefresh();
                  return "System state re-synced cleanly.";
                })
              }
            >
              <RefreshCw size={16} /> Sync System State & Reload
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={onInstallUpdate}
            >
              <Sparkles size={16} /> Install System Update Now
            </button>
          </div>
        </div>

        {/* Danger Zone Panel */}
        <div className="panel stack" style={{ borderColor: "#fca5a5" }}>
          <div className="panel-header">
            <h3 style={{ color: "#991b1b" }}><ShieldAlert size={18} /> Danger Zone: System Reset</h3>
            <span className="badge danger">Destructive Action</span>
          </div>

          <p className="subtle">
            Factory reset will immediately wipe configuration records and return the system to a clean state.
          </p>

          <div style={{ background: "#fef2f2", border: "1px solid #fecaca", padding: "0.85rem", borderRadius: "8px", fontSize: "0.85rem", color: "#991b1b" }} className="stack compact">
            <strong>Warning: Wiping will permanently delete:</strong>
            <ul style={{ margin: "0.3rem 0 0 1.2rem", padding: 0 }}>
              <li>All Projects ({projectCount}) & Connected Targets ({targetCount})</li>
              <li>All Encrypted Deploy Key Secrets ({secretCount})</li>
              <li>All Storage Vault Destinations ({destCount})</li>
              <li>All Backup Jobs ({jobCount}) & Schedules</li>
              <li>All Backup Execution Records ({runCount}) & Audit Trail</li>
              <li>Optionally all local backup ZIP archives and JSON manifests</li>
            </ul>
          </div>

          <div style={{ marginTop: "auto", paddingTop: "1rem" }}>
            <button
              type="button"
              className="danger-button"
              style={{ width: "100%", justifyContent: "center", padding: "0.75rem" }}
              onClick={() => {
                setConfirmText("");
                setShowWipeModal(true);
              }}
            >
              <Trash2 size={16} /> Factory Reset & Wipe Everything
            </button>
          </div>
        </div>
      </div>

      {/* Wipe Confirmation Modal */}
      {showWipeModal && (
        <div className="modal-backdrop" onClick={() => setShowWipeModal(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: "520px" }}>
            <div className="modal-header" style={{ borderBottom: "1px solid #fecaca", background: "#fef2f2" }}>
              <h3 style={{ color: "#991b1b" }}><AlertTriangle size={18} /> Confirm Full System Reset & Wipe</h3>
              <button type="button" className="close-btn" onClick={() => setShowWipeModal(false)}>✕</button>
            </div>
            <div className="modal-body stack">
              <p style={{ fontSize: "0.9rem", color: "#7f1d1d" }}>
                This action is irreversible. All database tables, encrypted secret keys, backup targets, and schedules will be permanently deleted.
              </p>

              <label style={{ display: "flex", alignItems: "center", gap: "0.5rem", fontSize: "0.88rem", background: "#f8fafc", padding: "0.6rem 0.85rem", borderRadius: "6px", border: "1px solid #e2e8f0" }}>
                <input
                  type="checkbox"
                  checked={wipeFiles}
                  onChange={(e) => setWipeFiles(e.target.checked)}
                />
                <span>Also delete all backup ZIP archives from <code>/home/user/backups</code></span>
              </label>

              <div style={{ marginTop: "0.5rem" }}>
                <label style={{ fontSize: "0.82rem", fontWeight: 600, display: "block", marginBottom: "0.35rem" }}>
                  To confirm, type <code style={{ color: "#991b1b" }}>WIPE-EVERYTHING</code> below:
                </label>
                <input
                  value={confirmText}
                  onChange={(e) => setConfirmText(e.target.value)}
                  placeholder="WIPE-EVERYTHING"
                  style={{ width: "100%", padding: "0.6rem", borderRadius: "6px", border: "1px solid #cbd5e1" }}
                  autoFocus
                />
              </div>
            </div>

            <div className="modal-footer button-row">
              <button type="button" className="secondary-button" onClick={() => setShowWipeModal(false)}>Cancel</button>
              <button
                type="button"
                className="danger-button"
                disabled={confirmText.trim() !== "WIPE-EVERYTHING" || actionLoading === "system-wipe"}
                onClick={() =>
                  void perform("system-wipe", async () => {
                    const res = await client.request<{ message: string }>("/api/v1/system/wipe", {
                      method: "POST",
                      body: JSON.stringify({ wipe_files: wipeFiles })
                    });
                    setShowWipeModal(false);
                    await onRefresh();
                    return res.message ?? "System factory reset completed cleanly.";
                  })
                }
              >
                <Trash2 size={16} /> Confirm Irreversible Factory Reset
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
