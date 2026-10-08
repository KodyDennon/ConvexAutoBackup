import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  CheckCircle2,
  Circle,
  Cloud,
  HardDrive,
  KeyRound,
  Loader2,
  Lock,
  PartyPopper,
  Play,
  ShieldCheck,
  XCircle
} from "lucide-react";
import {
  ApiClient,
  destinationLabel,
  destinationTypeLabel,
  isEncrypted,
  type BackupJob,
  type Project,
  type ServiceState,
  type StorageDestination
} from "../appState";
import { Field } from "../components/common";

interface SystemCheck {
  name: string;
  ok: boolean;
  detail: string;
}

interface Presets {
  r2: { bucket: string; endpoint: string } | null;
  default_local_root: string;
  min_passphrase_len: number;
}

interface KeyCheck {
  ok: boolean;
  deployment?: string;
  key_kind?: string | null;
  table_count?: number;
  message: string;
}

interface RunResult {
  run_id: string;
  status: string;
  error?: string | null;
}

type StepId = "system" | "project" | "convex" | "storage" | "encryption" | "schedule" | "review" | "backup";

const STEPS: Array<{ id: StepId; title: string; hint: string }> = [
  { id: "system", title: "System check", hint: "Make sure this server is ready" },
  { id: "project", title: "Project", hint: "Name what you are protecting" },
  { id: "convex", title: "Convex deployment", hint: "Connect with a deploy key" },
  { id: "storage", title: "Storage", hint: "Where backups are kept" },
  { id: "encryption", title: "Encryption", hint: "Lock backups with a passphrase" },
  { id: "schedule", title: "Schedule", hint: "How often to back up" },
  { id: "review", title: "Review & create", hint: "Save everything" },
  { id: "backup", title: "First backup", hint: "Prove it works" }
];

const INTERVAL_CHOICES: Array<[number, string]> = [
  [60, "Every hour"],
  [120, "Every 2 hours"],
  [240, "Every 4 hours (recommended)"],
  [360, "Every 6 hours"],
  [720, "Every 12 hours"],
  [1440, "Every 24 hours"]
];

type ProgressState = "pending" | "running" | "done" | "failed";
interface ProgressItem {
  key: string;
  label: string;
  state: ProgressState;
  detail?: string;
}

/** Converts a local "HH:MM" to the UTC "HH:MM" the scheduler stores. */
function localTimeToUtc(time: string): string {
  const [hours, minutes] = time.split(":").map((part) => Number.parseInt(part, 10));
  const date = new Date();
  date.setHours(hours || 0, minutes || 0, 0, 0);
  return `${String(date.getUTCHours()).padStart(2, "0")}:${String(date.getUTCMinutes()).padStart(2, "0")}`;
}

