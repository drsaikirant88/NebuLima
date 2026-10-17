import AddIcon from "@mui/icons-material/Add";
import { Box, Button, Stack, Typography } from "@mui/material";
import type React from "react";
import { useEffect, useState } from "react";
import { useProfiles } from "@/hooks/useProfiles";
import { useToast } from "@/hooks/useToast";
import { createProfile, deleteProfile, onCertTrustWarning, updateProfile } from "@/services/ipc";
import { PALETTE } from "@/theme/palette";
import type { ColimaProfile, IpcError } from "@/types/ipc";
import { DeleteProfileDialog } from "./DeleteProfileDialog";
import { ProfileFormDialog } from "./ProfileFormDialog";
import { ProfileRow } from "./ProfileRow";
import { formValueToCreateInput, type ProfileFormValue } from "./profileFormValue";

interface ProfilesViewProps {
  /** Called with a profile's name when its "view resources" action is clicked — the
   * caller is expected to navigate to the Containers view pre-filtered to it. */
  onViewResources?: (profileName: string) => void;
}

/**
 * Profiles view: list of profiles plus the create/edit form and a delete-confirmation
 * flow. The confirmation dialog opens for every delete so it can always offer the
 * wipe-assets checkbox; the backend's own `CONFIRMATION_REQUIRED` check still runs as
 * defense in depth since the dialog always passes `confirmed: true`.
 */
export const ProfilesView: React.FC<ProfilesViewProps> = ({ onViewResources }) => {
  const { profiles, isLoading } = useProfiles();
  const { showToast } = useToast();

  const [formOpen, setFormOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<ColimaProfile | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<IpcError | null>(null);

  const [deleteTarget, setDeleteTarget] = useState<ColimaProfile | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<IpcError | null>(null);
  // Wide enough to show a realistic long profile name (e.g. "ccs-forecasting-2ced3e1b-23425")
  // without truncating — Profiles has plenty of spare horizontal room, unlike the denser
  // Containers/Images/Volumes rows that default narrower.
  const [nameWidth, setNameWidth] = useState(280);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    onCertTrustWarning((payload) => showToast(payload.message, payload.severity))
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [showToast]);

  const openCreateForm = () => {
    setEditingProfile(null);
    setSaveError(null);
    setFormOpen(true);
  };

  const openEditForm = (profile: ColimaProfile) => {
    setEditingProfile(profile);
    setSaveError(null);
    setFormOpen(true);
  };

  const handleFormSubmit = async (value: ProfileFormValue) => {
    const input = formValueToCreateInput(value);

    if (editingProfile) {
      setIsSaving(true);
      setSaveError(null);
      try {
        await updateProfile(input);
        setFormOpen(false);
        showToast(`Profile "${input.name}" updated.`, "success");
      } catch (err) {
        setSaveError(err as IpcError);
      } finally {
        setIsSaving(false);
      }
      return;
    }

    // `colima start` + `colima stop` (the create primitive) can take a long time, so the
    // dialog closes as soon as creation is kicked off instead of blocking on it — the user
    // gets immediate feedback via toasts rather than a dialog that looks stuck.
    setSaveError(null);
    setFormOpen(false);
    showToast(`Creating profile "${input.name}"…`, "info");
    try {
      await createProfile(input);
      showToast(
        value.keepActive
          ? `Profile "${input.name}" created and running.`
          : `Profile "${input.name}" created, then stopped.`,
        "success"
      );
    } catch (err) {
      const ipcError = err as IpcError;
      showToast(`Failed to create profile "${input.name}": ${ipcError.message}`, "error");
    }
  };

  const requestDelete = (profile: ColimaProfile) => {
    setDeleteError(null);
    setDeleteTarget(profile);
  };

  const confirmDelete = async (wipeAssets: boolean) => {
    if (!deleteTarget) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteProfile(deleteTarget.name, true, wipeAssets);
      setDeleteTarget(null);
    } catch (err) {
      setDeleteError(err as IpcError);
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Box>
      <Stack direction="row" sx={{ justifyContent: "space-between", alignItems: "center", mb: 2 }}>
        <Typography variant="h6">Profiles</Typography>
        <Button startIcon={<AddIcon />} variant="contained" onClick={openCreateForm}>
          New profile
        </Button>
      </Stack>

      {deleteError && (
        <Typography variant="body2" sx={{ color: PALETTE.mutedMauve, mb: 1 }}>
          {deleteError.message}
        </Typography>
      )}

      {!isLoading && profiles.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          No Colima profiles yet. Create one to get started.
        </Typography>
      )}

      <Stack spacing={1}>
        {profiles.map((profile) => (
          <ProfileRow
            key={profile.name}
            profile={profile}
            nameWidth={nameWidth}
            onResizeName={setNameWidth}
            onEdit={openEditForm}
            onRequestDelete={requestDelete}
            onViewResources={(profileName) => onViewResources?.(profileName)}
          />
        ))}
      </Stack>

      <ProfileFormDialog
        open={formOpen}
        profile={editingProfile}
        isSubmitting={isSaving}
        errorMessage={saveError?.message}
        onCancel={() => setFormOpen(false)}
        onSubmit={handleFormSubmit}
      />

      <DeleteProfileDialog
        profile={deleteTarget}
        isSubmitting={isDeleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={confirmDelete}
      />
    </Box>
  );
};
