import { describe, expect, it } from "vitest";
import { countReadyStats, formatProtectedDeploymentCount, type DashboardStat } from "./appState";

describe("dashboard state helpers", () => {
  it("counts ready stats without treating setup or risk states as ready", () => {
    const stats: DashboardStat[] = [
      { label: "A", value: "1", detail: "ready", readiness: "ready" },
      { label: "B", value: "0", detail: "setup", readiness: "needs_setup" },
      { label: "C", value: "late", detail: "risk", readiness: "at_risk" }
    ];

    expect(countReadyStats(stats)).toBe(1);
  });

  it("formats deployment counts predictably for agent-visible UI text", () => {
    expect(formatProtectedDeploymentCount(0)).toBe("0");
    expect(formatProtectedDeploymentCount(12345)).toBe("12,345");
  });
});

