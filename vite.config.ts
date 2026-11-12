import { defineConfig } from "vitest/config";

// UI liegt in ui/, die Ausgabe in dist/ auf Projektebene (KONZEPT 3.4: nur dort von Tresorit ausgeschlossen).
export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/target/**"] } },
  build: { outDir: "../dist", emptyOutDir: true, target: "es2022" },
  test: { root: ".", include: ["ui/src/**/*.test.ts"] },
});
