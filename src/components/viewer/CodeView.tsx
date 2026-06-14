import { highlightLines } from "@/lib/highlight";

/**
 * Render a code file with line numbers, highlighting the cited inclusive line
 * range. Used for `line` locators. Highlighting is gutter + row tint; no syntax
 * coloring yet (a tokenizer lands in M5 polish).
 */
export function CodeView({ text, lineStart, lineEnd }: { text: string; lineStart: number; lineEnd: number }) {
  const lines = highlightLines(text, lineStart, lineEnd);
  return (
    <div className="overflow-x-auto px-2 py-3 font-mono text-xs leading-relaxed">
      {lines.map((line) => (
        <div
          key={line.number}
          className={
            "flex " + (line.highlighted ? "bg-accent-soft" : "")
          }
        >
          <span className="select-none pr-3 text-right text-ink-muted/60" style={{ minWidth: "3ch" }}>
            {line.number}
          </span>
          <code className="whitespace-pre text-ink">{line.text || " "}</code>
        </div>
      ))}
    </div>
  );
}
