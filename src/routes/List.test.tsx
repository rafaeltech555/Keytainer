import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitForElementToBeRemoved } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";
import type { ItemSummary } from "../lib/types";

const ipc = vi.hoisted(() => ({
  listItems: vi.fn(),
  listTags: vi.fn(),
  lock: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

import { List } from "./List";

const item = (over: Partial<ItemSummary> = {}): ItemSummary => ({
  id: "1",
  site_name: "GitHub",
  username: "octocat",
  url: null,
  tags: [],
  has_totp: false,
  updated_at: 0,
  ...over,
});

function renderList(props: Partial<Parameters<typeof List>[0]> = {}) {
  return renderWithI18n(
    <List
      refreshKey={0}
      onSelect={props.onSelect ?? vi.fn()}
      onLock={props.onLock ?? vi.fn()}
      onSettings={props.onSettings ?? vi.fn()}
    />,
  );
}

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  ipc.listItems.mockResolvedValue([]);
  ipc.listTags.mockResolvedValue([]);
  ipc.lock.mockResolvedValue(undefined);
});

describe("List", () => {
  it("shows the empty-vault message when there are no items", async () => {
    renderList();
    expect(
      await screen.findByText('No items yet — tap "Add" at the top right to start'),
    ).toBeInTheDocument();
  });

  it("renders items with their 2FA badge and tags", async () => {
    ipc.listItems.mockResolvedValue([
      item({ site_name: "GitHub", username: "octocat", has_totp: true, tags: ["work"] }),
    ]);
    renderList();
    expect(await screen.findByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("octocat")).toBeInTheDocument();
    expect(screen.getByText("2FA")).toBeInTheDocument();
    expect(screen.getByText("work")).toBeInTheDocument();
  });

  it("falls back to the unnamed label when site_name is blank", async () => {
    ipc.listItems.mockResolvedValue([item({ site_name: "" })]);
    renderList();
    expect(await screen.findByText("(unnamed)")).toBeInTheDocument();
  });

  it("shows the no-match message when a search returns nothing", async () => {
    ipc.listItems.mockResolvedValueOnce([item()]); // initial load
    const user = userEvent.setup();
    renderList();
    await screen.findByText("GitHub");

    ipc.listItems.mockResolvedValue([]); // subsequent query
    await user.type(screen.getByPlaceholderText("Search site, account, tag…"), "zz");

    expect(await screen.findByText("No matching items")).toBeInTheDocument();
    expect(ipc.listItems).toHaveBeenLastCalledWith("zz", undefined);
  });

  it("filters by tag chip", async () => {
    ipc.listItems.mockResolvedValue([item({ tags: ["work"] })]);
    ipc.listTags.mockResolvedValue(["work"]);
    const user = userEvent.setup();
    renderList();
    await screen.findByText("GitHub");

    await user.click(screen.getByRole("button", { name: "work" }));
    expect(ipc.listItems).toHaveBeenLastCalledWith("", "work");
  });

  it("locks the vault and notifies the parent", async () => {
    const onLock = vi.fn();
    const user = userEvent.setup();
    renderList({ onLock });
    await waitForElementToBeRemoved(() => screen.queryByText("Loading…"));

    await user.click(screen.getByRole("button", { name: "🔒 Lock" }));
    expect(ipc.lock).toHaveBeenCalledTimes(1);
    expect(onLock).toHaveBeenCalledTimes(1);
  });
});
