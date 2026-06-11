import { WorkspaceRail } from "@/components/layout/WorkspaceRail";
import { SourcesPanel } from "@/components/layout/SourcesPanel";
import { ChatSurface } from "@/components/layout/ChatSurface";
import { StatusBar } from "@/components/common/StatusBar";
import { useUiStore } from "@/stores/ui";

/**
 * Top-level layout: workspace rail · (collapsible) sources panel · chat surface,
 * with a status bar pinned to the bottom. The source viewer drawer slides in
 * over the chat surface on citation click (M2).
 */
export function AppShell() {
  const sourcesPanelOpen = useUiStore((s) => s.sourcesPanelOpen);

  return (
    <div className="flex h-full flex-col">
      <div className="flex min-h-0 flex-1">
        <WorkspaceRail />
        {sourcesPanelOpen ? <SourcesPanel /> : null}
        <ChatSurface />
      </div>
      <StatusBar />
    </div>
  );
}
