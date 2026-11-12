import { afterEach, describe, expect, it } from "vitest";
import { katalogSetzen, n, t, textErsetzen, tn, zahl } from "./sprache";

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
