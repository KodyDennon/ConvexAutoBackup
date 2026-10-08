import { useState } from "react";
import { ApiClient, cleanDeploymentName, type ServiceState } from "../appState";
import { Field, Select } from "../components/common";
import { DestinationChecklist } from "./setupDestinations";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

export function EditTargetModal({
  target,
  state,
  client,
  perform,
  onClose
}: {
  target: any;
  state: ServiceState;
  client: ApiClient;
  perform: Perform;
  onClose: () => void;
}) {
  const [name, setName] = useState(target.name);
  const [deployment, setDeployment] = useState(target.deployment);
  const [url, setUrl] = useState(target.url ?? "");
  const [secretId, setSecretId] = useState(target.secret?.id ?? "");

  const deployKeySecrets = (state.secrets ?? []).filter((s) => s.kind === "convex_deploy_key");

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-box" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Edit Target Deployment — {target.name}</h3>
          <button type="button" className="close-btn" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body stack">
          <Field label="Target Label">
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </Field>
          <Field label="Convex Deployment Name">
            <input value={deployment} onChange={(e) => setDeployment(cleanDeploymentName(e.target.value))} required />
          </Field>
          <Field label="Convex Cloud / Data API URL (Optional)">
            <input value={url} onChange={(e) => setUrl(e.target.value)} placeholder={`https://${deployment}.convex.cloud`} />
          </Field>
          <Field label="Assigned Deploy Key Secret">
            <Select
              value={secretId}
              onChange={setSecretId}
              items={deployKeySecrets.map((s) => [s.id, `${s.label} (ID: ${s.id.slice(0, 8)})`])}
              required
            />
          </Field>
        </div>
        <div className="modal-footer button-row">
          <button type="button" className="secondary-button" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="primary-button"
            onClick={() =>
              void perform(`update-target-${target.id}`, async () => {
                await client.request(`/api/v1/targets/${target.id}`, {
                  method: "PUT",
                  body: JSON.stringify({
                    name,
                    deployment: cleanDeploymentName(deployment),
                    url: url.trim() || undefined,
                    secret_id: secretId || null
                  })
                });
                onClose();
                return `Target "${name}" updated.`;
              })
            }
          >
            Save Target Changes
          </button>
        </div>
      </div>
    </div>
  );
}

export function EditJobModal({
  job,
  state,
  client,
  perform,
  onClose
}: {
  job: any;
  state: ServiceState;
  client: ApiClient;
  perform: Perform;
  onClose: () => void;
}) {
  const [name, setName] = useState(job.name);
  const [projectId, setProjectId] = useState(job.project_id);
  const [targetId, setTargetId] = useState(job.target_id);
  const [destinationId, setDestinationId] = useState(job.destination_id);
  const [includeFileStorage, setIncludeFileStorage] = useState(job.include_file_storage);
  const [extraDestinations, setExtraDestinations] = useState<string[]>(job.additional_destination_ids ?? []);

  const availableTargets = (state.targets ?? []).filter((t) => t.project_id === projectId);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-box" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Edit Backup Job — {job.name}</h3>
          <button type="button" className="close-btn" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body stack">
          <Field label="Job Name">
            <input value={name} onChange={(e) => setName(e.target.value)} required />
          </Field>
          <Field label="Assigned Project">
            <Select
              value={projectId}
              onChange={(val) => {
                setProjectId(val);
                const first = (state.targets ?? []).find((t) => t.project_id === val);
                if (first) setTargetId(first.id);
              }}
              items={(state.projects ?? []).map((p) => [p.id, p.name])}
              required
            />
          </Field>
          <Field label="Convex Target Deployment (Filtered to Project)">
            {availableTargets.length === 0 ? (
              <div style={{ background: "#fef2f2", padding: "0.5rem", borderRadius: "6px", color: "#991b1b", fontSize: "0.82rem" }}>
                ⚠️ No target connected to this project yet. Please create a target for this project first.
              </div>
            ) : (
              <Select
                value={targetId}
                onChange={setTargetId}
                items={availableTargets.map((t) => [t.id, `${t.name} (${t.deployment})`])}
                required
              />
            )}
          </Field>
          <Field label="Primary destination">
            <Select
              value={destinationId}
              onChange={setDestinationId}
              items={(state.destinations ?? []).map((d) => [d.id, d.name])}
              required
            />
          </Field>
          <div className="field">
            <span>Also copy each backup to</span>
            <DestinationChecklist state={state} primaryId={destinationId} selected={extraDestinations} setSelected={setExtraDestinations} />
          </div>
          <label className="checkbox-row">
            <input type="checkbox" checked={includeFileStorage} onChange={(event) => setIncludeFileStorage(event.target.checked)} />
            <span>Include file storage</span>
          </label>
        </div>
        <div className="modal-footer button-row">
          <button type="button" className="secondary-button" onClick={onClose}>Cancel</button>
          <button
            type="button"
            className="primary-button"
            onClick={() =>
              void perform(`update-job-${job.id}`, async () => {
                await client.request(`/api/v1/jobs/${job.id}`, {
                  method: "PUT",
                  body: JSON.stringify({
                    name,
                    project_id: projectId,
                    target_id: targetId,
                    destination_id: destinationId,
                    additional_destination_ids: extraDestinations.filter((id) => id !== destinationId),
                    include_file_storage: includeFileStorage
                  })
                });
                onClose();
                return `Backup job "${name}" updated.`;
              })
            }
          >
            Save Job Changes
          </button>
        </div>
      </div>
    </div>
  );
}
