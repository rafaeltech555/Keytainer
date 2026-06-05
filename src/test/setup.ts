import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// React Testing Library doesn't auto-clean with Vitest; unmount between tests.
afterEach(() => {
  cleanup();
});
