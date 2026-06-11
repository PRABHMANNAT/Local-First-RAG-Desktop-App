import { Message } from "@/components/chat/Message";
import type { ChatMessage } from "@/stores/chat";

/** Scrollable column of chat messages. */
export function MessageList({ messages }: { messages: ChatMessage[] }) {
  return (
    <div className="flex flex-col gap-3 px-4 py-6">
      {messages.map((m) => (
        <Message key={m.id} message={m} />
      ))}
    </div>
  );
}
