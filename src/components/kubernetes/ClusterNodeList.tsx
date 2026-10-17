import { Chip, Stack, Typography } from "@mui/material";
import type React from "react";
import { PALETTE, withAlpha } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { ClusterNode } from "@/types/ipc";

interface ClusterNodeListProps {
  nodes: ClusterNode[];
}

/**
 * The six-color palette has no distinct hues for success/warning, so readiness is told
 * apart by label + opacity ramp, never by color alone — same convention as
 * `ProfileStateBadge`.
 */
export const ClusterNodeList: React.FC<ClusterNodeListProps> = ({ nodes }) => {
  const { mode } = useThemeMode();
  const base = mode === "dark" ? PALETTE.warmSand : PALETTE.mutedMauve;

  if (nodes.length === 0) {
    return <Typography variant="caption">No nodes reported yet.</Typography>;
  }

  return (
    <Stack spacing={0.5}>
      {nodes.map((node) => (
        <Stack key={node.name} direction="row" spacing={1} sx={{ alignItems: "center" }}>
          <Typography variant="body2">{node.name}</Typography>
          <Chip
            size="small"
            label={node.ready ? "Ready" : "Not ready"}
            variant="outlined"
            sx={{
              backgroundColor: withAlpha(base, node.ready ? 0.28 : 0.14),
              color: mode === "dark" ? PALETTE.white : PALETTE.deepInk,
              borderColor: withAlpha(base, 0.4),
              fontWeight: node.ready ? 700 : 500,
            }}
          />
        </Stack>
      ))}
    </Stack>
  );
};
