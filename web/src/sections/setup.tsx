import { useEffect, useState } from "react";
import { Activity, CheckCircle2, DatabaseBackup, HardDrive, KeyRound, Layers, Play, Plus, ShieldCheck, Clock3, Trash2 } from "lucide-react";
import {
  ApiClient,
  destinationLabel,
  formatDateTime,
  sentenceCase,
  type SecretKind,
  type ServiceState
} from "../appState";
import { secretKinds } from "../constants";
import { Field, ResourceForm, Select } from "../components/common";
import { ScheduleForm } from "./setupScheduleForm";
import "./setupGuide.css";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;
type SetupTask = "project" | "secret" | "target" | "destination" | "job" | "schedule" | "backup" | "complete";

export function SetupSection({
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
  const activeTask = currentSetupTask(state);
  const [selectedTab, setSelectedTab] = useState<SetupTask>(activeTask);

  useEffect(() => {
    setSelectedTab(activeTask);
  }, [activeTask]);

  return (
    <div className="page-stack">
      <SetupGuide
        state={state}
        activeTask={activeTask}
        selectedTab={selectedTab}
        onSelectTab={setSelectedTab}
      />

      <nav className="setup-tabs-nav" aria-label="Setup steps navigation">
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "project" ? "active" : ""}`}
          onClick={() => setSelectedTab("project")}
        >
          <Layers size={16} /> 1. Projects ({(state.projects ?? []).length})
        </button>
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "secret" ? "active" : ""}`}
          onClick={() => setSelectedTab("secret")}
        >
          <KeyRound size={16} /> 2. Deploy Keys ({(state.secrets ?? []).filter(s => s.kind === "convex_deploy_key").length})
        </button>
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "target" ? "active" : ""}`}
          onClick={() => setSelectedTab("target")}
        >
          <DatabaseBackup size={16} /> 3. Targets ({(state.targets ?? []).length})
        </button>
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "destination" ? "active" : ""}`}
          onClick={() => setSelectedTab("destination")}
        >
          <HardDrive size={16} /> 4. Storage Vaults ({(state.destinations ?? []).length})
        </button>
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "job" ? "active" : ""}`}
          onClick={() => setSelectedTab("job")}
        >
          <Activity size={16} /> 5. Backup Jobs ({(state.jobs ?? []).length})
        </button>
        <button
          type="button"
          className={`setup-tab-btn ${selectedTab === "schedule" ? "active" : ""}`}
          onClick={() => setSelectedTab("schedule")}
        >
          <Clock3 size={16} /> 6. Schedules ({(state.schedules ?? []).length})
        </button>
      </nav>

      <section className="setup-workspace">
        <div className="setup-primary">
          <TabWorkspace
            tab={selectedTab}
            client={client}
            state={state}
            actionLoading={actionLoading}
            perform={perform}
          />
        </div>
        <aside className="panel setup-summary">
          <PanelSummary state={state} />
        </aside>
      </section>
    </div>
  );
}

function currentSetupTask(state: ServiceState): SetupTask {
  const latestRun = state.runs?.[0]?.run;
  const projects = state.projects ?? [];
  const targets = state.targets ?? [];
  const secrets = state.secrets ?? [];
  const destinations = state.destinations ?? [];
  const jobs = state.jobs ?? [];
  const schedules = state.schedules ?? [];

  if (projects.length === 0) return "project";
  if (targets.length === 0 && !secrets.some((secret) => secret.kind === "convex_deploy_key")) return "secret";
  if (targets.length === 0) return "target";
  if (destinations.length === 0) return "destination";
  if (jobs.length === 0) return "job";
  if (schedules.length === 0) return "schedule";
  if (latestRun?.status !== "succeeded") return "backup";
  return "complete";
}

function TabWorkspace({
  tab,
  client,
  state,
  actionLoading,
  perform
}: {
  tab: SetupTask;
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
}) {
  const projects = state.projects ?? [];
  const secrets = state.secrets ?? [];
  const targets = state.targets ?? [];
  const destinations = state.destinations ?? [];
  const jobs = state.jobs ?? [];
  const schedules = state.schedules ?? [];

  if (tab === "project") {
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 1: Create a Project</strong>
          <p>A Project is a name for your Convex application or customer environment (e.g. <code>Production App</code> or <code>Client Studio</code>).</p>
        </div>
        <div className="grid split-even">
          <ProjectForm client={client} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Configured Projects ({projects.length})</h3>
            {projects.length === 0 ? (
              <p className="subtle">No projects created yet. Use the form to add your first project.</p>
            ) : (
              <div className="card-list">
                {projects.map((p) => (
                  <div key={p.id} className="resource-card">
                    <div className="resource-card-header">
                      <strong>{p.name}</strong>
                      <span className="badge">Active</span>
                    </div>
                    {p.description && <p className="subtle">{p.description}</p>}
                    <code className="tiny-code">ID: {p.id}</code>
                    <div className="card-actions">
                      <button
                        className="danger-button small"
                        type="button"
                        disabled={actionLoading === `delete-project-${p.id}`}
                        onClick={() =>
                          void perform(`delete-project-${p.id}`, async () => {
                            if (!confirm(`Delete project "${p.name}"? Associated targets and jobs will also be deleted.`)) return;
                            await client.request(`/api/v1/projects/${p.id}`, { method: "DELETE" });
                            return `Project "${p.name}" deleted.`;
                          })
                        }
                      >
                        <Trash2 size={14} /> Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (tab === "secret") {
    const deployKeys = secrets.filter((s) => s.kind === "convex_deploy_key");
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 2: Add Convex Deploy Key</strong>
          <p>
            Copy a Deploy Key from your Convex Dashboard (<code>Deployment Settings</code> → <code>Deploy Keys</code>) or run <code>npx convex deploy-key</code> in your terminal.
            Keys are encrypted using your master key before saving.
          </p>
        </div>
        <div className="grid split-even">
          <SecretForm client={client} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Stored Deploy Keys ({deployKeys.length})</h3>
            {deployKeys.length === 0 ? (
              <p className="subtle">No deploy keys stored yet. Paste your <code>prod:...</code> key on the left.</p>
            ) : (
              <div className="card-list">
                {deployKeys.map((s) => (
                  <div key={s.id} className="resource-card">
                    <div className="resource-card-header">
                      <strong>{s.label}</strong>
                      <span className="badge success">Encrypted</span>
                    </div>
                    <p className="subtle">Kind: <code>{s.kind}</code></p>
                    <code className="tiny-code">Updated: {formatDateTime(s.updated_at)}</code>
                    <div className="card-actions">
                      <button
                        className="danger-button small"
                        type="button"
                        disabled={actionLoading === `delete-secret-${s.id}`}
                        onClick={() =>
                          void perform(`delete-secret-${s.id}`, async () => {
                            if (!confirm(`Delete secret "${s.label}"?`)) return;
                            await client.request(`/api/v1/secrets/${s.id}`, { method: "DELETE" });
                            return `Secret "${s.label}" deleted.`;
                          })
                        }
                      >
                        <Trash2 size={14} /> Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (tab === "target") {
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 3: Connect Convex Deployment Target</strong>
          <p>
            Link your Project to your Convex Deployment Name (e.g. <code>happy-animal-123</code>).
            Convex Cloud URLs are automatically resolved: <strong>Cloud Data URL</strong> = <code>https://&lt;deployment&gt;.convex.cloud</code> | <strong>Actions Site URL</strong> = <code>https://&lt;deployment&gt;.convex.site</code>.
          </p>
        </div>
        <div className="grid split-even">
          <TargetForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Connected Targets ({targets.length})</h3>
            {targets.length === 0 ? (
              <p className="subtle">No targets configured yet. Add a target to connect a deployment.</p>
            ) : (
              <div className="card-list">
                {targets.map((t) => (
                  <div key={t.id} className="resource-card">
                    <div className="resource-card-header">
                      <strong>{t.name}</strong>
                      <span className="badge">Convex Cloud</span>
                    </div>
                    <p>Deployment: <code>{t.deployment}</code></p>
                    <code className="tiny-code">Key Secret: {t.secret.label}</code>
                    <div className="card-actions">
                      <button
                        className="secondary-button small"
                        type="button"
                        disabled={actionLoading === `test-target-${t.id}`}
                        onClick={() =>
                          void perform(`test-target-${t.id}`, async () => {
                            const res = await client.request<{ message: string }>(`/api/v1/targets/${t.id}/test`, { method: "POST" });
                            return res.message ?? "Target connection verified.";
                          })
                        }
                      >
                        <CheckCircle2 size={14} /> Test
                      </button>
                      <button
                        className="danger-button small"
                        type="button"
                        disabled={actionLoading === `delete-target-${t.id}`}
                        onClick={() =>
                          void perform(`delete-target-${t.id}`, async () => {
                            if (!confirm(`Delete target "${t.name}"? Associated jobs will also be deleted.`)) return;
                            await client.request(`/api/v1/targets/${t.id}`, { method: "DELETE" });
                            return `Target "${t.name}" deleted.`;
                          })
                        }
                      >
                        <Trash2 size={14} /> Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (tab === "destination") {
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 4: Configure Storage Vault</strong>
          <p>Choose where backup zip archives will be stored: a local folder path (e.g. <code>/home/user/backups</code>) or an S3 bucket.</p>
        </div>
        <div className="grid split-even">
          <DestinationForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Storage Destinations ({destinations.length})</h3>
            {destinations.length === 0 ? (
              <p className="subtle">No storage vaults created yet.</p>
            ) : (
              <div className="card-list">
                {destinations.map((d) => (
                  <div key={d.id} className="resource-card">
                    <div className="resource-card-header">
                      <strong>{d.name}</strong>
                      <span className="badge">{d.kind.type === "local_filesystem" ? "Local Folder" : "S3 Bucket"}</span>
                    </div>
                    <p className="subtle">{destinationLabel(d)}</p>
                    <code className="tiny-code">Retention: Keep last {d.retention.keep_last} backups</code>
                    <div className="card-actions">
                      <button
                        className="danger-button small"
                        type="button"
                        disabled={actionLoading === `delete-dest-${d.id}`}
                        onClick={() =>
                          void perform(`delete-dest-${d.id}`, async () => {
                            if (!confirm(`Delete destination "${d.name}"? Associated jobs will also be deleted.`)) return;
                            await client.request(`/api/v1/destinations/${d.id}`, { method: "DELETE" });
                            return `Destination "${d.name}" deleted.`;
                          })
                        }
                      >
                        <Trash2 size={14} /> Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (tab === "job") {
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 5: Create Backup Job & Test Run</strong>
          <p>A Backup Job links your Convex Target and Storage Vault together. Once created, click <strong>▶ Run Backup Now</strong> to test immediate export!</p>
        </div>
        <div className="grid split-even">
          <JobForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Configured Backup Jobs ({jobs.length})</h3>
            {jobs.length === 0 ? (
              <p className="subtle">No backup jobs created yet. Fill in the form on the left.</p>
            ) : (
              <div className="card-list">
                {jobs.map((j) => {
                  const target = targets.find((t) => t.id === j.target_id);
                  const dest = destinations.find((d) => d.id === j.destination_id);
                  return (
                    <div key={j.id} className="resource-card job-card">
                      <div className="resource-card-header">
                        <strong>{j.name}</strong>
                        <span className="badge success">Ready to Run</span>
                      </div>
                      <p>Target: <code>{target?.deployment ?? j.target_id}</code></p>
                      <p>Vault: <code>{dest?.name ?? j.destination_id}</code></p>
                      <div className="card-actions">
                        <button
                          className="button-primary-action"
                          type="button"
                          disabled={actionLoading === `run-${j.id}`}
                          onClick={() =>
                            void perform(`run-${j.id}`, async () => {
                              await client.request(`/api/v1/jobs/${j.id}/run`, { method: "POST" });
                              return `Backup run started for "${j.name}". Check Runs section for details.`;
                            })
                          }
                        >
                          <Play size={16} /> Run Backup Now
                        </button>
                        <button
                          className="danger-button small"
                          type="button"
                          disabled={actionLoading === `delete-job-${j.id}`}
                          onClick={() =>
                            void perform(`delete-job-${j.id}`, async () => {
                              if (!confirm(`Delete backup job "${j.name}"?`)) return;
                              await client.request(`/api/v1/jobs/${j.id}`, { method: "DELETE" });
                              return `Job "${j.name}" deleted.`;
                            })
                          }
                        >
                          <Trash2 size={14} /> Delete
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (tab === "schedule" || tab === "backup" || tab === "complete") {
    return (
      <div className="tab-container stack">
        <div className="info-banner">
          <strong>Step 6: Automated Backup Schedule</strong>
          <p>Set an automated schedule for your backup jobs (e.g. Every 60 minutes, Daily at 02:00, or custom Cron expression).</p>
        </div>
        <div className="grid split-even">
          <ScheduleForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <div className="panel">
            <h3>Active Schedules ({schedules.length})</h3>
            {schedules.length === 0 ? (
              <p className="subtle">No automated schedules set up yet.</p>
            ) : (
              <div className="card-list">
                {schedules.map((s) => (
                  <div key={s.id} className="resource-card">
                    <div className="resource-card-header">
                      <strong>Schedule {s.id.slice(0, 8)}</strong>
                      <span className="badge success">Active</span>
                    </div>
                    <p>Mode: <code>{s.schedule.type}</code></p>
                    <p className="subtle">Next Due: {formatDateTime(s.next_due_at)}</p>
                    <div className="card-actions">
                      <button
                        className="danger-button small"
                        type="button"
                        disabled={actionLoading === `delete-schedule-${s.id}`}
                        onClick={() =>
                          void perform(`delete-schedule-${s.id}`, async () => {
                            if (!confirm(`Delete schedule?`)) return;
                            await client.request(`/api/v1/schedules/${s.id}`, { method: "DELETE" });
                            return `Schedule deleted.`;
                          })
                        }
                      >
                        <Trash2 size={14} /> Delete
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  return null;
}

function TaskFrame({ title, detail, children }: { title: string; detail: string; children: React.ReactNode }) {
  return (
    <div className="task-frame">
      <div className="task-frame-copy">
        <p className="eyebrow">Setup Wizard</p>
        <h2>{title}</h2>
        <p className="subtle">{detail}</p>
      </div>
      {children}
    </div>
  );
}

function PanelSummary({ state }: { state: ServiceState }) {
  const projects = state.projects ?? [];
  const secrets = state.secrets ?? [];
  const targets = state.targets ?? [];
  const destinations = state.destinations ?? [];
  const jobs = state.jobs ?? [];
  const schedules = state.schedules ?? [];

  const rows = [
    ["Projects", projects.length],
    ["Deploy keys", secrets.filter((secret) => secret.kind === "convex_deploy_key").length],
    ["Targets", targets.length],
    ["Destinations", destinations.length],
    ["Jobs", jobs.length],
    ["Schedules", schedules.length]
  ];

  return (
    <div className="stack compact">
      <div>
        <p className="eyebrow">Configured now</p>
        <h2>Install Inventory</h2>
      </div>
      <div className="inventory-list">
        {rows.map(([label, value]) => (
          <div className="inventory-row" key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      <p className="subtle">Every component is saved into your encrypted SQLite database.</p>
    </div>
  );
}

function SetupGuide({
  state,
  activeTask,
  selectedTab,
  onSelectTab
}: {
  state: ServiceState;
  activeTask: SetupTask;
  selectedTab: SetupTask;
  onSelectTab: (task: SetupTask) => void;
}) {
  const latestRun = state.runs?.[0]?.run;
  const projects = state.projects ?? [];
  const targets = state.targets ?? [];
  const destinations = state.destinations ?? [];
  const schedules = state.schedules ?? [];

  const steps: { id: SetupTask; label: string; complete: boolean; detail: string }[] = [
    { id: "project", label: "1. Project", complete: projects.length > 0, detail: "Name app" },
    { id: "secret", label: "2. Deploy Key", complete: state.secrets?.some(s => s.kind === "convex_deploy_key") ?? false, detail: "Paste prod key" },
    { id: "target", label: "3. Target", complete: targets.length > 0, detail: "Connect deployment" },
    { id: "destination", label: "4. Destination", complete: destinations.length > 0, detail: "Set local/S3 path" },
    { id: "job", label: "5. Backup Job", complete: (state.jobs ?? []).length > 0, detail: "Pair target + vault" },
    { id: "schedule", label: "6. Schedule", complete: schedules.length > 0, detail: "Set cadence" },
    { id: "backup", label: "7. Run Test", complete: latestRun?.status === "succeeded", detail: latestRun ? `Status: ${latestRun.status}` : "Run first export" }
  ];
  const completeCount = steps.filter((step) => step.complete).length;

  return (
    <section className="setup-guide">
      <div className="setup-guide-header">
        <div>
          <p className="eyebrow">Protection Pipeline</p>
          <h2>Interactive Setup Navigator</h2>
          <p className="subtle">Click any step below to jump directly to its configuration and manage resources.</p>
        </div>
        <div className="setup-progress" aria-label={`${completeCount} of ${steps.length} setup steps complete`}>
          <strong>{completeCount}/{steps.length}</strong>
          <span>steps ready</span>
        </div>
      </div>
      <div className="setup-rail">
        {steps.map((step) => (
          <button
            type="button"
            key={step.id}
            className={`setup-step-btn setup-step ${step.complete ? "complete" : ""} ${selectedTab === step.id ? "active" : ""}`}
            onClick={() => onSelectTab(step.id)}
          >
            <CheckCircle2 size={18} />
            <strong>{step.label}</strong>
            <span>{step.detail}</span>
          </button>
        ))}
      </div>
    </section>
  );
}

function ProjectForm({ client, actionLoading, perform }: { client: ApiClient; actionLoading: string | null; perform: Perform }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  return (
    <ResourceForm
      title="Create Project"
      icon={<Plus size={18} />}
      loading={actionLoading === "project"}
      submitLabel="Create project"
      onSubmit={() =>
        perform("project", async () => {
          await client.request("/api/v1/projects", {
            method: "POST",
            body: JSON.stringify({ name, description: description || null })
          });
          setName("");
          setDescription("");
          return "Project created.";
        })
      }
    >
      <Field label="Project name">
        <input value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Pilates Platform Prod" required />
      </Field>
      <Field label="Description (optional)">
        <input value={description} onChange={(event) => setDescription(event.target.value)} placeholder="Main production database" />
      </Field>
    </ResourceForm>
  );
}

function SecretForm({ client, actionLoading, perform }: { client: ApiClient; actionLoading: string | null; perform: Perform }) {
  const [label, setLabel] = useState("");
  const [kind, setKind] = useState<SecretKind>("convex_deploy_key");
  const [value, setValue] = useState("");

  return (
    <ResourceForm
      title="Store Deploy Key / Secret"
      icon={<KeyRound size={18} />}
      loading={actionLoading === "secret"}
      submitLabel="Save secret"
      onSubmit={() =>
        perform("secret", async () => {
          await client.request("/api/v1/secrets", {
            method: "POST",
            body: JSON.stringify({ label, kind, value })
          });
          setLabel("");
          setValue("");
          return "Encrypted secret saved.";
        })
      }
    >
      <Field label="Secret label">
        <input value={label} onChange={(event) => setLabel(event.target.value)} placeholder="e.g. prod-deploy-key" required />
      </Field>
      <Field label="Kind">
        <Select value={kind} onChange={(val) => setKind(val as SecretKind)} items={secretKinds.map((k) => [k, sentenceCase(k)])} required />
      </Field>
      <Field label="Secret value (Deploy Key)">
        <input value={value} onChange={(event) => setValue(event.target.value)} type="password" placeholder="prod:deployment-name|..." required />
      </Field>
    </ResourceForm>
  );
}

function TargetForm({ client, state, actionLoading, perform }: { client: ApiClient; state: ServiceState; actionLoading: string | null; perform: Perform }) {
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("");
  const [deployment, setDeployment] = useState("");
  const [url, setUrl] = useState("");
  const [secretId, setSecretId] = useState("");

  useEffect(() => {
    if (!projectId && state.projects?.[0]) setProjectId(state.projects[0].id);
    if (!secretId && state.secrets?.[0]) setSecretId(state.secrets[0].id);
  }, [projectId, secretId, state.projects, state.secrets]);

  return (
    <ResourceForm
      title="Connect Convex Target"
      icon={<DatabaseBackup size={18} />}
      loading={actionLoading === "target"}
      submitLabel="Create target"
      onSubmit={() =>
        perform("target", async () => {
          await client.request("/api/v1/targets/cloud", {
            method: "POST",
            body: JSON.stringify({
              project_id: projectId,
              name,
              deployment,
              url: url.trim() || undefined,
              deploy_key_secret_id: secretId || null
            })
          });
          setName("");
          setDeployment("");
          setUrl("");
          return "Convex target created.";
        })
      }
    >
      <Field label="Project">
        <Select value={projectId} onChange={setProjectId} items={(state.projects ?? []).map((project) => [project.id, project.name])} required />
      </Field>
      <Field label="Target label">
        <input value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Production Cloud" required />
      </Field>
      <Field label="Convex deployment name">
        <input value={deployment} onChange={(event) => setDeployment(event.target.value)} placeholder="e.g. happy-animal-123" required />
      </Field>
      <Field label="Convex Cloud / Data API URL (Optional)">
        <input value={url} onChange={(event) => setUrl(event.target.value)} placeholder={deployment ? `https://${deployment}.convex.cloud` : "https://<deployment>.convex.cloud"} />
        <span className="subtle" style={{ fontSize: "0.78rem", marginTop: "0.25rem", display: "block" }}>
          Cloud Data URL: <code>https://{deployment || "<deployment>"}.convex.cloud</code> | Actions Site URL: <code>https://{deployment || "<deployment>"}.convex.site</code>
        </span>
      </Field>
      <Field label="Deploy key secret">
        <Select value={secretId} onChange={setSecretId} items={(state.secrets ?? []).map((secret) => [secret.id, secret.label])} required />
      </Field>
    </ResourceForm>
  );
}

