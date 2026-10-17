import { useEffect, useState } from "react";
import { useProfiles } from "./useProfiles";
import type { ColimaProfile } from "@/types/ipc";

export interface UseResourceProfileFilterResult {
  mode: "all" | "single";
  profileName: string | null;
  profiles: ColimaProfile[];
  setProfileName: (profileName: string | null) => void;
}

/**
 * Backs the Containers/Images/Volumes views' "Profile" filter: `profileName: null` means
 * "all profiles". Resets to `null` whenever the currently-selected profile disappears from
 * `profiles-changed` — e.g. the profile was deleted — so the view never gets stuck
 * filtered to a profile that no longer exists. `initialProfileName` seeds the initial
 * selection, letting the Profiles view's "view resources" action hand a target profile down
 * pre-filtered.
 */
export function useResourceProfileFilter(
  initialProfileName: string | null = null
): UseResourceProfileFilterResult {
  const { profiles, isLoading } = useProfiles();
  const [profileName, setProfileName] = useState<string | null>(initialProfileName);

  useEffect(() => {
    // Skip while `useProfiles()` hasn't resolved its first snapshot yet — its initial
    // empty array would otherwise look identical to "the selected profile disappeared"
    // and wipe out an `initialProfileName` seeded from navigation before it ever renders.
    if (isLoading) return;
    if (profileName !== null && !profiles.some((profile) => profile.name === profileName)) {
      setProfileName(null);
    }
  }, [profiles, profileName, isLoading]);

  return {
    mode: profileName === null ? "all" : "single",
    profileName,
    profiles,
    setProfileName,
  };
}
