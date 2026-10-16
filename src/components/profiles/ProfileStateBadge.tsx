import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import ErrorIcon from "@mui/icons-material/Error";
import HourglassTopIcon from "@mui/icons-material/HourglassTop";
import StopCircleIcon from "@mui/icons-material/StopCircle";
import { Chip } from "@mui/material";
import type React from "react";
import { PALETTE, withAlpha } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { ProfileState } from "@/types/ipc";

const LABELS: Record<ProfileState, string> = {
  stopped: "Stopped",
  starting: "Starting",
  running: "Running",
  stopping: "Stopping",
  error: "Error",
};

const ICONS: Record<ProfileState, React.ReactNode> = {
  stopped: <StopCircleIcon fontSize="small" />,
  starting: <HourglassTopIcon fontSize="small" />,
  running: <CheckCircleIcon fontSize="small" />,
  stopping: <HourglassTopIcon fontSize="small" />,
  error: <ErrorIcon fontSize="small" />,
};

/**
 * Lifecycle-state badge. The six-color palette has no distinct hues for success/warning/
 * error, so states are told apart by label + icon shape and an opacity ramp, never by
 * color alone.
 */
export const ProfileStateBadge: React.FC<{ state: ProfileState }> = ({ state }) => {
  const { mode } = useThemeMode();
  const base = mode === "dark" ? PALETTE.warmSand : PALETTE.mutedMauve;
  const emphasis = state === "running" || state === "error" ? 0.28 : 0.14;

  return (
    <Chip
      size="small"
      icon={ICONS[state] as React.ReactElement}
      label={LABELS[state]}
      sx={{
        backgroundColor: withAlpha(base, emphasis),
        color: mode === "dark" ? PALETTE.white : PALETTE.deepInk,
        fontWeight: state === "error" ? 700 : 500,
        "& .MuiChip-icon": { color: "inherit" },
      }}
    />
  );
};
