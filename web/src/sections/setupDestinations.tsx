import { useEffect, useState } from "react";
import { HardDrive, KeyRound } from "lucide-react";
import { ApiClient, destinationTypeLabel, isEncrypted, type ServiceState, type StorageDestination } from "../appState";
import { Field, ResourceForm, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

function defaultBackupRoot(state: ServiceState): string {
  const dbPath = state.health?.database_path ?? "";
  const dataDir = dbPath.includes("/") ? dbPath.slice(0, dbPath.lastIndexOf("/")) : "";
  return dataDir ? `${dataDir}/backups` : "/data/backups";
}

function PassphraseFields({
  enabled,
  setEnabled,
  passphrase,
  setPassphrase,
  confirmPassphrase,
  setConfirmPassphrase
}: {
  enabled: boolean;
  setEnabled: (value: boolean) => void;
  passphrase: string;
  setPassphrase: (value: string) => void;
  confirmPassphrase: string;
  setConfirmPassphrase: (value: string) => void;
}) {
  return (
    <div className="stack" style={{ gap: "0.5rem" }}>
      <label className="checkbox-row">
        <input type="checkbox" checked={enabled} onChange={(event) => setEnabled(event.target.checked)} />
        <span>Encrypt backups with a passphrase (recommended)</span>
      </label>
      {enabled && (
        <>
          <Field label={`Passphrase (min ${MIN_PASSPHRASE_LEN} characters)`}>
            <input type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} required />
          </Field>
          <Field label="Confirm passphrase">
            <input type="password" autoComplete="new-password" value={confirmPassphrase} onChange={(event) => setConfirmPassphrase(event.target.value)} required />
          </Field>
          <p className="subtle" style={{ fontSize: "0.8rem" }}>
            Save this passphrase in your password manager. Backups are standard <code>.age</code> files and can only be
            decrypted with it — if it is lost, the backups cannot be recovered.
          </p>
        </>
      )}
    </div>
  );
}

const MIN_PASSPHRASE_LEN = 12;

function checkedPassphrase(enabled: boolean, passphrase: string, confirmPassphrase: string): string | null {
  if (!enabled) return null;
  if (passphrase.length < MIN_PASSPHRASE_LEN) throw new Error(`Passphrase must be at least ${MIN_PASSPHRASE_LEN} characters.`);
  if (passphrase !== confirmPassphrase) throw new Error("Passphrases do not match.");
  return passphrase;
}

