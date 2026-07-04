import { describe, expect, it } from "vitest";
import { buildUpdateNotice, selectLatestInstallableRelease } from "./releases";

describe("release helpers", () => {
  it("selects prerelease versions from the releases list", () => {
    const release = selectLatestInstallableRelease([
      { tag_name: "v0.1.0-beta.5", html_url: "https://example.test/beta5" },
      { tag_name: "v0.1.0-beta.4", html_url: "https://example.test/beta4" }
    ]);

    expect(release?.tag_name).toBe("v0.1.0-beta.5");
  });

  it("ignores drafts and suppresses notices for the current version", () => {
    const release = selectLatestInstallableRelease([
      { tag_name: "v0.1.0-beta.6", draft: true },
      { tag_name: "v0.1.0-beta.5", html_url: "https://example.test/beta5" }
    ]);

    expect(release?.tag_name).toBe("v0.1.0-beta.5");
    expect(buildUpdateNotice("0.1.0-beta.5", release)).toBeNull();
    expect(buildUpdateNotice("0.1.0-beta.4", release)).toContain("v0.1.0-beta.5");
  });
});
