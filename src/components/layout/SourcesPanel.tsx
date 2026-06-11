import { EmptyState } from "@/components/common/EmptyState";

/**
 * Left-of-main sources panel: a tree of ingested sources with sync status and
 * progress, plus a drag-and-drop target. M0 renders only the empty state — the
 * source tree and drop handling arrive with folder ingest in M1.
 */
export function SourcesPanel() {
  return (
    <aside
      aria-label="Sources"
      className="flex h-full w-64 flex-col border-r border-line bg-paper"
    >
      <header className="flex items-center justify-between px-4 py-3">
        <h2 className="font-display text-sm text-ink">Sources</h2>
      </header>
      <div className="flex-1">
        <EmptyState
          title="No sources yet"
          description="Drop a folder, paste a Git or YouTube URL, or drag in PDFs to start building this workspace."
          action={
            <button
              type="button"
              disabled
              className="rounded-md border border-line-strong px-3 py-1.5 text-sm text-ink-muted disabled:opacity-60"
            >
              Add source
            </button>
          }
        />
      </div>
    </aside>
  );
}
