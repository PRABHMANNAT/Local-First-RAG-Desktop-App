import { create } from "zustand";

interface UiState {
  /** Whether the collapsible sources panel is visible. */
  sourcesPanelOpen: boolean;
  toggleSourcesPanel: () => void;
  setSourcesPanelOpen: (open: boolean) => void;
}

/** Transient view state (panel toggles, focus). Not persisted in M0. */
export const useUiStore = create<UiState>((set) => ({
  sourcesPanelOpen: true,
  toggleSourcesPanel: () =>
    set((s) => ({ sourcesPanelOpen: !s.sourcesPanelOpen })),
  setSourcesPanelOpen: (open) => set({ sourcesPanelOpen: open }),
}));
