import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "@/ipc/client";
import { isTauri } from "@/lib/env";
import { useSourcesStore } from "@/stores/sources";

/**
 * Add-source control. Folders use the native picker; repos and web pages prompt
 * for a URL. Ingestion runs in the background; progress flows into the sources
 * store via events.
 */
export function AddSourceButton({ compact = false }: { compact?: boolean }) {
  const add = useSourcesStore((s) => s.add);
  const [openMenu, setOpenMenu] = useState(false);

  async function pickFolder() {
    setOpenMenu(false);
    if (!isTauri()) return;
    const selected = await open({ directory: true, multiple: false, title: "Choose a folder to ingest" });
    if (typeof selected !== "string") return;
    const id = await api.addFolderSource(selected);
    add({ id, kind: "folder", uri: selected, status: "ingesting" });
  }

  async function addRepo() {
    setOpenMenu(false);
    if (!isTauri()) return;
    const url = window.prompt("Public Git URL to clone (https or git@):")?.trim();
    if (!url) return;
    const id = await api.addRepoSource(url);
    add({ id, kind: "repo", uri: url, status: "ingesting" });
  }

  async function addUrl() {
    setOpenMenu(false);
    if (!isTauri()) return;
    const url = window.prompt("Web page URL to ingest:")?.trim();
    if (!url) return;
    const id = await api.addUrlSource(url);
    add({ id, kind: "url", uri: url, status: "ingesting" });
  }

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpenMenu((o) => !o)}
        disabled={!isTauri()}
        aria-haspopup="menu"
        aria-expanded={openMenu}
        className="rounded-md border border-line-strong px-3 py-1.5 text-sm text-ink hover:bg-paper-sunken disabled:cursor-not-allowed disabled:opacity-60"
        title={isTauri() ? "Add a source" : "Available in the desktop app"}
      >
        {compact ? "Add" : "Add source"}
      </button>
      {openMenu ? (
        <div
          role="menu"
          className="absolute z-10 mt-1 w-44 rounded-md border border-line bg-paper-raised py-1 text-sm shadow-sm"
        >
          <button type="button" role="menuitem" onClick={pickFolder} className="block w-full px-3 py-1.5 text-left text-ink hover:bg-paper-sunken">
            📁 Folder…
          </button>
          <button type="button" role="menuitem" onClick={addRepo} className="block w-full px-3 py-1.5 text-left text-ink hover:bg-paper-sunken">
             Git repository…
          </button>
          <button type="button" role="menuitem" onClick={addUrl} className="block w-full px-3 py-1.5 text-left text-ink hover:bg-paper-sunken">
            🔗 Web page…
          </button>
        </div>
      ) : null}
    </div>
  );
}
