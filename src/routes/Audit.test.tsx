import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithI18n } from "../test/render";
import type { AuditReport } from "../lib/types";

const ipc = vi.hoisted(() => ({
  auditPasswords: vi.fn(),
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

import { Audit } from "./Audit";

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
});

const report = (over: Partial<AuditReport> = {}): AuditReport => ({
  reused: [],
  weak: [],
  ...over,
});

describe("Audit", () => {
  it("shows the no-problems message for a clean report", async () => {
    ipc.auditPasswords.mockResolvedValue(report());
    renderWithI18n(<Audit onBack={vi.fn()} onSelect={vi.fn()} />);
    expect(await screen.findByText("No problems found ✓")).toBeInTheDocument();
  });

  it("renders reuse groups and weak items, selecting on click", async () => {
    ipc.auditPasswords.mockResolvedValue(
      report({
        reused: [
          {
            items: [
              { id: "a", site_name: "GitHub", username: "alice" },
              { id: "b", site_name: "GitLab", username: "alice" },
            ],
          },
        ],
        weak: [{ item: { id: "c", site_name: "Forum", username: "nick" }, score: 1 }],
      }),
    );
    const onSelect = vi.fn();
    const user = userEvent.setup();
    renderWithI18n(<Audit onBack={vi.fn()} onSelect={onSelect} />);

    expect(await screen.findByText("Reused passwords")).toBeInTheDocument();
    expect(screen.getByText("2 items share one password")).toBeInTheDocument();
    expect(screen.getByText("Weak passwords")).toBeInTheDocument();

    await user.click(screen.getByText("GitHub"));
    expect(onSelect).toHaveBeenCalledWith("a");
  });
});
