import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";

// Capture the "vault-locked" handler App registers so tests can emit the event.
const tauri = vi.hoisted(() => ({
  lockHandler: null as null | ((e: { payload: string }) => void),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, cb: (e: { payload: string }) => void) => {
    if (event === "vault-locked") tauri.lockHandler = cb;
    return Promise.resolve(() => {});
  },
}));

const ipc = vi.hoisted(() => ({
  vaultExists: vi.fn(),
  isUnlocked: vi.fn(),
  listItems: vi.fn(),
  listTags: vi.fn(),
  keychainAvailable: vi.fn(),
  keychainIsEnabled: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
  pingActivity: vi.fn(),
}));
vi.mock("./lib/ipc", () => ({ ipc }));

import App from "./App";

beforeEach(() => {
  tauri.lockHandler = null;
  ipc.vaultExists.mockResolvedValue(true);
  ipc.isUnlocked.mockResolvedValue(true);
  ipc.listItems.mockResolvedValue([]);
  ipc.listTags.mockResolvedValue([]);
  ipc.keychainAvailable.mockResolvedValue(false);
  ipc.keychainIsEnabled.mockResolvedValue(false);
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  ipc.pingActivity.mockResolvedValue(undefined);
});

async function emitLock(payload: "idle" | "manual") {
  // The handler is registered after the first boot await resolves.
  await screen.findByRole("heading", { name: "Keytainer", level: 1 });
  await act(async () => {
    tauri.lockHandler?.({ payload });
  });
}

describe("App boot routing", () => {
  it("shows the setup screen when no vault exists", async () => {
    ipc.vaultExists.mockResolvedValue(false);
    render(<App />);
    expect(await screen.findByText("Welcome to Keytainer")).toBeInTheDocument();
  });

  it("shows the unlock screen when a vault exists but is locked", async () => {
    ipc.isUnlocked.mockResolvedValue(false);
    render(<App />);
    expect(
      await screen.findByRole("heading", { name: "Unlock Keytainer" }),
    ).toBeInTheDocument();
  });

  it("shows the list screen when the vault is already unlocked", async () => {
    render(<App />);
    expect(
      await screen.findByRole("heading", { name: "Keytainer", level: 1 }),
    ).toBeInTheDocument();
  });
});

describe("App lock navigation", () => {
  it("navigates to the unlock screen with the idle notice on an idle lock", async () => {
    render(<App />);
    await emitLock("idle");

    expect(
      await screen.findByRole("heading", { name: "Unlock Keytainer" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Locked automatically (idle too long)"),
    ).toBeInTheDocument();
  });

  it("navigates to the unlock screen without the idle notice on a manual lock", async () => {
    render(<App />);
    await emitLock("manual");

    expect(
      await screen.findByRole("heading", { name: "Unlock Keytainer" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Locked automatically (idle too long)"),
    ).not.toBeInTheDocument();
  });
});
