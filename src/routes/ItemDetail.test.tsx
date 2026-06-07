import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";
import type { VaultItem } from "../lib/types";

const ipc = vi.hoisted(() => ({
  getSettings: vi.fn(),
  getItem: vi.fn(),
  addItem: vi.fn(),
  updateItem: vi.fn(),
  deleteItem: vi.fn(),
  generatePassword: vi.fn(),
  copyPassword: vi.fn(),
  computeTotp: vi.fn(),
  copyTotp: vi.fn(),
  getSystemLocale: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);

import { ItemDetail } from "./ItemDetail";

const vaultItem = (over: Partial<VaultItem> = {}): VaultItem => ({
  id: "1",
  site_name: "GitHub",
  username: "octocat",
  password: "s3cret",
  totp: null,
  url: null,
  notes: null,
  tags: [],
  created_at: 0,
  updated_at: 0,
  ...over,
});

function renderDetail(itemId: string, props: Partial<Parameters<typeof ItemDetail>[0]> = {}) {
  return renderWithI18n(
    <ItemDetail
      itemId={itemId}
      onClose={props.onClose ?? vi.fn()}
      onSaved={props.onSaved ?? vi.fn()}
      onDeleted={props.onDeleted ?? vi.fn()}
    />,
  );
}

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en", show_totp_code: true });
  ipc.getItem.mockResolvedValue(vaultItem());
  ipc.addItem.mockResolvedValue("new-id");
  ipc.updateItem.mockResolvedValue(undefined);
  ipc.deleteItem.mockResolvedValue(undefined);
  ipc.generatePassword.mockResolvedValue("GENERATEDpassword20!");
  strength.scorePassword.mockReturnValue(4);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ItemDetail — new item", () => {
  it("shows the new-item heading and keeps save disabled until a site name", async () => {
    const user = userEvent.setup();
    renderDetail("new");
    expect(screen.getByRole("heading", { name: "New item" })).toBeInTheDocument();

    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    await user.type(screen.getByLabelText("Site name"), "Example");
    expect(save).toBeEnabled();
  });

  it("adds the item with empty optionals normalized to null", async () => {
    const user = userEvent.setup();
    const onSaved = vi.fn();
    renderDetail("new", { onSaved });
    await user.type(screen.getByLabelText("Site name"), "Example");
    await user.type(screen.getByLabelText("Account"), "me");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(ipc.addItem).toHaveBeenCalledWith(
      expect.objectContaining({
        site_name: "Example",
        username: "me",
        url: null,
        notes: null,
        totp: null,
      }),
    );
    expect(onSaved).toHaveBeenCalledTimes(1);
  });

  it("parses the comma-separated tag input into an array", async () => {
    const user = userEvent.setup();
    renderDetail("new");
    await user.type(screen.getByLabelText("Site name"), "Example");
    await user.type(screen.getByLabelText("Tags (comma separated)"), "work, finance ,  ");
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(ipc.addItem).toHaveBeenCalledWith(
      expect.objectContaining({ tags: ["work", "finance"] }),
    );
  });

  it("fills the password field from the generator", async () => {
    const user = userEvent.setup();
    renderDetail("new");
    await user.click(screen.getByRole("button", { name: "Generate" }));

    expect(ipc.generatePassword).toHaveBeenCalledWith(20, true);
    expect(await screen.findByDisplayValue("GENERATEDpassword20!")).toBeInTheDocument();
  });

  it("reveals the 2FA fields when adding 2FA", async () => {
    const user = userEvent.setup();
    renderDetail("new");
    await user.click(screen.getByRole("button", { name: "Add 2FA (TOTP secret)" }));
    expect(screen.getByLabelText("Secret (base32)")).toBeInTheDocument();
  });

  it("warns before saving a weak password and saves on the second click", async () => {
    strength.scorePassword.mockReturnValue(1);
    const user = userEvent.setup();
    renderDetail("new");
    await user.type(screen.getByLabelText("Site name"), "Example");
    await user.type(screen.getByLabelText("Password"), "weakpw");

    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(ipc.addItem).not.toHaveBeenCalled();
    expect(
      screen.getByText("This password is weak. Click Save again to keep it anyway."),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save anyway" }));
    expect(ipc.addItem).toHaveBeenCalledTimes(1);
  });
});

describe("ItemDetail — existing item", () => {
  it("loads the item and populates the form", async () => {
    renderDetail("1");
    expect(await screen.findByDisplayValue("GitHub")).toBeInTheDocument();
    expect(screen.getByDisplayValue("octocat")).toBeInTheDocument();
    expect(ipc.getItem).toHaveBeenCalledWith("1");
    // Saved items expose the one-click copy action.
    expect(screen.getByRole("button", { name: "Copy password" })).toBeInTheDocument();
  });

  it("shows an error when loading fails", async () => {
    ipc.getItem.mockRejectedValue({ kind: "ItemNotFound", message: "no such item" });
    renderDetail("1");
    expect(await screen.findByText("no such item")).toBeInTheDocument();
  });

  it("deletes after the user confirms", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const onDeleted = vi.fn();
    const user = userEvent.setup();
    renderDetail("1", { onDeleted });
    await screen.findByDisplayValue("GitHub");

    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(ipc.deleteItem).toHaveBeenCalledWith("1");
    expect(onDeleted).toHaveBeenCalledTimes(1);
  });

  it("does not delete when the user cancels the confirm", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(false);
    const onDeleted = vi.fn();
    const user = userEvent.setup();
    renderDetail("1", { onDeleted });
    await screen.findByDisplayValue("GitHub");

    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(ipc.deleteItem).not.toHaveBeenCalled();
    expect(onDeleted).not.toHaveBeenCalled();
  });
});
