import { useEffect, useState } from "react";
import { Clock3 } from "lucide-react";
import { ApiClient, type ServiceState } from "../appState";
import { weekdays } from "../constants";
import { Field, ResourceForm, Select } from "../components/common";

type Perform = (key: string, action: () => Promise<string | null | undefined>) => Promise<void>;

export function ScheduleForm({
  client,
  state,
  actionLoading,
  perform
}: {
  client: ApiClient;
  state: ServiceState;
  actionLoading: string | null;
  perform: Perform;
}) {
  const [jobId, setJobId] = useState("");
  const [mode, setMode] = useState<"interval_minutes" | "daily" | "weekly" | "cron">("interval_minutes");
  const [every, setEvery] = useState(60);
  const [time, setTime] = useState("02:00:00");
  const [weekday, setWeekday] = useState("Mon");
  const [expression, setExpression] = useState("0 0 2 * * *");
  const [missedRunPolicy, setMissedRunPolicy] = useState<"run_once_on_resume" | "skip">("run_once_on_resume");

  useEffect(() => {
    if (!jobId && state.jobs[0]) setJobId(state.jobs[0].id);
  }, [jobId, state.jobs]);

  return (
    <ResourceForm
      title="Schedule"
      icon={<Clock3 size={18} />}
      loading={actionLoading === "schedule"}
      submitLabel="Create schedule"
      onSubmit={() =>
        perform("schedule", async () => {
          const schedule =
            mode === "interval_minutes"
              ? { type: mode, every }
              : mode === "daily"
                ? { type: mode, time }
                : mode === "weekly"
                  ? { type: mode, weekday, time }
                  : { type: mode, expression };
          await client.request("/api/v1/schedules", {
            method: "POST",
            body: JSON.stringify({ job_id: jobId, schedule, missed_run_policy: missedRunPolicy, enabled: true })
          });
          return "Schedule created.";
        })
      }
    >
      <Field label="Job">
        <Select value={jobId} onChange={setJobId} items={state.jobs.map((job) => [job.id, job.name])} required />
      </Field>
      <Field label="Mode">
        <select value={mode} onChange={(event) => setMode(event.target.value as "interval_minutes" | "daily" | "weekly" | "cron")}>
          <option value="interval_minutes">Interval</option>
          <option value="daily">Daily</option>
          <option value="weekly">Weekly</option>
          <option value="cron">Cron</option>
        </select>
      </Field>
      {mode === "interval_minutes" && (
        <Field label="Every minutes">
          <input value={every} onChange={(event) => setEvery(Number(event.target.value))} min={1} type="number" required />
        </Field>
      )}
      {(mode === "daily" || mode === "weekly") && (
        <Field label="UTC time">
          <input value={time} onChange={(event) => setTime(event.target.value)} pattern="^[0-2][0-9]:[0-5][0-9]:[0-5][0-9]$" required />
        </Field>
      )}
      {mode === "weekly" && (
        <Field label="Weekday">
          <select value={weekday} onChange={(event) => setWeekday(event.target.value)}>
            {weekdays.map((day) => (
              <option key={day} value={day}>
                {day}
              </option>
            ))}
          </select>
        </Field>
      )}
      {mode === "cron" && (
        <Field label="Cron expression">
          <input value={expression} onChange={(event) => setExpression(event.target.value)} required />
        </Field>
      )}
      <Field label="Missed runs">
        <select value={missedRunPolicy} onChange={(event) => setMissedRunPolicy(event.target.value as "run_once_on_resume" | "skip")}>
          <option value="run_once_on_resume">Run once on resume</option>
          <option value="skip">Skip missed runs</option>
        </select>
      </Field>
    </ResourceForm>
  );
}
