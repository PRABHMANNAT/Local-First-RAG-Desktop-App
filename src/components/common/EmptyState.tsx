import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

interface EmptyStateProps {
  /** Short icon glyph (we use a single character or small SVG later). */
  icon?: ReactNode;
  title: string;
  description: string;
  /** The one clear next action. Empty states without an action read as dead. */
  action?: ReactNode;
  className?: string;
}

/**
 * Deliberate empty state: one line of what's missing, one line of why it
 * matters, and exactly one next action. Used by every panel in its zero state.
 */
export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex h-full flex-col items-center justify-center px-8 text-center",
        className,
      )}
    >
      {icon ? (
        <div className="mb-4 text-ink-faint" aria-hidden>
          {icon}
        </div>
      ) : null}
      <h2 className="font-display text-base text-ink">{title}</h2>
      <p className="mt-1.5 max-w-xs text-sm text-ink-muted">{description}</p>
      {action ? <div className="mt-5">{action}</div> : null}
    </div>
  );
}
