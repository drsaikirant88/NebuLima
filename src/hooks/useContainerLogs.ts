import { useEffect, useRef, useState } from "react";
import {
  onContainerLogLine,
  subscribeContainerLogs,
  unsubscribeContainerLogs,
} from "@/services/ipc";
import type { ContainerLogLineEvent } from "@/types/ipc";

/** Bounds the client-side log buffer so a chatty container can't grow memory unboundedly
 * while the log pane stays open. */
export const LOG_LINES_LIMIT = 2000;

/** A received log line tagged with a monotonically increasing `seq`, stable across the
 * `.slice(-LOG_LINES_LIMIT)` truncation below (which otherwise shifts every line's array
 * index once the buffer is full) — React list keys must stay stable even as old lines drop. */
export type ContainerLogLine = ContainerLogLineEvent & { seq: number };

export interface UseContainerLogsResult {
  lines: ContainerLogLine[];
}

/**
 * Subscribes to a single container's log stream for as long as `containerId` is non-null,
 * swapping the backend subscription cleanly whenever the viewed container changes so
 * switching never leaves a stale subscription running.
 */
export function useContainerLogs(
  profileName: string | null,
  containerId: string | null
): UseContainerLogsResult {
  const [lines, setLines] = useState<ContainerLogLine[]>([]);
  const nextSeq = useRef(0);

  useEffect(() => {
    setLines([]);
    nextSeq.current = 0;
    if (!profileName || !containerId) return undefined;

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    subscribeContainerLogs(profileName, containerId).catch(() => undefined);

    onContainerLogLine((payload) => {
      if (payload.containerId !== containerId) return;
      const seq = nextSeq.current++;
      setLines((prev) => [...prev, { ...payload, seq }].slice(-LOG_LINES_LIMIT));
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
      unsubscribeContainerLogs(profileName, containerId).catch(() => undefined);
    };
  }, [profileName, containerId]);

  return { lines };
}
