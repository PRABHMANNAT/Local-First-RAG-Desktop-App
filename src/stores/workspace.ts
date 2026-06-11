import { create } from "zustand";
import type { WorkspaceInfo } from "@/ipc/client";

/** A workspace as listed in the app registry (mirrors the `workspace` table). */
export interface Workspace {
  id: string;
  name: string;
  icon: string | null;
}

interface WorkspaceState {
  workspaces: Workspace[];
  activeId: string | null;
  /** Backend info about the active workspace (embedder, ollama status). */
  info: WorkspaceInfo | null;
  setWorkspaces: (workspaces: Workspace[]) => void;
  setActive: (id: string | null) => void;
  setInfo: (info: WorkspaceInfo | null) => void;
}

/**
 * Workspace registry state. M0 starts empty (no workspaces yet); the create /
 * list commands that populate this land in M1.
 */
export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  workspaces: [],
  activeId: null,
  info: null,
  setWorkspaces: (workspaces) => set({ workspaces }),
  setActive: (id) => set({ activeId: id }),
  setInfo: (info) => set({ info }),
}));
