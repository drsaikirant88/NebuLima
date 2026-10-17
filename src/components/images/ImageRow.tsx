import DeleteIcon from "@mui/icons-material/Delete";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import { Box, Chip, IconButton, Stack, Typography } from "@mui/material";
import type React from "react";
import { glassSurface } from "@/theme/glass";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { Image } from "@/types/ipc";
import { ColumnResizeHandle } from "../shared/ColumnResizeHandle";

interface ImageRowProps {
  image: Image;
  referenceWidth: number;
  onResizeReference: (width: number) => void;
  onInspect: (image: Image) => void;
  onRequestDelete: (image: Image) => void;
}

function formatBytes(bytes: number): string {
  const mib = bytes / 1024 / 1024;
  return mib >= 1024 ? `${(mib / 1024).toFixed(1)} GiB` : `${mib.toFixed(0)} MiB`;
}

const reference = (image: Image): string =>
  image.repository ? `${image.repository}:${image.tag ?? "latest"}` : "<none>";

/**
 * One image's list row: repository:tag, size, created date, and an in-use indicator
 * (the basis for whether delete requires confirmation). The repository:tag column is
 * user-resizable and truncates with an ellipsis instead of growing the row. Inspect/delete
 * are plain callbacks up to `ImagesView`, which owns the inspect dialog and the
 * conditional-confirmation delete flow.
 */
export const ImageRow: React.FC<ImageRowProps> = ({
  image,
  referenceWidth,
  onResizeReference,
  onInspect,
  onRequestDelete,
}) => {
  const { mode } = useThemeMode();

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
          sx={{ alignItems: "center", flex: `0 0 ${referenceWidth}px`, minWidth: 0 }}
        >
          <Typography noWrap sx={{ fontWeight: 600, flex: 1, minWidth: 0 }}>
            {reference(image)}
          </Typography>
          <ColumnResizeHandle width={referenceWidth} onResize={onResizeReference} max={600} />
        </Stack>
        <Typography noWrap variant="body2" color="text.secondary" sx={{ flex: "0 0 90px" }}>
          {formatBytes(image.sizeBytes)}
        </Typography>
        <Typography noWrap variant="body2" color="text.secondary" sx={{ flex: "0 0 110px" }}>
          {new Date(image.createdAt).toLocaleDateString()}
        </Typography>
        <Chip label={image.profileName} size="small" variant="outlined" />
        {image.inUse && <Chip label="In use" size="small" />}

        <Box sx={{ flex: 1 }} />

        <IconButton
          aria-label={`Inspect ${reference(image)}`}
          size="small"
          onClick={() => onInspect(image)}
        >
          <InfoOutlinedIcon fontSize="small" />
        </IconButton>
        <IconButton
          aria-label={`Delete ${reference(image)}`}
          size="small"
          onClick={() => onRequestDelete(image)}
        >
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Stack>
    </Box>
  );
};
