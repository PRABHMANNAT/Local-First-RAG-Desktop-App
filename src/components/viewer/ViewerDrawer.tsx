import type { Citation } from "@/ipc/client";
import { useViewerStore } from "@/stores/viewer";
import { TextView } from "@/components/viewer/TextView";
import { CodeView } from "@/components/viewer/CodeView";
import { PdfView } from "@/components/viewer/PdfView";

function basename(uri: string): string {
  const parts = uri.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? uri;
}

/** Switch on the citation's locator kind to render the right span view. */
function ViewerBody({ citation }: { citation: Citation }) {
  const loc = citation.locator;
  if (!loc) {
    return <TextView text={citation.text} start={0} end={citation.text.length} />;
  }
  switch (loc.kind) {
    case "charspan":
      return <TextView text={citation.text} start={loc.char_start} end={loc.char_end} />;
    case "line":
      return <CodeView text={citation.text} lineStart={loc.line_start} lineEnd={loc.line_end} />;
    case "page":
      return (
        <PdfView
          text={citation.text}
          page={loc.page}
          start={loc.char_start}
          end={loc.char_end}
          bbox={loc.bbox}
        />
      );
    case "time":
      return (
        <div className="px-4 py-3 text-sm text-ink">
          <div className="mb-2 text-xs uppercase tracking-wide text-ink-muted">
            {formatTime(loc.start_seconds)} – {formatTime(loc.end_seconds)}
          </div>
          <p className="leading-relaxed">{citation.text}</p>
        </div>
      );
  }
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/**
 * The source viewer drawer. Slides in from the right when a citation is opened;
 * multiple open citations become tabs. Switches the body on the active tab's
 * locator kind (text / code / pdf / transcript) per PLAN.md §7.
 */
export function ViewerDrawer() {
  const { tabs, activeChunkId, isOpen, focus, close, closeAll } = useViewerStore();
  if (!isOpen || tabs.length === 0) return null;

  const active = tabs.find((t) => t.chunkId === activeChunkId) ?? tabs[0]!;

  return (
    <aside className="flex w-[420px] shrink-0 flex-col border-l border-line bg-paper" aria-label="Source viewer">
      <header className="flex items-center justify-between border-b border-line px-3 py-2">
        <span className="truncate text-sm font-medium text-ink" title={active.pathOrUrl}>
          {basename(active.pathOrUrl)}
        </span>
        <button
          type="button"
          onClick={closeAll}
          className="rounded px-1.5 text-ink-muted hover:bg-paper-sunken hover:text-ink"
          aria-label="Close source viewer"
        >
          ✕
        </button>
      </header>

      {tabs.length > 1 ? (
        <div className="flex gap-1 overflow-x-auto border-b border-line px-2 py-1.5" role="tablist">
          {tabs.map((t) => (
            <div
              key={t.chunkId}
              className={
                "flex items-center gap-1 rounded px-2 py-0.5 text-xs " +
                (t.chunkId === active.chunkId
                  ? "bg-accent-soft text-ink"
                  : "text-ink-muted hover:bg-paper-sunken")
              }
            >
              <button
                type="button"
                role="tab"
                aria-selected={t.chunkId === active.chunkId}
                onClick={() => focus(t.chunkId)}
                className="max-w-[120px] truncate"
                title={t.pathOrUrl}
              >
                {basename(t.pathOrUrl)}
              </button>
              <button
                type="button"
                onClick={() => close(t.chunkId)}
                className="text-ink-muted/70 hover:text-ink"
                aria-label={`Close ${basename(t.pathOrUrl)}`}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-y-auto">
        <ViewerBody citation={active} />
      </div>
    </aside>
  );
}
