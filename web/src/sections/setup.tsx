import { useEffect, useState } from "react";
import { Activity, CheckCircle2, DatabaseBackup, HardDrive, KeyRound, Play, Plus } from "lucide-react";
import {
  ApiClient,
  destinationLabel,
  sentenceCase,
  type SecretKind,
  type ServiceState
} from "../appState";
import { secretKinds } from "../constants";
import { Field, ResourceForm, ResourceList, Select } from "../components/common";
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

  return (
    <div className="page-stack">
      <SetupGuide state={state} activeTask={activeTask} />

      <section className="setup-workspace">
        <div className="setup-primary">
          <CurrentSetupTask
            task={activeTask}
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

      <details className="manual-config">
        <summary>Manual configuration</summary>
        <section className="form-grid">
          <ProjectForm client={client} actionLoading={actionLoading} perform={perform} />
          <SecretForm client={client} actionLoading={actionLoading} perform={perform} />
          <TargetForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <DestinationForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <JobForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
          <ScheduleForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
        </section>
      </details>

      <section className="split three">
        <ResourceList title="Projects" items={state.projects.map((project) => [project.name, project.description ?? project.id])} />
        <ResourceList title="Targets" items={state.targets.map((target) => [target.name, target.deployment])} />
        <ResourceList title="Destinations" items={state.destinations.map((destination) => [destination.name, destinationLabel(destination)])} />
      </section>
    </div>
  );
}

function currentSetupTask(state: ServiceState): SetupTask {
  const latestRun = state.runs[0]?.run;
  if (state.projects.length === 0) return "project";
  if (state.targets.length === 0 && !state.secrets.some((secret) => secret.kind === "convex_deploy_key")) return "secret";
  if (state.targets.length === 0) return "target";
  if (state.destinations.length === 0) return "destination";
  if (state.jobs.length === 0) return "job";
  if (state.schedules.length === 0) return "schedule";
  if (latestRun?.status !== "succeeded") return "backup";
  return "complete";
}

