/**
 * Design tokens mirrored for JS/Canvas use (charts, dynamic styles) where CSS
 * variables aren't reachable. Keep in sync with src/styles/globals.css.
 */
export const tokens = {
  color: {
    paper: "#fafaf7",
    paperRaised: "#ffffff",
    paperSunken: "#f3f2ec",
    ink: "#1c1b18",
    inkMuted: "#6b6862",
    inkFaint: "#9b978e",
    accent: "#c2613d",
    accentHover: "#a9512f",
    accentSoft: "#f3e3da",
    line: "#e7e4db",
    lineStrong: "#d8d4c8",
    ok: "#3b6d3b",
    warn: "#b07d23",
    danger: "#a8392c",
  },
  font: {
    display: '"Space Grotesk Variable", ui-sans-serif, system-ui, sans-serif',
    sans: '"Inter Variable", ui-sans-serif, system-ui, sans-serif',
    mono: 'ui-monospace, "SF Mono", "Cascadia Code", monospace',
  },
  radius: {
    sm: 4,
    md: 6,
    lg: 10,
  },
} as const;

export type Tokens = typeof tokens;