export function DestinationForm({ client, state, actionLoading, perform }: { client: ApiClient; state: ServiceState; actionLoading: string | null; perform: Perform }) {
  const [type, setType] = useState<"local" | "r2" | "s3">("local");
  const [name, setName] = useState("");
  const [root, setRoot] = useState("");
  const [accountId, setAccountId] = useState("");
  const [bucket, setBucket] = useState("");
  const [region, setRegion] = useState("us-east-1");
  const [endpoint, setEndpoint] = useState("");
  const [prefix, setPrefix] = useState("");
  const [accessKeyId, setAccessKeyId] = useState("");
  const [secretAccessKey, setSecretAccessKey] = useState("");
  const [secretId, setSecretId] = useState("");
  const [keepLast, setKeepLast] = useState("20");
  const [encrypt, setEncrypt] = useState(true);
  const [passphrase, setPassphrase] = useState("");
  const [confirmPassphrase, setConfirmPassphrase] = useState("");

  const s3Secrets = (state.secrets ?? []).filter((secret) => secret.kind === "s3_credentials");

  useEffect(() => {
    if (!root) setRoot(defaultBackupRoot(state));
  }, [root, state]);

  return (
    <ResourceForm
      title="Create Storage Destination"
      icon={<HardDrive size={18} />}
      loading={actionLoading === "destination"}
      submitLabel="Create destination"
      onSubmit={() =>
        perform("destination", async () => {
          const encryption_passphrase = checkedPassphrase(encrypt, passphrase, confirmPassphrase);
          const keep = Number.parseInt(keepLast, 10);
          const retention = { keep_last: Number.isFinite(keep) && keep > 0 ? keep : 20, keep_days: null, keep_weeklies: null, keep_monthlies: null };
          if (type === "local") {
            await client.request("/api/v1/destinations/local", {
              method: "POST",
              body: JSON.stringify({ name, root, retention, encryption_passphrase })
            });
          } else {
            const inlineKeys = accessKeyId.trim() && secretAccessKey.trim();
            if (!inlineKeys && !secretId) throw new Error("Enter an access key ID and secret, or pick saved credentials.");
            await client.request("/api/v1/destinations/s3", {
              method: "POST",
              body: JSON.stringify({
                name,
                bucket,
                region: type === "r2" ? "auto" : region,
                endpoint: type === "r2" ? `https://${accountId.trim()}.r2.cloudflarestorage.com` : endpoint.trim() || null,
                prefix: prefix.trim() || null,
                retention,
                credentials_secret_id: inlineKeys ? null : secretId,
                access_key_id: inlineKeys ? accessKeyId : null,
                secret_access_key: inlineKeys ? secretAccessKey : null,
                encryption_passphrase
              })
            });
          }
          setName("");
          setPassphrase("");
          setConfirmPassphrase("");
          setAccessKeyId("");
          setSecretAccessKey("");
          return "Storage destination created. Use “Test” on the card to confirm it works.";
        })
      }
    >
      <Field label="Type">
        <Select
          value={type}
          onChange={(val) => setType(val as "local" | "r2" | "s3")}
          items={[
            ["local", "Local folder"],
            ["r2", "Cloudflare R2"],
            ["s3", "Other S3-compatible storage"]
          ]}
          required
        />
      </Field>
      <Field label="Destination name">
        <input value={name} onChange={(event) => setName(event.target.value)} placeholder={type === "r2" ? "e.g. Cloudflare R2 offsite" : "e.g. Local Backup Folder"} required />
      </Field>
      {type === "local" ? (
        <Field label="Folder path">
          <input value={root} onChange={(event) => setRoot(event.target.value)} placeholder="/data/backups" required />
        </Field>
      ) : (
        <>
          {type === "r2" ? (
            <Field label="Cloudflare account ID">
              <input value={accountId} onChange={(event) => setAccountId(event.target.value)} placeholder="32-character account ID" required />
            </Field>
          ) : (
            <>
              <Field label="Endpoint URL (blank for AWS S3)">
                <input value={endpoint} onChange={(event) => setEndpoint(event.target.value)} placeholder="https://s3.example.com" />
              </Field>
              <Field label="Region">
                <input value={region} onChange={(event) => setRegion(event.target.value)} placeholder="us-east-1" required />
              </Field>
            </>
          )}
          <Field label="Bucket name">
            <input value={bucket} onChange={(event) => setBucket(event.target.value)} placeholder="convex-autobackup" required />
          </Field>
          <Field label="Folder prefix inside bucket (optional)">
            <input value={prefix} onChange={(event) => setPrefix(event.target.value)} placeholder="backups/" />
          </Field>
          <Field label="Access key ID">
            <input value={accessKeyId} onChange={(event) => setAccessKeyId(event.target.value)} autoComplete="off" />
          </Field>
          <Field label="Secret access key">
            <input type="password" value={secretAccessKey} onChange={(event) => setSecretAccessKey(event.target.value)} autoComplete="new-password" />
          </Field>
          {s3Secrets.length > 0 && (
            <Field label="…or use saved credentials">
              <Select value={secretId} onChange={setSecretId} items={s3Secrets.map((secret) => [secret.id, secret.label])} />
            </Field>
          )}
        </>
      )}
      <Field label="Keep the newest N backups">
        <input type="number" min={1} value={keepLast} onChange={(event) => setKeepLast(event.target.value)} required />
      </Field>
      <PassphraseFields
        enabled={encrypt}
        setEnabled={setEncrypt}
        passphrase={passphrase}
        setPassphrase={setPassphrase}
        confirmPassphrase={confirmPassphrase}
        setConfirmPassphrase={setConfirmPassphrase}
      />
    </ResourceForm>
  );
}

