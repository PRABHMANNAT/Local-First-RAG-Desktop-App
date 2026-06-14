import { create } from "zustand";

export type SourceStatus = "ingesting" | "ready" | "error" | "syncing";

export interface SourceItem {
  id: string;
  kind: string;
  uri: string;
  status: SourceStatus;
  /** Live ingest progress while status is `ingesting`/`syncing`. */
  progress?: { index: number; total: number };
  documents?: number;
  chunks?: number;
  error?: string;
}

interface SourcesState {
  items: SourceItem[];
  add: (item: SourceItem) => void;
  setProgress: (id: string, index: number, total: number) => void;
  /** Mark a source as re-syncing (user-triggered fetch + re-ingest). */
  markSyncing: (id: string) => void;
  markDone: (
    id: string,
    result: { ok: boolean; documents: number; chunks: number; error?: string },
  ) => void;
}

/** Source tree state: items plus their live ingest status. */
export const useSourcesStore = create<SourcesState>((set) => ({
  items: [],
  add: (item) => set((s) => ({ items: [...s.items, item] })),
  setProgress: (id, index, total) =>
    set((s) => ({
      items: s.items.map((it) =>
        it.id === id
          ? {
              ...it,
              status: it.status === "syncing" ? "syncing" : "ingesting",
              progress: { index, total },
            }
          : it,
      ),
    })),
  markSyncing: (id) =>
    set((s) => ({
      items: s.items.map((it) =>
        it.id === id ? { ...it, status: "syncing", error: undefined } : it,
      ),
    })),
  markDone: (id, result) =>
    set((s) => ({
      items: s.items.map((it) =>
        it.id === id
          ? {
              ...it,
              status: result.ok ? "ready" : "error",
              progress: undefined,
              documents: result.documents,
              chunks: result.chunks,
              error: result.error,
            }
          : it,
      ),
    })),
}));
