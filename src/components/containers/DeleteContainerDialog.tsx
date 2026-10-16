import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import type React from "react";
import type { Container } from "@/types/ipc";

interface DeleteContainerDialogProps {
  container: Container | null;
  isSubmitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Delete-confirmation dialog. Unlike profiles/images/volumes, container deletion
 * requires confirmation unconditionally, so — unlike `DeleteProfileDialog` — this is shown
 * for every delete request rather than only after a `CONFIRMATION_REQUIRED` response.
 */
export const DeleteContainerDialog: React.FC<DeleteContainerDialogProps> = ({
  container,
  isSubmitting,
  onCancel,
  onConfirm,
}) => (
  <Dialog open={container !== null} onClose={onCancel}>
    <DialogTitle>Delete container "{container?.name}"?</DialogTitle>
    <DialogContent>
      <DialogContentText>
        This will permanently remove the container. This cannot be undone.
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
