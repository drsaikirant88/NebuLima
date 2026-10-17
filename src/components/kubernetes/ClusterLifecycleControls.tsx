import { Button, Stack } from "@mui/material";
import type React from "react";
import type { KubernetesCluster } from "@/types/ipc";

interface ClusterLifecycleControlsProps {
  cluster: KubernetesCluster;
  onStart: () => void;
  onStop: () => void;
  onRequestReset: () => void;
  onRequestDelete: () => void;
}

/**
 * Start/stop/reset/delete controls for a Kubernetes cluster. All four buttons disable
 * together while `actionInProgress != 'none'` — the backend only ever has one lifecycle
 * action in flight per profile, so there is nothing a second click
 * could usefully do in that window besides race the first one.
 */
export const ClusterLifecycleControls: React.FC<ClusterLifecycleControlsProps> = ({
  cluster,
  onStart,
  onStop,
  onRequestReset,
  onRequestDelete,
}) => {
  const isBusy = cluster.actionInProgress !== "none";
  const canStop = cluster.state === "running" || cluster.state === "starting";

  return (
    <Stack direction="row" spacing={1}>
      {canStop ? (
        <Button size="small" disabled={isBusy} onClick={onStop}>
          Stop
        </Button>
      ) : (
        <Button size="small" disabled={isBusy} onClick={onStart}>
          Start
        </Button>
      )}
      <Button size="small" disabled={isBusy} onClick={onRequestReset}>
        Reset
      </Button>
      <Button size="small" disabled={isBusy} onClick={onRequestDelete}>
        Delete
      </Button>
    </Stack>
  );
};
