import type { SourceItem } from "@/stores/sources";
import { useSourcesStore } from "@/stores/sources";

/** Short basename of a path/uri for display. */
function basename(uri: string): string {
  const parts = uri.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? uri;
}

function StatusLabel({ item }: { item: SourceItem }) {
  if (item.status === "ingesting") {
    const p = item.progress;
    const pct = p && p.total > 0 ? Math.round((p.index / p.total) * 100) : 0;
    return <span className="text-ink-faint">ingesting… {pct}%</span>;
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

/** List of sources with live ingest status. */
export function SourceTree() {
  const items = useSourcesStore((s) => s.items);

  return (
    <ul className="flex flex-col gap-1 px-2">
      {items.map((item) => (
        <li
          key={item.id}
          className="rounded-md px-2 py-1.5 hover:bg-paper-sunken"
          title={item.uri}
        >
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-sm text-ink">{basename(item.uri)}</span>
            <span
              className={
                "h-1.5 w-1.5 shrink-0 rounded-full " +
                (item.status === "ready"
                  ? "bg-ok"
                  : item.status === "error"
                    ? "bg-danger"
                    : "bg-warn")
              }
              aria-hidden
            />
          </div>
          <div className="text-xs">
            <StatusLabel item={item} />
          </div>
        </li>
      ))}
    </ul>
  );
}
