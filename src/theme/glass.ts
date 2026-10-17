import type { CSSProperties } from "react";
import { PALETTE, withAlpha } from "./palette";

/**
 * The reusable frosted-glass surface mixin: a semi-transparent palette background plus
 * `backdrop-filter: blur()`. Used by every elevated surface — sidebar, titlebar, cards,
 * dialogs.
 */
export function glassSurface(mode: "dark" | "light", blurPx = 24): CSSProperties {
  const background =
    mode === "dark" ? withAlpha(PALETTE.slate, 0.55) : withAlpha(PALETTE.white, 0.6);
  const border =
    mode === "dark" ? withAlpha(PALETTE.warmSand, 0.12) : withAlpha(PALETTE.deepInk, 0.08);
  return {
    backgroundColor: background,
    backdropFilter: `blur(${blurPx}px)`,
    WebkitBackdropFilter: `blur(${blurPx}px)`,
    border: `1px solid ${border}`,
  };
}

/**
 * Full-window backdrop: on macOS this layers subtly over the native vibrancy material
 * configured in `tauri.conf.json`'s window `effects`; on Linux (no native blur-behind
 * support in webkitgtk) this *is* the fallback — same CSS, so no OS-sniffing branch is
 * needed on the frontend.
 */
export function windowBackdropGradient(mode: "dark" | "light"): CSSProperties {
  const gradient =
    mode === "dark"
      ? `radial-gradient(circle at 20% 0%, ${withAlpha(PALETTE.slate, 0.9)}, ${withAlpha(PALETTE.deepInk, 0.97)} 60%)`
      : `radial-gradient(circle at 20% 0%, ${withAlpha(PALETTE.white, 0.95)}, ${withAlpha(PALETTE.lightGrey, 0.98)} 60%)`;
  return { background: gradient };
}
