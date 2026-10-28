import { defineConfig, devices } from "@playwright/test";

/**
 * End-to-end tests.
 *
 * The suite drives the real UI in a browser against the in-memory mock
 * backend (see `src/lib/mock.ts`). The Windows-only behaviour — the NSIS
 * installer, the native print dialog, WebView2 PDF generation and the
 * auto-updater — cannot be exercised here and is covered by the manual
 * checklist in `docs/VERIFICATION.md`.
 */
export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"]],
  timeout: 30_000,
  use: {
    baseURL: "http://localhost:4173",
    trace: "on-first-retry",
    locale: "ar-EG",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "npm run preview -- --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
