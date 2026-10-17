import { Box } from "@mui/material";
import type React from "react";
import { PALETTE, withAlpha } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";

interface ColumnResizeHandleProps {
  width: number;
  onResize: (width: number) => void;
  min?: number;
  max?: number;
}

const DEFAULT_MIN_WIDTH = 100;
const DEFAULT_MAX_WIDTH = 480;

/**
 * A column-resize drag handle rendered as a short vertical grip line, so the resizable
 * boundary is visible up front instead of only revealing itself on hover (a bare hover-only
 * hitbox reads as empty space until the cursor stumbles onto it). `width` is read once per
 * drag (captured as `startWidth` at `pointerdown`), so each `pointermove` computes the new
 * width from a fixed baseline instead of compounding deltas.
 */
export const ColumnResizeHandle: React.FC<ColumnResizeHandleProps> = ({
  width,
  onResize,
  min = DEFAULT_MIN_WIDTH,
  max = DEFAULT_MAX_WIDTH,
}) => {
  const { mode } = useThemeMode();
  const gripColor =
    mode === "dark" ? withAlpha(PALETTE.warmSand, 0.4) : withAlpha(PALETTE.slate, 0.35);
  const gripColorHover =
    mode === "dark" ? withAlpha(PALETTE.warmSand, 0.9) : withAlpha(PALETTE.slate, 0.8);

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    event.stopPropagation();
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = width;

    const handleMove = (moveEvent: PointerEvent) => {
      const next = Math.min(max, Math.max(min, startWidth + (moveEvent.clientX - startX)));
      onResize(next);
    };
    const handleUp = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
    };
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
  };

  return (
    <Box
      onPointerDown={handlePointerDown}
      onClick={(event) => event.stopPropagation()}
      sx={{
        width: 12,
        height: 20,
        flexShrink: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: "col-resize",
        "&:hover .column-resize-grip": { backgroundColor: gripColorHover },
      }}
    >
      <Box
        className="column-resize-grip"
        sx={{
          width: 2,
          height: "100%",
          borderRadius: 1,
          backgroundColor: gripColor,
          transition: "background-color 120ms ease",
        }}
      />
    </Box>
  );
};
