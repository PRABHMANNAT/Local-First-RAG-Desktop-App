import { beforeEach, describe, expect, it } from "vitest";
import { useSourcesStore } from "@/stores/sources";

describe("sources store", () => {
  beforeEach(() => useSourcesStore.setState({ items: [] }));

  it("adds a source, tracks progress, and marks done", () => {
    const s = useSourcesStore.getState();
    s.add({ id: "s1", kind: "folder", uri: "/docs", status: "ingesting" });
    s.setProgress("s1", 3, 10);

    let item = useSourcesStore.getState().items[0]!;
    expect(item.status).toBe("ingesting");
    expect(item.progress).toEqual({ index: 3, total: 10 });

    s.markDone("s1", { ok: true, documents: 10, chunks: 42 });
    item = useSourcesStore.getState().items[0]!;
    expect(item.status).toBe("ready");
    expect(item.chunks).toBe(42);
    expect(item.progress).toBeUndefined();
  });

  it("marks a failed ingest as error", () => {
    const s = useSourcesStore.getState();
    s.add({ id: "s2", kind: "folder", uri: "/x", status: "ingesting" });
    s.markDone("s2", { ok: false, documents: 0, chunks: 0, error: "boom" });
    const item = useSourcesStore.getState().items[0]!;
    expect(item.status).toBe("error");
    expect(item.error).toBe("boom");
  });
});