function DestinationForm({ client, state, actionLoading, perform }: { client: ApiClient; state: ServiceState; actionLoading: string | null; perform: Perform }) {
  const [type, setType] = useState<"local" | "s3">("local");
  const [name, setName] = useState("");
  const [root, setRoot] = useState("/home/user/backups");
  const [bucket, setBucket] = useState("");
  const [region, setRegion] = useState("us-east-1");
  const [prefix, setPrefix] = useState("");
  const [secretId, setSecretId] = useState("");

  useEffect(() => {
    const s3Secret = (state.secrets ?? []).find((secret) => secret.kind === "s3_credentials") ?? (state.secrets ?? [])[0];
    if (!secretId && s3Secret) setSecretId(s3Secret.id);
  }, [secretId, state.secrets]);

  return (
    <ResourceForm
      title="Create Storage Destination"
      icon={<HardDrive size={18} />}
      loading={actionLoading === "destination"}
      submitLabel="Create destination"
      onSubmit={() =>
        perform("destination", async () => {
          if (type === "local") {
            await client.request("/api/v1/destinations/local", {
              method: "POST",
              body: JSON.stringify({ name, root })
            });
          } else {
            await client.request("/api/v1/destinations/s3", {
              method: "POST",
              body: JSON.stringify({
                name,
                bucket,
                region,
                prefix: prefix || null,
                credentials_secret_id: secretId || null
              })
            });
          }
          setName("");
          return "Storage destination created.";
        })
      }
    >
      <Field label="Type">
        <Select
          value={type}
          onChange={(val) => setType(val as "local" | "s3")}
          items={[
            ["local", "Local filesystem"],
            ["s3", "S3-compatible object storage"]
          ]}
          required
        />
      </Field>
      <Field label="Destination name">
        <input value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Local Backup Folder" required />
      </Field>
      {type === "local" ? (
        <Field label="Root directory path">
          <input value={root} onChange={(event) => setRoot(event.target.value)} placeholder="/home/user/backups" required />
        </Field>
      ) : (
        <>
          <Field label="Bucket name">
            <input value={bucket} onChange={(event) => setBucket(event.target.value)} placeholder="convex-backups-bucket" required />
          </Field>
          <Field label="Region">
            <input value={region} onChange={(event) => setRegion(event.target.value)} placeholder="us-east-1" required />
          </Field>
          <Field label="Prefix (optional)">
            <input value={prefix} onChange={(event) => setPrefix(event.target.value)} placeholder="backups/" />
          </Field>
          <Field label="S3 credentials secret">
            <Select value={secretId} onChange={setSecretId} items={(state.secrets ?? []).map((secret) => [secret.id, secret.label])} required />
          </Field>
        </>
      )}
    </ResourceForm>
  );
}

