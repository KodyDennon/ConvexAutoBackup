import { useState } from "react";
import { FileText, ListChecks, Search, User } from "lucide-react";
import { formatDateTime, relativeTime, sentenceCase, type AuditEvent } from "../appState";
import { PanelHeader } from "../components/common";

export function AuditSection({ events }: { events: AuditEvent[] }) {
  const [filterQuery, setFilterQuery] = useState("");
  const [selectedEvent, setSelectedEvent] = useState<AuditEvent | null>(null);

  const filteredEvents = events.filter((e) => {
    if (!filterQuery) return true;
    const q = filterQuery.toLowerCase();
    return (
      e.action.toLowerCase().includes(q) ||
      e.message.toLowerCase().includes(q) ||
      (e.resource_type && e.resource_type.toLowerCase().includes(q)) ||
      (e.resource_id && e.resource_id.toLowerCase().includes(q))
    );
  });

  return (
    <div className="page-stack">
      <section className="panel">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem", flexWrap: "wrap", gap: "1rem" }}>
          <PanelHeader icon={<ListChecks size={18} />} title="System Audit Trail & Log" detail={`${events.length} total events recorded`} />
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", background: "#f8fafc", border: "1px solid #cbd5e1", padding: "0.4rem 0.75rem", borderRadius: "8px", width: "100%", maxWidth: "320px" }}>
            <Search size={16} style={{ color: "#64748b" }} />
            <input
              type="text"
              value={filterQuery}
              onChange={(e) => setFilterQuery(e.target.value)}
              placeholder="Search by action, resource, message..."
              style={{ border: "none", background: "transparent", width: "100%", fontSize: "0.85rem", outline: "none" }}
            />
            {filterQuery && (
              <button type="button" onClick={() => setFilterQuery("")} style={{ border: "none", background: "transparent", cursor: "pointer", fontSize: "0.8rem", color: "#64748b" }}>✕</button>
            )}
          </div>
        </div>

        <div className="table">
          <div className="table-row table-head">
            <span>Timestamp</span>
            <span>Action</span>
            <span>Target Resource</span>
            <span>Message</span>
            <span>Details</span>
          </div>
          {filteredEvents.map((event) => {
            let actionClass = "badge info";
            if (event.action.includes("create") || event.action.includes("run") || event.action.includes("login")) actionClass = "badge success";
            if (event.action.includes("delete") || event.action.includes("revoke") || event.action.includes("failed")) actionClass = "badge danger";

            return (
              <div className="table-row" key={event.id}>
                <span title={formatDateTime(event.created_at)}>
                  {relativeTime(event.created_at)}
                </span>
                <span>
                  <span className={actionClass} style={{ fontFamily: "monospace", fontSize: "0.78rem" }}>
                    {event.action}
                  </span>
                </span>
                <span>
                  <code style={{ fontSize: "0.8rem", color: "#0369a1" }}>
                    {event.resource_id ? `${event.resource_type}:${event.resource_id.slice(0, 8)}` : event.resource_type}
                  </code>
                </span>
                <span style={{ fontSize: "0.85rem", color: "#334155" }}>{event.message}</span>
                <span>
                  <button
                    type="button"
                    className="secondary-button small"
                    onClick={() => setSelectedEvent(event)}
                  >
                    <FileText size={14} /> View
                  </button>
                </span>
              </div>
            );
          })}
          {filteredEvents.length === 0 && (
            <p className="empty">
              {filterQuery ? `No audit events matching "${filterQuery}".` : "No audit events recorded yet."}
            </p>
          )}
        </div>
      </section>

      {/* Audit Event Detail Modal */}
      {selectedEvent && (
        <div className="modal-backdrop" onClick={() => setSelectedEvent(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3>Audit Event Details — {selectedEvent.action}</h3>
              <button type="button" className="close-btn" onClick={() => setSelectedEvent(null)}>✕</button>
            </div>
            <div className="modal-body stack">
              <p><strong>Timestamp:</strong> {formatDateTime(selectedEvent.created_at)} ({relativeTime(selectedEvent.created_at)})</p>
              <p><strong>Action Type:</strong> <code>{selectedEvent.action}</code></p>
              <p><strong>Resource:</strong> <code>{selectedEvent.resource_type}{selectedEvent.resource_id ? `:${selectedEvent.resource_id}` : ""}</code></p>
              <p><strong>Event ID:</strong> <code>{selectedEvent.id}</code></p>
              
              <p><strong>Actor:</strong> <code>{selectedEvent.actor}</code></p>
              
              <div style={{ background: "#f8fafc", border: "1px solid #e2e8f0", padding: "0.85rem", borderRadius: "8px", marginTop: "0.5rem" }}>
                <strong style={{ display: "block", marginBottom: "0.35rem", fontSize: "0.85rem" }}>Log Description:</strong>
                <p style={{ margin: 0, fontSize: "0.88rem", color: "#1e293b" }}>{selectedEvent.message}</p>
              </div>
            </div>
            <div className="modal-footer">
              <button type="button" className="secondary-button" onClick={() => setSelectedEvent(null)}>Close</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
