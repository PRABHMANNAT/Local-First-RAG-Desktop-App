import { useEffect, useState } from "react";
import { isTauri } from "@/lib/env";
import { api } from "@/ipc/client";
import { onTick } from "@/ipc/events";

type IpcHealth = "checking" | "ok" | "preview" | "error";

/**
 * Bottom status bar. Shows the privacy posture badge (🟢 local-only by default)
 * and an IPC health indicator that exercises both halves of the contract on
 * mount: a `ping` round-trip and a short `tick` event stream.
 */
export function StatusBar() {
  const [health, setHealth] = useState<IpcHealth>("checking");
  const [version, setVersion] = useState<string>("");
  const [ticks, setTicks] = useState(0);

  useEffect(() => {
    if (!isTauri()) {
      setHealth("preview");
      return;
    }
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    (async () => {
      try {
        const [pong, ver] = await Promise.all([
          api.ping("mnemos"),
          api.appVersion(),
        ]);
        if (cancelled) return;
        setVersion(ver);
        setHealth(pong.message === "pong" ? "ok" : "error");
        unlisten = await onTick(() => setTicks((n) => n + 1));
        await api.startTick(3, 200);
      } catch {
        if (!cancelled) setHealth("error");
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return (
    <footer className="flex items-center justify-between border-t border-line bg-paper-sunken px-4 py-1.5 text-xs text-ink-muted">
      <span className="flex items-center gap-1.5">
        <span aria-hidden>🟢</span>
        <span>Local-only</span>
      </span>
      <span className="flex items-center gap-3">
        {version ? <span>v{version}</span> : null}
        <IpcBadge health={health} ticks={ticks} />
      </span>
    </footer>
  );
}

function IpcBadge({ health, ticks }: { health: IpcHealth; ticks: number }) {
  const label: Record<IpcHealth, string> = {
    checking: "IPC: checking…",
    ok: `IPC: ok (${ticks} ticks)`,
    preview: "Preview mode",
    error: "IPC: error",
  };
  const color: Record<IpcHealth, string> = {
    checking: "text-ink-faint",
    ok: "text-ok",
    preview: "text-ink-faint",
    error: "text-danger",
  };
  return <span className={color[health]}>{label[health]}</span>;
}
