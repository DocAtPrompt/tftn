import { defineConfig } from "vitest/config";

// UI liegt in ui/, die Ausgabe in dist/ auf Projektebene (KONZEPT 3.4: nur dort von Tresorit ausgeschlossen).
export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/target/**"] } },
  build: { outDir: "../dist", emptyOutDir: true, target: "es2022" },
  // CSS nur für stil.test.ts (liest stil.css?raw); sonst ersetzt vitest CSS durch Leeres.
  test: { root: ".", include: ["ui/src/**/*.test.ts"], css: { include: [/stil\.css/] } },
});
