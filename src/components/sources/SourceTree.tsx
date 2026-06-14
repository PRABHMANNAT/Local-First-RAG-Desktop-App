import type { SourceItem } from "@/stores/sources";
import { useSourcesStore } from "@/stores/sources";
import { api } from "@/ipc/client";
import { iconFor, isSyncable, labelFor } from "@/lib/sourceKind";

/** Short basename of a path/uri for display. */
function basename(uri: string): string {
  const parts = uri.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? uri;
}

function StatusLabel({ item }: { item: SourceItem }) {
  if (item.status === "ingesting" || item.status === "syncing") {
    const p = item.progress;
    const pct = p && p.total > 0 ? Math.round((p.index / p.total) * 100) : 0;
    const verb = item.status === "syncing" ? "syncing" : "ingesting";
    return (
      <span className="text-ink-faint">
        {verb}… {pct}%
      </span>
    );
  }
  if (item.status === "error") {
    return <span className="text-danger">error</span>;
  }
  return (
    <span className="text-ink-faint">
      {item.documents ?? 0} docs · {item.chunks ?? 0} chunks
    </span>
  );
}

/** List of sources with live ingest status and per-kind affordances. */
export function SourceTree() {
  const items = useSourcesStore((s) => s.items);
  const markSyncing = useSourcesStore((s) => s.markSyncing);

  async function sync(id: string) {
    markSyncing(id);
    await api.syncSource(id);
  }

  return (
    <ul className="flex flex-col gap-1 px-2">
      {items.map((item) => {
        const busy = item.status === "ingesting" || item.status === "syncing";
        return (
          <li key={item.id} className="rounded-md px-2 py-1.5 hover:bg-paper-sunken" title={item.uri}>
            <div className="flex items-center justify-between gap-2">
              <span className="flex min-w-0 items-center gap-1.5">
                <span aria-label={labelFor(item.kind)} title={labelFor(item.kind)}>
                  {iconFor(item.kind)}
                </span>
                <span className="truncate text-sm text-ink">{basename(item.uri)}</span>
              </span>
              <span className="flex shrink-0 items-center gap-1.5">
                {isSyncable(item.kind) ? (
                  <button
                    type="button"
                    onClick={() => sync(item.id)}
                    disabled={busy}
                    className="text-xs text-ink-faint hover:text-ink disabled:opacity-50"
                    title="Re-sync this source"
                    aria-label={`Sync ${basename(item.uri)}`}
                  >
                    ⟳
                  </button>
                ) : null}
                <span
                  className={
                    "h-1.5 w-1.5 rounded-full " +
                    (item.status === "ready"
                      ? "bg-ok"
                      : item.status === "error"
                        ? "bg-danger"
                        : "bg-warn")
                  }
                  aria-hidden
                />
              </span>
            </div>
            <div className="text-xs">
              <StatusLabel item={item} />
            </div>
          </li>
        );
      })}
    </ul>
  );
}
