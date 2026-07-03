export type BackupReadiness = "needs_setup" | "ready" | "at_risk";

export interface DashboardStat {
  label: string;
  value: string;
  detail: string;
  readiness: BackupReadiness;
}

export function countReadyStats(stats: DashboardStat[]): number {
  return stats.filter((stat) => stat.readiness === "ready").length;
}

export function formatProtectedDeploymentCount(count: number): string {
  if (count === 0) {
    return "0";
  }
  return count.toLocaleString("en-US");
}

