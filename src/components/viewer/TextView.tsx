import { segmentByCharSpan } from "@/lib/highlight";

/**
 * Render flowing text (markdown, plain text, extracted URL content) with the
 * cited character span highlighted. Used for `charspan` locators.
 */
export function TextView({ text, start, end }: { text: string; start: number; end: number }) {
  const segments = segmentByCharSpan(text, start, end);
  return (
    <pre className="whitespace-pre-wrap break-words px-4 py-3 font-sans text-sm leading-relaxed text-ink">
      {segments.map((seg, i) =>
        seg.highlighted ? (
          <mark key={i} className="rounded bg-accent-soft px-0.5 text-ink">
            {seg.text}
          </mark>
        ) : (
          <span key={i}>{seg.text}</span>
        ),
      )}
    </pre>
  );
}
