import { useEffect, useState } from "react";
import { listProfiles, onProfilesChanged } from "@/services/ipc";
import type { ColimaProfile } from "@/types/ipc";

export interface UseProfilesResult {
  profiles: ColimaProfile[];
  isLoading: boolean;
}

/**
 * Loads the current profile list and keeps it live via the `profiles-changed` event —
 * every profile command and the backend's filesystem watch funnel through it, so this
 * hook never needs to poll.
 */
export function useProfiles(): UseProfilesResult {
  const [profiles, setProfiles] = useState<ColimaProfile[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    setIsLoading(true);
    listProfiles()
      .then((result) => {
        if (!cancelled) setProfiles(result);
      })
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    onProfilesChanged((result) => setProfiles(result))
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
  }, []);

  return { profiles, isLoading };
}
