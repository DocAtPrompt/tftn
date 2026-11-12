import { describe, expect, it } from "vitest";
import { type Treffer, hinzufuegenKnopf, indexKnopf, indexText, verbindungText, entfernenKnopf, helferText, holbarKnopf, wechselKnopf } from "./pakete";

const treffer = (t: Partial<Treffer> = {}): Treffer => ({
  name: "cetz",
  version: "0.4.2",
  beschreibung: null,
  lizenz: null,
  im_projekt: false,
  im_zwischenspeicher: false,
  helfer: false,
  ...t,
});

describe("Fach „Pakete“ (PLAN-M2 AP9)", () => {
  it("hinzufuegen_knopf", () => {
    expect(hinzufuegenKnopf(treffer({ im_projekt: true }), false, false)).toEqual({ an: false, titel: "Schon im Projekt" });
    const offline = hinzufuegenKnopf(treffer(), true, false);
    expect(offline.an).toBe(false);
    expect(offline.titel).toBe("Ganz offline – nur Pakete im Zwischenspeicher");
    expect(hinzufuegenKnopf(treffer({ im_zwischenspeicher: true }), true, false).an).toBe(true);
    expect(hinzufuegenKnopf(treffer(), false, false).an).toBe(true);
    expect(hinzufuegenKnopf(treffer(), false, false).titel).toContain("packages.typst.org");
    expect(hinzufuegenKnopf(treffer({ im_zwischenspeicher: true }), false, true).an).toBe(false);
  });

  it("index_knopf", () => {
    expect(indexKnopf(null, false, false)).toMatchObject({ an: true, text: "Index holen" });
    expect(indexKnopf({ datum: "2026-11-08", pakete: 3 }, false, false)).toMatchObject({ an: true, text: "Index aktualisieren" });
    const offline = indexKnopf(null, true, false);
    expect(offline.an).toBe(false);
    expect(offline.titel.toLowerCase()).toContain("ganz offline");
    expect(indexKnopf(null, false, true).an).toBe(false);
  });

  it("verbindung_nur_waehrend_lauf", () => {
    expect(verbindungText(true)).toContain("packages.typst.org");
    expect(verbindungText(false)).toBe("");
  });

  it("index_text", () => {
    expect(indexText({ datum: "2026-11-08", pakete: 3 })).toBe("Index vom 8. 11. 2026 · 3 Pakete");
    expect(indexText({ datum: "2026-01-02", pakete: 1 })).toBe("Index vom 2. 1. 2026 · 1 Paket");
    expect(indexText(null)).toBe("Noch kein Index – „Index holen“");
  });
});

describe("Knöpfe AP10", () => {
  it("holbar_offline", () => {
    expect(holbarKnopf({ im_zwischenspeicher: false }, true, false).an).toBe(false);
    expect(holbarKnopf({ im_zwischenspeicher: true }, true, false).an).toBe(true);
    expect(holbarKnopf({ im_zwischenspeicher: false }, false, false).an).toBe(true);
    expect(holbarKnopf({ im_zwischenspeicher: true }, false, true).an).toBe(false);
  });

  it("entfernen_benutzt", () => {
    const k = entfernenKnopf({ benutzt: true }, false);
    expect(k.an).toBe(false);
    expect(k.titel).toContain("wird noch importiert");
    expect(entfernenKnopf({ benutzt: false }, false).an).toBe(true);
    expect(entfernenKnopf({ benutzt: false }, true).an).toBe(false);
  });

  it("wechsel_offline", () => {
    expect(wechselKnopf(true, false)).toMatchObject({ an: false, titel: expect.stringContaining("offline") });
    expect(wechselKnopf(false, false).an).toBe(true);
    expect(wechselKnopf(false, true).an).toBe(false);
  });

  it("helfer_text", () => {
    expect(helferText({ helfer: true }, true)).toBeNull();
    expect(helferText({ helfer: false }, false)).toBeNull();
    expect(helferText({ helfer: false }, true)).toBe("Für diese Version keine Helfer");
  });
});
