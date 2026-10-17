import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import type React from "react";
import type { Image } from "@/types/ipc";

interface DeleteImageDialogProps {
  image: Image | null;
  isSubmitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Delete-confirmation dialog: shown only after an unconfirmed `delete_image` call comes
 * back with `CONFIRMATION_REQUIRED` (the image is referenced by a current container) —
 * mirrors `DeleteProfileDialog`'s conditional pattern, not `DeleteContainerDialog`'s
 * unconditional one.
 */
export const DeleteImageDialog: React.FC<DeleteImageDialogProps> = ({
  image,
  isSubmitting,
  onCancel,
  onConfirm,
}) => (
  <Dialog open={image !== null} onClose={onCancel}>
    <DialogTitle>Delete image "{image?.repository ?? image?.id}"?</DialogTitle>
    <DialogContent>
      <DialogContentText>
        This image is used by a container. Deleting it may break that container. This cannot be
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
