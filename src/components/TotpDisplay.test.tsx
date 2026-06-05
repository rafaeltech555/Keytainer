import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import { TotpDisplay } from "./TotpDisplay";
import type { TotpState } from "../lib/types";

const computeTotp = vi.fn();
const copyTotp = vi.fn();
vi.mock("../lib/ipc", () => ({
  ipc: {
    computeTotp: (id: string) => computeTotp(id),
    copyTotp: (id: string) => copyTotp(id),
  },
}));

const state = (over: Partial<TotpState> = {}): TotpState => ({
  code: "123456",
  remaining_seconds: 25,
  period: 30,
  ...over,
});

beforeEach(() => {
  vi.useFakeTimers();
  computeTotp.mockResolvedValue(state());
  copyTotp.mockResolvedValue(undefined);
});

afterEach(() => {
  // Cancel the polling interval without firing pending callbacks (which would
  // update state on an about-to-unmount component, outside act()).
  vi.clearAllTimers();
  vi.useRealTimers();
});

describe("TotpDisplay", () => {
  it("shows the computing placeholder, then the grouped code", async () => {
    render(<TotpDisplay itemId="a" showCode />);
    expect(screen.getByText("計算中…")).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0); // flush the immediate tick()
    });

    expect(computeTotp).toHaveBeenCalledWith("a");
    // 6-digit codes are rendered as two groups of three.
    expect(screen.getByText("123 456")).toBeInTheDocument();
  });

  it("re-polls every second and reflects the new code", async () => {
    render(<TotpDisplay itemId="a" showCode />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(computeTotp).toHaveBeenCalledTimes(1);

    computeTotp.mockResolvedValue(state({ code: "654321", remaining_seconds: 24 }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });

    expect(computeTotp).toHaveBeenCalledTimes(2);
    expect(screen.getByText("654 321")).toBeInTheDocument();
  });

  it("masks the code when showCode is false", async () => {
    render(<TotpDisplay itemId="a" showCode={false} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText("● ● ● ● ● ●")).toBeInTheDocument();
    expect(screen.queryByText("123 456")).not.toBeInTheDocument();
  });

  it("renders an AppError message when computeTotp fails", async () => {
    computeTotp.mockRejectedValue({ kind: "InvalidTotpSecret", message: "bad secret" });
    render(<TotpDisplay itemId="a" showCode />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText("bad secret")).toBeInTheDocument();
  });

  it("stops polling after unmount", async () => {
    const { unmount } = render(<TotpDisplay itemId="a" showCode />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(computeTotp).toHaveBeenCalledTimes(1);

    unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(computeTotp).toHaveBeenCalledTimes(1);
  });

  it("copies the code and fires onCopy", async () => {
    const onCopy = vi.fn();
    render(<TotpDisplay itemId="a" showCode onCopy={onCopy} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "複製" }));
      await vi.advanceTimersByTimeAsync(0); // flush copyTotp resolution
    });

    expect(copyTotp).toHaveBeenCalledWith("a");
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "已複製 ✓" })).toBeInTheDocument();
  });
});
