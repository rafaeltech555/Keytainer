import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";

// Mock the whole ipc surface the Unlock route + I18nProvider touch.
// vi.hoisted runs before the hoisted vi.mock factory, so `ipc` is initialized.
const ipc = vi.hoisted(() => ({
  unlock: vi.fn(),
  unlockWithKeychain: vi.fn(),
  keychainAvailable: vi.fn(),
  keychainIsEnabled: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

import { Unlock } from "./Unlock";
import { I18nProvider } from "../lib/i18n";

const err = (kind: string, message = "x") => ({ kind, message });

function renderUnlock(onUnlocked = vi.fn()) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <I18nProvider>{children}</I18nProvider>
  );
  return render(<Unlock onUnlocked={onUnlocked} />, { wrapper });
}

async function submitPassword(pw: string) {
  const user = userEvent.setup();
  await user.type(screen.getByLabelText("Master password"), pw);
  await user.click(screen.getByRole("button", { name: "Unlock" }));
}

beforeEach(() => {
  // English UI so we can assert on the reference strings.
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  // No keychain by default; individual tests opt in.
  ipc.keychainAvailable.mockResolvedValue(false);
  ipc.keychainIsEnabled.mockResolvedValue(false);
});

describe("Unlock — password error mapping", () => {
  it("maps WrongPassword to the localized message", async () => {
    ipc.unlock.mockRejectedValue(err("WrongPassword"));
    renderUnlock();
    await submitPassword("hunter2");
    expect(await screen.findByText("Wrong master password")).toBeInTheDocument();
  });

  it("maps VaultCorrupt to the localized message", async () => {
    ipc.unlock.mockRejectedValue(err("VaultCorrupt"));
    renderUnlock();
    await submitPassword("hunter2");
    expect(
      await screen.findByText("Vault file is corrupt or not a Keytainer file"),
    ).toBeInTheDocument();
  });

  it("falls back to the raw message for an unmapped AppError kind", async () => {
    ipc.unlock.mockRejectedValue(err("Io", "disk on fire"));
    renderUnlock();
    await submitPassword("hunter2");
    expect(await screen.findByText("disk on fire")).toBeInTheDocument();
  });

  it("stringifies a non-AppError rejection", async () => {
    ipc.unlock.mockRejectedValue("boom");
    renderUnlock();
    await submitPassword("hunter2");
    expect(await screen.findByText("boom")).toBeInTheDocument();
  });

  it("calls onUnlocked on success", async () => {
    ipc.unlock.mockResolvedValue(undefined);
    const onUnlocked = vi.fn();
    renderUnlock(onUnlocked);
    await submitPassword("correct horse");
    expect(ipc.unlock).toHaveBeenCalledWith("correct horse");
    expect(onUnlocked).toHaveBeenCalledTimes(1);
  });
});

describe("Unlock — keychain quick-unlock error mapping", () => {
  beforeEach(() => {
    ipc.keychainAvailable.mockResolvedValue(true);
    ipc.keychainIsEnabled.mockResolvedValue(true);
  });

  it("maps KeychainUnavailable to the localized message", async () => {
    ipc.unlockWithKeychain.mockRejectedValue(err("KeychainUnavailable"));
    renderUnlock();
    const user = userEvent.setup();
    await user.click(
      await screen.findByRole("button", { name: /One-tap unlock/ }),
    );
    expect(
      await screen.findByText("Keychain unavailable, use your master password instead"),
    ).toBeInTheDocument();
  });

  it("maps a WrongPassword from the keychain to the mismatch message", async () => {
    ipc.unlockWithKeychain.mockRejectedValue(err("WrongPassword"));
    renderUnlock();
    const user = userEvent.setup();
    await user.click(
      await screen.findByRole("button", { name: /One-tap unlock/ }),
    );
    expect(
      await screen.findByText("The keychain key doesn't match the vault, use your master password"),
    ).toBeInTheDocument();
  });
});
