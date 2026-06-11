import { create } from "zustand";
import type { Citation } from "@/ipc/client";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  citations?: Citation[];
  rejected?: boolean;
}

interface ChatState {
  messages: ChatMessage[];
  asking: boolean;
  append: (message: ChatMessage) => void;
  setAsking: (asking: boolean) => void;
  reset: () => void;
}

/** Conversation state for the active chat surface. */
export const useChatStore = create<ChatState>((set) => ({
  messages: [],
  asking: false,
  append: (message) => set((s) => ({ messages: [...s.messages, message] })),
  setAsking: (asking) => set({ asking }),
  reset: () => set({ messages: [], asking: false }),
}));
