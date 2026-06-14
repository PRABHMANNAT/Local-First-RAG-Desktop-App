/** The source kinds Mnemos can ingest, mirrored from the Rust `source.kind`. */
export type SourceKind = "folder" | "repo" | "youtube" | "pdf" | "url";

/** A short glyph for each source kind, used in the source tree. */
const ICONS: Record<SourceKind, string> = {
  folder: "📁",
  repo: "",
  youtube: "▶",
  pdf: "",
  url: "🔗",
};

/** Human label for each source kind. */
const LABELS: Record<SourceKind, string> = {
  folder: "Folder",
  repo: "Repository",
  youtube: "YouTube",
  pdf: "PDF",
  url: "Web page",
};

/** Icon glyph for a source kind; falls back to a generic doc glyph. */
export function iconFor(kind: string): string {
  return ICONS[kind as SourceKind] ?? "📄";
}

/** Human-readable label for a source kind; falls back to the raw kind. */
export function labelFor(kind: string): string {
  return LABELS[kind as SourceKind] ?? kind;
}

/** Whether a source kind supports user-triggered Sync (re-fetch). Folders
 * watch the filesystem; repo/url/youtube are pull-on-demand (PLAN §5). */
export function isSyncable(kind: string): boolean {
  return kind === "repo" || kind === "url" || kind === "youtube";
}
