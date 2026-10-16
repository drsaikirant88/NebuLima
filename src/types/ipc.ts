// The frontend's view of the Tauri IPC surface. `src/services/ipc.ts` is the only
// module allowed to import these types alongside `@tauri-apps/api/core`.

export interface IpcError {
  code: string;
  message: string;
  detail?: string;
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

export type ProfileState = "stopped" | "starting" | "running" | "stopping" | "error";
export type ContainerRuntime = "docker" | "containerd";

export interface NetworkConfig {
  dns: string[];
  ipAddress: string | null;
}

export interface Mount {
  hostPath: string;
  guestPath: string;
  writable: boolean;
}

export type CaCertMode = "none" | "file" | "systemBundle";

export interface CaCertConfig {
  mode: CaCertMode;
  /** Only meaningful when `mode === "file"`. */
  certPath?: string;
}

export interface ColimaProfile {
  name: string;
  state: ProfileState;
  errorReason?: string | null;
  cpu: number;
  memoryGiB: number;
  diskGiB: number;
  runtime: ContainerRuntime;
  kubernetesEnabled: boolean;
  network: NetworkConfig;
  mounts: Mount[];
  caCert: CaCertConfig;
  dockerSocketPath: string | null;
  isActive: boolean;
}

export interface CreateProfileInput {
  name: string;
  cpu: number;
  memoryGiB: number;
  diskGiB: number;
  runtime: ContainerRuntime;
  kubernetesEnabled: boolean;
  network: NetworkConfig;
  mounts: Mount[];
  caCert?: CaCertConfig;
  /** When `false` (the default), the newly created profile is stopped once configured, as
   * before; `true` leaves it running. */
  keepActive: boolean;
}

export type UpdateProfileInput = Partial<Omit<CreateProfileInput, "name">> & { name: string };

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

export type ContainerStatus = "created" | "running" | "paused" | "stopped" | "exited";

export interface ContainerPort {
  containerPort: number;
  hostPort: number | null;
  protocol: "tcp" | "udp";
}

export interface ResourceUsageSample {
  containerId: string;
  cpuPercent: number;
  memoryBytes: number;
  sampledAt: string;
}

export interface ContainerLatestStats {
  cpuPercent: number;
  memoryBytes: number;
  memoryLimitBytes: number;
  sampledAt: string;
}

export interface Container {
  id: string;
  name: string;
  image: string;
  status: ContainerStatus;
  uptimeSeconds: number | null;
  ports: ContainerPort[];
  latestStats: ContainerLatestStats | null;
  profileName: string;
}

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

export interface Image {
  id: string;
  repository: string | null;
  tag: string | null;
  sizeBytes: number;
  createdAt: string;
  inUse: boolean;
  profileName: string;
}

export interface ImageLayer {
  digest: string;
  sizeBytes: number;
}

export interface ImageInspectDetail {
  id: string;
  env: string[];
  cmd: string[] | null;
  entrypoint: string[] | null;
  workingDir: string | null;
  labels: Record<string, string>;
  exposedPorts: string[];
  layers: ImageLayer[];
}

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

export interface Volume {
  name: string;
  sizeBytes: number | null;
  attachedContainerNames: string[];
  profileName: string;
}

export interface PruneVolumesResult {
  removedNames: string[];
}

// ---------------------------------------------------------------------------
// Resource profile filter (shared across Containers/Images/Volumes views)
// ---------------------------------------------------------------------------

export interface ResourceProfileFilter {
  mode: "all" | "single";
  profileName: string | null;
}

// ---------------------------------------------------------------------------
// Kubernetes
// ---------------------------------------------------------------------------

export type KubernetesClusterState =
  | "unavailable"
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "error";

export type KubernetesClusterAction = "none" | "starting" | "stopping" | "resetting" | "deleting";

export interface ClusterNode {
  name: string;
  ready: boolean;
}

export type ClusterPodPhase = "Pending" | "Running" | "Succeeded" | "Failed" | "Unknown";

export interface ClusterPod {
  name: string;
  namespace: string;
  phase: ClusterPodPhase;
}

export interface KubernetesCluster {
  profileName: string;
  state: KubernetesClusterState;
  nodes: ClusterNode[];
  actionInProgress: KubernetesClusterAction;
  lastError: string | null;
}

// ---------------------------------------------------------------------------
// Settings & diagnostics
// ---------------------------------------------------------------------------

export type ThemeMode = "dark" | "light";

export interface NebuLimaSettings {
  themeMode: ThemeMode;
  lastActiveProfileName: string | null;
}

export interface SettingsPatch {
  themeMode?: ThemeMode;
  lastActiveProfileName?: string | null;
}

export interface DiagnosticLogPath {
  path: string;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

export interface ContainersChangedEvent {
  profileName: string;
  items: Container[];
}

export interface ImagesChangedEvent {
  profileName: string;
  items: Image[];
}

export interface VolumesChangedEvent {
  profileName: string;
  items: Volume[];
}

export interface KubernetesClusterChangedEvent {
  profileName: string;
  cluster: KubernetesCluster;
}

export interface ContainerStatsEvent {
  containerId: string;
  sample: ResourceUsageSample;
}

export interface ContainerLogLineEvent {
  containerId: string;
  stream: "stdout" | "stderr";
  line: string;
  timestamp: string;
}

export interface DiagnosticLogWrittenEvent {
  level: "info" | "warn" | "error";
  message: string;
}

export interface CertTrustWarningEvent {
  profileName: string;
  message: string;
  severity: "success" | "error";
}