function JobForm({ client, state, actionLoading, perform }: { client: ApiClient; state: ServiceState; actionLoading: string | null; perform: Perform }) {
  const [projectId, setProjectId] = useState("");
  const [targetId, setTargetId] = useState("");
  const [destinationId, setDestinationId] = useState("");
  const [name, setName] = useState("Full backup");
  const [includeFileStorage, setIncludeFileStorage] = useState(true);

  useEffect(() => {
    if (!projectId && state.projects?.[0]) setProjectId(state.projects[0].id);
    if (!targetId && state.targets?.[0]) setTargetId(state.targets[0].id);
    if (!destinationId && state.destinations?.[0]) setDestinationId(state.destinations[0].id);
  }, [destinationId, projectId, state.destinations, state.projects, state.targets, targetId]);

  return (
    <ResourceForm
      title="Create Backup Job"
      icon={<Activity size={18} />}
      loading={actionLoading === "job"}
      submitLabel="Create job"
      onSubmit={() =>
        perform("job", async () => {
          await client.request("/api/v1/jobs", {
            method: "POST",
            body: JSON.stringify({
              project_id: projectId,
              target_id: targetId,
              destination_id: destinationId,
              name,
              include_file_storage: includeFileStorage
            })
          });
          return "Backup job created.";
        })
      }
    >
      <Field label="Job name">
        <input value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Daily Production Export" required />
      </Field>
      <Field label="Project">
        <Select value={projectId} onChange={setProjectId} items={(state.projects ?? []).map((project) => [project.id, project.name])} required />
      </Field>
      <Field label="Convex target">
        <Select value={targetId} onChange={setTargetId} items={(state.targets ?? []).map((target) => [target.id, target.name])} required />
      </Field>
      <Field label="Storage destination">
        <Select value={destinationId} onChange={setDestinationId} items={(state.destinations ?? []).map((destination) => [destination.id, destination.name])} required />
      </Field>
    </ResourceForm>
  );
}
