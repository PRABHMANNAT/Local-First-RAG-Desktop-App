import type { Locator } from "@/ipc/client";

/** A run of text the viewer renders, flagged for highlight emphasis. */
export interface Segment {
  text: string;
  highlighted: boolean;
}

/**
 * Split flowing `text` into three segments around a character span
 * `[start, end)`: before, the highlighted span, after. Offsets are clamped to
 * the text bounds so an out-of-range locator degrades gracefully (no throw,
 * just an empty or truncated highlight). Empty leading/trailing runs are
 * dropped so the viewer doesn't render blank nodes.
 */
export function segmentByCharSpan(text: string, start: number, end: number): Segment[] {
  const lo = Math.max(0, Math.min(start, text.length));
  const hi = Math.max(lo, Math.min(end, text.length));
  const segments: Segment[] = [
    { text: text.slice(0, lo), highlighted: false },
    { text: text.slice(lo, hi), highlighted: true },
    { text: text.slice(hi), highlighted: false },
  ];
  return segments.filter((s) => s.text.length > 0);
}

/** A source line paired with whether the locator highlights it. */
export interface HighlightedLine {
  number: number;
  text: string;
  highlighted: boolean;
}

/**
 * Annotate each line of `text` with whether it falls in the inclusive 1-based
 * range `[lineStart, lineEnd]`. Line numbers are 1-based to match editors and
 * the `line` locator. The range is clamped, so a locator past EOF highlights
 * nothing rather than throwing.
 */
export function highlightLines(text: string, lineStart: number, lineEnd: number): HighlightedLine[] {
  const lines = text.split("\n");
  const lo = Math.min(lineStart, lineEnd);
  const hi = Math.max(lineStart, lineEnd);
  return lines.map((line, i) => {
    const number = i + 1;
    return { number, text: line, highlighted: number >= lo && number <= hi };
  });
}

/**
 * Resolve the highlightable character span a locator points at, if any. Both
 * `charspan` and `page` locators carry a char span; `line` and `time` do not
 * (they're handled by their own views). Returns `null` when there's no span.
 */
export function charSpanOf(locator: Locator | null): { start: number; end: number } | null {
  if (!locator) return null;
  if (locator.kind === "charspan" || locator.kind === "page") {
    return { start: locator.char_start, end: locator.char_end };
  }
  return null;
}
