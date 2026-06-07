import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";
import type { Settings as SettingsType } from "../lib/types";

const ipc = vi.hoisted(() => ({
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
  keychainAvailable: vi.fn(),
  keychainIsEnabled: vi.fn(),
  keychainEnable: vi.fn(),
  keychainDisable: vi.fn(),
  changePassword: vi.fn(),
  exportVault: vi.fn(),
  importVault: vi.fn(),
  getSystemLocale: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

const dialog = vi.hoisted(() => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);

const updater = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => updater);
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);

import { Settings } from "./Settings";

const settings = (over: Partial<SettingsType> = {}): SettingsType => ({
  auto_lock_seconds: 300,
  clipboard_clear_seconds: 30,
  show_totp_code: true,
  locale: "system",
  ...over,
});

const renderSettings = () => renderWithI18n(<Settings onClose={vi.fn()} />);

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue(settings());
  ipc.saveSettings.mockResolvedValue(undefined);
  ipc.keychainAvailable.mockResolvedValue(false);
  ipc.keychainIsEnabled.mockResolvedValue(false);
  ipc.changePassword.mockResolvedValue(undefined);
  ipc.importVault.mockResolvedValue({ added: 0, updated: 0 });
  strength.scorePassword.mockReturnValue(4);
});

describe("Settings — load & save", () => {
  it("renders the settings once loaded", async () => {
    renderSettings();
    expect(await screen.findByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("persists settings and flashes the saved confirmation", async () => {
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.click(screen.getByRole("button", { name: "Save settings" }));
    expect(ipc.saveSettings).toHaveBeenCalledWith(settings());
    expect(await screen.findByText("Saved ✓")).toBeInTheDocument();
  });

  it("persists a language change immediately", async () => {
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.selectOptions(screen.getByLabelText("Interface language"), "en");
    expect(ipc.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ locale: "en" }),
    );
  });
});

describe("Settings — change password", () => {
  it("disables the button and warns when the new passwords differ", async () => {
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.type(screen.getByLabelText("Current password"), "oldpassword");
    await user.type(screen.getByLabelText("New password (at least 8 characters)"), "newpassword");
    await user.type(screen.getByLabelText("Confirm new password"), "different123");

    expect(screen.getByText("The two entries don't match")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Change password" })).toBeDisabled();
  });

  it("maps a wrong current password to the localized message", async () => {
    ipc.changePassword.mockRejectedValue({ kind: "WrongPassword", message: "x" });
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.type(screen.getByLabelText("Current password"), "wrongpassword");
    await user.type(screen.getByLabelText("New password (at least 8 characters)"), "newpassword");
    await user.type(screen.getByLabelText("Confirm new password"), "newpassword");
    await user.click(screen.getByRole("button", { name: "Change password" }));

    expect(await screen.findByText("Current password is wrong")).toBeInTheDocument();
  });

  it("confirms success after a valid change", async () => {
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.type(screen.getByLabelText("Current password"), "oldpassword");
    await user.type(screen.getByLabelText("New password (at least 8 characters)"), "newpassword");
    await user.type(screen.getByLabelText("Confirm new password"), "newpassword");
    await user.click(screen.getByRole("button", { name: "Change password" }));

    expect(ipc.changePassword).toHaveBeenCalledWith("oldpassword", "newpassword");
    expect(await screen.findByText("Master password changed ✓")).toBeInTheDocument();
  });

  it("keeps the change button disabled when the new password is too weak", async () => {
    strength.scorePassword.mockReturnValue(1);
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.type(screen.getByLabelText("Current password"), "oldpassword");
    await user.type(screen.getByLabelText("New password (at least 8 characters)"), "weakish12");
    await user.type(screen.getByLabelText("Confirm new password"), "weakish12");
    expect(screen.getByText("Password is too weak — add length or variety.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Change password" })).toBeDisabled();
  });
});

describe("Settings — keychain", () => {
  it("explains when the keychain is unsupported", async () => {
    renderSettings();
    expect(
      await screen.findByText(/This machine can't use the system keychain/),
    ).toBeInTheDocument();
  });

  it("enables the keychain when toggled on", async () => {
    ipc.keychainAvailable.mockResolvedValue(true);
    ipc.keychainIsEnabled.mockResolvedValue(false);
    ipc.keychainEnable.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.click(
      screen.getByRole("checkbox", { name: /Store the current decryption key/ }),
    );
    expect(ipc.keychainEnable).toHaveBeenCalledTimes(1);
  });
});

describe("Settings — updater", () => {
  it("reports being up to date when no update is found", async () => {
    updater.check.mockResolvedValue(null);
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await screen.findByText("You're on the latest version ✓")).toBeInTheDocument();
  });

  it("announces an available update with its version", async () => {
    updater.check.mockResolvedValue({ version: "9.9.9", downloadAndInstall: vi.fn() });
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(
      await screen.findByText("Update available: 9.9.9. Download and install?"),
    ).toBeInTheDocument();
  });
});

describe("Settings — backup import", () => {
  it("maps a wrong backup password on import", async () => {
    dialog.open.mockResolvedValue("/tmp/backup.json");
    ipc.importVault.mockRejectedValue({ kind: "WrongPassword", message: "x" });
    const user = userEvent.setup();
    renderSettings();
    await screen.findByRole("heading", { name: "Settings" });

    await user.type(screen.getByLabelText("Backup password"), "guess");
    await user.click(screen.getByRole("button", { name: "Import…" }));

    expect(await screen.findByText("Wrong backup password")).toBeInTheDocument();
  });
});
