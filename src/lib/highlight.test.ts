import { describe, expect, it } from "vitest";
import { charSpanOf, highlightLines, segmentByCharSpan } from "@/lib/highlight";

describe("segmentByCharSpan", () => {
  it("splits into before / highlight / after", () => {
    const segs = segmentByCharSpan("hello world foo", 6, 11);
    expect(segs).toEqual([
      { text: "hello ", highlighted: false },
      { text: "world", highlighted: true },
      { text: " foo", highlighted: false },
    ]);
  });

  it("drops empty leading and trailing runs", () => {
    const segs = segmentByCharSpan("world", 0, 5);
    expect(segs).toEqual([{ text: "world", highlighted: true }]);
  });

  it("clamps an out-of-range span instead of throwing", () => {
    const segs = segmentByCharSpan("abc", 1, 999);
    expect(segs).toEqual([
      { text: "a", highlighted: false },
      { text: "bc", highlighted: true },
    ]);
  });
});

describe("highlightLines", () => {
  it("marks the inclusive 1-based line range", () => {
    const lines = highlightLines("a\nb\nc\nd", 2, 3);
    expect(lines.map((l) => l.highlighted)).toEqual([false, true, true, false]);
    expect(lines[1]).toEqual({ number: 2, text: "b", highlighted: true });
  });

  it("normalizes a reversed range", () => {
    const lines = highlightLines("a\nb\nc", 3, 1);
    expect(lines.every((l) => l.highlighted)).toBe(true);
  });
});

describe("charSpanOf", () => {
  it("extracts a span from charspan and page locators", () => {
    expect(charSpanOf({ kind: "charspan", char_start: 2, char_end: 9 })).toEqual({ start: 2, end: 9 });
    expect(charSpanOf({ kind: "page", page: 1, char_start: 0, char_end: 4 })).toEqual({ start: 0, end: 4 });
  });

  it("returns null for line, time, and absent locators", () => {
    expect(charSpanOf({ kind: "line", line_start: 1, line_end: 2 })).toBeNull();
    expect(charSpanOf({ kind: "time", start_seconds: 0, end_seconds: 1 })).toBeNull();
    expect(charSpanOf(null)).toBeNull();
  });
});
