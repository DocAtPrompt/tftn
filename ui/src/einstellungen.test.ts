import { describe, expect, it } from "vitest";
import { type EinstellungenJs, einstellHinweise, tasteDurchlassen } from "./einstellungen";

const basis: EinstellungenJs = {
  ordner: "/x",
  synchronisiert: true,
  hinweis: null,
  ganz_offline: false,
  kuerzel: {},
  fehler: null,
  warnung: null,
};

describe("einstellHinweise", () => {
  it("fehlerhafte Datei sperrt, übersprungene Einträge nicht", () => {
    expect(einstellHinweise({ ...basis, fehler: "tasten.toml, Zeile 2: x" }, []).gesperrt).toBe(true);
    const w = einstellHinweise({ ...basis, warnung: "tasten.toml: kein Text bei a – übersprungen" }, ["b: „Mod-Strg“ ist kein gültiges Kürzel – ignoriert."]);
    expect(w.gesperrt).toBe(false);
    expect(w.warnungen).toHaveLength(2);
    expect(w.warnungen[1]).toContain("Mod-Strg");
  });

  it("ohne Befund nichts", () => {
    expect(einstellHinweise(basis, [])).toEqual({ gesperrt: false, fehler: null, warnungen: [] });
  });
});

describe("tasteDurchlassen", () => {
  it("Tippen ins Such- oder Pfadfeld geht durch, auch während ein Kürzel aufgenommen wird", () => {
    expect(tasteDurchlassen({ tagName: "INPUT", type: "search" })).toBe(true);
    expect(tasteDurchlassen({ tagName: "INPUT", type: "text" })).toBe(true);
    expect(tasteDurchlassen({ tagName: "TEXTAREA" })).toBe(true);
  });

  it("Tabelle, Knöpfe und Schalter fangen die Taste für die Aufnahme ab", () => {
    expect(tasteDurchlassen({ tagName: "TR" })).toBe(false);
    expect(tasteDurchlassen({ tagName: "BUTTON" })).toBe(false);
    expect(tasteDurchlassen({ tagName: "INPUT", type: "checkbox" })).toBe(false);
    expect(tasteDurchlassen(null)).toBe(false);
  });
});
