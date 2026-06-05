import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

// The provider pulls the OS locale and saved settings on mount; stub both.
const getSystemLocale = vi.fn();
const getSettings = vi.fn();
vi.mock("./ipc", () => ({
  ipc: {
    getSystemLocale: () => getSystemLocale(),
    getSettings: () => getSettings(),
  },
}));

import { resolveSystemLang, resolveLang, I18nProvider, useI18n } from "./i18n";

describe("resolveSystemLang", () => {
  it.each([
    ["zh-TW", "zh-TW"],
    ["zh_Hant", "zh-TW"],
    ["ZH-tw", "zh-TW"],
    ["zh", "zh-TW"],
    ["en-US", "en"],
    ["en", "en"],
    ["fr-FR", "en"],
    ["", "en"],
  ])("maps OS locale %s -> %s", (osLocale, expected) => {
    expect(resolveSystemLang(osLocale)).toBe(expected);
  });
});

describe("resolveLang", () => {
  it("follows the OS locale when pref is 'system'", () => {
    expect(resolveLang("system", "zh-TW")).toBe("zh-TW");
    expect(resolveLang("system", "en-US")).toBe("en");
  });

  it("honours an explicit pref regardless of OS locale", () => {
    expect(resolveLang("en", "zh-TW")).toBe("en");
    expect(resolveLang("zh-TW", "en-US")).toBe("zh-TW");
  });
});

describe("translate (via useI18n.t)", () => {
  beforeEach(() => {
    getSystemLocale.mockResolvedValue("en");
    getSettings.mockResolvedValue({ locale: "system" });
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <I18nProvider>{children}</I18nProvider>
  );

  it("returns the language string for the resolved language", async () => {
    getSettings.mockResolvedValue({ locale: "zh-TW" });
    const { result } = renderHook(() => useI18n(), { wrapper });
    await waitFor(() => expect(result.current.lang).toBe("zh-TW"));
    expect(result.current.t("unlock_btn")).toBe("解鎖");
  });

  it("interpolates {name}-style placeholders", async () => {
    getSettings.mockResolvedValue({ locale: "en" });
    const { result } = renderHook(() => useI18n(), { wrapper });
    await waitFor(() => expect(result.current.lang).toBe("en"));
    expect(result.current.t("settings_import_done", { added: 2, updated: 1 })).toBe(
      "Imported: 2 added, 1 updated",
    );
  });

  it("interpolates every occurrence of a placeholder", async () => {
    getSettings.mockResolvedValue({ locale: "en" });
    const { result } = renderHook(() => useI18n(), { wrapper });
    await waitFor(() => expect(result.current.lang).toBe("en"));
    // settings_export_done has a single {path}; ensure replacement happens.
    expect(result.current.t("settings_export_done", { path: "/tmp/b.kbk" })).toBe(
      "Exported encrypted backup to /tmp/b.kbk",
    );
  });

  it("defaults to following the system locale when settings load fails", async () => {
    getSystemLocale.mockResolvedValue("zh-TW");
    getSettings.mockRejectedValue(new Error("no vault"));
    const { result } = renderHook(() => useI18n(), { wrapper });
    await waitFor(() => expect(result.current.lang).toBe("zh-TW"));
    expect(result.current.t("unlock_btn")).toBe("解鎖");
  });
});
