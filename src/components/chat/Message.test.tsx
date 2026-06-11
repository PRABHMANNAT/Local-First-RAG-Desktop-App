import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Message } from "@/components/chat/Message";
import type { ChatMessage } from "@/stores/chat";

describe("Message", () => {
  it("renders a user message verbatim", () => {
    const m: ChatMessage = { id: "1", role: "user", content: "What is X?" };
    render(<Message message={m} />);
    expect(screen.getByText("What is X?")).toBeInTheDocument();
  });

  it("turns inline citation markers into footnote numbers and a sources list", () => {
    const m: ChatMessage = {
      id: "2",
      role: "assistant",
      content: "X is a thing [^c1].",
      citations: [
        {
          chunkId: "c1",
          text: "X is a thing.",
          structuralPath: "Intro",
          locator: { kind: "charspan", char_start: 0, char_end: 12 },
          pathOrUrl: "/docs/a.md",
          score: 0.8,
          usedInAnswer: true,
        },
      ],
    };
    render(<Message message={m} />);
    // Footnote number "1" appears both as the inline superscript and in the
    // sources list (two occurrences); the raw marker text does not appear.
    expect(screen.getAllByText("1")).toHaveLength(2);
    expect(screen.queryByText(/\[\^c1\]/)).not.toBeInTheDocument();
    // Footnote shows the source basename.
    expect(screen.getByText(/a\.md/)).toBeInTheDocument();
  });
});
