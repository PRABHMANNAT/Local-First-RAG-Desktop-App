import { EmptyState } from "@/components/common/EmptyState";
import { Composer } from "@/components/chat/Composer";
import { MessageList } from "@/components/chat/MessageList";
import { useChatStore } from "@/stores/chat";

/**
 * Main chat surface: a centered column of messages with a persistent composer
 * pinned to the bottom. Shows the zero state until the first message.
 */
export function ChatSurface() {
  const messages = useChatStore((s) => s.messages);

  return (
    <main className="flex h-full flex-1 flex-col bg-paper">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex min-h-full w-full max-w-[720px] flex-col">
          {messages.length === 0 ? (
            <EmptyState
              title="Ask your sources"
              description="Add a source, then ask a question. Answers cite the exact page, line, or timestamp they came from."
              action={<span className="text-xs text-ink-faint">Type below to begin</span>}
            />
          ) : (
            <MessageList messages={messages} />
          )}
        </div>
      </div>
      <Composer />
    </main>
  );
}