export function DestinationEncryptionControls({
  destination,
  client,
  actionLoading,
  perform
}: {
  destination: StorageDestination;
  client: ApiClient;
  actionLoading: string | null;
  perform: Perform;
}) {
  const [open, setOpen] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [confirmPassphrase, setConfirmPassphrase] = useState("");
  const encrypted = isEncrypted(destination);

  if (!open) {
    return (
      <>
        <button className="secondary-button small" type="button" onClick={() => setOpen(true)}>
          <KeyRound size={14} /> {encrypted ? "Change passphrase" : "Set passphrase"}
        </button>
        {encrypted && (
          <button
            className="secondary-button small"
            type="button"
            disabled={actionLoading === `encryption-${destination.id}`}
            onClick={() =>
              void perform(`encryption-${destination.id}`, async () => {
                if (!confirm(`Stop encrypting new backups in "${destination.name}"? Existing encrypted backups stay encrypted.`)) return null;
                await client.request(`/api/v1/destinations/${destination.id}/encryption`, {
                  method: "PUT",
                  body: JSON.stringify({ passphrase: null })
                });
                return `Encryption disabled for new backups in "${destination.name}".`;
              })
            }
          >
            Disable encryption
          </button>
        )}
      </>
    );
  }

  return (
    <div className="stack" style={{ width: "100%", gap: "0.5rem" }}>
      <Field label="New passphrase">
        <input type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} />
      </Field>
      <Field label="Confirm passphrase">
        <input type="password" autoComplete="new-password" value={confirmPassphrase} onChange={(event) => setConfirmPassphrase(event.target.value)} />
      </Field>
      {encrypted && (
        <p className="subtle" style={{ fontSize: "0.8rem" }}>
          New backups use the new passphrase. Older backups still need the passphrase they were made with — keep both.
        </p>
      )}
      <div className="button-row">
        <button className="secondary-button small" type="button" onClick={() => setOpen(false)}>Cancel</button>
        <button
          className="primary-button small"
          type="button"
          disabled={actionLoading === `encryption-${destination.id}`}
          onClick={() =>
            void perform(`encryption-${destination.id}`, async () => {
              const value = checkedPassphrase(true, passphrase, confirmPassphrase);
              await client.request(`/api/v1/destinations/${destination.id}/encryption`, {
                method: "PUT",
                body: JSON.stringify({ passphrase: value })
              });
              setOpen(false);
              setPassphrase("");
              setConfirmPassphrase("");
              return `Passphrase saved for "${destination.name}". New backups will be encrypted.`;
            })
          }
        >
          Save passphrase
        </button>
      </div>
    </div>
  );
}

export function DestinationChecklist({
  state,
  primaryId,
  selected,
  setSelected
}: {
  state: ServiceState;
  primaryId: string;
  selected: string[];
  setSelected: (ids: string[]) => void;
}) {
  const others = (state.destinations ?? []).filter((destination) => destination.id !== primaryId);
  if (others.length === 0) {
    return <p className="subtle" style={{ fontSize: "0.82rem" }}>Add another destination (e.g. Cloudflare R2) to keep an offsite copy of every backup.</p>;
  }
  return (
    <div className="stack" style={{ gap: "0.35rem" }}>
      {others.map((destination) => (
        <label key={destination.id} className="checkbox-row">
          <input
            type="checkbox"
            checked={selected.includes(destination.id)}
            onChange={(event) =>
              setSelected(event.target.checked ? [...selected, destination.id] : selected.filter((id) => id !== destination.id))
            }
          />
          <span>
            {destination.name} <span className="subtle">({destinationTypeLabel(destination)}{isEncrypted(destination) ? ", encrypted" : ""})</span>
          </span>
        </label>
      ))}
    </div>
  );
}

