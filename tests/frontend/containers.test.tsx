import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ContainersView } from "../../src/components/containers/ContainersView";
import { ThemeModeProvider } from "../../src/theme/ThemeModeProvider";
import type { ColimaProfile, Container } from "../../src/types/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

function buildContainer(overrides: Partial<Container> = {}): Container {
  return {
    id: "abc123",
    name: "my-container",
    image: "alpine:3",
    status: "running",
    uptimeSeconds: 3600,
    ports: [],
    latestStats: null,
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

function renderContainersView(initialProfileFilter?: string | null) {
  return render(
    <ThemeModeProvider>
      <ContainersView initialProfileFilter={initialProfileFilter} />
    </ThemeModeProvider>
  );
}

describe("ContainersView", () => {
  let containers: Container[];
  let profiles: ColimaProfile[];
  let listenCallbacks: Record<string, (payload: unknown) => void>;

  beforeEach(() => {
    containers = [buildContainer()];
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
        case "list_containers":
          if (args?.profile) {
            return containers.filter((container) => container.profileName === args.profile);
          }
          return containers;
        case "start_container":
        case "stop_container":
        case "subscribe_container_logs":
        case "unsubscribe_container_logs":
          return undefined;
        case "delete_container":
          if (args?.confirmed !== true) {
            throw { code: "CONFIRMATION_REQUIRED", message: "Confirm deletion" };
          }
          return undefined;
        default:
          throw new Error(`unexpected invoke: ${cmd}`);
      }
    });
  });

  it("renders the container list with its status badge and image", async () => {
    renderContainersView();
    expect(await screen.findByText("my-container")).toBeInTheDocument();
    expect(screen.getByText("alpine:3")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
  });

  it("starts a stopped container", async () => {
    containers = [buildContainer({ status: "stopped", uptimeSeconds: null })];
    renderContainersView();
    await screen.findByText("my-container");

    fireEvent.click(screen.getByRole("button", { name: "Start" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("start_container", {
        profile: "default",
        id: "abc123",
      })
    );
  });

  it("stops a running container", async () => {
    renderContainersView();
    await screen.findByText("my-container");

    fireEvent.click(screen.getByRole("button", { name: "Stop" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("stop_container", {
        profile: "default",
        id: "abc123",
      })
    );
  });

  it("always shows a confirmation dialog before deleting, and deletes only after confirming", async () => {
    renderContainersView();
    await screen.findByText("my-container");

    fireEvent.click(screen.getByRole("button", { name: "Delete my-container" }));
    expect(await screen.findByText('Delete container "my-container"?')).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith("delete_container", expect.anything());

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_container", {
        profile: "default",
        id: "abc123",
        confirmed: true,
      })
    );
  });

  it("toggles the inline log panel open and closed for a single row", async () => {
    renderContainersView();
    await screen.findByText("my-container");

    expect(screen.queryByText("Logs — my-container")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View logs for my-container" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("subscribe_container_logs", {
        profile: "default",
        id: "abc123",
      })
    );
    expect(await screen.findByText("Logs — my-container")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View logs for my-container" }));
    await waitFor(() => expect(screen.queryByText("Logs — my-container")).not.toBeInTheDocument());
  });

  it("expanding a different row's logs collapses the first row's panel", async () => {
    containers = [
      buildContainer({ id: "a1", name: "container-a" }),
      buildContainer({ id: "b1", name: "container-b" }),
    ];
    renderContainersView();
    await screen.findByText("container-a");

    fireEvent.click(screen.getByRole("button", { name: "View logs for container-a" }));
    expect(await screen.findByText("Logs — container-a")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View logs for container-b" }));
    await waitFor(() => expect(screen.queryByText("Logs — container-a")).not.toBeInTheDocument());
    expect(await screen.findByText("Logs — container-b")).toBeInTheDocument();
  });

  it("defaults the profile filter to 'All profiles'", async () => {
    renderContainersView();
    await screen.findByText("my-container");

    expect(screen.getByLabelText("Profile")).toHaveValue("");
    expect(screen.getByText("All profiles")).toBeInTheDocument();
  });

  it("narrows the rendered list to the selected profile", async () => {
    containers = [
      buildContainer({ id: "a1", name: "container-a", profileName: "default" }),
      buildContainer({ id: "b1", name: "container-b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderContainersView();
    await screen.findByText("container-a");
    expect(screen.getByText("container-b")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_containers", { profile: "profile-b" })
    );
    await waitFor(() => expect(screen.queryByText("container-a")).not.toBeInTheDocument());
    expect(screen.getByText("container-b")).toBeInTheDocument();
  });

  it("falls back to 'All profiles' once the selected profile disappears from profiles-changed", async () => {
    containers = [
      buildContainer({ id: "a1", name: "container-a", profileName: "default" }),
      buildContainer({ id: "b1", name: "container-b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderContainersView();
    await screen.findByText("container-a");

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });
    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue("profile-b"));

    profiles = [buildProfile({ name: "default" })];
    act(() => {
      listenCallbacks["profiles-changed"]?.(profiles);
    });

    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue(""));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_containers", { profile: undefined })
    );
  });

  it("initializes the profile filter to an incoming navigation-provided profile name", async () => {
    containers = [
      buildContainer({ id: "a1", name: "container-a", profileName: "default" }),
      buildContainer({ id: "b1", name: "container-b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderContainersView("profile-b");
    await screen.findByText("container-b");

    expect(screen.getByLabelText("Profile")).toHaveValue("profile-b");
    expect(screen.queryByText("container-a")).not.toBeInTheDocument();
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_containers", { profile: "profile-b" })
    );
  });
});
