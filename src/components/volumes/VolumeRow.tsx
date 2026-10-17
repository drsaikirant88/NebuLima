import DeleteIcon from "@mui/icons-material/Delete";
import { Box, Chip, IconButton, Stack, Typography } from "@mui/material";
import type React from "react";
import { glassSurface } from "@/theme/glass";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { Volume } from "@/types/ipc";
import { ColumnResizeHandle } from "../shared/ColumnResizeHandle";

interface VolumeRowProps {
  volume: Volume;
  nameWidth: number;
  onResizeName: (width: number) => void;
  onRequestDelete: (volume: Volume) => void;
}

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  const mib = bytes / 1024 / 1024;
  return mib >= 1024 ? `${(mib / 1024).toFixed(1)} GiB` : `${mib.toFixed(0)} MiB`;
}

/**
 * One volume's list row: name, size (or "—" when the driver doesn't report one), and the
 * attached-container names that gate `delete_volume`'s confirmation flow. The name column
 * is user-resizable and truncates with an ellipsis instead of growing the row.
 */
export const VolumeRow: React.FC<VolumeRowProps> = ({
  volume,
  nameWidth,
  onResizeName,
  onRequestDelete,
}) => {
  const { mode } = useThemeMode();
  const isAttached = volume.attachedContainerNames.length > 0;

  return (
    <Box sx={{ p: 1.5, borderRadius: 2, ...glassSurface(mode, 12) }}>
      <Stack
        direction="row"
        spacing={1.5}
        sx={{ alignItems: "center", flexWrap: "nowrap", overflowX: "auto", pb: 0.5 }}
      >
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ alignItems: "center", flex: `0 0 ${nameWidth}px`, minWidth: 0 }}
        >
          <Typography noWrap sx={{ fontWeight: 600, flex: 1, minWidth: 0 }}>
            {volume.name}
          </Typography>
          <ColumnResizeHandle width={nameWidth} onResize={onResizeName} max={600} />
        </Stack>
        <Typography noWrap variant="body2" color="text.secondary" sx={{ flex: "0 0 90px" }}>
          {formatBytes(volume.sizeBytes)}
        </Typography>
        <Chip label={volume.profileName} size="small" variant="outlined" />
        {isAttached ? (
          <Chip label={`Attached: ${volume.attachedContainerNames.join(", ")}`} size="small" />
        ) : (
          <Typography variant="body2" color="text.secondary">
            Unattached
          </Typography>
        )}

        <Box sx={{ flex: 1 }} />

        <IconButton
          aria-label={`Delete ${volume.name}`}
          size="small"
          onClick={() => onRequestDelete(volume)}
        >
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Stack>
    </Box>
  );
};
