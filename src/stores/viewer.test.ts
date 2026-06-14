import { beforeEach, describe, expect, it } from "vitest";
import { useViewerStore } from "@/stores/viewer";
import type { Citation } from "@/ipc/client";

function citation(chunkId: string): Citation {
  return {
    chunkId,
    text: "some text",
    structuralPath: null,
    locator: { kind: "charspan", char_start: 0, char_end: 4 },
    pathOrUrl: `/docs/${chunkId}.md`,
    score: 1,
    usedInAnswer: true,
  };
}

describe("viewer store", () => {
  beforeEach(() => useViewerStore.setState({ tabs: [], activeChunkId: null, isOpen: false }));

  it("opens a citation as a focused tab", () => {
    useViewerStore.getState().open(citation("c1"));
    const s = useViewerStore.getState();
    expect(s.isOpen).toBe(true);
    expect(s.tabs).toHaveLength(1);
    expect(s.activeChunkId).toBe("c1");
  });

  it("dedups by chunkId but refocuses on reopen", () => {
    const s = useViewerStore.getState();
    s.open(citation("c1"));
    s.open(citation("c2"));
    s.open(citation("c1"));
    const after = useViewerStore.getState();
    expect(after.tabs).toHaveLength(2);
    expect(after.activeChunkId).toBe("c1");
  });

  it("closing the active tab focuses the previous one", () => {
    const s = useViewerStore.getState();
    s.open(citation("c1"));
    s.open(citation("c2"));
    s.close("c2");
    const after = useViewerStore.getState();
    expect(after.tabs.map((t) => t.chunkId)).toEqual(["c1"]);
    expect(after.activeChunkId).toBe("c1");
    expect(after.isOpen).toBe(true);
  });

  it("closing the last tab closes the drawer", () => {
    const s = useViewerStore.getState();
    s.open(citation("c1"));
    s.close("c1");
    const after = useViewerStore.getState();
    expect(after.tabs).toHaveLength(0);
    expect(after.isOpen).toBe(false);
    expect(after.activeChunkId).toBeNull();
  });

  it("closeAll resets the drawer", () => {
    const s = useViewerStore.getState();
    s.open(citation("c1"));
    s.open(citation("c2"));
    s.closeAll();
    expect(useViewerStore.getState().tabs).toHaveLength(0);
    expect(useViewerStore.getState().isOpen).toBe(false);
  });
});
