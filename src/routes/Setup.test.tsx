import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";

const ipc = vi.hoisted(() => ({
  createVault: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

import { Setup } from "./Setup";

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  ipc.createVault.mockResolvedValue(undefined);
});

const pwField = () => screen.getByLabelText(/Master password \(at least 8/);
const confirmField = () => screen.getByLabelText("Type it again");
const createBtn = () => screen.getByRole("button", { name: "Create vault" });

describe("Setup", () => {
  it("keeps the create button disabled until a valid matching password", async () => {
    const user = userEvent.setup();
    renderWithI18n(<Setup onCreated={vi.fn()} />);
    expect(createBtn()).toBeDisabled();

    await user.type(pwField(), "short"); // < 8 chars
    expect(createBtn()).toBeDisabled();

    await user.clear(pwField());
    await user.type(pwField(), "longenough");
    await user.type(confirmField(), "longenough");
    expect(createBtn()).toBeEnabled();
  });

  it("shows an inline mismatch warning when confirmation differs", async () => {
    const user = userEvent.setup();
    renderWithI18n(<Setup onCreated={vi.fn()} />);
    await user.type(pwField(), "longenough");
    await user.type(confirmField(), "different");
    expect(screen.getByText("The two entries don't match")).toBeInTheDocument();
    expect(createBtn()).toBeDisabled();
  });

  it("creates the vault and fires onCreated on success", async () => {
    const user = userEvent.setup();
    const onCreated = vi.fn();
    renderWithI18n(<Setup onCreated={onCreated} />);
    await user.type(pwField(), "longenough");
    await user.type(confirmField(), "longenough");
    await user.click(createBtn());
    expect(ipc.createVault).toHaveBeenCalledWith("longenough");
    expect(onCreated).toHaveBeenCalledTimes(1);
  });

  it("surfaces a backend error and does not navigate", async () => {
    ipc.createVault.mockRejectedValue({ kind: "Io", message: "disk full" });
    const user = userEvent.setup();
    const onCreated = vi.fn();
    renderWithI18n(<Setup onCreated={onCreated} />);
    await user.type(pwField(), "longenough");
    await user.type(confirmField(), "longenough");
    await user.click(createBtn());
    expect(await screen.findByText("disk full")).toBeInTheDocument();
    expect(onCreated).not.toHaveBeenCalled();
  });
});
