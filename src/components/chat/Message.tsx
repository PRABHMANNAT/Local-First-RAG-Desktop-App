import type { ReactNode } from "react";
import type { ChatMessage } from "@/stores/chat";
import type { Citation } from "@/ipc/client";

function basename(uri: string): string {
  const parts = uri.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? uri;
}

/**
 * Render assistant content, replacing inline `[^chunk_id]` markers with
 * superscript footnote numbers. M1 renders them as plain (non-interactive)
 * markers; the click-through citation drawer arrives in M2.
 */
function renderContent(message: ChatMessage): ReactNode {
  const used = (message.citations ?? []).filter((c) => c.usedInAnswer);
  if (message.role !== "assistant" || used.length === 0) {
    return message.content;
  }
  const numberById = new Map<string, number>();
  used.forEach((c, i) => numberById.set(c.chunkId, i + 1));

  const parts: ReactNode[] = [];
  const re = /\[\^([^\]]+)\]/g;
  let last = 0;
  let key = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(message.content)) !== null) {
    parts.push(message.content.slice(last, m.index));
    const id = m[1] ?? "";
    const n = numberById.get(id);
    if (n !== undefined) {
      parts.push(
        <sup key={key++} className="ml-0.5 text-[10px] font-medium text-accent">
          {n}
        </sup>,
      );
    }
    last = re.lastIndex;
  }
  parts.push(message.content.slice(last));
  return parts;
}

function Footnotes({ citations }: { citations: Citation[] }) {
  const used = citations.filter((c) => c.usedInAnswer);
  if (used.length === 0) return null;
  return (
    <ol className="mt-3 space-y-1 border-t border-line pt-2 text-xs text-ink-muted">
      {used.map((c, i) => (
        <li key={c.chunkId} className="flex gap-2">
          <span className="text-accent">{i + 1}</span>
          <span className="truncate" title={c.pathOrUrl}>
            {basename(c.pathOrUrl)}
            {c.structuralPath ? ` · ${c.structuralPath}` : ""}
          </span>
        </li>
      ))}
    </ol>
  );
}

/** A single chat message bubble. */
export function Message({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
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
        <div className="whitespace-pre-wrap">{renderContent(message)}</div>
        {message.citations ? <Footnotes citations={message.citations} /> : null}
      </div>
    </div>
  );
}
