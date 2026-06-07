import { describe, it, expect } from "vitest";
import { scorePassword, MIN_MASTER_SCORE } from "./strength";

describe("scorePassword", () => {
  it("scores a common dictionary password below the master minimum", () => {
    expect(scorePassword("password")).toBeLessThan(MIN_MASTER_SCORE);
  });

  it("scores a long random passphrase at good or above", () => {
    expect(scorePassword("correct-horse-battery-staple-9173")).toBeGreaterThanOrEqual(3);
  });

  it("treats an empty string as the weakest score", () => {
    expect(scorePassword("")).toBe(0);
  });

  it("uses fair (2) as the master-password minimum", () => {
    expect(MIN_MASTER_SCORE).toBe(2);
  });
});
