import { describe, expect, it } from "vitest";
import type { EintragJs } from "./bausteine";
import { eigeneGruppen, textAuszug, textbausteinHinweis, titelVorschlag } from "./textbausteine";

describe("titelVorschlag", () => {
  it("erste Zeile mit Inhalt, ohne Marken davor", () => {
    expect(titelVorschlag("\n\n  Kontrolle in 6 Wochen.\nZweite")).toBe("Kontrolle in 6 Wochen.");
    expect(titelVorschlag("= Procedere\nText")).toBe("Procedere");
    expect(titelVorschlag("- Punkt eins\n- Punkt zwei")).toBe("Punkt eins");
    expect(titelVorschlag("  \n ")).toBe("");
  });

  it("lange Zeilen am Wortende gekürzt", () => {
    const v = titelVorschlag("Der Patient wurde über Risiken, Nebenwirkungen und Alternativen ausführlich aufgeklärt.");
    expect(v.endsWith(" …")).toBe(true);
    expect(v.length).toBeLessThanOrEqual(42);
    expect(v).toBe("Der Patient wurde über Risiken …");
  });
});

describe("eigeneGruppen", () => {
  const e = (herkunft: EintragJs["herkunft"], gruppe: string | null): EintragJs => ({ id: "x", titel: "x", gruppe, herkunft, modus: ["markup"], kuerzel: null });
  it("nur die eigenen, einmal, sortiert", () => {
    expect(eigeneGruppen([e("eigene", "Procedere"), e("typst", "Text"), e("eigene", "Brief"), e("eigene", "Procedere"), e("eigene", null)])).toEqual(["Brief", "Procedere"]);
  });
});

describe("textbausteinHinweis", () => {
  it("meldet neue Fehler und die Reparatur, sonst nichts", () => {
    expect(textbausteinHinweis(null, "textbausteine.toml, Zeile 2: x")).toBe("Eigene Textbausteine: textbausteine.toml, Zeile 2: x");
    expect(textbausteinHinweis("a", "a")).toBeNull();
    expect(textbausteinHinweis("a", null)).toBe("textbausteine.toml ist wieder in Ordnung.");
    expect(textbausteinHinweis(null, null)).toBeNull();
  });
});

describe("textAuszug", () => {
  it("die ersten Zeilen", () => {
    expect(textAuszug("a\nb")).toBe("a\nb");
    expect(textAuszug("1\n2\n3\n4\n5\n6", 4)).toBe("1\n2\n3\n4\n…");
  });
});
