import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithI18n } from "../test/render";

const ipc = vi.hoisted(() => ({
  getSystemLocale: vi.fn(),
  getSettings: vi.fn(),
}));
vi.mock("../lib/ipc", () => ({ ipc }));

const strength = vi.hoisted(() => ({
  scorePassword: vi.fn(),
  MIN_MASTER_SCORE: 2,
}));
vi.mock("../lib/strength", () => strength);

import { StrengthMeter } from "./StrengthMeter";

beforeEach(() => {
  ipc.getSystemLocale.mockResolvedValue("en");
  ipc.getSettings.mockResolvedValue({ locale: "en" });
  strength.scorePassword.mockReturnValue(2);
});

describe("StrengthMeter", () => {
  it("renders nothing for an empty password", () => {
    const { container } = renderWithI18n(<StrengthMeter password="" />);
    expect(container.querySelector(".strength-meter")).toBeNull();
  });

  it("shows the localized label for the computed score", async () => {
    strength.scorePassword.mockReturnValue(2);
    renderWithI18n(<StrengthMeter password="whatever" />);
    expect(await screen.findByText(/Fair/)).toBeInTheDocument();
  });

  it("reflects the score level in the container class", () => {
    strength.scorePassword.mockReturnValue(4);
    const { container } = renderWithI18n(<StrengthMeter password="whatever" />);
    expect(container.querySelector(".strength-meter")?.className).toContain("level-4");
  });
});
