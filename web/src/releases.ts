export interface GitHubRelease {
  tag_name?: string;
  html_url?: string;
  draft?: boolean;
}

export function selectLatestInstallableRelease(releases: GitHubRelease[]): GitHubRelease | null {
  return releases.find((release) => Boolean(release.tag_name) && !release.draft) ?? null;
}

export function buildUpdateNotice(currentVersion: string, release: GitHubRelease | null): string | null {
  if (!release?.tag_name) {
    return null;
  }
  const current = currentVersion.startsWith("v") ? currentVersion : `v${currentVersion}`;
  if (release.tag_name === current) {
    return null;
  }
  return `Update available: ${release.tag_name}. Download it from ${release.html_url ?? "GitHub Releases"}.`;
}

export async function fetchLatestInstallableRelease(signal: AbortSignal): Promise<GitHubRelease | null> {
  const response = await fetch("https://api.github.com/repos/KodyDennon/ConvexAutoBackup/releases?per_page=20", {
    signal,
    headers: { Accept: "application/vnd.github+json" }
  });
  if (!response.ok) {
    return null;
  }
  const releases: unknown = await response.json();
  if (!Array.isArray(releases)) {
    return null;
  }
  return selectLatestInstallableRelease(releases as GitHubRelease[]);
}
