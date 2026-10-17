import { useEffect, useState } from "react";
import { listContainers, onContainerStats, onContainersChanged } from "@/services/ipc";
import type { Container, IpcError } from "@/types/ipc";

export interface UseContainersResult {
  containers: Container[];
  isLoading: boolean;
  error: IpcError | null;
}

/**
 * Loads the current container list for `profile` (`null` = every running profile) and
 * keeps it live via the `containers-changed` and `container-stats` events, merging
 * updates into the matching rows without a full re-fetch.
 */
export function useContainers(profile: string | null): UseContainersResult {
  const [containers, setContainers] = useState<Container[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    let unlistenChanged: (() => void) | undefined;
    let unlistenStats: (() => void) | undefined;
    let cancelled = false;

    setIsLoading(true);
    setError(null);
    listContainers(profile ?? undefined)
      .then((result) => {
        if (!cancelled) setContainers(result);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err as IpcError);
          setContainers([]);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    onContainersChanged(({ profileName, items }) => {
      setContainers((prev) => {
        if (profile !== null) {
          return profileName === profile ? items : prev;
        }
        return [...prev.filter((container) => container.profileName !== profileName), ...items];
      });
    })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlistenChanged = fn;
        }
      })
      .catch(() => undefined);

    onContainerStats(({ containerId, sample }) => {
      setContainers((prev) =>
        prev.map((container) =>
          container.id === containerId
            ? {
                ...container,
                latestStats: {
                  cpuPercent: sample.cpuPercent,
                  memoryBytes: sample.memoryBytes,
                  memoryLimitBytes: container.latestStats?.memoryLimitBytes ?? 0,
                  sampledAt: sample.sampledAt,
                },
              }
            : container
        )
      );
    })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlistenStats = fn;
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      unlistenChanged?.();
      unlistenStats?.();
    };
  }, [profile]);

  return { containers, isLoading, error };
}
