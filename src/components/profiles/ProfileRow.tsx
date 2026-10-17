import DeleteIcon from "@mui/icons-material/Delete";
import EditIcon from "@mui/icons-material/Edit";
import VisibilityIcon from "@mui/icons-material/Visibility";
import { Box, Button, Chip, IconButton, Stack, Typography } from "@mui/material";
import type React from "react";
import { KubernetesSection } from "@/components/kubernetes/KubernetesSection";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import { setActiveProfile, startProfile, stopProfile } from "@/services/ipc";
import { glassSurface } from "@/theme/glass";
import { PALETTE } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import type { ColimaProfile } from "@/types/ipc";
import { ColumnResizeHandle } from "../shared/ColumnResizeHandle";
import { ProfileStateBadge } from "./ProfileStateBadge";

interface ProfileRowProps {
  profile: ColimaProfile;
  nameWidth: number;
  onResizeName: (width: number) => void;
  onEdit: (profile: ColimaProfile) => void;
  onRequestDelete: (profile: ColimaProfile) => void;
  onViewResources: (profileName: string) => void;
}

/**
 * One profile's list row: start/stop/set-active, each driven through its own
 * `useAsyncAction` for independent pending/error feedback, plus edit/delete/view-resources
 * (a shortcut into the Containers/Images/Volumes views, pre-filtered to this profile). The
 * name column is user-resizable and truncates with an ellipsis instead of growing the row.
 */
export const ProfileRow: React.FC<ProfileRowProps> = ({
  profile,
  nameWidth,
  onResizeName,
  onEdit,
  onRequestDelete,
  onViewResources,
}) => {
  const { mode } = useThemeMode();
  const startAction = useAsyncAction((name: string) => startProfile(name));
  const stopAction = useAsyncAction((name: string) => stopProfile(name));
  const activateAction = useAsyncAction((name: string) => setActiveProfile(name));

  const isBusy = startAction.isPending || stopAction.isPending || activateAction.isPending;
  const error = startAction.error ?? stopAction.error ?? activateAction.error;

  return (
    <Box
      sx={{
        p: 1.5,
        borderRadius: 2,
        ...glassSurface(mode, 12),
      }}
    >
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
            {profile.name}
          </Typography>
          <ColumnResizeHandle width={nameWidth} onResize={onResizeName} max={600} />
        </Stack>
        <ProfileStateBadge state={profile.state} />
        {profile.isActive && <Chip size="small" label="Active" variant="outlined" />}

        <Box sx={{ flex: 1 }} />

        {profile.state === "stopped" && (
          <Button size="small" disabled={isBusy} onClick={() => startAction.run(profile.name)}>
            Start
          </Button>
        )}
        {profile.state === "running" && (
          <Button size="small" disabled={isBusy} onClick={() => stopAction.run(profile.name)}>
            Stop
          </Button>
        )}
        {!profile.isActive && (
          <Button size="small" disabled={isBusy} onClick={() => activateAction.run(profile.name)}>
            Set active
          </Button>
        )}

        <IconButton
          aria-label={`View resources for ${profile.name}`}
          size="small"
          onClick={() => onViewResources(profile.name)}
        >
          <VisibilityIcon fontSize="small" />
        </IconButton>
        <IconButton
          aria-label={`Edit ${profile.name}`}
          size="small"
          onClick={() => onEdit(profile)}
        >
          <EditIcon fontSize="small" />
        </IconButton>
        <IconButton
          aria-label={`Delete ${profile.name}`}
          size="small"
          onClick={() => onRequestDelete(profile)}
        >
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Stack>

      {profile.state === "error" && profile.errorReason && (
        <Typography variant="caption" sx={{ color: PALETTE.mutedMauve, display: "block", mt: 0.5 }}>
          {profile.errorReason}
        </Typography>
      )}
      {error && (
        <Typography variant="caption" sx={{ color: PALETTE.mutedMauve, display: "block", mt: 0.5 }}>
          {error.message}
        </Typography>
      )}
      {profile.kubernetesEnabled && (
        <Box sx={{ mt: 1 }}>
          <KubernetesSection profile={profile} />
        </Box>
      )}
    </Box>
  );
};
