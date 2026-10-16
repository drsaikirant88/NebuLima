import type {
  CaCertConfig,
  ColimaProfile,
  ContainerRuntime,
  CreateProfileInput,
  Mount,
} from "@/types/ipc";

export interface ProfileFormValue {
  name: string;
  cpu: number;
  memoryGiB: number;
  diskGiB: number;
  runtime: ContainerRuntime;
  kubernetesEnabled: boolean;
  dns: string;
  ipAddress: string;
  mounts: Mount[];
  caCert: CaCertConfig;
  /** Only meaningful on create (ignored/not shown on edit). */
  keepActive: boolean;
}

export function toFormValue(profile: ColimaProfile | null): ProfileFormValue {
  if (!profile) {
    return {
      name: "",
      cpu: 2,
      memoryGiB: 2,
      diskGiB: 60,
      runtime: "docker",
      kubernetesEnabled: false,
      dns: "",
      ipAddress: "",
      mounts: [],
      caCert: { mode: "none" },
      keepActive: false,
    };
  }
  return {
    name: profile.name,
    cpu: profile.cpu,
    memoryGiB: profile.memoryGiB,
    diskGiB: profile.diskGiB,
    runtime: profile.runtime,
    kubernetesEnabled: profile.kubernetesEnabled,
    dns: profile.network.dns.join(", "),
    ipAddress: profile.network.ipAddress ?? "",
    mounts: profile.mounts,
    caCert: profile.caCert,
    keepActive: false,
  };
}

// Mirrors the backend's `validate_profile_name` (src-tauri/src/commands/profiles.rs):
// a name starting with `-` would be parsed by colima's own CLI as a flag instead of a
// profile name, so this must stay in sync with that check.
export const PROFILE_NAME_PATTERN = /^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,62}$/;

export function formValueToCreateInput(value: ProfileFormValue): CreateProfileInput {
  return {
    name: value.name.trim().toLowerCase(),
    cpu: value.cpu,
    memoryGiB: value.memoryGiB,
    diskGiB: value.diskGiB,
    runtime: value.runtime,
    kubernetesEnabled: value.kubernetesEnabled,
    network: {
      dns: value.dns
        .split(",")
        .map((d) => d.trim())
        .filter(Boolean),
      ipAddress: value.ipAddress.trim() || null,
    },
    mounts: value.mounts,
    caCert: value.caCert,
    keepActive: value.keepActive,
  };
}
