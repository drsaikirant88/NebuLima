import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesView } from "../../src/components/profiles/ProfilesView";
import { ToastProvider } from "../../src/components/shell/ToastProvider";
import { ThemeModeProvider } from "../../src/theme/ThemeModeProvider";
import type { ColimaProfile } from "../../src/types/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

function buildProfile(overrides: Partial<ColimaProfile> = {}): ColimaProfile {
  return {
    name: "default",
    state: "stopped",
    cpu: 2,
    memoryGiB: 2,
    diskGiB: 60,
    runtime: "docker",
    kubernetesEnabled: false,
    network: { dns: [], ipAddress: null },
    mounts: [],
    caCert: { mode: "none" as const },
    dockerSocketPath: null,
    isActive: false,
    ...overrides,
  };
}

function renderProfilesView(onViewResources?: (profileName: string) => void) {
  return render(
    <ThemeModeProvider>
      <ToastProvider>
        <ProfilesView onViewResources={onViewResources} />
      </ToastProvider>
    </ThemeModeProvider>
  );
}

describe("ProfilesView", () => {
  let profiles: ColimaProfile[];

  beforeEach(() => {
    profiles = [buildProfile()];
    mockedListen.mockResolvedValue(() => undefined);
    mockedInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case "get_settings":
          return { themeMode: "dark", lastActiveProfileName: null };
        case "list_profiles":
          return profiles;
        case "create_profile":
          return buildProfile({ ...(args?.input as Partial<ColimaProfile>) });
        case "update_profile":
          return buildProfile({ ...(args?.input as Partial<ColimaProfile>) });
        case "start_profile":
        case "stop_profile":
          return undefined;
        case "set_active_profile":
          return buildProfile({ name: args?.name as string, isActive: true });
        case "delete_profile":
          return undefined;
        default:
          throw new Error(`unexpected invoke: ${cmd}`);
      }
    });
  });

  it("renders the profile list with its lifecycle-state badge", async () => {
    renderProfilesView();
    expect(await screen.findByText("default")).toBeInTheDocument();
    expect(screen.getByText("Stopped")).toBeInTheDocument();
  });

  it("creates a profile via the create form", async () => {
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "New profile" }));
    fireEvent.change(await screen.findByLabelText("Name", { exact: false }), {
      target: { value: "work" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "create_profile",
        expect.objectContaining({ input: expect.objectContaining({ name: "work" }) })
      )
    );
  });

  it("edits a stopped profile's resources", async () => {
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Edit default" }));
    const cpuField = screen.getByLabelText("CPUs") as HTMLInputElement;
    expect(cpuField).not.toBeDisabled();
    fireEvent.change(cpuField, { target: { value: "4" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "update_profile",
        expect.objectContaining({ input: expect.objectContaining({ name: "default", cpu: 4 }) })
      )
    );
  });

  it("locks resource fields while a profile is running", async () => {
    profiles = [buildProfile({ state: "running" })];
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Edit default" }));
    expect(screen.getByLabelText("CPUs")).toBeDisabled();
    expect(screen.getByLabelText("Memory (GiB)")).toBeDisabled();
  });

  it("starts a stopped profile", async () => {
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("start_profile", { name: "default" })
    );
  });

  it("stops a running profile", async () => {
    profiles = [buildProfile({ state: "running" })];
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("stop_profile", { name: "default" })
    );
  });

  it("shows the delete-confirmation dialog for an active profile, then deletes on confirm", async () => {
    profiles = [buildProfile({ isActive: true })];
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Delete default" }));
    expect(await screen.findByText('Delete profile "default"?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_profile", {
        name: "default",
        confirmed: true,
        wipeAssets: false,
      })
    );
  });

  it("always shows the delete-confirmation dialog, even for an idle, inactive profile", async () => {
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Delete default" }));
    expect(await screen.findByText('Delete profile "default"?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_profile", {
        name: "default",
        confirmed: true,
        wipeAssets: false,
      })
    );
  });

  it("passes wipeAssets: true when the 'also delete all profile data' checkbox is checked", async () => {
    renderProfilesView();
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "Delete default" }));
    expect(await screen.findByText('Delete profile "default"?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("checkbox", { name: /Also delete all profile data/i }));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_profile", {
        name: "default",
        confirmed: true,
        wipeAssets: true,
      })
    );
  });

  it("navigates to that profile's resources via the 'view resources' action", async () => {
    const onViewResources = vi.fn();
    renderProfilesView(onViewResources);
    await screen.findByText("default");

    fireEvent.click(screen.getByRole("button", { name: "View resources for default" }));

    expect(onViewResources).toHaveBeenCalledWith("default");
  });
});
