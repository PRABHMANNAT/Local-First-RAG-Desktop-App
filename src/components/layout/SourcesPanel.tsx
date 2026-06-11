import { EmptyState } from "@/components/common/EmptyState";
import { AddSourceButton } from "@/components/sources/AddSourceButton";
import { SourceTree } from "@/components/sources/SourceTree";
import { useSourcesStore } from "@/stores/sources";

/**
 * Left-of-main sources panel: a tree of ingested sources with live status, plus
 * the add-source action. Empty until the first source is added.
 */
export function SourcesPanel() {
  const hasSources = useSourcesStore((s) => s.items.length > 0);

  return (
    <aside
      aria-label="Sources"
      className="flex h-full w-64 flex-col border-r border-line bg-paper"
    >
      <header className="flex items-center justify-between px-4 py-3">
        <h2 className="font-display text-sm text-ink">Sources</h2>
        {hasSources ? <AddSourceButton compact /> : null}
      </header>
      <div className="flex-1 overflow-y-auto">
        {hasSources ? (
          <SourceTree />
        ) : (
          <EmptyState
            title="No sources yet"
            description="Drop in a folder to start building this workspace. Repos, PDFs, URLs, and YouTube come next."
            action={<AddSourceButton />}
          />
        )}
      </div>
    </aside>
  );
}
