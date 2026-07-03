import { useEffect, useState } from "react";
import { Activity, Clock3, DatabaseBackup, HardDrive, KeyRound, Plus } from "lucide-react";
import {
  ApiClient,
  destinationLabel,
  sentenceCase,
  type SecretKind,
  type ServiceState
} from "../appState";
import { secretKinds, weekdays } from "../constants";
import { Field, ResourceForm, ResourceList, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

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
  return (
    <div className="page-stack">
      <section className="form-grid">
        <ProjectForm client={client} actionLoading={actionLoading} perform={perform} />
        <SecretForm client={client} actionLoading={actionLoading} perform={perform} />
        <TargetForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
        <DestinationForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
        <JobForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
        <ScheduleForm client={client} state={state} actionLoading={actionLoading} perform={perform} />
      </section>

      <section className="split three">
        <ResourceList title="Projects" items={state.projects.map((project) => [project.name, project.description ?? project.id])} />
        <ResourceList title="Targets" items={state.targets.map((target) => [target.name, target.deployment])} />
        <ResourceList title="Destinations" items={state.destinations.map((destination) => [destination.name, destinationLabel(destination)])} />
      </section>
    </div>
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

function ScheduleForm({ client, state, actionLoading, perform }: { client: ApiClient; state: ServiceState; actionLoading: string | null; perform: Perform }) {
  const [jobId, setJobId] = useState("");
  const [mode, setMode] = useState<"interval_minutes" | "daily" | "weekly" | "cron">("interval_minutes");
  const [every, setEvery] = useState(60);
  const [time, setTime] = useState("02:00:00");
  const [weekday, setWeekday] = useState("Mon");
  const [expression, setExpression] = useState("0 0 2 * * *");
  const [missedRunPolicy, setMissedRunPolicy] = useState<"run_once_on_resume" | "skip">("run_once_on_resume");

  useEffect(() => {
    if (!jobId && state.jobs[0]) setJobId(state.jobs[0].id);
  }, [jobId, state.jobs]);

  return (
    <ResourceForm
      title="Schedule"
      icon={<Clock3 size={18} />}
      loading={actionLoading === "schedule"}
      submitLabel="Create schedule"
      onSubmit={() =>
        perform("schedule", async () => {
          const schedule =
            mode === "interval_minutes"
              ? { type: mode, every }
              : mode === "daily"
                ? { type: mode, time }
                : mode === "weekly"
                  ? { type: mode, weekday, time }
                  : { type: mode, expression };
          await client.request("/api/v1/schedules", {
            method: "POST",
            body: JSON.stringify({ job_id: jobId, schedule, missed_run_policy: missedRunPolicy, enabled: true })
          });
          return "Schedule created.";
        })
      }
    >
      <Field label="Job">
        <Select value={jobId} onChange={setJobId} items={state.jobs.map((job) => [job.id, job.name])} required />
      </Field>
      <Field label="Mode">
        <select value={mode} onChange={(event) => setMode(event.target.value as "interval_minutes" | "daily" | "weekly" | "cron")}>
          <option value="interval_minutes">Interval</option>
          <option value="daily">Daily</option>
          <option value="weekly">Weekly</option>
          <option value="cron">Cron</option>
        </select>
      </Field>
      {mode === "interval_minutes" && (
        <Field label="Every minutes">
          <input value={every} onChange={(event) => setEvery(Number(event.target.value))} min={1} type="number" required />
        </Field>
      )}
      {(mode === "daily" || mode === "weekly") && (
        <Field label="UTC time">
          <input value={time} onChange={(event) => setTime(event.target.value)} pattern="^[0-2][0-9]:[0-5][0-9]:[0-5][0-9]$" required />
        </Field>
      )}
      {mode === "weekly" && (
        <Field label="Weekday">
          <select value={weekday} onChange={(event) => setWeekday(event.target.value)}>
            {weekdays.map((day) => (
              <option key={day} value={day}>
                {day}
              </option>
            ))}
          </select>
        </Field>
      )}
      {mode === "cron" && (
        <Field label="Cron expression">
          <input value={expression} onChange={(event) => setExpression(event.target.value)} required />
        </Field>
      )}
      <Field label="Missed runs">
        <select value={missedRunPolicy} onChange={(event) => setMissedRunPolicy(event.target.value as "run_once_on_resume" | "skip")}>
          <option value="run_once_on_resume">Run once on resume</option>
          <option value="skip">Skip missed runs</option>
        </select>
      </Field>
    </ResourceForm>
  );
}
