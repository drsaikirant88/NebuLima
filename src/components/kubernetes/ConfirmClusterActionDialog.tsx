import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
} from "@mui/material";
import type React from "react";

export type ClusterActionKind = "reset" | "delete";

interface ConfirmClusterActionDialogProps {
  action: ClusterActionKind | null;
  isSubmitting: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

const COPY: Record<ClusterActionKind, { title: string; body: string; confirmLabel: string }> = {
  reset: {
    title: "Reset this Kubernetes cluster?",
    body: "This recreates the embedded k3s cluster from scratch. Anything deployed to it will be lost. This cannot be undone.",
    confirmLabel: "Reset",
  },
  delete: {
    title: "Delete this Kubernetes cluster?",
    body: "This permanently removes the embedded k3s cluster. Anything deployed to it will be lost. This cannot be undone.",
    confirmLabel: "Delete",
  },
};

/**
 * `reset_cluster`/`delete_cluster` require `confirmed: true` unconditionally — unlike
 * `DeleteVolumeDialog`'s "ask only if the backend says so" pattern, every reset/delete click
 * is guaranteed to need confirmation, so this dialog is always shown before either command
 * is called at all.
 */
export const ConfirmClusterActionDialog: React.FC<ConfirmClusterActionDialogProps> = ({
  action,
  isSubmitting,
  onCancel,
  onConfirm,
}) => {
  const copy = action ? COPY[action] : null;
  return (
    <Dialog open={action !== null} onClose={onCancel}>
      <DialogTitle>{copy?.title}</DialogTitle>
      <DialogContent>
        <DialogContentText>{copy?.body}</DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button color="inherit" variant="contained" disabled={isSubmitting} onClick={onConfirm}>
          {copy?.confirmLabel}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
