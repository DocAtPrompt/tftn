import { describe, expect, it } from "vitest";
import { type WidgetArt, type Wert, eingabeZuWert, wahlOptionen, wertZuEingabe } from "./werte";

const zahl = (mit: object = {}): WidgetArt => ({ art: "zahl", mit: { min: null, max: null, ganzzahl: false, ...mit } });
const laenge = (einheiten: string[] = []): WidgetArt => ({ art: "laenge", mit: { einheiten } });
const prozent = (mit: object = {}): WidgetArt => ({ art: "prozent", mit: { min: null, max: null, ...mit } });

function wert(art: WidgetArt, text: string): Wert {
  const e = eingabeZuWert(art, text);
  if ("fehler" in e) throw new Error(`unerwarteter Fehler: ${e.fehler}`);
  return e.wert;
}

function fehler(art: WidgetArt, text: string): string {
  const e = eingabeZuWert(art, text);
  if ("wert" in e) throw new Error(`unerwartet gültig: ${JSON.stringify(e.wert)}`);
  return e.fehler;
}

describe("eingaben_deutsch", () => {
  it("zahl", () => {
    expect(wert(zahl(), "1,5")).toEqual({ art: "zahl", wert: 1.5 });
    expect(wert(zahl(), " 1.5 ")).toEqual({ art: "zahl", wert: 1.5 });
    expect(wert(zahl(), "-2")).toEqual({ art: "zahl", wert: -2 });
    expect(fehler(zahl({ ganzzahl: true }), "1,5")).toContain("ganze Zahl");
    expect(fehler(zahl(), "")).toBe("leer – Wert eingeben");
    expect(fehler(zahl(), "zwei")).toContain("Zahl");
    expect(fehler(zahl({ min: 1 }), "0")).toBe("mindestens 1");
    expect(fehler(zahl({ max: 6 }), "7")).toBe("höchstens 6");
  });

  it("tausenderpunkt_ist_mehrdeutig", () => {
    // Abschlussprüfung: „1.000“ meint im Deutschen tausend; nie still als 1 schreiben.
    expect(fehler(zahl(), "1.000")).toContain("mehrdeutig");
    expect(fehler(zahl(), "-12.500")).toContain("mehrdeutig");
    expect(fehler(laenge(), "1.000 mm")).toContain("mehrdeutig");
    expect(fehler(prozent(), "1.000 %")).toContain("mehrdeutig");
    expect(fehler({ art: "punkt" }, "1.000; 2")).toContain("mehrdeutig");
    expect(fehler({ art: "punkt" }, "1cm; 2.000cm")).toContain("mehrdeutig");
    expect(wert(zahl(), "1,000")).toEqual({ art: "zahl", wert: 1 });
    expect(wert(zahl(), "1.5")).toEqual({ art: "zahl", wert: 1.5 });
    expect(wert(zahl(), "1000")).toEqual({ art: "zahl", wert: 1000 });
  });

  it("laenge", () => {
    expect(wert(laenge(), "12 pt")).toEqual({ art: "laenge", wert: { betrag: 12, einheit: "pt" } });
    expect(wert(laenge(), "2,5cm")).toEqual({ art: "laenge", wert: { betrag: 2.5, einheit: "cm" } });
    expect(fehler(laenge(), "12px")).toContain("Einheit");
    expect(fehler(laenge(["mm"]), "12 pt")).toContain("Einheit");
    expect(fehler(laenge(), "12")).toContain("Einheit");
  });

  it("prozent", () => {
    expect(wert(prozent(), "70 %")).toEqual({ art: "prozent", wert: 70 });
    expect(wert(prozent(), "70")).toEqual({ art: "prozent", wert: 70 });
    expect(wert(prozent(), "12,5%")).toEqual({ art: "prozent", wert: 12.5 });
    expect(fehler(prozent({ min: 1 }), "0")).toBe("mindestens 1");
  });

  it("datum", () => {
    expect(wert({ art: "datum" }, "2026-11-07")).toEqual({ art: "datum", wert: { jahr: 2026, monat: 11, tag: 7 } });
    expect(fehler({ art: "datum" }, "2026-02-30")).toContain("Datum");
    expect(fehler({ art: "datum" }, "7.11.2026")).toContain("Datum");
  });

  it("punkt und liste", () => {
    expect(wert({ art: "punkt" }, "1cm; 2,5cm")).toEqual({
      art: "punkt",
      wert: [
        { art: "laenge", wert: { betrag: 1, einheit: "cm" } },
        { art: "laenge", wert: { betrag: 2.5, einheit: "cm" } },
      ],
    });
    expect(wert({ art: "punkt" }, "1; 2")).toEqual({
      art: "punkt",
      wert: [
        { art: "zahl", wert: 1 },
        { art: "zahl", wert: 2 },
      ],
    });
    expect(fehler({ art: "punkt" }, "1cm")).toContain("zwei");
    const liste: WidgetArt = { art: "liste", mit: { element: { art: "text", mit: { mehrzeilig: false } } } };
    expect(wert(liste, "a\nb")).toEqual({
      art: "liste",
      wert: [
        { art: "text", wert: "a" },
        { art: "text", wert: "b" },
      ],
    });
    expect(wert(liste, "")).toEqual({ art: "liste", wert: [] });
  });

  it("text, inhalt, wahl, datei, label, farbe, schalter", () => {
    expect(wert({ art: "text", mit: { mehrzeilig: false } }, "")).toEqual({ art: "text", wert: "" });
    expect(fehler({ art: "text", mit: { mehrzeilig: false } }, "a\nb")).toContain("Zeilenumbruch");
    expect(wert({ art: "text", mit: { mehrzeilig: true } }, "a\nb")).toEqual({ art: "text", wert: "a\nb" });
    expect(wert({ art: "inhalt", mit: { rumpf: true } }, "*fett*")).toEqual({ art: "inhalt", wert: "*fett*" });
    expect(wert({ art: "wahl", mit: { werte: ["info", "merke"] } }, "merke")).toEqual({ art: "text", wert: "merke" });
    expect(fehler({ art: "wahl", mit: { werte: ["info"] } }, "x")).toContain("info");
    expect(wert({ art: "datei", mit: { endungen: ["png"] } }, "bilder/a.PNG")).toEqual({ art: "text", wert: "bilder/a.PNG" });
    expect(fehler({ art: "datei", mit: { endungen: ["png"] } }, "a.jpg")).toContain("Endung");
    expect(wert({ art: "label" }, "abb-1")).toEqual({ art: "label", wert: "abb-1" });
    expect(fehler({ art: "label" }, "a b")).toContain("Label");
    expect(wert({ art: "farbe", mit: { raum: "rgb" } }, "#0a7fFF")).toEqual({ art: "farbe", wert: { rgb: [10, 127, 255] } });
    expect(wert({ art: "farbe", mit: { raum: "cmyk" } }, "0; 50; 100; 0")).toEqual({ art: "farbe", wert: { cmyk: [0, 50, 100, 0] } });
    expect(fehler({ art: "farbe", mit: { raum: "cmyk" } }, "0; 50; 120; 0")).toContain("0 bis 100");
    expect(wert({ art: "schalter" }, "true")).toEqual({ art: "schalter", wert: true });
    expect(wert({ art: "schalter" }, "false")).toEqual({ art: "schalter", wert: false });
  });
});

