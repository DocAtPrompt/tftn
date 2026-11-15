import { describe, expect, it } from "vitest";
import css from "./stil.css?raw";

/** Die Auswahl vor `zoom: var(--ui-zoom)`. */
function gezoomt(): string[] {
  const m = /([^{}]*)\{\s*zoom:\s*var\(--ui-zoom\);?\s*\}/.exec(css);
  return (m?.[1] ?? "").split(",").map((s) => s.trim()).filter((s) => s && !s.startsWith("/*"));
}

describe("Oberflächengröße", () => {
  it("wirkt auf alle Leisten und Dialoge", () => {
    expect(css.length).toBeGreaterThan(1000);
    const liste = gezoomt();
    for (const s of ["#kopf", "#hinweis", "#status", "#editor-knoepfe", ".neu-dialog", "#export-menue", "#baustein-palette", ".knopf-menue", "#seitenleiste", "#palette", "#einstellungen"]) {
      expect(liste, s).toContain(s);
    }
  });
});
