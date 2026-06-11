import { useWorkspaceStore } from "@/stores/workspace";
import { cn } from "@/lib/cn";

/**
 * Narrow left rail of workspaces. Each workspace is a small rounded tile with
 * its icon/initial. M0 shows the empty rail plus a disabled "new workspace"
 * affordance; wiring lands in M1.
 */
export function WorkspaceRail() {
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeId = useWorkspaceStore((s) => s.activeId);
  const setActive = useWorkspaceStore((s) => s.setActive);

  return (
    <nav
      aria-label="Workspaces"
      className="flex h-full w-14 flex-col items-center gap-2 border-r border-line bg-paper-sunken py-3"
    >
      {workspaces.map((ws) => {
        const isActive = ws.id === activeId;
        return (
          <button
            key={ws.id}
            type="button"
            onClick={() => setActive(ws.id)}
            aria-current={isActive ? "true" : undefined}
            title={ws.name}
            className={cn(
              "flex h-9 w-9 items-center justify-center rounded-md text-sm transition-colors",
              isActive
                ? "bg-accent text-paper-raised"
                : "bg-paper-raised text-ink-muted hover:text-ink",
            )}
          >
            {ws.icon ?? ws.name.charAt(0).toUpperCase()}
          </button>
        );
      })}
      <button
        type="button"
        aria-label="New workspace"
        title="New workspace"
        className="flex h-9 w-9 items-center justify-center rounded-md border border-dashed border-line-strong text-ink-faint hover:text-ink-muted"
      >
        +
      </button>
    </nav>
  );
}
