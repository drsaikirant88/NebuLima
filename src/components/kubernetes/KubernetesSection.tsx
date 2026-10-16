import { Stack, Typography } from "@mui/material";
import type React from "react";
import { useState } from "react";
import { ClusterLifecycleControls } from "@/components/kubernetes/ClusterLifecycleControls";
import { ClusterNodeList } from "@/components/kubernetes/ClusterNodeList";
import { ClusterPodList } from "@/components/kubernetes/ClusterPodList";
import {
  ConfirmClusterActionDialog,
  type ClusterActionKind,
} from "@/components/kubernetes/ConfirmClusterActionDialog";
import { useKubernetesCluster } from "@/hooks/useKubernetesCluster";
import type { ColimaProfile } from "@/types/ipc";

interface KubernetesSectionProps {
  profile: ColimaProfile;
}

/**
 * Renders the Kubernetes cluster state and lifecycle controls for a profile. Always
 * mounts so the not-enabled message is available even when `kubernetesEnabled` is
 * false — `enabled` is threaded into `useKubernetesCluster` so `get_cluster_state` is never
 * invoked in that case.
 */
export const KubernetesSection: React.FC<KubernetesSectionProps> = ({ profile }) => {
  const { cluster, error, actionError, startCluster, stopCluster, resetCluster, deleteCluster } =
    useKubernetesCluster(profile.name, profile.kubernetesEnabled);
  const [confirmAction, setConfirmAction] = useState<ClusterActionKind | null>(null);

  if (!profile.kubernetesEnabled) {
    return <Typography variant="caption">Kubernetes is not enabled for this profile.</Typography>;
  }

  if (!cluster || cluster.state === "unavailable") {
    return <Typography variant="caption">Kubernetes cluster is unavailable.</Typography>;
  }

  const handleConfirm = async () => {
    if (confirmAction === "reset") await resetCluster();
    if (confirmAction === "delete") await deleteCluster();
    setConfirmAction(null);
  };

  return (
    <Stack spacing={0.5}>
      <ClusterNodeList nodes={cluster.nodes} />
      <ClusterLifecycleControls
        cluster={cluster}
        onStart={startCluster}
        onStop={stopCluster}
        onRequestReset={() => setConfirmAction("reset")}
        onRequestDelete={() => setConfirmAction("delete")}
      />
      <ConfirmClusterActionDialog
        action={confirmAction}
        isSubmitting={cluster.actionInProgress !== "none"}
        onCancel={() => setConfirmAction(null)}
        onConfirm={handleConfirm}
      />
      <ClusterPodList profileName={profile.name} isClusterRunning={cluster.state === "running"} />
      {(error ?? actionError) ? (
        <Typography variant="caption">{(error ?? actionError)?.message}</Typography>
      ) : null}
    </Stack>
  );
};
