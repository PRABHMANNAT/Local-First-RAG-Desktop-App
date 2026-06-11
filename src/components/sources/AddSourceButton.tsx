import { open } from "@tauri-apps/plugin-dialog";
import { api } from "@/ipc/client";
import { isTauri } from "@/lib/env";
import { useSourcesStore } from "@/stores/sources";

/**
 * Opens a folder picker and registers the chosen folder as a source. Ingestion
 * runs in the background; progress flows into the sources store via events.
 */
export function AddSourceButton({ compact = false }: { compact?: boolean }) {
  const add = useSourcesStore((s) => s.add);

  async function pick() {
    if (!isTauri()) return;
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder to ingest",
    });
    if (typeof selected !== "string") return;
    const id = await api.addFolderSource(selected);
    add({ id, kind: "folder", uri: selected, status: "ingesting" });
  }

  return (
    <button
      type="button"
      onClick={pick}
      disabled={!isTauri()}
      className="rounded-md border border-line-strong px-3 py-1.5 text-sm text-ink hover:bg-paper-sunken disabled:cursor-not-allowed disabled:opacity-60"
      title={isTauri() ? "Add a folder source" : "Available in the desktop app"}
    >
      {compact ? "Add folder" : "Add folder source"}
    </button>
  );
}
