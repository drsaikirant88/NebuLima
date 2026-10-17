import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import type React from "react";
import type { Volume } from "@/types/ipc";

interface DeleteVolumeDialogProps {
  volume: Volume | null;
  isSubmitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Delete-confirmation dialog: shown only after an unconfirmed `delete_volume` call comes
 * back with `CONFIRMATION_REQUIRED` (the volume is attached to a current container) —
 * mirrors `DeleteImageDialog`'s conditional pattern.
 */
export const DeleteVolumeDialog: React.FC<DeleteVolumeDialogProps> = ({
  volume,
  isSubmitting,
  onCancel,
  onConfirm,
}) => (
  <Dialog open={volume !== null} onClose={onCancel}>
    <DialogTitle>Delete volume "{volume?.name}"?</DialogTitle>
    <DialogContent>
      <DialogContentText>
        This volume is attached to a container. Deleting it may break that container. This cannot be
        undone.
      </DialogContentText>
    </DialogContent>
    <DialogActions>
      <Button onClick={onCancel}>Cancel</Button>
      <Button color="inherit" variant="contained" disabled={isSubmitting} onClick={onConfirm}>
        Delete
      </Button>
    </DialogActions>
  </Dialog>
);
