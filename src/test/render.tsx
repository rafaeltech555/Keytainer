import { render } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { I18nProvider } from "../lib/i18n";

/**
 * Render a component wrapped in the I18nProvider. The provider pulls the OS
 * locale and saved settings from the (mocked) ipc on mount, so each test file
 * must mock `../lib/ipc` with `getSystemLocale` + `getSettings` resolving — set
 * them to "en" to assert against the reference English strings.
 */
export function renderWithI18n(ui: ReactElement) {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <I18nProvider>{children}</I18nProvider>
  );
  return render(ui, { wrapper });
}
