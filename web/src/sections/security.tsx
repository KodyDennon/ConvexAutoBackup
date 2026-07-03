import { useEffect, useState } from "react";
import { KeyRound, Trash2, UserPlus } from "lucide-react";
import {
  ApiClient,
  formatDateTime,
  sentenceCase,
  type ApiToken,
  type Role,
  type ServiceState
} from "../appState";
import { roles } from "../constants";
import { EmptyRow, Field, PanelHeader, ResourceForm, Select, SimpleTable } from "../components/common";

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
      <section className="form-grid">
        <ResourceForm
          title="User"
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
              return "User created.";
            })
          }
        >
          <Field label="Email">
            <input value={email} onChange={(event) => setEmail(event.target.value)} type="email" required />
          </Field>
          <Field label="Password">
            <input value={password} onChange={(event) => setPassword(event.target.value)} minLength={12} type="password" required />
          </Field>
          <Field label="Role">
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
          title="API token"
          icon={<KeyRound size={18} />}
          loading={actionLoading === "token"}
          submitLabel="Create token"
          onSubmit={() =>
            perform("token", async () => {
              const response = await client.request<{ api_token: ApiToken }>("/api/v1/tokens", {
                method: "POST",
                body: JSON.stringify({ user_id: tokenUserId, name: tokenName })
              });
              onTokenCreated(response.api_token.token ?? null);
              setTokenName("agent-token");
              return "API token created. Store it now; it will not be shown again.";
            })
          }
        >
          <Field label="User">
            <Select value={tokenUserId} onChange={setTokenUserId} items={state.users.map((user) => [user.id, `${user.email} · ${user.role}`])} required />
          </Field>
          <Field label="Token name">
            <input value={tokenName} onChange={(event) => setTokenName(event.target.value)} required />
          </Field>
        </ResourceForm>
      </section>

      <section className="split">
        <div className="panel">
          <PanelHeader icon={<UserPlus size={18} />} title="Users" detail={`${state.users.length} accounts`} />
          <SimpleTable
            headers={["Email", "Role", "Created"]}
            rows={state.users.map((user) => [user.email, sentenceCase(user.role), formatDateTime(user.created_at)])}
          />
        </div>
        <div className="panel">
          <PanelHeader icon={<KeyRound size={18} />} title="API tokens" detail={`${state.tokens.filter((token) => !token.revoked_at).length} active`} />
          <div className="table">
            <div className="table-row table-head">
              <span>Name</span>
              <span>User</span>
              <span>Status</span>
              <span>Action</span>
            </div>
            {state.tokens.map((apiToken) => (
              <div className="table-row" key={apiToken.id}>
                <span>{apiToken.name}</span>
                <span>{state.users.find((user) => user.id === apiToken.user_id)?.email ?? apiToken.user_id}</span>
                <span>{apiToken.revoked_at ? `Revoked ${formatDateTime(apiToken.revoked_at)}` : "Active"}</span>
                <button
                  className="icon-button"
                  type="button"
                  title="Revoke token"
                  disabled={Boolean(apiToken.revoked_at) || actionLoading === `revoke-${apiToken.id}`}
                  onClick={() =>
                    void perform(`revoke-${apiToken.id}`, async () => {
                      await client.request(`/api/v1/tokens/${apiToken.id}`, { method: "DELETE" });
                      return "API token revoked.";
                    })
                  }
                >
                  <Trash2 size={15} />
                </button>
              </div>
            ))}
            {state.tokens.length === 0 && <EmptyRow message="Create an API token for agent or automation access." />}
          </div>
        </div>
      </section>
    </div>
  );
}
