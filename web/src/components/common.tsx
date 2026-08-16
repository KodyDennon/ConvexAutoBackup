import React, { FormEvent, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ListChecks,
  Plus,
  type LucideIcon
} from "lucide-react";
import {
  formatDateTime,
  sentenceCase,
  type BackupJob,
  type RunRecord
} from "../appState";

export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
    </label>
  );
}

export function Select({
  value,
  onChange,
  items,
  required
}: {
  value: string;
  onChange: (value: string) => void;
  items: Array<[string, string]>;
  required?: boolean;
}) {
  return (
    <select value={value} onChange={(event) => onChange(event.target.value)} required={required}>
      <option value="" disabled={required}>
        Select
      </option>
      {items.map(([id, label]) => (
        <option key={id} value={id}>
          {label}
        </option>
      ))}
    </select>
  );
}

export function ResourceForm({
  title,
  icon,
  loading,
  submitLabel,
  onSubmit,
  children
}: {
  title: string;
  icon: React.ReactNode;
  loading: boolean;
  submitLabel: string;
  onSubmit: () => void;
  children: React.ReactNode;
}) {
  return (
    <form
      className="panel resource-form"
      onSubmit={(event: FormEvent) => {
        event.preventDefault();
        onSubmit();
      }}
    >
      <PanelHeader icon={icon} title={title} />
      <div className="form-body">{children}</div>
      <button type="submit" disabled={loading}>
        <Plus size={16} /> {loading ? "Working" : submitLabel}
      </button>
    </form>
  );
}

export function PanelHeader({ icon, title, detail }: { icon: React.ReactNode; title: string; detail?: string }) {
  return (
    <div className="panel-header">
      <div>
        <span className="panel-icon">{icon}</span>
        <h2>{title}</h2>
      </div>
      {detail && <p>{detail}</p>}
    </div>
  );
}

export function NavButton({
  active,
  icon,
  onClick,
  children
}: {
  active: boolean;
  icon: React.ReactNode;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button className={active ? "nav-item active" : "nav-item"} type="button" onClick={onClick}>
      {icon}
      {children}
    </button>
  );
}

export function SystemMessages({
  error,
  notice,
  oneTimeToken
}: {
  error: string | null;
  notice: string | null;
  oneTimeToken: string | null;
}) {
  return (
    <>
      {error && (
        <div className="message error-message">
          <AlertTriangle size={17} /> {error}
        </div>
      )}
      {notice && (
        <div className="message notice-message">
          <CheckCircle2 size={17} /> {notice}
        </div>
      )}
      {oneTimeToken && (
        <div className="token-box">
          <span>One-time token</span>
          <code>{oneTimeToken}</code>
        </div>
      )}
    </>
  );
}

export function StatusLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="status-line">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function StatusTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="status-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function FindingList({ findings }: { findings: string[] }) {
  return (
    <ul className="finding-list">
      {findings.length > 0 ? findings.map((finding) => <li key={finding}>{finding}</li>) : <li>No findings.</li>}
    </ul>
  );
}

export function ResourceList({ title, items }: { title: string; items: Array<[string, string]> }) {
  return (
    <div className="panel">
      <PanelHeader icon={<ListChecks size={18} />} title={title} detail={`${items.length} configured`} />
      <div className="resource-list">
        {items.map(([name, detail]) => (
          <div key={`${name}-${detail}`} className="resource-row">
            <strong>{name}</strong>
            <span>{detail}</span>
          </div>
        ))}
        {items.length === 0 && <p className="empty">None configured.</p>}
      </div>
    </div>
  );
}

export function SimpleTable({
  headers,
  rows,
  emptyMessage = "No records."
}: {
  headers: string[];
  rows: string[][];
  emptyMessage?: string;
}) {
  return (
    <div className="table">
      <div className="table-row table-head" style={{ gridTemplateColumns: `repeat(${headers.length}, minmax(0, 1fr))` }}>
        {headers.map((header) => (
          <span key={header}>{header}</span>
        ))}
      </div>
      {rows.map((row) => (
        <div className="table-row" key={row.join("|")} style={{ gridTemplateColumns: `repeat(${headers.length}, minmax(0, 1fr))` }}>
          {row.map((cell, index) => (
            <span key={`${cell}-${index}`}>{cell}</span>
          ))}
        </div>
      ))}
      {rows.length === 0 && <EmptyRow message={emptyMessage} />}
    </div>
  );
}

export function EmptyRow({ message }: { message: string }) {
  return <p className="empty">{message}</p>;
}

export function RunList({ runs, jobs, compact = false }: { runs: RunRecord[]; jobs: BackupJob[]; compact?: boolean }) {
  const [activeManifestRecord, setActiveManifestRecord] = useState<RunRecord | null>(null);

  return (
    <>
      <div className="table">
        <div className="table-row table-head">
          <span>Status</span>
          <span>Job</span>
          <span>Started</span>
          {!compact && <span>Manifest Details</span>}
        </div>
        {runs.map((record) => (
          <div className="table-row" key={record.run.id}>
            <span className={`status-pill ${record.run.status}`}>{sentenceCase(record.run.status)}</span>
            <span>{jobs.find((job) => job.id === record.run.job_id)?.name ?? record.run.job_id.slice(0, 8)}</span>
            <span>{formatDateTime(record.run.started_at)}</span>
            {!compact && (
              <span>
                {record.manifest_json ? (
                  <button
                    type="button"
                    className="secondary-button small"
                    onClick={() => setActiveManifestRecord(record)}
                  >
                    View Manifest
                  </button>
                ) : (
                  <span className="subtle">{record.run.error ? `Error: ${record.run.error}` : record.run.manifest_path ?? "Pending"}</span>
                )}
              </span>
            )}
          </div>
        ))}
        {runs.length === 0 && <EmptyRow message="No backup runs have been recorded." />}
      </div>

      {activeManifestRecord && (
        <div className="modal-backdrop" onClick={() => setActiveManifestRecord(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>Backup Manifest — Run #{activeManifestRecord.run.id.slice(0, 8)}</h3>
              <button type="button" className="close-btn" onClick={() => setActiveManifestRecord(null)}>✕</button>
            </div>
            <div className="modal-body stack">
              <p><strong>Status:</strong> <span className={`status-pill ${activeManifestRecord.run.status}`}>{sentenceCase(activeManifestRecord.run.status)}</span></p>
              <p><strong>Manifest Path:</strong> <code>{activeManifestRecord.run.manifest_path ?? "N/A"}</code></p>
              <h4>Raw Manifest JSON:</h4>
              <pre className="json-pre">{activeManifestRecord.manifest_json}</pre>
            </div>
            <div className="modal-footer">
              <button type="button" className="secondary-button" onClick={() => setActiveManifestRecord(null)}>Close</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export type Icon = LucideIcon;
