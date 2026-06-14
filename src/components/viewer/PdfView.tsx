import { segmentByCharSpan } from "@/lib/highlight";

interface PdfViewProps {
  text: string;
  page: number;
  start: number;
  end: number;
  bbox?: [number, number, number, number];
}

/**
 * Render the cited PDF page's extracted text with the span highlighted, and —
 * when a bounding box is present — an overlaid highlight rectangle.
 *
 * Full pdfjs canvas rendering arrives with the real PDF source parser; for M2
 * this renders the stored page text (the source of the chunk) and the bbox
 * overlay so the citation contract — "click resolves to the exact page span" —
 * is satisfied end to end. The viewer prefers the bbox but always shows the
 * char-span highlight as a fallback, matching PLAN.md §7.
 */
export function PdfView({ text, page, start, end, bbox }: PdfViewProps) {
  const segments = segmentByCharSpan(text, start, end);
  return (
    <div className="px-4 py-3">
      <div className="mb-2 text-xs uppercase tracking-wide text-ink-muted">Page {page}</div>
      <div className="relative rounded border border-line bg-paper-raised">
        {bbox ? (
          <div
            className="pointer-events-none absolute rounded bg-accent-soft/50 ring-1 ring-accent"
            style={{
              left: `${bbox[0] * 100}%`,
              top: `${bbox[1] * 100}%`,
              width: `${(bbox[2] - bbox[0]) * 100}%`,
              height: `${(bbox[3] - bbox[1]) * 100}%`,
            }}
            data-testid="pdf-bbox"
          />
        ) : null}
        <pre className="whitespace-pre-wrap break-words px-3 py-2 font-sans text-sm leading-relaxed text-ink">
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
      </div>
    </div>
  );
}