function CurrentSetupTask({
  task,
  client,
  state,
  actionLoading,
  perform
}: {
  task: SetupTask;
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
}) {
  if (task === "project") {
    return (
      <TaskFrame title="Start with a project" detail="Name the Convex app or customer deployment you want protected.">
        <ProjectForm client={client} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "secret") {
    return (
      <TaskFrame title="Store the Convex deploy key" detail="Use a deploy key with permission to export the target deployment. It is encrypted before being stored.">
        <SecretForm client={client} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "target") {
    return (
      <TaskFrame title="Connect the Convex deployment" detail="Point the project at the Convex deployment name that should be exported.">
        <TargetForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "destination") {
    return (
      <TaskFrame title="Choose where backups land" detail="Start with a local vault, then add S3-compatible offsite storage when you are ready.">
        <DestinationForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "job") {
    return (
      <TaskFrame title="Create the backup job" detail="Bind the target and destination together into a runnable full backup.">
        <JobForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "schedule") {
    return (
      <TaskFrame title="Automate the cadence" detail="Choose how often the service should check and run this backup job.">
        <ScheduleForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
      </TaskFrame>
    );
  }
  if (task === "backup") {
    const firstJob = state.jobs[0];
    return (
      <section className="panel setup-run-card">
        <div>
          <p className="eyebrow">Final setup step</p>
          <h2>Run and verify the first backup</h2>
          <p className="subtle">This proves the deploy key, Convex export runner, destination path, and manifest writing all work together.</p>
        </div>
        <button
          type="button"
          disabled={!firstJob || actionLoading === "first-backup"}
          onClick={() =>
            perform("first-backup", async () => {
              if (!firstJob) return "Create a backup job before running the first backup.";
              await client.request(`/api/v1/jobs/${firstJob.id}/run`, { method: "POST" });
              return "First backup run started. Check Runs for the result, then verify the archive.";
            })
          }
        >
          <Play size={16} /> Run first backup
        </button>
      </section>
    );
  }
  return (
    <section className="panel setup-run-card complete">
      <CheckCircle2 size={24} />
      <div>
        <p className="eyebrow">Protected</p>
        <h2>Backups are configured</h2>
        <p className="subtle">Use Runs for manual execution and restore verification, or DR Center for schedule and readiness checks.</p>
      </div>
    </section>
  );
}

function TaskFrame({ title, detail, children }: { title: string; detail: string; children: React.ReactNode }) {
  return (
    <div className="task-frame">
      <div className="task-frame-copy">
        <p className="eyebrow">Current task</p>
        <h2>{title}</h2>
        <p className="subtle">{detail}</p>
      </div>
      {children}
    </div>
  );
}

function PanelSummary({ state }: { state: ServiceState }) {
  const rows = [
    ["Projects", state.projects.length],
    ["Deploy keys", state.secrets.filter((secret) => secret.kind === "convex_deploy_key").length],
    ["Targets", state.targets.length],
    ["Destinations", state.destinations.length],
    ["Jobs", state.jobs.length],
    ["Schedules", state.schedules.length]
  ];

  return (
    <div className="stack compact">
      <div>
        <p className="eyebrow">Configured now</p>
        <h2>Install inventory</h2>
      </div>
      <div className="inventory-list">
        {rows.map(([label, value]) => (
          <div className="inventory-row" key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      <p className="subtle">The setup flow only advances after the server confirms each saved resource.</p>
    </div>
  );
}

function SetupGuide({
  state,
  activeTask
}: {
  state: ServiceState;
  activeTask: SetupTask;
}) {
  const latestRun = state.runs[0]?.run;
  const steps = [
    { id: "project", label: "Project", complete: state.projects.length > 0, detail: "Backup owner is named" },
    { id: "target", label: "Convex target", complete: state.targets.length > 0, detail: "Deployment and deploy key are connected" },
    { id: "destination", label: "Destination", complete: state.destinations.length > 0, detail: "Local or S3 vault is ready" },
    { id: "schedule", label: "Schedule", complete: state.schedules.length > 0, detail: "Automatic cadence is set" },
    { id: "backup", label: "First backup", complete: latestRun?.status === "succeeded", detail: latestRun ? `Latest run: ${latestRun.status}` : "Run and verify the first archive" }
  ];
  const completeCount = steps.filter((step) => step.complete).length;

  return (
    <section className="setup-guide">
      <div className="setup-guide-header">
        <div>
          <p className="eyebrow">First-run protection path</p>
          <h2>Get this deployment backed up</h2>
          <p className="subtle">Complete the current task below. Manual configuration stays available for advanced edits.</p>
        </div>
        <div className="setup-progress" aria-label={`${completeCount} of ${steps.length} setup steps complete`}>
          <strong>{completeCount}/{steps.length}</strong>
          <span>ready</span>
        </div>
      </div>
      <div className="setup-rail">
        {steps.map((step) => (
          <div className={`setup-step ${step.complete ? "complete" : ""} ${activeTask === step.id ? "active" : ""}`} key={step.label}>
            <CheckCircle2 size={18} />
            <strong>{step.label}</strong>
            <span>{step.detail}</span>
          </div>
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
      title="Project"
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
      <Field label="Name">
        <input value={name} onChange={(event) => setName(event.target.value)} required />
      </Field>
      <Field label="Description">
        <input value={description} onChange={(event) => setDescription(event.target.value)} />
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
      title="Encrypted secret"
      icon={<KeyRound size={18} />}
      loading={actionLoading === "secret"}
      submitLabel="Store secret"
      onSubmit={() =>
        perform("secret", async () => {
          await client.request("/api/v1/secrets", {
            method: "POST",
            body: JSON.stringify({ label, kind, value })
          });
          setLabel("");
          setValue("");
          return "Secret encrypted and stored.";
        })
      }
    >
      <Field label="Label">
        <input value={label} onChange={(event) => setLabel(event.target.value)} required />
      </Field>
      <Field label="Kind">
        <select value={kind} onChange={(event) => setKind(event.target.value as SecretKind)}>
          {secretKinds.map((option) => (
            <option key={option} value={option}>
              {sentenceCase(option)}
            </option>
          ))}
        </select>
      </Field>
      <Field label="Secret value">
        <input value={value} onChange={(event) => setValue(event.target.value)} type="password" required />
      </Field>
    </ResourceForm>
  );
}

function TargetForm({
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
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("Production");
  const [deployment, setDeployment] = useState("");
  const [secretMode, setSecretMode] = useState<"stored" | "env">("stored");
  const [secretId, setSecretId] = useState("");
  const [deployKeyEnv, setDeployKeyEnv] = useState("CONVEX_DEPLOY_KEY");

  useEffect(() => {
    if (!projectId && state.projects[0]) setProjectId(state.projects[0].id);
    if (!secretId && state.secrets[0]) setSecretId(state.secrets[0].id);
  }, [projectId, secretId, state.projects, state.secrets]);

  return (
    <ResourceForm
      title="Convex target"
      icon={<DatabaseBackup size={18} />}
      loading={actionLoading === "target"}
      submitLabel="Add target"
      onSubmit={() =>
        perform("target", async () => {
          await client.request("/api/v1/targets/cloud", {
            method: "POST",
            body: JSON.stringify({
              project_id: projectId,
              name,
              deployment,
              deploy_key_secret_id: secretMode === "stored" ? secretId : null,
              deploy_key_env: secretMode === "env" ? deployKeyEnv : null
            })
          });
          setDeployment("");
          return "Convex target added.";
        })
      }
    >
      <Field label="Project">
        <Select value={projectId} onChange={setProjectId} items={state.projects.map((project) => [project.id, project.name])} required />
      </Field>
      <Field label="Target name">
        <input value={name} onChange={(event) => setName(event.target.value)} required />
      </Field>
      <Field label="Deployment">
        <input value={deployment} onChange={(event) => setDeployment(event.target.value)} required />
      </Field>
      <Field label="Deploy key source">
        <select value={secretMode} onChange={(event) => setSecretMode(event.target.value as "stored" | "env")}>
          <option value="stored">Encrypted secret</option>
          <option value="env">Environment variable</option>
        </select>
      </Field>
      {secretMode === "stored" ? (
        <Field label="Stored secret">
          <Select value={secretId} onChange={setSecretId} items={state.secrets.map((secret) => [secret.id, secret.label])} required />
        </Field>
      ) : (
        <Field label="Deploy key env">
          <input value={deployKeyEnv} onChange={(event) => setDeployKeyEnv(event.target.value)} required />
        </Field>
      )}
    </ResourceForm>
  );
}

function DestinationForm({
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
  const [kind, setKind] = useState<"local" | "s3">("local");
  const [name, setName] = useState("Primary vault");
  const [root, setRoot] = useState("/var/lib/convex-autobackup/backups");
  const [bucket, setBucket] = useState("");
  const [region, setRegion] = useState("");
  const [endpoint, setEndpoint] = useState("");
  const [prefix, setPrefix] = useState("");
  const [secretId, setSecretId] = useState("");
  const [keepLast, setKeepLast] = useState(20);
  const [keepDays, setKeepDays] = useState(30);

  useEffect(() => {
    const s3Secret = state.secrets.find((secret) => secret.kind === "s3_credentials") ?? state.secrets[0];
    if (!secretId && s3Secret) setSecretId(s3Secret.id);
  }, [secretId, state.secrets]);

  return (
    <ResourceForm
      title="Destination"
      icon={<HardDrive size={18} />}
      loading={actionLoading === "destination"}
      submitLabel="Create destination"
      onSubmit={() =>
        perform("destination", async () => {
          const retention = { keep_last: keepLast, keep_days: keepDays, keep_weeklies: 12, keep_monthlies: 12 };
          if (kind === "local") {
            await client.request("/api/v1/destinations/local", {
              method: "POST",
              body: JSON.stringify({ name, root, retention })
            });
          } else {
            await client.request("/api/v1/destinations/s3", {
              method: "POST",
              body: JSON.stringify({
                name,
                bucket,
                region: region || null,
                endpoint: endpoint || null,
                prefix: prefix || null,
                credentials_secret_id: secretId,
                retention
              })
            });
          }
          return "Destination created.";
        })
      }
    >
      <Field label="Destination type">
        <select value={kind} onChange={(event) => setKind(event.target.value as "local" | "s3")}>
          <option value="local">Local filesystem</option>
          <option value="s3">S3-compatible</option>
        </select>
      </Field>
      <Field label="Name">
        <input value={name} onChange={(event) => setName(event.target.value)} required />
      </Field>
      {kind === "local" ? (
        <Field label="Root path">
          <input value={root} onChange={(event) => setRoot(event.target.value)} required />
        </Field>
      ) : (
        <>
          <Field label="Bucket">
            <input value={bucket} onChange={(event) => setBucket(event.target.value)} required />
          </Field>
          <Field label="Region">
            <input value={region} onChange={(event) => setRegion(event.target.value)} />
          </Field>
          <Field label="Endpoint">
            <input value={endpoint} onChange={(event) => setEndpoint(event.target.value)} />
          </Field>
          <Field label="Prefix">
            <input value={prefix} onChange={(event) => setPrefix(event.target.value)} />
          </Field>
          <Field label="Credentials secret">
            <Select value={secretId} onChange={setSecretId} items={state.secrets.map((secret) => [secret.id, secret.label])} required />
          </Field>
        </>
      )}
      <div className="two-fields">
        <Field label="Keep last">
          <input value={keepLast} onChange={(event) => setKeepLast(Number(event.target.value))} min={1} type="number" required />
        </Field>
        <Field label="Keep days">
          <input value={keepDays} onChange={(event) => setKeepDays(Number(event.target.value))} min={1} type="number" required />
        </Field>
      </div>
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
    if (!projectId && state.projects[0]) setProjectId(state.projects[0].id);
    if (!targetId && state.targets[0]) setTargetId(state.targets[0].id);
    if (!destinationId && state.destinations[0]) setDestinationId(state.destinations[0].id);
  }, [destinationId, projectId, state.destinations, state.projects, state.targets, targetId]);

  return (
    <ResourceForm
      title="Backup job"
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
      <Field label="Project">
        <Select value={projectId} onChange={setProjectId} items={state.projects.map((project) => [project.id, project.name])} required />
      </Field>
      <Field label="Target">
        <Select value={targetId} onChange={setTargetId} items={state.targets.map((target) => [target.id, target.name])} required />
      </Field>
      <Field label="Destination">
        <Select value={destinationId} onChange={setDestinationId} items={state.destinations.map((destination) => [destination.id, destination.name])} required />
      </Field>
      <Field label="Job name">
        <input value={name} onChange={(event) => setName(event.target.value)} required />
      </Field>
      <label className="check-row">
        <input checked={includeFileStorage} onChange={(event) => setIncludeFileStorage(event.target.checked)} type="checkbox" />
        Include Convex file storage
      </label>
    </ResourceForm>
  );
}
