import { Box, Button, Stack, Typography } from "@mui/material";
import type React from "react";
import { useState } from "react";
import { useAsyncAction } from "@/hooks/useAsyncAction";
import { useResourceProfileFilter } from "@/hooks/useResourceProfileFilter";
import { useVolumes } from "@/hooks/useVolumes";
import { deleteVolume, pruneVolumes } from "@/services/ipc";
import { PALETTE } from "@/theme/palette";
import type { IpcError, Volume } from "@/types/ipc";
import { ResourceProfileFilter } from "../shared/ResourceProfileFilter";
import { DeleteVolumeDialog } from "./DeleteVolumeDialog";
import { VolumeRow } from "./VolumeRow";

/**
 * Volumes view: a live list, a bulk-prune action for every unattached volume, and a
 * delete-confirmation flow. `delete_volume` is always attempted unconfirmed first; the
 * confirmation dialog only appears when the backend reports `CONFIRMATION_REQUIRED` (the
 * volume is attached to a current container). The "Profile" filter scopes the list to one
 * profile or every running profile; `prune_volumes` prunes across every running profile
 * when the filter is "All profiles" — an unattached volume is wasted disk regardless of
 * which profile owns it.
 */
interface VolumesViewProps {
  /** Pre-selects the "Profile" filter — set when navigating here from the Profiles
   * view's "view resources" action rather than the sidebar. */
  initialProfileFilter?: string | null;
}

export const VolumesView: React.FC<VolumesViewProps> = ({ initialProfileFilter = null }) => {
  const { profiles, profileName, setProfileName } = useResourceProfileFilter(initialProfileFilter);
  const { volumes, isLoading, error } = useVolumes(profileName);
  const pruneAction = useAsyncAction((profile: string | null) => pruneVolumes(profile));
  const [nameWidth, setNameWidth] = useState(220);

  const [deleteTarget, setDeleteTarget] = useState<Volume | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<IpcError | null>(null);

  const requestDelete = async (volume: Volume) => {
    setDeleteError(null);
    try {
      await deleteVolume(volume.profileName, volume.name, false);
    } catch (err) {
      const ipcError = err as IpcError;
      if (ipcError.code === "CONFIRMATION_REQUIRED") {
        setDeleteTarget(volume);
      } else {
        setDeleteError(ipcError);
      }
    }
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteVolume(deleteTarget.profileName, deleteTarget.name, true);
      setDeleteTarget(null);
    } catch (err) {
      setDeleteError(err as IpcError);
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Box>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center", mb: 2 }}>
        <Typography variant="h6">Volumes</Typography>
        <Box sx={{ flex: 1 }} />
        <ResourceProfileFilter profiles={profiles} value={profileName} onChange={setProfileName} />
        <Button
          size="small"
          disabled={pruneAction.isPending}
          onClick={() => pruneAction.run(profileName)}
        >
          Prune unattached volumes
        </Button>
      </Stack>

      {pruneAction.error && (
        <Typography variant="body2" sx={{ color: PALETTE.mutedMauve, mb: 1 }}>
          {pruneAction.error.message}
        </Typography>
      )}
      {pruneAction.result && (
        <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
          {pruneAction.result.removedNames.length > 0
            ? `Removed: ${pruneAction.result.removedNames.join(", ")}`
            : "No unattached volumes to remove."}
        </Typography>
      )}
      {deleteError && (
        <Typography variant="body2" sx={{ color: PALETTE.mutedMauve, mb: 1 }}>
          {deleteError.message}
        </Typography>
      )}

      {!isLoading && error && (
        <Typography variant="body2" color="text.secondary">
          {error.message}
        </Typography>
      )}

      {!isLoading && !error && volumes.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          No volumes found for the selected profile.
        </Typography>
      )}

      <Stack spacing={1}>
        {volumes.map((volume) => (
          <VolumeRow
            key={volume.name}
            volume={volume}
            nameWidth={nameWidth}
            onResizeName={setNameWidth}
            onRequestDelete={requestDelete}
          />
        ))}
      </Stack>

      <DeleteVolumeDialog
        volume={deleteTarget}
        isSubmitting={isDeleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={confirmDelete}
      />
    </Box>
  );
};