describe("rueckweg", () => {
  const faelle: [WidgetArt, Wert[]][] = [
    [zahl(), [0, 1, -1, 1.5, -0.25, 12, 100, 1e-3, 3.14159, 1000000].map((w) => ({ art: "zahl", wert: w }))],
    [
      laenge(),
      [0, 1, 1.5, 12, -3, 0.25, 100, 2.54, 10, 7].map((b, i) => ({
        art: "laenge",
        wert: { betrag: b, einheit: ["pt", "mm", "cm", "in", "em"][i % 5] },
      })),
    ],
    [prozent(), [0, 1, 50, 70, 100, 12.5, 33.3, 99, 5, 150].map((w) => ({ art: "prozent", wert: w }))],
    [
      { art: "datum" },
      ["2026-01-01", "2026-11-07", "2000-02-29", "1999-12-31", "2024-06-15", "1-01-01", "9999-12-31", "2026-03-10", "2026-07-04", "2030-10-31"].map(
        (d) => {
          const [j, m, t] = d.split("-").map(Number);
          return { art: "datum", wert: { jahr: j, monat: m, tag: t } };
        },
      ),
    ],
    [{ art: "farbe", mit: { raum: "rgb" } }, [[0, 0, 0], [255, 255, 255], [10, 127, 255], [1, 2, 3]].map((rgb) => ({ art: "farbe", wert: { rgb } }))],
    [{ art: "farbe", mit: { raum: "cmyk" } }, [[0, 0, 0, 0], [100, 100, 100, 100], [0, 50, 100, 0]].map((cmyk) => ({ art: "farbe", wert: { cmyk } }))],
    [{ art: "schalter" }, [true, false].map((w) => ({ art: "schalter", wert: w }))],
    [{ art: "text", mit: { mehrzeilig: true } }, ["", "a", "Größe", "a\nb", "„Zitat“", "😀", "x, y", "a;b", " leer ", "\\"].map((w) => ({ art: "text", wert: w }))],
    [{ art: "inhalt", mit: { rumpf: false } }, ["", "*fett*", "$x$", "a\nb", "#emph[e]"].map((w) => ({ art: "inhalt", wert: w }))],
    [{ art: "label" }, ["a", "abb-1", "tab_2", "fig.3"].map((w) => ({ art: "label", wert: w }))],
    [
      { art: "punkt" },
      [
        { art: "punkt", wert: [{ art: "zahl", wert: 1 }, { art: "zahl", wert: 2 }] },
        { art: "punkt", wert: [{ art: "laenge", wert: { betrag: 1.5, einheit: "cm" } }, { art: "laenge", wert: { betrag: -2, einheit: "pt" } }] },
      ],
    ],
    [
      { art: "liste", mit: { element: { art: "text", mit: { mehrzeilig: false } } } },
      [
        { art: "liste", wert: [] },
        { art: "liste", wert: [{ art: "text", wert: "a" }] },
        { art: "liste", wert: [{ art: "text", wert: "a" }, { art: "text", wert: "b c" }] },
      ],
    ],
  ];
  for (const [art, werte] of faelle) {
    it(art.art + (art.mit?.raum ? `/${art.mit.raum}` : ""), () => {
      for (const w of werte) {
        expect(eingabeZuWert(art, wertZuEingabe(art, w))).toEqual({ wert: w });
      }
    });
  }
});

describe("wahlOptionen", () => {
  it("ein Wert außerhalb der Liste erscheint als eigene, gewählte Option statt der ersten", () => {
    const o = wahlOptionen(["info", "warnung"], "hinweis", false);
    expect(o.filter((x) => x.gewaehlt).map((x) => x.wert)).toEqual(["hinweis"]);
    expect(o[0].text).toContain("hinweis");
    expect(o.slice(1).map((x) => x.wert)).toEqual(["info", "warnung"]);
  });
  it("Wert in der Liste und fehlender Wert", () => {
    expect(wahlOptionen(["a", "b"], "b", false).map((x) => [x.wert, x.gewaehlt])).toEqual([["a", false], ["b", true]]);
    const leer = wahlOptionen(["a", "b"], undefined, true);
    expect(leer[0]).toEqual({ wert: "", text: "—", gewaehlt: true });
    expect(leer.filter((x) => x.gewaehlt)).toHaveLength(1);
  });
});
