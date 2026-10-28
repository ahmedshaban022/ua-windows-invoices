import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

const root = import.meta.dirname;

// Tauri expects a fixed dev port and does not want vite to fall back to another one.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Windows 10/11 ships WebView2 (Chromium). Target the older baseline so
    // the app also runs on machines with an outdated Evergreen runtime.
    target: "chrome105",
    minify: true,
    sourcemap: false,
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      input: {
        // The application shell.
        main: resolve(root, "index.html"),
        // The chrome-free render host used for PDF / print / image export.
        render: resolve(root, "render.html"),
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: false,
  },
});
