import { ListChecks } from "lucide-react";
import { formatDateTime, type AuditEvent } from "../appState";
import { PanelHeader, SimpleTable } from "../components/common";

export function AuditSection({ events }: { events: AuditEvent[] }) {
  return (
    <section className="panel">
      <PanelHeader icon={<ListChecks size={18} />} title="Audit events" detail={`${events.length} latest events`} />
      <SimpleTable
        headers={["Time", "Action", "Resource", "Message"]}
        rows={events.map((event) => [
          formatDateTime(event.created_at),
          event.action,
          event.resource_id ? `${event.resource_type}:${event.resource_id}` : event.resource_type,
          event.message
        ])}
        emptyMessage="No audit events recorded yet."
      />
    </section>
  );
}
