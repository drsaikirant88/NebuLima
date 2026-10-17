import { useCallback, useEffect, useState } from "react";
import {
  deleteCluster as deleteClusterIpc,
  getClusterState,
  onKubernetesClusterChanged,
  resetCluster as resetClusterIpc,
  startCluster as startClusterIpc,
  stopCluster as stopClusterIpc,
} from "@/services/ipc";
import type { IpcError, KubernetesCluster } from "@/types/ipc";

export interface UseKubernetesClusterResult {
  cluster: KubernetesCluster | null;
  isLoading: boolean;
  error: IpcError | null;
  /** Surfaces a failed start/stop/reset/delete call separately from `error` (the initial
   * query failure), so an action failure doesn't get silently overwritten the next time the
   * query re-runs. */
  actionError: IpcError | null;
  startCluster: () => Promise<void>;
  stopCluster: () => Promise<void>;
  resetCluster: () => Promise<void>;
  deleteCluster: () => Promise<void>;
}

/**
 * Loads the current `KubernetesCluster` snapshot for `profileName` and keeps it live via
 * `kubernetes-cluster-changed` — never polls itself, since the backend's own
 * lifecycle-action completions and safety-net poll are the sole sources of that event.
 * `enabled` gates both the initial fetch and the event subscription so a
 * Kubernetes-disabled profile never attempts a cluster state query, matching
 * `get_cluster_state`'s own short-circuit on the backend.
 */
export function useKubernetesCluster(
  profileName: string,
  enabled: boolean
): UseKubernetesClusterResult {
  const [cluster, setCluster] = useState<KubernetesCluster | null>(null);
  const [isLoading, setIsLoading] = useState(enabled);
  const [error, setError] = useState<IpcError | null>(null);
  const [actionError, setActionError] = useState<IpcError | null>(null);

  useEffect(() => {
    if (!enabled) {
      setCluster(null);
      setIsLoading(false);
      setError(null);
      return undefined;
    }

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    setIsLoading(true);
    setError(null);
    getClusterState(profileName)
      .then((result) => {
        if (!cancelled) setCluster(result);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err as IpcError);
          setCluster(null);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    onKubernetesClusterChanged(({ profileName: changedProfile, cluster: changedCluster }) => {
      if (changedProfile === profileName) setCluster(changedCluster);
    })
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
  }, [profileName, enabled]);

  // The backend already guards against a second concurrent action
  // (`KUBERNETES_ACTION_IN_PROGRESS`) — this check just avoids an avoidable round trip
  // for a call that is guaranteed to be rejected while one is already in flight.
  const runAction = useCallback(
    async (action: (profile: string) => Promise<void>) => {
      if (cluster && cluster.actionInProgress !== "none") return;
      setActionError(null);
      try {
        await action(profileName);
      } catch (err) {
        setActionError(err as IpcError);
      }
    },
    [cluster, profileName]
  );

  return {
    cluster,
    isLoading,
    error,
    actionError,
    startCluster: () => runAction(startClusterIpc),
    stopCluster: () => runAction(stopClusterIpc),
    resetCluster: () => runAction((profile) => resetClusterIpc(profile, true)),
    deleteCluster: () => runAction((profile) => deleteClusterIpc(profile, true)),
  };
}
