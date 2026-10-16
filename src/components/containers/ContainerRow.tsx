import ArticleIcon from "@mui/icons-material/Article";
import DeleteIcon from "@mui/icons-material/Delete";
import { Box, Button, Chip, IconButton, Stack, Typography } from "@mui/material";
import type React from "react";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import { startContainer, stopContainer } from "@/services/ipc";
import { glassSurface } from "@/theme/glass";
import { PALETTE } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { Container } from "@/types/ipc";
import { ColumnResizeHandle } from "../shared/ColumnResizeHandle";
import { ContainerLogPanel } from "./ContainerLogPanel";
import { ContainerStatusBadge } from "./ContainerStatusBadge";
import { ResourceGraph } from "./ResourceGraph";

export type ResizableColumn = "name" | "image";

interface ContainerRowProps {
  container: Container;
  isExpanded: boolean;
  onToggleExpanded: (container: Container) => void;
  onRequestDelete: (container: Container) => void;
  columnWidths: Record<ResizableColumn, number>;
  onResizeColumn: (column: ResizableColumn, width: number) => void;
}

function formatUptime(seconds: number | null): string {
  if (seconds === null) return "—";
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return hrs > 0 ? `${hrs}h ${mins}m` : `${mins}m`;
}

function formatBytes(bytes: number): string {
  const mib = bytes / 1024 / 1024;
  return mib >= 1024 ? `${(mib / 1024).toFixed(1)} GiB` : `${mib.toFixed(0)} MiB`;
}

/**
 * One container's list row: start/stop via independent `useAsyncAction`s, a live
 * CPU/memory summary fed by `useContainers`' `container-stats` merge, and delete/view-logs
 * actions. Clicking the row (or its "view logs" icon) toggles an inline panel showing the
 * resource graph above the live log tail — one row expanded at a time, enforced by the
 * parent's single `expandedId`.
 */
export const ContainerRow: React.FC<ContainerRowProps> = ({
  container,
  isExpanded,
  onToggleExpanded,
  onRequestDelete,
  columnWidths,
  onResizeColumn,
}) => {
  const { mode } = useThemeMode();
  const startAction = useAsyncAction((profile: string, id: string) => startContainer(profile, id));
  const stopAction = useAsyncAction((profile: string, id: string) => stopContainer(profile, id));

  const isBusy = startAction.isPending || stopAction.isPending;
  const error = startAction.error ?? stopAction.error;
  const canStart =
    container.status === "stopped" ||
    container.status === "exited" ||
    container.status === "created";

  return (
    <Box
      sx={{
        p: 1.5,
        borderRadius: 2,
        cursor: "pointer",
        ...glassSurface(mode, 12),
        // `inset` keeps the selection ring entirely within the row's own border box — a
        // regular `outline` paints 2px *outside* the box, which near the left edge of the
        // scrollable list gets visually clipped by the sidebar's own painted surface.
        boxShadow: isExpanded ? `inset 0 0 0 2px ${PALETTE.warmSand}` : "none",
      }}
      onClick={() => onToggleExpanded(container)}
    >
      {/* `nowrap` + `overflowX: "auto"` instead of letting the action buttons wrap to a
          second line on a narrow window — a horizontal scrollbar on just this row reads as
          "modern app" instead of "broken layout". */}
      <Stack
        direction="row"
        spacing={1.5}
        sx={{ alignItems: "center", flexWrap: "nowrap", overflowX: "auto", pb: 0.5 }}
      >
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ alignItems: "center", flex: `0 0 ${columnWidths.name}px`, minWidth: 0 }}
        >
          <Typography noWrap sx={{ fontWeight: 600, flex: 1, minWidth: 0 }}>
            {container.name}
          </Typography>
          <ColumnResizeHandle
            width={columnWidths.name}
            onResize={(width) => onResizeColumn("name", width)}
          />
        </Stack>
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ alignItems: "center", flex: `0 0 ${columnWidths.image}px`, minWidth: 0 }}
        >
          <Typography noWrap variant="body2" color="text.secondary" sx={{ flex: 1, minWidth: 0 }}>
            {container.image}
          </Typography>
          <ColumnResizeHandle
            width={columnWidths.image}
            onResize={(width) => onResizeColumn("image", width)}
          />
        </Stack>
        <ContainerStatusBadge status={container.status} />
        <Chip label={container.profileName} size="small" variant="outlined" />
        <Typography variant="body2" color="text.secondary">
          {formatUptime(container.uptimeSeconds)}
        </Typography>

        <Box sx={{ flex: 1 }} />

        {container.latestStats && (
          <Typography noWrap variant="body2" color="text.secondary" sx={{ flex: "0 1 170px" }}>
            {container.latestStats.cpuPercent.toFixed(1)}% CPU ·{" "}
            {formatBytes(container.latestStats.memoryBytes)}
          </Typography>
        )}

        {canStart ? (
          <Button
            size="small"
            disabled={isBusy}
            onClick={(event) => {
              event.stopPropagation();
              startAction.run(container.profileName, container.id);
            }}
          >
            Start
          </Button>
        ) : (
          <Button
            size="small"
            disabled={isBusy}
            onClick={(event) => {
              event.stopPropagation();
              stopAction.run(container.profileName, container.id);
            }}
          >
            Stop
          </Button>
        )}

        <IconButton
          aria-label={`View logs for ${container.name}`}
          size="small"
          onClick={(event) => {
            event.stopPropagation();
            onToggleExpanded(container);
          }}
        >
          <ArticleIcon fontSize="small" />
        </IconButton>
        <IconButton
          aria-label={`Delete ${container.name}`}
          size="small"
          onClick={(event) => {
            event.stopPropagation();
            onRequestDelete(container);
          }}
        >
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Stack>

      {error && (
        <Typography variant="caption" sx={{ color: PALETTE.mutedMauve, display: "block", mt: 0.5 }}>
          {error.message}
        </Typography>
      )}

      {isExpanded && (
        <Stack spacing={1} sx={{ mt: 1 }} onClick={(event) => event.stopPropagation()}>
          <ResourceGraph containerId={container.id} />
          <ContainerLogPanel
            containerId={container.id}
            containerName={container.name}
            profileName={container.profileName}
          />
        </Stack>
      )}
    </Box>
  );
};
