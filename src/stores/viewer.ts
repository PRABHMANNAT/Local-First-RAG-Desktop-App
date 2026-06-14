import { create } from "zustand";
import type { Citation } from "@/ipc/client";

/**
 * An open source-viewer tab. Keyed by `chunkId`; opening the same citation
 * twice focuses the existing tab rather than duplicating it.
 */
export type ViewerTab = Citation;

interface ViewerState {
  /** Open tabs, in the order they were opened. */
  tabs: ViewerTab[];
  /** `chunkId` of the focused tab, or null when the drawer is closed. */
  activeChunkId: string | null;
  /** Whether the drawer is visible. */
  isOpen: boolean;
  /** Open a citation in the drawer, focusing it (dedup by chunkId). */
  open: (citation: Citation) => void;
  /** Focus an already-open tab. */
  focus: (chunkId: string) => void;
  /** Close one tab; closes the drawer when the last tab goes. */
  close: (chunkId: string) => void;
  /** Close the drawer and drop all tabs. */
  closeAll: () => void;
}

/** State for the citation source-viewer drawer (M2). */
export const useViewerStore = create<ViewerState>((set) => ({
  tabs: [],
  activeChunkId: null,
  isOpen: false,
  open: (citation) =>
    set((s) => {
      const exists = s.tabs.some((t) => t.chunkId === citation.chunkId);
      return {
        tabs: exists ? s.tabs : [...s.tabs, citation],
        activeChunkId: citation.chunkId,
        isOpen: true,
      };
    }),
  focus: (chunkId) => set({ activeChunkId: chunkId, isOpen: true }),
  close: (chunkId) =>
    set((s) => {
      const tabs = s.tabs.filter((t) => t.chunkId !== chunkId);
      const wasActive = s.activeChunkId === chunkId;
      const nextActive = wasActive ? (tabs[tabs.length - 1]?.chunkId ?? null) : s.activeChunkId;
      return { tabs, activeChunkId: nextActive, isOpen: tabs.length > 0 };
    }),
  closeAll: () => set({ tabs: [], activeChunkId: null, isOpen: false }),
}));
