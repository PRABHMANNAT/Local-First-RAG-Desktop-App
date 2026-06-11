import { EmptyState } from "@/components/common/EmptyState";

/**
 * Main chat surface: a single centered column (max ~720px) of messages with a
 * persistent composer at the bottom. M0 shows the zero state; messages, the
 * composer, and citation markers land in M1.
 */
export function ChatSurface() {
  return (
    <main className="flex h-full flex-1 flex-col bg-paper">
      <div className="mx-auto flex w-full max-w-[720px] flex-1 flex-col">
        <EmptyState
          title="Ask your sources"
          description="Once you've added a source, ask a question and get an answer with citations back to the exact page, line, or timestamp."
          action={
            <span className="text-xs text-ink-faint">
              Add a source to begin
            </span>
          }
        />
      </div>
    </main>
  );
}
