import { useState } from "react";
import { DatabaseBackup, KeyRound, Lock } from "lucide-react";
import { ApiClient, type ApiToken, type HealthResponse } from "../appState";
import { Field, StatusLine, SystemMessages } from "./common";

export function AuthShell({ health, children }: { health: HealthResponse | null; children: React.ReactNode }) {
  return (
    <main className="auth-layout">
      <section className="auth-panel">
        <div className="brand auth-brand">
          <DatabaseBackup size={30} />
          <div>
            <strong>ConvexAutoBackup</strong>
            <span>Local database backup control plane</span>
          </div>
        </div>
        {children}
      </section>
      <section className="auth-status">
        <StatusLine label="Service" value={health?.status ?? "checking"} />
        <StatusLine label="Version" value={health?.version ?? "unknown"} />
        <StatusLine label="Database" value={health?.database_path ?? "not connected"} />
      </section>
    </main>
  );
}

export function BootstrapForm({
  onAuthenticated,
  onError,
  error,
  notice,
  oneTimeToken
}: {
  onAuthenticated: (token: string, message: string) => void;
  onError: (message: string) => void;
  error: string | null;
  notice: string | null;
  oneTimeToken: string | null;
}) {
  const [email, setEmail] = useState("owner@example.com");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);

  return (
    <form
      className="stack"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmitting(true);
        void new ApiClient(null)
          .request<{ api_token: ApiToken }>("/api/v1/bootstrap", {
            method: "POST",
            body: JSON.stringify({ email, password, role: "owner" })
          })
          .then((response) => {
            if (!response.api_token.token) {
              throw new Error("Bootstrap did not return a token");
            }
            onAuthenticated(response.api_token.token, "Owner created. Store the bootstrap token before leaving this screen.");
          })
          .catch((caught) => onError(caught instanceof Error ? caught.message : "Bootstrap failed"))
          .finally(() => setSubmitting(false));
      }}
    >
      <h1>Create the first owner</h1>
      <p className="subtle">The first account receives owner access and an API token for the CLI, HTTP API, and local web console.</p>
      <SystemMessages error={error} notice={notice} oneTimeToken={oneTimeToken} />
      <Field label="Owner email">
        <input value={email} onChange={(event) => setEmail(event.target.value)} type="email" required />
      </Field>
      <Field label="Owner password">
        <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" minLength={12} required />
      </Field>
      <button type="submit" disabled={submitting}>
        <Lock size={16} /> Create owner
      </button>
    </form>
  );
}

export function LoginForm({
  onAuthenticated,
  onError,
  error,
  notice,
  oneTimeToken
}: {
  onAuthenticated: (token: string, message: string) => void;
  onError: (message: string) => void;
  error: string | null;
  notice: string | null;
  oneTimeToken: string | null;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);

  return (
    <form
      className="stack"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmitting(true);
        void new ApiClient(null)
          .request<{ api_token: ApiToken }>("/api/v1/login", {
            method: "POST",
            body: JSON.stringify({ email, password })
          })
          .then((response) => {
            if (!response.api_token.token) {
              throw new Error("Login did not return a token");
            }
            onAuthenticated(response.api_token.token, "Signed in. A new API token was created for this browser.");
          })
          .catch((caught) => onError(caught instanceof Error ? caught.message : "Login failed"))
          .finally(() => setSubmitting(false));
      }}
    >
      <h1>Sign in</h1>
      <p className="subtle">Email/password login creates a revocable API token for this browser session.</p>
      <SystemMessages error={error} notice={notice} oneTimeToken={oneTimeToken} />
      <Field label="Email">
        <input value={email} onChange={(event) => setEmail(event.target.value)} type="email" required />
      </Field>
      <Field label="Password">
        <input value={password} onChange={(event) => setPassword(event.target.value)} type="password" required />
      </Field>
      <button type="submit" disabled={submitting}>
        <KeyRound size={16} /> Sign in
      </button>
    </form>
  );
}
