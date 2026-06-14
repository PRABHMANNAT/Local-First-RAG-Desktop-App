import type { ReactNode } from "react";
import type { ChatMessage } from "@/stores/chat";
import type { Citation } from "@/ipc/client";
import { useViewerStore } from "@/stores/viewer";

function basename(uri: string): string {
  const parts = uri.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? uri;
}

/**
 * Render assistant content, replacing inline `[^chunk_id]` markers with
 * clickable superscript footnote numbers. Clicking a marker opens the cited
 * chunk in the source viewer drawer at its locator (M2).
 */
function renderContent(message: ChatMessage, openCitation: (c: Citation) => void): ReactNode {
  const used = (message.citations ?? []).filter((c) => c.usedInAnswer);
  if (message.role !== "assistant" || used.length === 0) {
    return message.content;
  }
  const byId = new Map<string, { n: number; citation: Citation }>();
  used.forEach((c, i) => byId.set(c.chunkId, { n: i + 1, citation: c }));

  const parts: ReactNode[] = [];
  const re = /\[\^([^\]]+)\]/g;
  let last = 0;
  let key = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(message.content)) !== null) {
    parts.push(message.content.slice(last, m.index));
    const id = m[1] ?? "";
    const hit = byId.get(id);
    if (hit !== undefined) {
      parts.push(
        <button
          type="button"
          key={key++}
          onClick={() => openCitation(hit.citation)}
          className="ml-0.5 align-super text-[10px] font-medium text-accent hover:underline"
          aria-label={`Open citation ${hit.n}`}
        >
          {hit.n}
        </button>,
      );
    }
    last = re.lastIndex;
  }
  parts.push(message.content.slice(last));
  return parts;
}

function Footnotes({
  citations,
  openCitation,
}: {
  citations: Citation[];
  openCitation: (c: Citation) => void;
}) {
  const used = citations.filter((c) => c.usedInAnswer);
  if (used.length === 0) return null;
  return (
    <ol className="mt-3 space-y-1 border-t border-line pt-2 text-xs text-ink-muted">
      {used.map((c, i) => (
        <li key={c.chunkId} className="flex gap-2">
          <span className="text-accent">{i + 1}</span>
          <button
            type="button"
            onClick={() => openCitation(c)}
            className="truncate text-left hover:text-ink hover:underline"
            title={c.pathOrUrl}
          >
            {basename(c.pathOrUrl)}
            {c.structuralPath ? ` · ${c.structuralPath}` : ""}
          </button>
        </li>
      ))}
    </ol>
  );
}

/** A single chat message bubble. */
export function Message({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  const openCitation = useViewerStore((s) => s.open);
  return (
    <div className={isUser ? "flex justify-end" : "flex justify-start"}>
      <div
        className={
          "max-w-[90%] rounded-lg px-3 py-2 text-sm leading-relaxed " +
          (isUser
            ? "bg-accent-soft text-ink"
            : message.rejected
              ? "bg-paper-sunken text-ink-muted"
              : "bg-paper-raised text-ink ring-1 ring-line")
        }
      >
        <div className="whitespace-pre-wrap">{renderContent(message, openCitation)}</div>
        {message.citations ? (
          <Footnotes citations={message.citations} openCitation={openCitation} />
        ) : null}
      </div>
    </div>
  );
}
