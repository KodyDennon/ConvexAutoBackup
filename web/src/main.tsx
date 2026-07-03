import React from "react";
import { createRoot } from "react-dom/client";
import { Activity, DatabaseBackup, KeyRound, RotateCcw, ShieldCheck } from "lucide-react";
import "./styles.css";

const stats = [
  { label: "Protected deployments", value: "0", detail: "Add Convex Cloud or self-hosted targets" },
  { label: "Next scheduled run", value: "Not scheduled", detail: "Intervals, calendar times, and guided cron" },
  { label: "Latest backup", value: "No runs yet", detail: "Full exports include file storage by default" },
  { label: "DR readiness", value: "Needs setup", detail: "Restore drills and evidence reports will appear here" }
];

function App() {
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
        <nav>
          <a className="active" href="#dashboard"><Activity size={18} />Dashboard</a>
          <a href="#security"><ShieldCheck size={18} />Security</a>
          <a href="#secrets"><KeyRound size={18} />Secrets</a>
          <a href="#dr"><RotateCcw size={18} />DR Center</a>
        </nav>
      </aside>
      <section className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">Local/LAN control plane</p>
            <h1>Convex backup operations</h1>
          </div>
          <button type="button">Create first target</button>
        </header>

        <section className="grid">
          {stats.map((stat) => (
            <article className="metric" key={stat.label}>
              <span>{stat.label}</span>
              <strong>{stat.value}</strong>
              <p>{stat.detail}</p>
            </article>
          ))}
        </section>

        <section className="panel">
          <div>
            <h2>First-run checklist</h2>
            <p>These are the foundation workflows the implementation will fill in as milestones land.</p>
          </div>
          <ol>
            <li>Create the first admin password before normal LAN access.</li>
            <li>Add Convex deploy keys or self-hosted credentials through the secrets vault.</li>
            <li>Choose local filesystem and/or S3-compatible backup destinations.</li>
            <li>Schedule full backups with file storage included by default.</li>
            <li>Run a restore drill and export a DR evidence report.</li>
          </ol>
        </section>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<App />);

