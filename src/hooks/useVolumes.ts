import { useEffect, useState } from "react";
import { listVolumes, onVolumesChanged } from "@/services/ipc";
import type { IpcError, Volume } from "@/types/ipc";

export interface UseVolumesResult {
  volumes: Volume[];
  isLoading: boolean;
  error: IpcError | null;
}

/**
 * Loads the current volume list for `profile` (`null` = every running profile) and keeps
 * it live via the `volumes-changed` event — emitted both for this app's own prune/delete
 * calls and for changes made externally — merging updates into the matching profile's slice.
 */
export function useVolumes(profile: string | null): UseVolumesResult {
  const [volumes, setVolumes] = useState<Volume[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    setIsLoading(true);
    setError(null);
    listVolumes(profile ?? undefined)
      .then((result) => {
        if (!cancelled) setVolumes(result);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err as IpcError);
          setVolumes([]);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    onVolumesChanged(({ profileName, items }) => {
      setVolumes((prev) => {
        if (profile !== null) {
          return profileName === profile ? items : prev;
        }
        return [...prev.filter((volume) => volume.profileName !== profileName), ...items];
      });
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
  }, [profile]);

  return { volumes, isLoading, error };
}
