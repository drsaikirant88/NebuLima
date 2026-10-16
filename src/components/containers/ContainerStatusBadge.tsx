import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import ErrorIcon from "@mui/icons-material/Error";
import HourglassTopIcon from "@mui/icons-material/HourglassTop";
import PauseCircleIcon from "@mui/icons-material/PauseCircle";
import StopCircleIcon from "@mui/icons-material/StopCircle";
import { Chip } from "@mui/material";
import type React from "react";
import { PALETTE, withAlpha } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { ContainerStatus } from "@/types/ipc";

const LABELS: Record<ContainerStatus, string> = {
  created: "Created",
  running: "Running",
  paused: "Paused",
  stopped: "Stopped",
  exited: "Exited",
};

const ICONS: Record<ContainerStatus, React.ReactNode> = {
  created: <HourglassTopIcon fontSize="small" />,
  running: <CheckCircleIcon fontSize="small" />,
  paused: <PauseCircleIcon fontSize="small" />,
  stopped: <StopCircleIcon fontSize="small" />,
  exited: <ErrorIcon fontSize="small" />,
};

/**
 * Lifecycle-status badge. As with `ProfileStateBadge`, the locked six-color palette has
 * no distinct hues for success/warning/error, so states are told apart by label + icon
 * shape and an opacity ramp.
 */
export const ContainerStatusBadge: React.FC<{ status: ContainerStatus }> = ({ status }) => {
  const { mode } = useThemeMode();
  const base = mode === "dark" ? PALETTE.warmSand : PALETTE.mutedMauve;
  const emphasis = status === "running" || status === "exited" ? 0.28 : 0.14;

  return (
    <Chip
      size="small"
      icon={ICONS[status] as React.ReactElement}
      label={LABELS[status]}
      sx={{
        backgroundColor: withAlpha(base, emphasis),
        color: mode === "dark" ? PALETTE.white : PALETTE.deepInk,
        fontWeight: status === "exited" ? 700 : 500,
        "& .MuiChip-icon": { color: "inherit" },
      }}
    />
  );
};
