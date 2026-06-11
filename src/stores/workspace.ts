import { create } from "zustand";

/** A workspace as listed in the app registry (mirrors the `workspace` table). */
export interface Workspace {
  id: string;
  name: string;
  icon: string | null;
}

interface WorkspaceState {
  workspaces: Workspace[];
  activeId: string | null;
  setWorkspaces: (workspaces: Workspace[]) => void;
  setActive: (id: string | null) => void;
}

/**
 * Workspace registry state. M0 starts empty (no workspaces yet); the create /
 * list commands that populate this land in M1.
 */
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspaces: [],
  activeId: null,
  setWorkspaces: (workspaces) => set({ workspaces }),
  setActive: (id) => set({ activeId: id }),
}));
