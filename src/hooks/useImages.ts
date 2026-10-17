import { useEffect, useState } from "react";
import { listImages, onImagesChanged } from "@/services/ipc";
import type { Image, IpcError } from "@/types/ipc";

export interface UseImagesResult {
  images: Image[];
  isLoading: boolean;
  error: IpcError | null;
}

/**
 * Loads the current image list for `profile` (`null` = every running profile) and keeps
 * it live via the `images-changed` event, merging updates into the matching profile's slice.
 */
export function useImages(profile: string | null): UseImagesResult {
  const [images, setImages] = useState<Image[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    setIsLoading(true);
    setError(null);
    listImages(profile ?? undefined)
      .then((result) => {
        if (!cancelled) setImages(result);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err as IpcError);
          setImages([]);
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });

    onImagesChanged(({ profileName, items }) => {
      setImages((prev) => {
        if (profile !== null) {
          return profileName === profile ? items : prev;
        }
        return [...prev.filter((image) => image.profileName !== profileName), ...items];
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

  return { images, isLoading, error };
}
