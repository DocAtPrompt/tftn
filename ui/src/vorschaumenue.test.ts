import { describe, expect, it } from "vitest";
import type { HierJs } from "./seitenleiste";
import type { FeldJs } from "./werte";
import { menueEintraege } from "./vorschaumenue";

const feld = (parameter: string, art: string, gesperrt: string | null = null): FeldJs => ({
  parameter,
  titel: parameter,
  hilfe: null,
  standard: null,
  art: { art },
  wert: null,
  quelltext: null,
  gesperrt,
  fehlt: false,
});

const haupt = { art: "haupt" as const, rev: 1, pos: 5 };

describe("menueEintraege", () => {
  it("menue_fuer_hinweis", () => {
    const hier: HierJs = {
      rev: 1,
      ebenen: [{ baustein: "Hinweis", funktion: "hinweis", von: 1, bis: 30, felder: [feld("art", "wahl"), feld("farbe", "farbe", "berechnet – im Text bearbeiten")] }],
      markup: [],
      tabelle: null,
    };
    const m = menueEintraege(haupt, hier);
    expect(m.map((e) => e.art)).toEqual(["titel", "aendern", "wert", "code"]);
    expect(m[0]).toEqual({ art: "titel", text: "Hinweis" });
    expect(m[2]).toMatchObject({ art: "wert", aufruf: 1, feld: { parameter: "art" } });
  });

  it("zitat_im_menue", () => {
    const hier: HierJs = {
      rev: 1,
      ebenen: [],
      markup: [{ art: "zitat", ebene: null, aufzaehlung: null, block: null, schluessel: "mei2020", zusatz: "S. 3" }],
      tabelle: null,
    };
    expect(menueEintraege(haupt, hier)).toEqual([{ art: "titel", text: "Zitat" }, { art: "aendern" }, { art: "code" }]);
  });

  it("feld_ergibt_formular", () => {
    expect(menueEintraege({ art: "feld", feld: "titel" }, null)).toEqual([
      { art: "titel", text: "Titelei" },
      { art: "formular", feld: "titel" },
    ]);
  });

  it("menue_fuer_vorlage_und_nichts", () => {
    expect(menueEintraege({ art: "vorlage" }, null)).toEqual([{ art: "formular", feld: null }]);
    expect(menueEintraege({ art: "nichts" }, null)).toEqual([]);
    expect(menueEintraege({ art: "datei", datei: "lib.typ" }, null)).toEqual([]);
    expect(menueEintraege({ art: "url", url: "https://x" }, null)).toEqual([]);
  });

  it("menue_fuer_tabelle", () => {
    const hier: HierJs = {
      rev: 1,
      ebenen: [],
      markup: [],
      tabelle: { von: 1, bis: 20, tabelle: { spalten: 1, zeilen: 1, spuren: null, weitere: [], zellen: [] }, gesperrt: null },
    };
    expect(menueEintraege(haupt, hier)).toEqual([{ art: "titel", text: "Tabelle" }, { art: "aendern" }, { art: "code" }]);
  });

  it("markup_und_nichts_einzustellen", () => {
    const ueberschrift: HierJs = { rev: 1, ebenen: [], markup: [{ art: "ueberschrift", ebene: 1, aufzaehlung: null, block: null }], tabelle: null };
    expect(menueEintraege(haupt, ueberschrift)).toEqual([{ art: "titel", text: "Überschrift" }, { art: "aendern" }, { art: "code" }]);
    expect(menueEintraege(haupt, { rev: 1, ebenen: [], markup: [], tabelle: null })).toEqual([{ art: "code" }]);
    expect(menueEintraege(haupt, null)).toEqual([{ art: "code" }]);
  });

  it("menue_formel_bietet_das_formelfeld", () => {
    const hier: HierJs = { rev: 1, ebenen: [], markup: [{ art: "formel", block: true }] as HierJs["markup"], tabelle: null };
    expect(menueEintraege(haupt, hier).map((e) => e.art)).toEqual(["titel", "aendern", "formel", "code"]);
  });
});
