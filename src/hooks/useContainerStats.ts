import { useEffect, useState } from "react";
import { onContainerStats } from "@/services/ipc";
import type { ResourceUsageSample } from "@/types/ipc";

/** 1Hz `container-stats` samples over the last minute is enough history for a live
 * resource trend graph without the client-side buffer growing unbounded. */
export const STATS_HISTORY_LIMIT = 60;

export interface UseContainerStatsResult {
  history: ResourceUsageSample[];
}

/**
 * Bounds a rolling window of `container-stats` samples for a single container, for
 * `ResourceGraph`. Resets the window whenever `containerId` changes, so switching the
 * viewed container never shows a stale trend line.
 */
export function useContainerStats(containerId: string | null): UseContainerStatsResult {
  const [history, setHistory] = useState<ResourceUsageSample[]>([]);

  useEffect(() => {
    setHistory([]);
    if (!containerId) return undefined;

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    onContainerStats((payload) => {
      if (payload.containerId !== containerId) return;
      setHistory((prev) => [...prev, payload.sample].slice(-STATS_HISTORY_LIMIT));
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
  }, [containerId]);

  return { history };
}
