import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { VolumesView } from "../../src/components/volumes/VolumesView";
import { ThemeModeProvider } from "../../src/theme/ThemeModeProvider";
import type { ColimaProfile, Volume } from "../../src/types/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

function buildVolume(overrides: Partial<Volume> = {}): Volume {
  return {
    name: "app-data",
    sizeBytes: 5 * 1024 * 1024,
    attachedContainerNames: [],
    profileName: "default",
    ...overrides,
  };
}

function buildProfile(overrides: Partial<ColimaProfile> = {}): ColimaProfile {
  return {
    name: "default",
    state: "running",
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

function renderVolumesView() {
  return render(
    <ThemeModeProvider>
      <VolumesView />
    </ThemeModeProvider>
  );
}

describe("VolumesView", () => {
  let volumes: Volume[];
  let profiles: ColimaProfile[];
  let listenCallbacks: Record<string, (payload: unknown) => void>;

  beforeEach(() => {
    volumes = [buildVolume()];
    profiles = [buildProfile()];
    listenCallbacks = {};
    mockedListen.mockImplementation(
      (eventName: string, handler: (event: { payload: unknown }) => void) => {
        listenCallbacks[eventName] = (payload: unknown) => handler({ payload });
        return Promise.resolve(() => undefined);
      }
    );
    mockedInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      switch (cmd) {
        case "get_settings":
          return { themeMode: "dark", lastActiveProfileName: null };
        case "list_profiles":
          return profiles;
        case "list_volumes":
          if (args?.profile) {
            return volumes.filter((volume) => volume.profileName === args.profile);
          }
          return volumes;
        case "prune_volumes":
          return {
            removedNames: volumes
              .filter((v) => v.attachedContainerNames.length === 0)
              .map((v) => v.name),
          };
        case "delete_volume": {
          const target = volumes.find((v) => v.name === args?.name);
          if (!args?.confirmed && target && target.attachedContainerNames.length > 0) {
            throw { code: "CONFIRMATION_REQUIRED", message: "Confirm deletion" };
          }
          return undefined;
        }
        default:
          throw new Error(`unexpected invoke: ${cmd}`);
      }
    });
  });

  it("renders the volume list with its name and size", async () => {
    renderVolumesView();
    expect(await screen.findByText("app-data")).toBeInTheDocument();
    expect(screen.getByText("5 MiB")).toBeInTheDocument();
    expect(screen.getByText("Unattached")).toBeInTheDocument();
  });

  it("shows the attached container names for a volume in use", async () => {
    volumes = [buildVolume({ attachedContainerNames: ["web"] })];
    renderVolumesView();
    expect(await screen.findByText("app-data")).toBeInTheDocument();
    expect(screen.getByText("Attached: web")).toBeInTheDocument();
  });

  it("prunes unattached volumes and reports what was removed", async () => {
    renderVolumesView();
    await screen.findByText("app-data");

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "default" } });
    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue("default"));

    fireEvent.click(screen.getByRole("button", { name: "Prune unattached volumes" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("prune_volumes", { profile: "default" })
    );
    expect(await screen.findByText("Removed: app-data")).toBeInTheDocument();
  });

  it("prunes across every running profile while the filter is set to 'All profiles'", async () => {
    renderVolumesView();
    await screen.findByText("app-data");

    const pruneButton = screen.getByRole("button", { name: "Prune unattached volumes" });
    expect(pruneButton).not.toBeDisabled();

    fireEvent.click(pruneButton);

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("prune_volumes", { profile: null })
    );
    expect(await screen.findByText("Removed: app-data")).toBeInTheDocument();
  });

  it("deletes an unattached volume without showing a confirmation dialog", async () => {
    renderVolumesView();
    await screen.findByText("app-data");

    fireEvent.click(screen.getByRole("button", { name: "Delete app-data" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_volume", {
        profile: "default",
        name: "app-data",
        confirmed: false,
      })
    );
    expect(screen.queryByText('Delete volume "app-data"?')).not.toBeInTheDocument();
  });

  it("shows the confirmation dialog when deleting an attached volume, then deletes on confirm", async () => {
    volumes = [buildVolume({ attachedContainerNames: ["web"] })];
    renderVolumesView();
    await screen.findByText("app-data");

    fireEvent.click(screen.getByRole("button", { name: "Delete app-data" }));
    expect(await screen.findByText('Delete volume "app-data"?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_volume", {
        profile: "default",
        name: "app-data",
        confirmed: true,
      })
    );
  });

  it("defaults the profile filter to 'All profiles'", async () => {
    renderVolumesView();
    await screen.findByText("app-data");

    expect(screen.getByLabelText("Profile")).toHaveValue("");
    expect(screen.getByText("All profiles")).toBeInTheDocument();
  });

  it("narrows the rendered list to the selected profile", async () => {
    volumes = [
      buildVolume({ name: "volume-a", profileName: "default" }),
      buildVolume({ name: "volume-b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderVolumesView();
    await screen.findByText("volume-a");
    expect(screen.getByText("volume-b")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_volumes", { profile: "profile-b" })
    );
    await waitFor(() => expect(screen.queryByText("volume-a")).not.toBeInTheDocument());
    expect(screen.getByText("volume-b")).toBeInTheDocument();
  });

  it("falls back to 'All profiles' once the selected profile disappears from profiles-changed", async () => {
    volumes = [
      buildVolume({ name: "volume-a", profileName: "default" }),
      buildVolume({ name: "volume-b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderVolumesView();
    await screen.findByText("volume-a");

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });
    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue("profile-b"));

    profiles = [buildProfile({ name: "default" })];
    act(() => {
      listenCallbacks["profiles-changed"]?.(profiles);
    });

    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue(""));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_volumes", { profile: undefined })
    );
  });
});