export function SetupWizard({
  client,
  state,
  onFinished,
  onCancel,
  refresh
}: {
  client: ApiClient;
  state: ServiceState;
  onFinished: () => void;
  onCancel?: () => void;
  refresh: () => Promise<void>;
}) {
  const [step, setStep] = useState<StepId>("system");

  // System
  const [checks, setChecks] = useState<SystemCheck[] | null>(null);
  const [presets, setPresets] = useState<Presets | null>(null);
  const [systemError, setSystemError] = useState<string | null>(null);

  // Project
  const projectsWithoutJobs = useMemo(
    () => (state.projects ?? []).filter((project) => !(state.jobs ?? []).some((job) => job.project_id === project.id)),
    [state.projects, state.jobs]
  );
  const [projectMode, setProjectMode] = useState<"new" | "existing">("new");
  const [projectName, setProjectName] = useState("");
  const [projectDescription, setProjectDescription] = useState("");
  const [existingProjectId, setExistingProjectId] = useState("");

  // Convex
  const [deployKey, setDeployKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [keyCheck, setKeyCheck] = useState<KeyCheck | null>(null);
  const [checkingKey, setCheckingKey] = useState(false);
  const [includeFileStorage, setIncludeFileStorage] = useState(true);

  // Storage
  const [reuseDestinationIds, setReuseDestinationIds] = useState<string[]>([]);
  const [addLocal, setAddLocal] = useState(false);
  const [localRoot, setLocalRoot] = useState("");
  const [addR2, setAddR2] = useState(false);
  const [keepLast, setKeepLast] = useState(20);

  // Encryption
  const [passphrase, setPassphrase] = useState("");
  const [confirmPassphrase, setConfirmPassphrase] = useState("");
  const [savedPassphrase, setSavedPassphrase] = useState(false);
  const [skipEncryption, setSkipEncryption] = useState(false);
  const [encryptReused, setEncryptReused] = useState(true);

  // Schedule
  const [scheduleMode, setScheduleMode] = useState<"interval" | "daily">("interval");
  const [everyMinutes, setEveryMinutes] = useState(240);
  const [dailyTime, setDailyTime] = useState("02:00");

  // Create
  const [progress, setProgress] = useState<ProgressItem[]>([]);
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<{
    projectId?: string;
    secretId?: string;
    targetId?: string;
    localId?: string;
    r2Id?: string;
    encryptedReused?: string[];
    job?: BackupJob;
    scheduleId?: string;
  }>({});

  // First backup
  const [running, setRunning] = useState(false);
  const [runResult, setRunResult] = useState<RunResult | null>(null);
  const [verifyResult, setVerifyResult] = useState<{ ok: boolean; encrypted?: boolean; destination_name?: string } | null>(null);
  const [runError, setRunError] = useState<string | null>(null);

  const minLen = presets?.min_passphrase_len ?? 12;
  // Destinations this wizard run created are shown as "new", not as existing ones.
  const existingDestinations = (state.destinations ?? []).filter(
    (destination) => destination.id !== created.localId && destination.id !== created.r2Id
  );

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      client.request<{ checks: SystemCheck[] }>("/api/v1/system/checks"),
      client.request<Presets>("/api/v1/setup/presets")
    ])
      .then(([system, loadedPresets]) => {
        if (cancelled) return;
        setChecks(system.checks);
        setPresets(loadedPresets);
        setLocalRoot(loadedPresets.default_local_root);
        const hasLocal = existingDestinations.some((d) => d.kind.type === "local_filesystem");
        const hasR2 = existingDestinations.some((d) => d.kind.type === "s3_compatible");
        // Reuse what already exists; offer to create what is missing.
        setReuseDestinationIds(existingDestinations.map((d) => d.id));
        setAddLocal(!hasLocal);
        setAddR2(Boolean(loadedPresets.r2) && !hasR2);
      })
      .catch((caught) => !cancelled && setSystemError(caught instanceof Error ? caught.message : "System check failed"));
    return () => {
      cancelled = true;
    };
    // Run once when the wizard opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client]);

  const resolvedProjectName =
    projectMode === "existing"
      ? (state.projects ?? []).find((project) => project.id === existingProjectId)?.name ?? ""
      : projectName.trim();

  const selectedReused = existingDestinations.filter((d) => reuseDestinationIds.includes(d.id));
  const destinationCount = selectedReused.length + (addLocal ? 1 : 0) + (addR2 ? 1 : 0);
  const hasOffsite = selectedReused.some((d) => d.kind.type === "s3_compatible") || addR2;
  const newDestinationCount = (addLocal ? 1 : 0) + (addR2 ? 1 : 0);
  const unencryptedReused = selectedReused.filter((d) => !isEncrypted(d));
  const needsPassphrase = newDestinationCount > 0 || (unencryptedReused.length > 0 && encryptReused);
  const passphraseValid = passphrase.length >= minLen && passphrase === confirmPassphrase && savedPassphrase;

  // Once everything is created, the form steps are locked and only review/backup remain.
  const locked = Boolean(created.job && created.scheduleId);
  const stepValid: Record<StepId, boolean> = {
    system: Boolean(checks) && checks!.filter((check) => check.name !== "Offsite storage preset").every((check) => check.ok),
    project: locked || projectMode === "new" ? locked || projectName.trim().length > 0 : Boolean(existingProjectId),
    convex: locked || Boolean(keyCheck?.ok),
    storage: destinationCount > 0 && (!addLocal || localRoot.trim().length > 0),
    encryption: locked || !needsPassphrase || skipEncryption || passphraseValid,
    schedule: scheduleMode === "interval" ? everyMinutes > 0 : /^\d{2}:\d{2}$/.test(dailyTime),
    review: Boolean(created.job && created.scheduleId),
    backup: Boolean(verifyResult?.ok)
  };

  const stepIndex = STEPS.findIndex((item) => item.id === step);
  const furthestAllowed = STEPS.findIndex((item) => !stepValid[item.id]);
  const canOpen = (index: number) =>
    !creating &&
    !running &&
    index <= (furthestAllowed === -1 ? STEPS.length - 1 : furthestAllowed) &&
    (!locked || index >= STEPS.findIndex((item) => item.id === "review"));

  const goNext = () => {
    if (stepIndex < STEPS.length - 1) setStep(STEPS[stepIndex + 1].id);
  };
  const goBack = () => {
    if (stepIndex > 0) setStep(STEPS[stepIndex - 1].id);
  };

  const checkKey = async () => {
    setCheckingKey(true);
    setKeyCheck(null);
    try {
      setKeyCheck(
        await client.request<KeyCheck>("/api/v1/setup/check-deploy-key", {
          method: "POST",
          body: JSON.stringify({ deploy_key: deployKey.trim() })
        })
      );
    } catch (caught) {
      setKeyCheck({ ok: false, message: caught instanceof Error ? caught.message : "Check failed" });
    } finally {
      setCheckingKey(false);
    }
  };

  const retention = { keep_last: keepLast, keep_days: null, keep_weeklies: null, keep_monthlies: null };
  const effectivePassphrase = needsPassphrase && !skipEncryption ? passphrase : null;

  const createEverything = async () => {
    setCreating(true);
    const items: ProgressItem[] = [
      { key: "project", label: projectMode === "new" ? `Create project “${resolvedProjectName}”` : `Use project “${resolvedProjectName}”`, state: "pending" },
      { key: "secret", label: "Store the deploy key encrypted", state: "pending" },
      { key: "target", label: `Connect deployment ${keyCheck?.deployment ?? ""}`, state: "pending" },
      ...(addLocal ? [{ key: "local", label: "Create local backup folder", state: "pending" as ProgressState }] : []),
      ...(addR2 ? [{ key: "r2", label: "Create Cloudflare R2 destination", state: "pending" as ProgressState }] : []),
      ...(effectivePassphrase && encryptReused
        ? unencryptedReused.map((d) => ({ key: `enc-${d.id}`, label: `Encrypt “${d.name}”`, state: "pending" as ProgressState }))
        : []),
      { key: "test", label: "Test every destination (write, read, delete)", state: "pending" },
      { key: "job", label: "Create backup job", state: "pending" },
      { key: "schedule", label: "Create schedule", state: "pending" }
    ];
    // Keep finished items from a previous attempt.
    setProgress(items.map((item) => ({ ...item, state: progress.find((p) => p.key === item.key && p.state === "done") ? "done" : "pending" })));
    const mark = (key: string, value: ProgressState, detail?: string) =>
      setProgress((current) => current.map((item) => (item.key === key ? { ...item, state: value, detail } : item)));
    const next = { ...created };
    const runStep = async (key: string, action: () => Promise<string | void>) => {
      mark(key, "running");
      try {
        const detail = await action();
        mark(key, "done", detail || undefined);
      } catch (caught) {
        mark(key, "failed", caught instanceof Error ? caught.message : "Failed");
        throw caught;
      }
    };

    try {
      await runStep("project", async () => {
        if (next.projectId) return;
        if (projectMode === "existing") {
          next.projectId = existingProjectId;
          return;
        }
        const result = await client.request<{ project: Project }>("/api/v1/projects", {
          method: "POST",
          body: JSON.stringify({ name: projectName.trim(), description: projectDescription.trim() || null })
        });
        next.projectId = result.project.id;
      });
      await runStep("secret", async () => {
        if (next.secretId) return;
        const result = await client.request<{ secret: { id: string } }>("/api/v1/secrets", {
          method: "POST",
          body: JSON.stringify({ label: `${resolvedProjectName} deploy key`, kind: "convex_deploy_key", value: deployKey.trim() })
        });
        next.secretId = result.secret.id;
      });
      await runStep("target", async () => {
        if (next.targetId) return;
        const kind = keyCheck?.key_kind ? ` (${keyCheck.key_kind})` : "";
        const result = await client.request<{ target: { id: string } }>("/api/v1/targets/cloud", {
          method: "POST",
          body: JSON.stringify({
            project_id: next.projectId,
            name: `${resolvedProjectName}${kind}`,
            deployment: keyCheck?.deployment,
            deploy_key_secret_id: next.secretId
          })
        });
        next.targetId = result.target.id;
      });
      if (addLocal) {
        await runStep("local", async () => {
          if (next.localId) return;
          const result = await client.request<{ destination: StorageDestination }>("/api/v1/destinations/local", {
            method: "POST",
            body: JSON.stringify({ name: "Local (this server)", root: localRoot.trim(), retention, encryption_passphrase: effectivePassphrase })
          });
          next.localId = result.destination.id;
        });
      }
      if (addR2) {
        await runStep("r2", async () => {
          if (next.r2Id) return;
          const result = await client.request<{ destination: StorageDestination }>("/api/v1/setup/presets/r2", {
            method: "POST",
            body: JSON.stringify({ name: "Cloudflare R2 (offsite)", retention, encryption_passphrase: effectivePassphrase })
          });
          next.r2Id = result.destination.id;
        });
      }
      if (effectivePassphrase && encryptReused) {
        for (const destination of unencryptedReused) {
          await runStep(`enc-${destination.id}`, async () => {
            if (next.encryptedReused?.includes(destination.id)) return;
            await client.request(`/api/v1/destinations/${destination.id}/encryption`, {
              method: "PUT",
              body: JSON.stringify({ passphrase: effectivePassphrase })
            });
            next.encryptedReused = [...(next.encryptedReused ?? []), destination.id];
          });
        }
      }
      const destinationIds = [
        ...(addLocal && next.localId ? [next.localId] : []),
        ...selectedReused.filter((d) => d.kind.type === "local_filesystem").map((d) => d.id),
        ...(addR2 && next.r2Id ? [next.r2Id] : []),
        ...selectedReused.filter((d) => d.kind.type !== "local_filesystem").map((d) => d.id)
      ];
      await runStep("test", async () => {
        const failures: string[] = [];
        for (const id of destinationIds) {
          const result = await client.request<{ ok: boolean; detail: string }>(`/api/v1/destinations/${id}/test`, { method: "POST" });
          if (!result.ok) failures.push(result.detail);
        }
        if (failures.length > 0) throw new Error(failures.join("; "));
        return `${destinationIds.length} destination${destinationIds.length === 1 ? "" : "s"} OK`;
      });
      await runStep("job", async () => {
        if (next.job) return;
        const result = await client.request<{ job: BackupJob }>("/api/v1/jobs", {
          method: "POST",
          body: JSON.stringify({
            project_id: next.projectId,
            target_id: next.targetId,
            destination_id: destinationIds[0],
            additional_destination_ids: destinationIds.slice(1),
            name: `Full backup - ${resolvedProjectName}`,
            include_file_storage: includeFileStorage
          })
        });
        next.job = result.job;
      });
      await runStep("schedule", async () => {
        if (next.scheduleId) return;
        const schedule =
          scheduleMode === "interval"
            ? { type: "interval_minutes", every: everyMinutes }
            : { type: "daily", time: localTimeToUtc(dailyTime) };
        const result = await client.request<{ schedule: { id: string; next_due_at: string } }>("/api/v1/schedules", {
          method: "POST",
          body: JSON.stringify({ job_id: next.job!.id, schedule, missed_run_policy: "run_once_on_resume", enabled: true })
        });
        next.scheduleId = result.schedule.id;
        return `Next run ${new Date(result.schedule.next_due_at).toLocaleString()}`;
      });
      // Remove destinations this run created but the user deselected after a failed test.
      for (const [selected, id] of [[addLocal, next.localId], [addR2, next.r2Id]] as const) {
        if (!selected && id) {
          await client.request(`/api/v1/destinations/${id}`, { method: "DELETE" }).catch(() => undefined);
        }
      }
      if (!addLocal) next.localId = undefined;
      if (!addR2) next.r2Id = undefined;
      setDeployKey("");
      setPassphrase("");
      setConfirmPassphrase("");
    } catch {
      // The failed item shows its error; "Retry" resumes from it without duplicating finished items.
    } finally {
      setCreated(next);
      setCreating(false);
      await refresh();
    }
  };

  const runFirstBackup = async () => {
    if (!created.job) return;
    setRunning(true);
    setRunError(null);
    setRunResult(null);
    setVerifyResult(null);
    try {
      const result = await client.request<{ run: RunResult }>(`/api/v1/jobs/${created.job.id}/run`, { method: "POST" });
      setRunResult(result.run);
      if (result.run.status === "failed") {
        setRunError(result.run.error ?? "Backup failed");
        return;
      }
      const verification = await client.request<{ verification: { ok: boolean; encrypted?: boolean; destination_name?: string } }>(
        `/api/v1/runs/${result.run.run_id}/verify`,
        { method: "POST" }
      );
      setVerifyResult(verification.verification);
      if (!verification.verification.ok) setRunError("The backup was stored but did not verify. Check the Runs page for details.");
    } catch (caught) {
      setRunError(caught instanceof Error ? caught.message : "Backup failed");
    } finally {
      setRunning(false);
      await refresh();
    }
  };

  return (
    <div className="wizard">
      <aside className="wizard-steps" aria-label="Setup steps">
        <p className="eyebrow">Add a project to backups</p>
        <ol>
          {STEPS.map((item, index) => {
            const done = stepValid[item.id] && index < stepIndex;
            return (
              <li key={item.id}>
                <button
                  type="button"
                  className={`wizard-step ${item.id === step ? "active" : ""} ${done ? "done" : ""}`}
                  disabled={!canOpen(index)}
                  onClick={() => setStep(item.id)}
                >
                  <span className="wizard-step-icon">{done ? <CheckCircle2 size={18} /> : <span>{index + 1}</span>}</span>
                  <span>
                    <strong>{item.title}</strong>
                    <small>{item.hint}</small>
                  </span>
                </button>
              </li>
            );
          })}
        </ol>
        {onCancel && (
          <button type="button" className="secondary-button small wizard-exit" onClick={onCancel} disabled={creating || running}>
            Exit wizard
          </button>
        )}
      </aside>

      <section className="wizard-panel panel">
        {step === "system" && (
          <WizardStep title="Let's check this server first" detail="Everything below must be green before backups can run.">
            {systemError && <p className="callout warning">{systemError}</p>}
            {!checks && !systemError && <p className="subtle"><Loader2 size={16} className="spin" /> Running checks…</p>}
            {checks && (
              <ul className="check-list">
                {checks.map((check) => (
                  <li key={check.name} className={check.ok ? "ok" : check.name === "Offsite storage preset" ? "info" : "bad"}>
                    {check.ok ? <CheckCircle2 size={18} /> : check.name === "Offsite storage preset" ? <Circle size={18} /> : <XCircle size={18} />}
                    <span>
                      <strong>{check.name}</strong>
                      <small>{check.detail}</small>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </WizardStep>
        )}

        {step === "project" && (
          <WizardStep title="Which project are you protecting?" detail="A project groups one Convex app and its backups. Use the app or client name.">
            {projectsWithoutJobs.length > 0 && (
              <div className="segmented">
                <button type="button" className={projectMode === "new" ? "active" : ""} onClick={() => setProjectMode("new")}>New project</button>
                <button type="button" className={projectMode === "existing" ? "active" : ""} onClick={() => setProjectMode("existing")}>Existing project</button>
              </div>
            )}
            {projectMode === "new" ? (
              <>
                <Field label="Project name">
                  <input autoFocus value={projectName} onChange={(event) => setProjectName(event.target.value)} placeholder="e.g. Tides Pilates" />
                </Field>
                <Field label="Description (optional)">
                  <input value={projectDescription} onChange={(event) => setProjectDescription(event.target.value)} placeholder="e.g. Production booking app" />
                </Field>
              </>
            ) : (
              <Field label="Project">
                <select value={existingProjectId} onChange={(event) => setExistingProjectId(event.target.value)}>
                  <option value="">Select a project</option>
                  {projectsWithoutJobs.map((project) => (
                    <option key={project.id} value={project.id}>{project.name}</option>
                  ))}
                </select>
              </Field>
            )}
          </WizardStep>
        )}

        {step === "convex" && (
          <WizardStep
            title="Connect the Convex deployment"
            detail="Paste a deploy key from the Convex dashboard → your project → Settings → Deploy keys. Use the Production key to back up production."
          >
            <Field label="Deploy key">
              <div className="input-with-action">
                <input
                  type={showKey ? "text" : "password"}
                  autoComplete="off"
                  spellCheck={false}
                  value={deployKey}
                  onChange={(event) => {
                    setDeployKey(event.target.value);
                    setKeyCheck(null);
                  }}
                  placeholder="prod:happy-otter-123|eyJ…"
                />
                <button type="button" className="secondary-button small" onClick={() => setShowKey((value) => !value)}>
                  {showKey ? "Hide" : "Show"}
                </button>
              </div>
            </Field>
            <button type="button" className="primary-button" disabled={!deployKey.trim() || checkingKey} onClick={() => void checkKey()}>
              {checkingKey ? <Loader2 size={16} className="spin" /> : <KeyRound size={16} />} {checkingKey ? "Contacting Convex…" : "Check key"}
            </button>
            {keyCheck && (
              <div className={`callout ${keyCheck.ok ? "success" : "warning"}`}>
                {keyCheck.ok ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />} {keyCheck.message}
                {keyCheck.ok && keyCheck.key_kind && keyCheck.key_kind !== "prod" && (
                  <p style={{ margin: "0.4rem 0 0" }}>
                    Heads up: this is a <strong>{keyCheck.key_kind}</strong> key, not production.
                  </p>
                )}
              </div>
            )}
            <p className="subtle small-print">
              The check only lists table names — it never reads or changes your data. The key is stored encrypted on this server.
            </p>
            <label className="checkbox-row">
              <input type="checkbox" checked={includeFileStorage} onChange={(event) => setIncludeFileStorage(event.target.checked)} />
              <span>Include uploaded files (Convex file storage)</span>
            </label>
          </WizardStep>
        )}

        {step === "storage" && (
          <WizardStep title="Where should backups go?" detail="Keep at least one copy off this server so a dead disk doesn't take your backups with it.">
            {existingDestinations.length > 0 && (
              <div className="stack" style={{ gap: "0.5rem" }}>
                <strong className="group-label">Existing destinations</strong>
                {existingDestinations.map((destination) => (
                  <label key={destination.id} className="choice-card">
                    <input
                      type="checkbox"
                      checked={reuseDestinationIds.includes(destination.id)}
                      onChange={(event) =>
                        setReuseDestinationIds(
                          event.target.checked
                            ? [...reuseDestinationIds, destination.id]
                            : reuseDestinationIds.filter((id) => id !== destination.id)
                        )
                      }
                    />
                    {destination.kind.type === "local_filesystem" ? <HardDrive size={20} /> : <Cloud size={20} />}
                    <span>
                      <strong>{destination.name}</strong>
                      <small>{destinationTypeLabel(destination)} · {destinationLabel(destination)}</small>
                    </span>
                    {isEncrypted(destination) ? <span className="badge success"><Lock size={12} /> Encrypted</span> : <span className="badge warning">Not encrypted</span>}
                  </label>
                ))}
              </div>
            )}
            <strong className="group-label">{existingDestinations.length > 0 ? "Add new" : "Destinations"}</strong>
            <label className="choice-card">
              <input type="checkbox" checked={addLocal} onChange={(event) => setAddLocal(event.target.checked)} />
              <HardDrive size={20} />
              <span>
                <strong>Local folder on this server</strong>
                <small>Fast restores. Not safe on its own if the server is lost.</small>
              </span>
            </label>
            {addLocal && (
              <Field label="Folder">
                <input value={localRoot} onChange={(event) => setLocalRoot(event.target.value)} />
              </Field>
            )}
            {presets?.r2 ? (
              <label className="choice-card">
                <input type="checkbox" checked={addR2} onChange={(event) => setAddR2(event.target.checked)} />
                <Cloud size={20} />
                <span>
                  <strong>Cloudflare R2 (offsite) — ready to go</strong>
                  <small>Bucket <code>{presets.r2.bucket}</code> is already set up for this server. No keys to paste.</small>
                </span>
              </label>
            ) : (
              <p className="subtle small-print">
                To add an offsite bucket (Cloudflare R2, AWS S3, …), create it under Setup → Storage Vaults, then come back.
              </p>
            )}
            <Field label="Keep the newest N backups in each destination">
              <input type="number" min={1} value={keepLast} onChange={(event) => setKeepLast(Math.max(1, Number(event.target.value) || 1))} />
            </Field>
            {destinationCount > 0 && !hasOffsite && (
              <p className="callout warning"><AlertTriangle size={16} /> All copies would live on this server. Add an offsite destination if you can.</p>
            )}
          </WizardStep>
        )}

        {step === "encryption" && (
          <WizardStep
            title="Lock your backups with a passphrase"
            detail="Backups are encrypted before they're saved or uploaded. You set this once — every backup, verify and restore uses it automatically."
          >
            {!needsPassphrase ? (
              <p className="callout success"><ShieldCheck size={16} /> All selected destinations are already encrypted. Nothing to do here.</p>
            ) : (
              <>
                {unencryptedReused.length > 0 && (
                  <label className="checkbox-row">
                    <input type="checkbox" checked={encryptReused} onChange={(event) => setEncryptReused(event.target.checked)} />
                    <span>Also encrypt new backups in {unencryptedReused.map((d) => `“${d.name}”`).join(", ")}</span>
                  </label>
                )}
                {!skipEncryption && (
                  <>
                    <Field label={`Passphrase (at least ${minLen} characters)`}>
                      <input type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} />
                    </Field>
                    <PassphraseMeter passphrase={passphrase} minLen={minLen} />
                    <Field label="Type it again">
                      <input type="password" autoComplete="new-password" value={confirmPassphrase} onChange={(event) => setConfirmPassphrase(event.target.value)} />
                    </Field>
                    {confirmPassphrase && passphrase !== confirmPassphrase && <p className="danger-text small-print">Passphrases don't match.</p>}
                    <label className="checkbox-row">
                      <input type="checkbox" checked={savedPassphrase} onChange={(event) => setSavedPassphrase(event.target.checked)} />
                      <span>I saved this passphrase in my password manager. Without it, backups can't be recovered if this server is lost.</span>
                    </label>
                  </>
                )}
                <label className="checkbox-row subtle">
                  <input type="checkbox" checked={skipEncryption} onChange={(event) => setSkipEncryption(event.target.checked)} />
                  <span>Skip encryption (not recommended — anyone with bucket or disk access can read backups)</span>
                </label>
              </>
            )}
          </WizardStep>
        )}

        {step === "schedule" && (
          <WizardStep title="How often should it back up?" detail="Backups run automatically in the background. Missed runs (e.g. after a reboot) run once on startup.">
            <div className="segmented">
              <button type="button" className={scheduleMode === "interval" ? "active" : ""} onClick={() => setScheduleMode("interval")}>Every few hours</button>
              <button type="button" className={scheduleMode === "daily" ? "active" : ""} onClick={() => setScheduleMode("daily")}>Once a day</button>
            </div>
            {scheduleMode === "interval" ? (
              <Field label="Frequency">
                <select value={everyMinutes} onChange={(event) => setEveryMinutes(Number(event.target.value))}>
                  {INTERVAL_CHOICES.map(([minutes, label]) => (
                    <option key={minutes} value={minutes}>{label}</option>
                  ))}
                </select>
              </Field>
            ) : (
              <Field label={`Time (your local time — stored as ${localTimeToUtc(dailyTime)} UTC)`}>
                <input type="time" value={dailyTime} onChange={(event) => setDailyTime(event.target.value)} />
              </Field>
            )}
            <p className="subtle small-print">
              With “keep newest {keepLast}”, {scheduleMode === "interval" ? `that's about ${Math.round((keepLast * everyMinutes) / 60 / 24 * 10) / 10} days` : `that's ${keepLast} days`} of history.
            </p>
          </WizardStep>
        )}

        {step === "review" && (
          <WizardStep title="Review and create" detail="Nothing has been saved yet. This creates everything and tests each destination.">
            <dl className="review-list">
              <dt>Project</dt>
              <dd>{resolvedProjectName}</dd>
              <dt>Convex deployment</dt>
              <dd>{keyCheck?.deployment} {keyCheck?.key_kind && <span className="badge">{keyCheck.key_kind}</span>} · {includeFileStorage ? "with files" : "database only"}</dd>
              <dt>Saves to</dt>
              <dd>
                {addLocal && <span className="chip"><HardDrive size={12} /> Local · {localRoot}</span>}
                {addR2 && <span className="chip"><Cloud size={12} /> Cloudflare R2</span>}
                {selectedReused.map((d) => <span className="chip" key={d.id}>{d.name}</span>)}
              </dd>
              <dt>Encryption</dt>
              <dd>{effectivePassphrase ? <><Lock size={13} /> Passphrase (age)</> : needsPassphrase ? <span className="danger-text">Off</span> : "Already encrypted"}</dd>
              <dt>Schedule</dt>
              <dd>{scheduleMode === "interval" ? INTERVAL_CHOICES.find(([m]) => m === everyMinutes)?.[1] : `Daily at ${dailyTime}`} · keep newest {keepLast}</dd>
            </dl>
            {progress.length > 0 && (
              <ul className="progress-list">
                {progress.map((item) => (
                  <li key={item.key} className={item.state}>
                    {item.state === "done" && <CheckCircle2 size={16} />}
                    {item.state === "running" && <Loader2 size={16} className="spin" />}
                    {item.state === "failed" && <XCircle size={16} />}
                    {item.state === "pending" && <Circle size={16} />}
                    <span>
                      {item.label}
                      {item.detail && <small>{item.detail}</small>}
                    </span>
                  </li>
                ))}
              </ul>
            )}
            {!stepValid.review && (
              <button type="button" className="primary-button" disabled={creating} onClick={() => void createEverything()}>
                {creating ? <Loader2 size={16} className="spin" /> : <CheckCircle2 size={16} />}
                {creating ? "Creating…" : progress.some((item) => item.state === "failed") ? "Retry" : "Create everything"}
              </button>
            )}
          </WizardStep>
        )}

        {step === "backup" && (
          <WizardStep title="Run the first backup" detail="This exports the deployment now, saves it everywhere, and verifies it can be read back and decrypted.">
            {!runResult && !running && (
              <button type="button" className="primary-button" onClick={() => void runFirstBackup()}>
                <Play size={16} /> Run first backup
              </button>
            )}
            {running && <p className="callout info"><Loader2 size={16} className="spin" /> Backing up… large deployments can take a few minutes.</p>}
            {runError && (
              <div className="callout warning">
                <AlertTriangle size={16} /> {runError}
                <div style={{ marginTop: "0.5rem" }}>
                  <button type="button" className="secondary-button small" onClick={() => void runFirstBackup()}>Try again</button>
                </div>
              </div>
            )}
            {runResult && verifyResult?.ok && (
              <div className="wizard-success">
                <PartyPopper size={32} />
                <h3>{resolvedProjectName} is protected</h3>
                <p>
                  First backup {runResult.status === "partial" ? "finished with a warning (one copy failed — see Runs)" : "succeeded"} and verified
                  {verifyResult.encrypted ? " (decrypted with your passphrase)" : ""}. Future backups run automatically.
                </p>
              </div>
            )}
          </WizardStep>
        )}

        <footer className="wizard-footer">
          <button type="button" className="secondary-button" onClick={goBack} disabled={stepIndex === 0 || creating || running || (locked && step === "review")}>
            <ArrowLeft size={16} /> Back
          </button>
          {step === "backup" ? (
            <button type="button" className="primary-button" disabled={running} onClick={onFinished}>
              {verifyResult?.ok ? "Finish" : "Finish without testing"}
            </button>
          ) : (
            <button type="button" className="primary-button" disabled={!stepValid[step] || creating} onClick={goNext}>
              Continue <ArrowRight size={16} />
            </button>
          )}
        </footer>
      </section>
    </div>
  );
}

function WizardStep({ title, detail, children }: { title: string; detail: string; children: React.ReactNode }) {
  return (
    <div className="wizard-step-body">
      <header>
        <h2>{title}</h2>
        <p className="subtle">{detail}</p>
      </header>
      <div className="stack">{children}</div>
    </div>
  );
}

function PassphraseMeter({ passphrase, minLen }: { passphrase: string; minLen: number }) {
  const classes = [/[a-z]/, /[A-Z]/, /\d/, /[^a-zA-Z0-9]/].filter((pattern) => pattern.test(passphrase)).length;
  const score = passphrase.length === 0 ? 0 : passphrase.length < minLen ? 1 : passphrase.length >= 20 || classes >= 3 ? 3 : 2;
  const labels = ["", "Too short", "OK", "Strong"];
  return (
    <div className={`meter meter-${score}`} aria-live="polite">
      <span />
      <small>{labels[score]}</small>
    </div>
  );
}
