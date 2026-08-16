import { useCallback, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Activity,
  DatabaseBackup,
  FolderGit2,
  HardDrive,
  ListChecks,
  LogOut,
  Play,
  RefreshCw,
  RotateCcw,
  ShieldCheck
} from "lucide-react";
import {
  ApiClient,
  filterStateByProject,
  type ApiToken,
  type AuditEvent,
  type BackupJob,
  type ConvexTarget,
  type DrReport,
  type HealthResponse,
  type JobSchedule,
  type Project,
  type RunRecord,
  type ServiceState,
  type StoredSecret,
  type StorageDestination,
  type User
} from "./appState";
import { TOKEN_STORAGE_KEY } from "./constants";
import { AuthShell, BootstrapForm, LoginForm } from "./components/auth";
import { NavButton, SystemMessages } from "./components/common";
import { buildUpdateNotice, fetchLatestInstallableRelease } from "./releases";
import { AuditSection } from "./sections/audit";
import { Dashboard, dashboardStats } from "./sections/dashboard";
import { DrSection } from "./sections/dr";
import { RunsSection } from "./sections/runs";
import { SecuritySection } from "./sections/security";
import { SetupSection } from "./sections/setup";
import "./styles.css";

type ActiveSection = "dashboard" | "setup" | "runs" | "security" | "dr" | "audit";

const emptyState: ServiceState = {
  health: null,
  users: [],
  tokens: [],
  secrets: [],
  projects: [],
  targets: [],
  destinations: [],
  jobs: [],
  schedules: [],
  runs: [],
  auditEvents: [],
  drReport: null
};

function normalizeDrReport(report: DrReport | null | undefined): DrReport | null {
  if (!report) {
    return null;
  }
  return {
    ...report,
    findings: Array.isArray(report.findings) ? report.findings : []
  };
}

