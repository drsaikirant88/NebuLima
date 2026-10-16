import {
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControlLabel,
} from "@mui/material";
import type React from "react";
import { useState } from "react";
import type { ColimaProfile } from "@/types/ipc";

interface DeleteProfileDialogProps {
  profile: ColimaProfile | null;
  isSubmitting: boolean;
  onCancel: () => void;
  onConfirm: (wipeAssets: boolean) => void;
}

/**
 * Delete-confirmation dialog, shown for every delete so it can always offer the
 * wipe-assets checkbox — `colima delete -d -f` additionally removes the profile's disk,
 * volumes, and images, which the default (registration-only) delete leaves behind.
 */
export const DeleteProfileDialog: React.FC<DeleteProfileDialogProps> = ({
  profile,
  isSubmitting,
  onCancel,
  onConfirm,
}) => {
  const [wipeAssets, setWipeAssets] = useState(false);

  return (
    <Dialog
      open={profile !== null}
      onClose={onCancel}
      slotProps={{ transition: { onExited: () => setWipeAssets(false) } }}
    >
      <DialogTitle>Delete profile "{profile?.name}"?</DialogTitle>
      <DialogContent>
        <DialogContentText>
          This will stop and remove the profile. This cannot be undone.
        </DialogContentText>
        <FormControlLabel
          control={
            <Checkbox
              checked={wipeAssets}
              onChange={(event) => setWipeAssets(event.target.checked)}
            />
          }
          label="Also delete all profile data (disks, volumes, images)"
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button
          color="inherit"
          variant="contained"
          disabled={isSubmitting}
          onClick={() => onConfirm(wipeAssets)}
        >
          Delete
        </Button>
      </DialogActions>
    </Dialog>
  );
};
