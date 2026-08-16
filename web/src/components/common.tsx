import React, { FormEvent, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  FileText,
  ListChecks,
  Plus,
  type LucideIcon
} from "lucide-react";
import {
  formatBytes,
  formatDateTime,
  formatDurationMs,
  relativeTime,
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
    <button className={`nav-button ${active ? "active" : ""}`} type="button" onClick={onClick}>
      {icon}
      <span>{children}</span>
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
    <div className="system-messages">
      {error && (
        <div className="alert danger">
          <AlertTriangle size={18} />
          <span>{error}</span>
        </div>
      )}
      {notice && (
        <div className="alert success">
          <CheckCircle2 size={18} />
          <span>{notice}</span>
        </div>
      )}
      {oneTimeToken && (
        <div className="alert info">
          <ListChecks size={18} />
          <div>
            <strong>Auth Token:</strong> <code>{oneTimeToken}</code>
          </div>
        </div>
      )}
    </div>
  );
}

export function KeyValueGrid({ items }: { items: Array<[string, string | number | boolean | null | undefined]> }) {
  return (
    <dl className="kv-grid">
      {items.map(([key, value]) => (
        <div key={key}>
          <dt>{key}</dt>
          <dd>{value === null || value === undefined ? "N/A" : String(value)}</dd>
        </div>
      ))}
    </dl>
  );
}

export function CardList({
  title,
  subtitle,
  items
}: {
  title: string;
  subtitle?: string;
  items: Array<{ id: string; title: string; subtitle?: string; value?: string; badge?: string }>;
}) {
  return (
    <div className="card-list">
      <div className="card-list-header">
        <h3>{title}</h3>
        {subtitle && <p>{subtitle}</p>}
      </div>
      <div className="card-list-items">
        {items.map((item) => (
          <div className="item-card" key={item.id}>
            <div>
              <strong>{item.title}</strong>
              {item.subtitle && <p>{item.subtitle}</p>}
            </div>
            <div>
              {item.value && <span>{item.value}</span>}
              {item.badge && <span className="badge">{item.badge}</span>}
            </div>
          </div>
        ))}
        {items.length === 0 && <p className="empty">None configured.</p>}
      </div>
    </div>
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
  const [activeModalRecord, setActiveModalRecord] = useState<RunRecord | null>(null);

  return (
    <>
      <div className="table">
        <div className="table-row table-head">
          <span>Status</span>
          <span>Job</span>
          <span>Started</span>
          {!compact && <span>Size & Details</span>}
        </div>
        {runs.map((record) => {
          let manifestObj: any = null;
          if (record.manifest_json) {
            try {
              manifestObj = JSON.parse(record.manifest_json);
            } catch {
              manifestObj = null;
            }
          }

          return (
            <div className="table-row" key={record.run.id}>
              <span>
                <span className={`status-pill ${record.run.status}`}>
                  {record.run.status === "succeeded" && "✓ "}
                  {record.run.status === "failed" && "✕ "}
                  {sentenceCase(record.run.status)}
                </span>
              </span>
              <span>
                <strong>{jobs.find((job) => job.id === record.run.job_id)?.name ?? record.run.job_id.slice(0, 8)}</strong>
              </span>
              <span title={formatDateTime(record.run.started_at)}>
                {relativeTime(record.run.started_at)}
              </span>
              {!compact && (
                <span>
                  {manifestObj ? (
                    <button
                      type="button"
                      className="secondary-button small"
                      onClick={() => setActiveModalRecord(record)}
                    >
                      <FileText size={14} /> {formatBytes(manifestObj.bytes_exported)} · Details
                    </button>
                  ) : record.run.error ? (
                    <button
                      type="button"
                      className="danger-button small"
                      onClick={() => setActiveModalRecord(record)}
                    >
                      <AlertTriangle size={14} /> View Error
                    </button>
                  ) : (
                    <span className="subtle">In Progress</span>
                  )}
                </span>
              )}
            </div>
          );
        })}
        {runs.length === 0 && <EmptyRow message="No backup runs have been recorded." />}
      </div>

      {activeModalRecord && (
        <div className="modal-backdrop" onClick={() => setActiveModalRecord(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>
                {activeModalRecord.run.status === "succeeded" ? "Backup Manifest Summary" : "Backup Error Report"} — Run #{activeModalRecord.run.id.slice(0, 8)}
              </h3>
              <button type="button" className="close-btn" onClick={() => setActiveModalRecord(null)}>✕</button>
            </div>
            <div className="modal-body stack">
              <p><strong>Status:</strong> <span className={`status-pill ${activeModalRecord.run.status}`}>{sentenceCase(activeModalRecord.run.status)}</span></p>
              <p><strong>Started:</strong> {formatDateTime(activeModalRecord.run.started_at)} ({relativeTime(activeModalRecord.run.started_at)})</p>

              {activeModalRecord.run.error && (
                <div style={{ background: "#fef2f2", border: "1px solid #fca5a5", padding: "1rem", borderRadius: "8px", color: "#991b1b" }}>
                  <strong style={{ display: "block", marginBottom: "0.25rem" }}>Error Description:</strong>
                  <code>{activeModalRecord.run.error}</code>
                </div>
              )}

              {activeModalRecord.manifest_json && (
                <>
                  <p><strong>Manifest Location:</strong> <code>{activeModalRecord.run.manifest_path ?? "Stored in DB"}</code></p>
                  <h4>Manifest Payload:</h4>
                  <pre className="json-pre">{activeModalRecord.manifest_json}</pre>
                </>
              )}
            </div>
            <div className="modal-footer">
              <button type="button" className="secondary-button" onClick={() => setActiveModalRecord(null)}>Close</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export type Icon = LucideIcon;