function App() {
  const [token, setToken] = useState<string | null>(() => localStorage.getItem(TOKEN_STORAGE_KEY));
  const [activeSection, setActiveSection] = useState<ActiveSection>("dashboard");
  const [selectedProjectId, setSelectedProjectId] = useState<string>("all");
  const [state, setState] = useState<ServiceState>(emptyState);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [updateNotice, setUpdateNotice] = useState<string | null>(null);
  const [oneTimeToken, setOneTimeToken] = useState<string | null>(null);

  const client = useMemo(() => new ApiClient(token), [token]);

  const refresh = useCallback(async (showLoading = true) => {
    if (showLoading) setLoading(true);
    try {
      const health = await new ApiClient(null).request<HealthResponse>("/api/v1/health");
      if (!health.users_configured || !token) {
        setState({ ...emptyState, health });
        return;
      }

      const [
        users,
        tokens,
        secrets,
        projects,
        targets,
        destinations,
        jobs,
        schedules,
        runs,
        auditEvents,
        drReport
      ] = await Promise.all([
        client.request<{ users: User[] }>("/api/v1/users"),
        client.request<{ api_tokens: ApiToken[] }>("/api/v1/tokens"),
        client.request<{ secrets: StoredSecret[] }>("/api/v1/secrets"),
        client.request<{ projects: Project[] }>("/api/v1/projects"),
        client.request<{ targets: ConvexTarget[] }>("/api/v1/targets"),
        client.request<{ destinations: StorageDestination[] }>("/api/v1/destinations"),
        client.request<{ jobs: BackupJob[] }>("/api/v1/jobs"),
        client.request<{ schedules: JobSchedule[] }>("/api/v1/schedules"),
        client.request<{ runs: RunRecord[] }>("/api/v1/runs"),
        client.request<{ audit_events: AuditEvent[] }>("/api/v1/audit"),
        client.request<{ dr_report: DrReport }>("/api/v1/dr/report")
      ]);

      setState({
        health,
        users: users.users ?? [],
        tokens: tokens.api_tokens ?? [],
        secrets: secrets.secrets ?? [],
        projects: projects.projects ?? [],
        targets: targets.targets ?? [],
        destinations: destinations.destinations ?? [],
        jobs: jobs.jobs ?? [],
        schedules: schedules.schedules ?? [],
        runs: runs.runs ?? [],
        auditEvents: auditEvents.audit_events ?? [],
        drReport: normalizeDrReport(drReport.dr_report)
      });
    } catch (caught) {
      if (showLoading) {
        setError(caught instanceof Error ? caught.message : "Failed to load service state");
      }
    } finally {
      if (showLoading) {
        setLoading(false);
      }
    }
  }, [client, token]);

  useEffect(() => {
    void refresh(true);
    if (!token) return;
    const interval = setInterval(() => {
      void refresh(false);
    }, 3000);
    return () => clearInterval(interval);
  }, [refresh, token]);

  useEffect(() => {
    const version = state.health?.version;
    if (!version) return;
    const controller = new AbortController();
    void fetchLatestInstallableRelease(controller.signal)
      .then((release) => {
        setUpdateNotice(buildUpdateNotice(version, release));
      })
      .catch(() => {
        setUpdateNotice(null);
      });
    return () => controller.abort();
  }, [state.health?.version]);

  const authenticate = (newToken: string, message: string) => {
    localStorage.setItem(TOKEN_STORAGE_KEY, newToken);
    setToken(newToken);
    setOneTimeToken(newToken);
    setNotice(message);
    setError(null);
  };

  const logout = () => {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
    setToken(null);
    setState((current) => ({ ...emptyState, health: current.health }));
    setNotice("Local browser token removed.");
  };

  const perform = async (key: string, action: () => Promise<string | null | undefined>) => {
    setActionLoading(key);
    setError(null);
    setNotice(null);
    try {
      const message = await action();
      if (message) {
        setNotice(message);
      }
      await refresh(false);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Action failed");
    } finally {
      setActionLoading(null);
    }
  };

  const scopedState = useMemo(() => filterStateByProject(state, selectedProjectId), [state, selectedProjectId]);
  const stats = useMemo(() => dashboardStats(scopedState), [scopedState]);
  const isBackupRunning = (state.runs ?? []).some((r) => r.run.status === "queued") || actionLoading !== null;

  const handleRunJob = (jobId: string) => {
    void perform(`run-${jobId}`, async () => {
      await client.request(`/api/v1/jobs/${jobId}/run`, { method: "POST" });
      return "Backup run finished.";
    });
  };

  if (loading && !state.health) {
    return (
      <main className="center-screen">
        <DatabaseBackup size={34} />
        <p>Loading local control plane</p>
      </main>
    );
  }

  if (state.health && !state.health.users_configured) {
    return (
      <AuthShell health={state.health}>
        <BootstrapForm
          onAuthenticated={authenticate}
          onError={setError}
          error={error}
          notice={notice}
          oneTimeToken={oneTimeToken}
        />
      </AuthShell>
    );
  }

  if (!token) {
    return (
      <AuthShell health={state.health}>
        <LoginForm
          onAuthenticated={authenticate}
          onError={setError}
          error={error}
          notice={notice}
          oneTimeToken={oneTimeToken}
        />
      </AuthShell>
    );
  }

  const selectedProjectObj = (state.projects ?? []).find((p) => p.id === selectedProjectId);

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand">
          <DatabaseBackup size={28} />
          <div>
            <strong>ConvexAutoBackup</strong>
            <span>Self-hosted DR</span>
          </div>
        </div>
        <nav aria-label="Primary">
          <NavButton active={activeSection === "dashboard"} icon={<Activity size={18} />} onClick={() => setActiveSection("dashboard")}>
            Dashboard
          </NavButton>
          <NavButton active={activeSection === "setup"} icon={<HardDrive size={18} />} onClick={() => setActiveSection("setup")}>
            Setup
          </NavButton>
          <NavButton active={activeSection === "runs"} icon={<Play size={18} />} onClick={() => setActiveSection("runs")}>
            Runs
          </NavButton>
          <NavButton active={activeSection === "security"} icon={<ShieldCheck size={18} />} onClick={() => setActiveSection("security")}>
            Security
          </NavButton>
          <NavButton active={activeSection === "dr"} icon={<RotateCcw size={18} />} onClick={() => setActiveSection("dr")}>
            DR Center
          </NavButton>
          <NavButton active={activeSection === "audit"} icon={<ListChecks size={18} />} onClick={() => setActiveSection("audit")}>
            Audit
          </NavButton>
        </nav>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">Local/LAN control plane</p>
            <h1>Convex backup operations</h1>
            <p className="subtle">
              {state.health?.service} {state.health?.version} · {state.health?.database_path}
            </p>
          </div>
          <div className="topbar-actions">
            {/* Project Scope Switcher */}
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", background: "#ffffff", border: "1px solid #cbd5e1", padding: "0.4rem 0.85rem", borderRadius: "8px", boxShadow: "0 1px 2px rgba(0,0,0,0.04)" }}>
              <FolderGit2 size={16} style={{ color: "#0284c7" }} />
              <span style={{ fontSize: "0.78rem", fontWeight: 700, color: "#64748b", textTransform: "uppercase", letterSpacing: "0.03em" }}>Project Scope:</span>
              <select
                value={selectedProjectId}
                onChange={(e) => setSelectedProjectId(e.target.value)}
                style={{ border: "none", background: "transparent", fontWeight: 700, fontSize: "0.85rem", color: "#0f172a", cursor: "pointer", outline: "none" }}
              >
                <option value="all">⚡ All Projects ({(state.projects ?? []).length})</option>
                {(state.projects ?? []).map((p) => (
                  <option key={p.id} value={p.id}>📁 {p.name}</option>
                ))}
              </select>
            </div>

            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.5rem",
                padding: "0.4rem 0.85rem",
                borderRadius: "9999px",
                background: isBackupRunning ? "#eff6ff" : "#f0fdf4",
                border: `1px solid ${isBackupRunning ? "#93c5fd" : "#bbf7d0"}`,
                fontSize: "0.82rem",
                fontWeight: 600,
                color: isBackupRunning ? "#1d4ed8" : "#15803d"
              }}
            >
              <span className={`pulse-dot ${isBackupRunning ? "running" : ""}`} />
              {isBackupRunning ? "Backup In Progress..." : "System Active (Live)"}
            </div>
            <button className="secondary-button" type="button" onClick={() => void refresh(true)} disabled={loading}>
              <RefreshCw size={16} className={loading ? "spin" : ""} /> Refresh
            </button>
            <button className="secondary-button danger-text" type="button" onClick={logout}>
              <LogOut size={16} /> Sign out
            </button>
          </div>
        </header>

        {selectedProjectId !== "all" && selectedProjectObj && (
          <div style={{ background: "#f0f9ff", border: "1px solid #bae6fd", padding: "0.6rem 1rem", borderRadius: "8px", display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.85rem", color: "#0369a1", marginBottom: "1rem" }}>
            <span>
              <strong>Filtered View:</strong> Showing targets, runs, and disaster recovery stats isolated to project <strong>{selectedProjectObj.name}</strong>.
            </span>
            <button
              type="button"
              className="secondary-button small"
              onClick={() => setSelectedProjectId("all")}
            >
              Reset to All Projects
            </button>
          </div>
        )}

        {isBackupRunning && (
          <div className="live-banner">
            <RefreshCw className="spin" size={18} />
            <span>Backup run currently in progress — live monitoring updates every 3s...</span>
          </div>
        )}

        <SystemMessages error={error} notice={notice ?? updateNotice} oneTimeToken={oneTimeToken} />

        {activeSection === "dashboard" && (
          <Dashboard
            stats={stats}
            state={scopedState}
            onSelectProject={setSelectedProjectId}
            onRunJob={handleRunJob}
            actionLoading={actionLoading}
          />
        )}
        {activeSection === "setup" && <SetupSection client={client} state={state} actionLoading={actionLoading} perform={perform} />}
        {activeSection === "runs" && <RunsSection client={client} state={scopedState} actionLoading={actionLoading} perform={perform} />}
        {activeSection === "security" && (
          <SecuritySection
            client={client}
            state={state}
            actionLoading={actionLoading}
            perform={perform}
            onTokenCreated={setOneTimeToken}
          />
        )}
        {activeSection === "dr" && <DrSection client={client} state={scopedState} actionLoading={actionLoading} perform={perform} />}
        {activeSection === "audit" && <AuditSection events={state.auditEvents} />}
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
