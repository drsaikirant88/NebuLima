import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Stack,
  Typography,
} from "@mui/material";
import type React from "react";
import type { ImageInspectDetail } from "@/types/ipc";

interface ImageInspectDialogProps {
  open: boolean;
  title: string;
  detail: ImageInspectDetail | null;
  isLoading: boolean;
  errorMessage?: string;
  onClose: () => void;
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <Stack direction="row" spacing={1}>
      <Typography variant="body2" sx={{ fontWeight: 600, minWidth: 100 }}>
        {label}
      </Typography>
      <Typography variant="body2" color="text.secondary" sx={{ wordBreak: "break-word" }}>
        {value}
      </Typography>
    </Stack>
  );
}

/**
 * Inspect dialog: renders the full `ImageInspectDetail` (env, cmd, entrypoint, working
 * dir, labels, exposed ports, and the best-effort reconstructed layer sizes).
 */
export const ImageInspectDialog: React.FC<ImageInspectDialogProps> = ({
  open,
  title,
  detail,
  isLoading,
  errorMessage,
  onClose,
}) => (
  <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
    <DialogTitle>{title}</DialogTitle>
    <DialogContent>
      {isLoading && <Typography variant="body2">Loading…</Typography>}
      {errorMessage && <Typography variant="body2">{errorMessage}</Typography>}
      {detail && (
        <Stack spacing={1}>
          <Field label="Cmd" value={detail.cmd?.join(" ") ?? "—"} />
          <Field label="Entrypoint" value={detail.entrypoint?.join(" ") ?? "—"} />
          <Field label="Working dir" value={detail.workingDir ?? "—"} />
          <Field label="Env" value={detail.env.length > 0 ? detail.env.join(", ") : "—"} />
          <Field
            label="Ports"
            value={detail.exposedPorts.length > 0 ? detail.exposedPorts.join(", ") : "—"}
          />
          <Field
            label="Labels"
            value={
              Object.keys(detail.labels).length > 0
                ? Object.entries(detail.labels)
                    .map(([key, value]) => `${key}=${value}`)
                    .join(", ")
                : "—"
            }
          />
          <Typography variant="body2" sx={{ fontWeight: 600, mt: 1 }}>
            Layers ({detail.layers.length})
          </Typography>
          {detail.layers.length === 0 && (
            <Typography variant="body2" color="text.secondary">
              No filesystem layers.
            </Typography>
          )}
          {detail.layers.map((layer) => (
            <Typography
              key={layer.digest}
              variant="caption"
              color="text.secondary"
              sx={{ wordBreak: "break-all" }}
            >
              {layer.digest} — {(layer.sizeBytes / 1024 / 1024).toFixed(1)} MiB
            </Typography>
          ))}
        </Stack>
      )}
    </DialogContent>
    <DialogActions>
      <Button onClick={onClose}>Close</Button>
    </DialogActions>
  </Dialog>
);
