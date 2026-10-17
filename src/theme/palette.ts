// The locked six-color palette. No other hex value, tint, shade, or named color may
// appear anywhere in the UI — alpha variants of these six are the only permitted way to
// produce the glassmorphism blur effect.
export const PALETTE = {
  white: "#FFFFFF",
  lightGrey: "#EBEBEB",
  deepInk: "#181722",
  slate: "#393F4A",
  warmSand: "#DCD2CC",
  mutedMauve: "#8A7875",
} as const;

export type PaletteColor = (typeof PALETTE)[keyof typeof PALETTE];

/** `#RRGGBB` + alpha → `rgba(...)`, the only sanctioned way to vary these six colors. */
export function withAlpha(hex: PaletteColor, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
