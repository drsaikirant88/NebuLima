// The sole IPC boundary module: every `invoke()`/`listen()` call in the app goes through
// here, never directly from a component.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CertTrustWarningEvent,
  ClusterPod,
  ColimaProfile,
  Container,
  ContainerLogLineEvent,
  ContainersChangedEvent,
  ContainerStatsEvent,
  CreateProfileInput,
  DiagnosticLogPath,
  DiagnosticLogWrittenEvent,
  Image,
  ImageInspectDetail,
  ImagesChangedEvent,
  KubernetesCluster,
  KubernetesClusterChangedEvent,
  NebuLimaSettings,
  PruneVolumesResult,
  SettingsPatch,
  UpdateProfileInput,
  Volume,
  VolumesChangedEvent,
} from "@/types/ipc";

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

export const listProfiles = () => invoke<ColimaProfile[]>("list_profiles");
export const createProfile = (input: CreateProfileInput) =>
  invoke<ColimaProfile>("create_profile", { input });
export const updateProfile = (input: UpdateProfileInput) =>
  invoke<ColimaProfile>("update_profile", { input });
export const startProfile = (name: string) => invoke<void>("start_profile", { name });
export const stopProfile = (name: string) => invoke<void>("stop_profile", { name });
export const deleteProfile = (name: string, confirmed: boolean, wipeAssets: boolean) =>
  invoke<void>("delete_profile", { name, confirmed, wipeAssets });
export const setActiveProfile = (name: string) =>
  invoke<ColimaProfile>("set_active_profile", { name });
export const checkKubectlAvailable = () => invoke<boolean>("check_kubectl_available");

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

export const listContainers = (profile?: string) =>
  invoke<Container[]>("list_containers", { profile });
export const startContainer = (profile: string, id: string) =>
  invoke<void>("start_container", { profile, id });
export const stopContainer = (profile: string, id: string) =>
  invoke<void>("stop_container", { profile, id });
export const deleteContainer = (profile: string, id: string, confirmed: boolean) =>
  invoke<void>("delete_container", { profile, id, confirmed });
export const subscribeContainerLogs = (profile: string, id: string) =>
  invoke<void>("subscribe_container_logs", { profile, id });
export const unsubscribeContainerLogs = (profile: string, id: string) =>
  invoke<void>("unsubscribe_container_logs", { profile, id });

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

export const listImages = (profile?: string) => invoke<Image[]>("list_images", { profile });
export const inspectImage = (profile: string, id: string) =>
  invoke<ImageInspectDetail>("inspect_image", { profile, id });
export const deleteImage = (profile: string, id: string, confirmed: boolean) =>
  invoke<void>("delete_image", { profile, id, confirmed });

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

export const listVolumes = (profile?: string) => invoke<Volume[]>("list_volumes", { profile });
export const pruneVolumes = (profile: string | null) =>
  invoke<PruneVolumesResult>("prune_volumes", { profile });
export const deleteVolume = (profile: string, name: string, confirmed: boolean) =>
  invoke<void>("delete_volume", { profile, name, confirmed });

// ---------------------------------------------------------------------------
// Kubernetes
// ---------------------------------------------------------------------------

export const getClusterState = (profile: string) =>
  invoke<KubernetesCluster>("get_cluster_state", { profile });
export const startCluster = (profile: string) => invoke<void>("start_cluster", { profile });
export const stopCluster = (profile: string) => invoke<void>("stop_cluster", { profile });
export const resetCluster = (profile: string, confirmed: boolean) =>
  invoke<void>("reset_cluster", { profile, confirmed });
export const deleteCluster = (profile: string, confirmed: boolean) =>
  invoke<void>("delete_cluster", { profile, confirmed });
export const listClusterPods = (profile: string) =>
  invoke<ClusterPod[]>("list_cluster_pods", { profile });

// ---------------------------------------------------------------------------
// Settings & diagnostics
// ---------------------------------------------------------------------------

export const getSettings = () => invoke<NebuLimaSettings>("get_settings");
export const updateSettings = (patch: SettingsPatch) =>
  invoke<NebuLimaSettings>("update_settings", { patch });
export const getDiagnosticLogPath = () => invoke<DiagnosticLogPath>("get_diagnostic_log_path");

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

export const onProfilesChanged = (cb: (profiles: ColimaProfile[]) => void): Promise<UnlistenFn> =>
  listen<ColimaProfile[]>("profiles-changed", (event) => cb(event.payload));

export const onContainersChanged = (
  cb: (payload: ContainersChangedEvent) => void
): Promise<UnlistenFn> =>
  listen<ContainersChangedEvent>("containers-changed", (event) => cb(event.payload));

export const onImagesChanged = (cb: (payload: ImagesChangedEvent) => void): Promise<UnlistenFn> =>
  listen<ImagesChangedEvent>("images-changed", (event) => cb(event.payload));

export const onVolumesChanged = (cb: (payload: VolumesChangedEvent) => void): Promise<UnlistenFn> =>
  listen<VolumesChangedEvent>("volumes-changed", (event) => cb(event.payload));

export const onContainerStats = (cb: (payload: ContainerStatsEvent) => void): Promise<UnlistenFn> =>
  listen<ContainerStatsEvent>("container-stats", (event) => cb(event.payload));

export const onContainerLogLine = (
  cb: (payload: ContainerLogLineEvent) => void
): Promise<UnlistenFn> =>
  listen<ContainerLogLineEvent>("container-log-line", (event) => cb(event.payload));

export const onKubernetesClusterChanged = (
  cb: (payload: KubernetesClusterChangedEvent) => void
): Promise<UnlistenFn> =>
  listen<KubernetesClusterChangedEvent>("kubernetes-cluster-changed", (event) => cb(event.payload));

export const onDiagnosticLogWritten = (
  cb: (payload: DiagnosticLogWrittenEvent) => void
): Promise<UnlistenFn> =>
  listen<DiagnosticLogWrittenEvent>("diagnostic-log-written", (event) => cb(event.payload));

export const onCertTrustWarning = (
  cb: (payload: CertTrustWarningEvent) => void
): Promise<UnlistenFn> =>
  listen<CertTrustWarningEvent>("cert-trust-warning", (event) => cb(event.payload));
