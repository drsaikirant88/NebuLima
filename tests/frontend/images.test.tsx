import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ImagesView } from "../../src/components/images/ImagesView";
import { ThemeModeProvider } from "../../src/theme/ThemeModeProvider";
import type { ColimaProfile, Image, ImageInspectDetail } from "../../src/types/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

function buildImage(overrides: Partial<Image> = {}): Image {
  return {
    id: "sha256:abc123",
    repository: "alpine",
    tag: "3.19",
    sizeBytes: 7 * 1024 * 1024,
    createdAt: "2024-01-01T00:00:00.000Z",
    inUse: false,
    profileName: "default",
    ...overrides,
  };
}

function buildInspectDetail(overrides: Partial<ImageInspectDetail> = {}): ImageInspectDetail {
  return {
    id: "sha256:abc123",
    env: ["PATH=/usr/bin"],
    cmd: ["/bin/sh"],
    entrypoint: null,
    workingDir: null,
    labels: {},
    exposedPorts: [],
    layers: [],
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

function renderImagesView() {
  return render(
    <ThemeModeProvider>
      <ImagesView />
    </ThemeModeProvider>
  );
}

describe("ImagesView", () => {
  let images: Image[];
  let profiles: ColimaProfile[];
  let listenCallbacks: Record<string, (payload: unknown) => void>;

  beforeEach(() => {
    images = [buildImage()];
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
        case "list_images":
          if (args?.profile) {
            return images.filter((image) => image.profileName === args.profile);
          }
          return images;
        case "inspect_image":
          return buildInspectDetail({ id: args?.id as string });
        case "delete_image": {
          const target = images.find((image) => image.id === args?.id);
          if (!args?.confirmed && target?.inUse) {
            throw { code: "CONFIRMATION_REQUIRED", message: "Confirm deletion" };
          }
          return undefined;
        }
        default:
          throw new Error(`unexpected invoke: ${cmd}`);
      }
    });
  });

  it("renders the image list with its repository, tag, and size", async () => {
    renderImagesView();
    expect(await screen.findByText("alpine:3.19")).toBeInTheDocument();
    expect(screen.getByText("7 MiB")).toBeInTheDocument();
  });

  it("shows an in-use indicator for images referenced by a container", async () => {
    images = [buildImage({ inUse: true })];
    renderImagesView();
    expect(await screen.findByText("alpine:3.19")).toBeInTheDocument();
    expect(screen.getByText("In use")).toBeInTheDocument();
  });

  it("opens the inspect dialog with the image's config", async () => {
    renderImagesView();
    await screen.findByText("alpine:3.19");

    fireEvent.click(screen.getByRole("button", { name: "Inspect alpine:3.19" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("inspect_image", {
        profile: "default",
        id: "sha256:abc123",
      })
    );
    expect(await screen.findByText("/bin/sh")).toBeInTheDocument();
  });

  it("deletes an image not in use without showing a confirmation dialog", async () => {
    renderImagesView();
    await screen.findByText("alpine:3.19");

    fireEvent.click(screen.getByRole("button", { name: "Delete alpine:3.19" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_image", {
        profile: "default",
        id: "sha256:abc123",
        confirmed: false,
      })
    );
    expect(screen.queryByText('Delete image "alpine"?')).not.toBeInTheDocument();
  });

  it("shows the confirmation dialog when deleting an in-use image, then deletes on confirm", async () => {
    images = [buildImage({ inUse: true })];
    renderImagesView();
    await screen.findByText("alpine:3.19");

    fireEvent.click(screen.getByRole("button", { name: "Delete alpine:3.19" }));
    expect(await screen.findByText('Delete image "alpine"?')).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("delete_image", {
        profile: "default",
        id: "sha256:abc123",
        confirmed: true,
      })
    );
  });

  it("defaults the profile filter to 'All profiles'", async () => {
    renderImagesView();
    await screen.findByText("alpine:3.19");

    expect(screen.getByLabelText("Profile")).toHaveValue("");
    expect(screen.getByText("All profiles")).toBeInTheDocument();
  });

  it("narrows the rendered list to the selected profile", async () => {
    images = [
      buildImage({ id: "sha256:a1", repository: "alpine", tag: "a", profileName: "default" }),
      buildImage({ id: "sha256:b1", repository: "alpine", tag: "b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderImagesView();
    await screen.findByText("alpine:a");
    expect(screen.getByText("alpine:b")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_images", { profile: "profile-b" })
    );
    await waitFor(() => expect(screen.queryByText("alpine:a")).not.toBeInTheDocument());
    expect(screen.getByText("alpine:b")).toBeInTheDocument();
  });

  it("falls back to 'All profiles' once the selected profile disappears from profiles-changed", async () => {
    images = [
      buildImage({ id: "sha256:a1", repository: "alpine", tag: "a", profileName: "default" }),
      buildImage({ id: "sha256:b1", repository: "alpine", tag: "b", profileName: "profile-b" }),
    ];
    profiles = [buildProfile({ name: "default" }), buildProfile({ name: "profile-b" })];
    renderImagesView();
    await screen.findByText("alpine:a");

    fireEvent.change(screen.getByLabelText("Profile"), { target: { value: "profile-b" } });
    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue("profile-b"));

    profiles = [buildProfile({ name: "default" })];
    act(() => {
      listenCallbacks["profiles-changed"]?.(profiles);
    });

    await waitFor(() => expect(screen.getByLabelText("Profile")).toHaveValue(""));
    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith("list_images", { profile: undefined })
    );
  });
});
