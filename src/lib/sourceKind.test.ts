import { describe, expect, it } from "vitest";
import { iconFor, isSyncable, labelFor } from "@/lib/sourceKind";

describe("sourceKind helpers", () => {
  it("maps known kinds to icons and labels", () => {
    expect(labelFor("repo")).toBe("Repository");
    expect(labelFor("url")).toBe("Web page");
    expect(iconFor("folder")).toBe("📁");
  });

  it("falls back for unknown kinds", () => {
    expect(labelFor("mystery")).toBe("mystery");
    expect(iconFor("mystery")).toBe("📄");
  });

  it("knows which kinds are syncable", () => {
    expect(isSyncable("repo")).toBe(true);
    expect(isSyncable("url")).toBe(true);
    expect(isSyncable("youtube")).toBe(true);
    expect(isSyncable("folder")).toBe(false);
    expect(isSyncable("pdf")).toBe(false);
  });
});
