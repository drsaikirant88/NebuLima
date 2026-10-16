import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { KubernetesSection } from "../../src/components/kubernetes/KubernetesSection";
import { ThemeModeProvider } from "../../src/theme/ThemeModeProvider";
import type { ColimaProfile, KubernetesCluster } from "../../src/types/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

function buildProfile(overrides: Partial<ColimaProfile> = {}): ColimaProfile {
  return {
    name: "default",
    state: "running",
    cpu: 2,
    memoryGiB: 2,
    diskGiB: 60,
    runtime: "docker",
    kubernetesEnabled: true,
    network: { dns: [], ipAddress: null },
    mounts: [],
    caCert: { mode: "none" as const },
    dockerSocketPath: null,
    isActive: false,
    ...overrides,
  };
}

function buildCluster(overrides: Partial<KubernetesCluster> = {}): KubernetesCluster {
  return {
    profileName: "default",
    state: "unavailable",
    nodes: [],
    actionInProgress: "none",
    lastError: null,
    ...overrides,
  };
}

function renderSection(profile: ColimaProfile) {
  return render(
    <ThemeModeProvider>
      <KubernetesSection profile={profile} />
    </ThemeModeProvider>
  );
}

describe("KubernetesSection", () => {
  beforeEach(() => {
    mockedListen.mockResolvedValue(() => undefined);
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      throw new Error(`unexpected invoke: ${cmd}`);
    });
  });

  it("shows a not-enabled message and never queries cluster state when Kubernetes is disabled", async () => {
    renderSection(buildProfile({ kubernetesEnabled: false, state: "stopped" }));

    expect(await screen.findByText(/not enabled/i)).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("get_cluster_state", expect.anything());
  });

  it("shows an unavailable message for a Kubernetes-enabled profile that isn't running", async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      if (cmd === "get_cluster_state") return buildCluster({ state: "unavailable" });
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    renderSection(buildProfile({ kubernetesEnabled: true, state: "stopped" }));

    expect(await screen.findByText(/unavailable/i)).toBeInTheDocument();
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("get_cluster_state", { profile: "default" })
    );
  });

  it("renders node readiness once the cluster is running", async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      if (cmd === "get_cluster_state") {
        return buildCluster({
          state: "running",
          nodes: [{ name: "colima", ready: true }],
        });
      }
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    renderSection(buildProfile({ kubernetesEnabled: true, state: "running" }));

    expect(await screen.findByText("colima")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
  });

  // Reset/delete always route through a confirmation dialog before the IPC command is
  // ever called — `reset_cluster`/`delete_cluster` require `confirmed: true` unconditionally,
  // so there is no "ask only after the backend objects" case to cover here.
  it("shows a confirmation dialog before calling reset_cluster, and does not call it until confirmed", async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      if (cmd === "get_cluster_state") {
        return buildCluster({ state: "running", nodes: [{ name: "colima", ready: true }] });
      }
      if (cmd === "reset_cluster") return undefined;
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    renderSection(buildProfile({ kubernetesEnabled: true, state: "running" }));
    await screen.findByText("colima");

    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(await screen.findByText(/reset this kubernetes cluster/i)).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("reset_cluster", expect.anything());

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Reset" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("reset_cluster", {
        profile: "default",
        confirmed: true,
      })
    );
  });

  it("shows a confirmation dialog before calling delete_cluster, and does not call it until confirmed", async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      if (cmd === "get_cluster_state") {
        return buildCluster({ state: "running", nodes: [{ name: "colima", ready: true }] });
      }
      if (cmd === "delete_cluster") return undefined;
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    renderSection(buildProfile({ kubernetesEnabled: true, state: "running" }));
    await screen.findByText("colima");

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(await screen.findByText(/delete this kubernetes cluster/i)).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("delete_cluster", expect.anything());

    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_cluster", {
        profile: "default",
        confirmed: true,
      })
    );
  });

  it("disables the lifecycle action buttons while an action is already in progress", async () => {
    mockedInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { themeMode: "dark", lastActiveProfileName: null };
      if (cmd === "get_cluster_state") {
        return buildCluster({ state: "running", actionInProgress: "resetting" });
      }
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    renderSection(buildProfile({ kubernetesEnabled: true, state: "running" }));

    expect(await screen.findByRole("button", { name: "Stop" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Reset" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Delete" })).toBeDisabled();
  });
});
