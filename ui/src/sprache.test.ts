import { afterEach, describe, expect, it } from "vitest";
import { katalogSetzen, n, t, texteUebersetzen, textErsetzen, tn, zahl } from "./sprache";

afterEach(() => katalogSetzen({}));

describe("Sprache der Oberfläche", () => {
  it("ohne Übersetzung bleibt der deutsche Text, Platzhalter werden gefüllt", () => {
    expect(t("Speichern")).toBe("Speichern");
    expect(t("{n} von {m} Wörtern", { n: 3, m: 10 })).toBe("3 von 10 Wörtern");
    expect(t("{unbekannt} bleibt", {})).toBe("{unbekannt} bleibt");
  });

  it("mit Katalog übersetzt t, n markiert nur, tn übersetzt den markierten Wert", () => {
    katalogSetzen({ Speichern: "Save", "{n} von {m} Wörtern": "{n} of {m} words", Fett: "Bold" });
    expect(t("Speichern")).toBe("Save");
    expect(t("{n} von {m} Wörtern", { n: 3, m: 10 })).toBe("3 of 10 words");
    const wert = n("Fett");
    expect(wert).toBe("Fett");
    expect(tn(wert)).toBe("Bold");
    expect(t("Unbekannt")).toBe("Unbekannt");
  });

  it("übersetzt Text der Seite, Leerraum bleibt, Unbekanntes bleibt unberührt", () => {
    katalogSetzen({ "Öffnen …": "Open …" });
    expect(textErsetzen("\n      Öffnen …")).toBe("\n      Open …");
    expect(textErsetzen("…")).toBeNull();
    expect(textErsetzen("  ")).toBeNull();
  });

  it("Tausendertrenner nach Oberflächensprache", () => {
    expect(zahl(1234567.4)).toBe("1.234.567");
    katalogSetzen({}, "en");
    expect(zahl(1234567)).toBe("1,234,567");
  });
});

describe("Texte aus Dokumenttypen und Paketbeschreibungen", () => {
  it("übersetzt sichtbare Felder in jeder Tiefe, Werte bleiben", () => {
    katalogSetzen({ Hinweiskasten: "Note box", Farbe: "Colour", Sprache: "Language", Deutsch: "German", Literatur: "References" });
    const ein = {
      titel: "Hinweiskasten",
      gruppe: null,
      felder: [{ titel: "Farbe", hilfe: "Unbekannt", wert: { art: "text", wert: "Farbe" } }],
      formular: { felder: [{ titel: "Sprache", gruppe: "Literatur", art: { art: "wahl", werte: ["de", "Deutsch"], beschriftungen: ["Deutsch", "Englisch"] } }] },
      werte: { titel: "Literatur" },
      braucht: ["Literatur"],
      standard: "Farbe",
    };
    const aus = texteUebersetzen(ein);
    expect(aus.titel).toBe("Note box");
    expect(aus.gruppe).toBeNull();
    expect(aus.felder[0].titel).toBe("Colour");
    expect(aus.felder[0].hilfe).toBe("Unbekannt");
    expect(aus.felder[0].wert.wert).toBe("Farbe");
    const f = aus.formular.felder[0];
    expect(f.titel).toBe("Language");
    expect(f.gruppe).toBe("References");
    expect(f.art.werte).toEqual(["de", "Deutsch"]);
    expect(f.art.beschriftungen).toEqual(["German", "Englisch"]);
    expect(aus.werte.titel).toBe("Literatur");
    expect(aus.braucht).toEqual(["References"]);
    expect(aus.standard).toBe("Farbe");
  });

  it("lässt Zahlen, null und Listen ohne Objekte unberührt", () => {
    katalogSetzen({ a: "b" });
    expect(texteUebersetzen(null)).toBeNull();
    expect(texteUebersetzen([1, "a"])).toEqual([1, "a"]);
    expect(texteUebersetzen("a")).toBe("a");
  });
});
