import { useEffect, useState } from "react";
import { KeyRound, Shield, ShieldCheck, Trash2, UserPlus, Users } from "lucide-react";
import {
  ApiClient,
  formatDateTime,
  relativeTime,
  sentenceCase,
  type ApiToken,
  type Role,
  type ServiceState
} from "../appState";
import { roles } from "../constants";
import { EmptyRow, Field, PanelHeader, ResourceForm, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

export function SecuritySection({
  client,
  state,
  actionLoading,
  perform,
  onTokenCreated
}: {
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
  onTokenCreated: (token: string | null) => void;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("operator");
  const [tokenUserId, setTokenUserId] = useState("");
  const [tokenName, setTokenName] = useState("agent-token");

  useEffect(() => {
    if (!tokenUserId && state.users[0]) setTokenUserId(state.users[0].id);
  }, [state.users, tokenUserId]);

  return (
    <div className="page-stack">
      {/* Role Hierarchy Matrix */}
      <section className="panel">
        <PanelHeader icon={<ShieldCheck size={18} />} title="Access Control & Role Hierarchy" detail="Role-based permissions enforcement across CLI, HTTP API, and Web Console" />
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "0.85rem", marginTop: "0.5rem" }}>
          <div style={{ background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: "8px", padding: "0.85rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.35rem" }}>
              <strong style={{ color: "#7f1d1d", fontSize: "0.95rem" }}>Owner</strong>
              <span className="badge danger">Full Control</span>
            </div>
            <p style={{ margin: 0, fontSize: "0.8rem", color: "#475569" }}>
              Full system control, database restores, secrets decryption, user management, and token creation.
            </p>
          </div>

          <div style={{ background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: "8px", padding: "0.85rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.35rem" }}>
              <strong style={{ color: "#0369a1", fontSize: "0.95rem" }}>Admin</strong>
              <span className="badge info">Manage</span>
            </div>
            <p style={{ margin: 0, fontSize: "0.8rem", color: "#475569" }}>
              Configure projects, targets, backup jobs, destinations, and schedules. Cannot trigger database restores.
            </p>
          </div>

          <div style={{ background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: "8px", padding: "0.85rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.35rem" }}>
              <strong style={{ color: "#15803d", fontSize: "0.95rem" }}>Operator</strong>
              <span className="badge success">Operate</span>
            </div>
            <p style={{ margin: 0, fontSize: "0.8rem", color: "#475569" }}>
              Trigger manual backups, run due schedules, verify checksums, and refresh disaster recovery reports.
            </p>
          </div>

          <div style={{ background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: "8px", padding: "0.85rem" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.35rem" }}>
              <strong style={{ color: "#475569", fontSize: "0.95rem" }}>Viewer</strong>
              <span className="badge">Read Only</span>
            </div>
            <p style={{ margin: 0, fontSize: "0.8rem", color: "#475569" }}>
              Read-only inspection of status metrics, backup runs, schedules, and audit events.
            </p>
          </div>
        </div>
      </section>

      {/* Forms Grid */}
      <section className="form-grid">
        <ResourceForm
          title="Create User Account"
          icon={<UserPlus size={18} />}
          loading={actionLoading === "user"}
          submitLabel="Create user"
          onSubmit={() =>
            perform("user", async () => {
              await client.request("/api/v1/users", {
                method: "POST",
                body: JSON.stringify({ email, password, role })
              });
              setEmail("");
              setPassword("");
              return `User "${email}" created with ${sentenceCase(role)} role.`;
            })
          }
        >
          <Field label="Email Address">
            <input value={email} onChange={(event) => setEmail(event.target.value)} type="email" placeholder="admin@mycompany.com" required />
          </Field>
          <Field label="Password (Min 12 characters)">
            <input value={password} onChange={(event) => setPassword(event.target.value)} minLength={12} type="password" required />
          </Field>
          <Field label="Assigned Role">
            <select value={role} onChange={(event) => setRole(event.target.value as Role)}>
              {roles.map((option) => (
                <option key={option} value={option}>
                  {sentenceCase(option)}
                </option>
              ))}
            </select>
          </Field>
        </ResourceForm>

        <ResourceForm
          title="Create API Token (Bearer Auth)"
          icon={<KeyRound size={18} />}
          loading={actionLoading === "token"}
          submitLabel="Generate token"
          onSubmit={() =>
            perform("token", async () => {
              const response = await client.request<{ api_token: ApiToken }>("/api/v1/tokens", {
                method: "POST",
                body: JSON.stringify({ user_id: tokenUserId, name: tokenName })
              });
              onTokenCreated(response.api_token.token ?? null);
              setTokenName("agent-token");
              return "API token generated! Copy the token from the top notification box; it will not be shown again.";
            })
          }
        >
          <Field label="Associated User Account">
            <Select value={tokenUserId} onChange={setTokenUserId} items={state.users.map((user) => [user.id, `${user.email} (${sentenceCase(user.role)})`])} required />
          </Field>
          <Field label="Token Identifier / Purpose">
            <input value={tokenName} onChange={(event) => setTokenName(event.target.value)} placeholder="e.g. CI-CD-Agent-Token" required />
          </Field>
        </ResourceForm>
      </section>

      {/* Users & Tokens Split */}
      <section className="split">
        <div className="panel">
          <PanelHeader icon={<Users size={18} />} title="User Accounts" detail={`${state.users.length} registered accounts`} />
          <div className="table">
            <div className="table-row table-head">
              <span>Email</span>
              <span>Role</span>
              <span>Created</span>
            </div>
            {state.users.map((user) => (
              <div className="table-row" key={user.id}>
                <span><strong>{user.email}</strong></span>
                <span>
                  <span className={`badge ${user.role === "owner" ? "danger" : user.role === "admin" ? "info" : "success"}`}>
                    {sentenceCase(user.role)}
                  </span>
                </span>
                <span title={formatDateTime(user.created_at)}>
                  {relativeTime(user.created_at)}
                </span>
              </div>
            ))}
          </div>
        </div>

        <div className="panel">
          <PanelHeader icon={<KeyRound size={18} />} title="Active API Tokens" detail={`${state.tokens.filter((token) => !token.revoked_at).length} active`} />
          <div className="table">
            <div className="table-row table-head">
              <span>Name</span>
              <span>User</span>
              <span>Status</span>
              <span>Action</span>
            </div>
            {state.tokens.map((apiToken) => (
              <div className="table-row" key={apiToken.id}>
                <span><strong>{apiToken.name}</strong></span>
                <span>{state.users.find((user) => user.id === apiToken.user_id)?.email ?? apiToken.user_id.slice(0, 8)}</span>
                <span>
                  {apiToken.revoked_at ? (
                    <span className="badge danger">Revoked</span>
                  ) : (
                    <span className="badge success">Active</span>
                  )}
                </span>
                <span>
                  <button
                    className="danger-button small"
                    type="button"
                    title="Revoke token"
                    disabled={Boolean(apiToken.revoked_at) || actionLoading === `revoke-${apiToken.id}`}
                    onClick={() =>
                      void perform(`revoke-${apiToken.id}`, async () => {
                        if (!confirm(`Revoke API token "${apiToken.name}"? Applications using this token will lose access.`)) return;
                        await client.request(`/api/v1/tokens/${apiToken.id}`, { method: "DELETE" });
                        return `API token "${apiToken.name}" revoked successfully.`;
                      })
                    }
                  >
                    <Trash2 size={14} /> Revoke
                  </button>
                </span>
              </div>
            ))}
            {state.tokens.length === 0 && <EmptyRow message="Create an API token for automated CLI or subagent access." />}
          </div>
        </div>
      </section>
    </div>
  );
}
