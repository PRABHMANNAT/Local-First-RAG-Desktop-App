import { useState } from "react";
import { api } from "@/ipc/client";
import { isTauri } from "@/lib/env";
import { useChatStore } from "@/stores/chat";

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `m_${Date.now()}_${Math.floor(Math.random() * 1e6)}`;
}

/** Persistent input at the bottom of the chat surface. Enter sends; Shift+Enter
 * inserts a newline. */
export function Composer() {
  const [text, setText] = useState("");
  const asking = useChatStore((s) => s.asking);
  const append = useChatStore((s) => s.append);
  const setAsking = useChatStore((s) => s.setAsking);

  async function send() {
    const query = text.trim();
    if (!query || asking) return;
    setText("");
    append({ id: newId(), role: "user", content: query });

    if (!isTauri()) {
      append({
        id: newId(),
        role: "assistant",
        content: "Chat runs in the desktop app, where the local model answers from your sources.",
        rejected: true,
      });
      return;
    }

    setAsking(true);
    try {
      const answer = await api.ask(query);
      append({
        id: answer.conversationId,
        role: "assistant",
        content: answer.answer,
        citations: answer.citations,
        rejected: answer.rejected,
      });
    } catch (e) {
      append({
        id: newId(),
        role: "assistant",
        content: `Something went wrong: ${String(e)}`,
        rejected: true,
      });
    } finally {
      setAsking(false);
    }
  }

  return (
    <div className="border-t border-line bg-paper px-4 py-3">
      <div className="mx-auto flex w-full max-w-[720px] items-end gap-2">
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void send();
            }
          }}
          rows={1}
          placeholder="Ask your sources…"
          className="min-h-[40px] flex-1 resize-none rounded-md border border-line bg-paper-raised px-3 py-2 text-sm text-ink outline-none focus:border-line-strong"
        />
        <button
          type="button"
          onClick={() => void send()}
          disabled={asking || !text.trim()}
          className="h-10 rounded-md bg-accent px-4 text-sm text-paper-raised hover:bg-accent-hover disabled:opacity-50"
        >
          {asking ? "…" : "Ask"}
        </button>
      </div>
    </div>
  );
}
