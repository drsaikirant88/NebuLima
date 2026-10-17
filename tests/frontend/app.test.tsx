import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "../../src/App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
const mockedListen = listen as unknown as ReturnType<typeof vi.fn>;

mockedListen.mockResolvedValue(() => undefined);
mockedInvoke.mockImplementation(async (cmd: string) => {
  switch (cmd) {
    case "get_settings":
      return { themeMode: "dark", lastActiveProfileName: null };
    case "list_profiles":
      return [];
    default:
      throw new Error(`unexpected invoke: ${cmd}`);
  }
});

describe("App Skeleton", () => {
  it("renders NebuLima title", async () => {
    render(<App />);
    expect(await screen.findByText("NebuLima")).toBeInTheDocument();
  });
});
