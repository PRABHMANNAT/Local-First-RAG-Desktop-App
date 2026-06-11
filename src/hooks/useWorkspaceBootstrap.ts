import { useEffect } from "react";
import { isTauri } from "@/lib/env";
import { api } from "@/ipc/client";
import { onIngestDone, onIngestProgress } from "@/ipc/events";
import { useWorkspaceStore } from "@/stores/workspace";
import { useSourcesStore } from "@/stores/sources";

/**
 * On mount (inside Tauri), open the default workspace and subscribe to ingest
 * progress/done events, routing them into the sources store. No-op in the
 * browser/test preview.
 */
export function useWorkspaceBootstrap() {
  const setInfo = useWorkspaceStore((s) => s.setInfo);
  const setProgress = useSourcesStore((s) => s.setProgress);
  const markDone = useSourcesStore((s) => s.markDone);

  useEffect(() => {
    if (!isTauri()) return;
    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    let cancelled = false;

    (async () => {
      try {
        const info = await api.openDefaultWorkspace();
        if (!cancelled) setInfo(info);
      } catch {
        // Surfaced elsewhere; bootstrap stays silent.
      }
      unlistenProgress = await onIngestProgress((p) =>
        setProgress(p.sourceId, p.index, p.total),
      );
      unlistenDone = await onIngestDone((d) =>
        markDone(d.sourceId, {
          ok: d.ok,
          documents: d.documents,
          chunks: d.chunks,
          error: d.error,
        }),
      );
    })();

    return () => {
      cancelled = true;
      unlistenProgress?.();
      unlistenDone?.();
    };
  }, [setInfo, setProgress, markDone]);
}
