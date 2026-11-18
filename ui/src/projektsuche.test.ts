import { describe, expect, it } from "vitest";
import {
  GRENZE,
  type SuchAnfrage,
  alleErsetzenFrage,
  anfrageFehler,
  ausschnitt,
  ersatzFuer,
  ersetzungen,
  fundBei,
  fundeIn,
  lf,
  projektSuchen,
  zusammenfassung,
} from "./projektsuche";
import { aktionDerTaste, aktionen, belegen, konflikte } from "./tasten";

const anfrage = (suche: string, mehr: Partial<SuchAnfrage> = {}): SuchAnfrage => ({
  suche,
  ersetzen: "",
  grossKlein: false,
  ganzesWort: false,
  regex: false,
  ...mehr,
});

/** Ersetzt wie eine Transaktion: alle Änderungen auf einmal, von hinten. */
function anwenden(text: string, a: SuchAnfrage): string {
  return ersetzungen(text, a)
    .reverse()
    .reduce((s, c) => s.slice(0, c.from) + c.insert + s.slice(c.to), text);
}

describe("Im ganzen Dokument suchen", () => {
  const dateien = [
    { datei: "main.typ", text: "= Einleitung\nDie Größe der Straße.\n#include \"kapitel/eins.typ\"\n" },
    { datei: "kapitel/eins.typ", text: "= Methode\nGRÖSSE, Größe und größe\n" },
    { datei: "kapitel/leer.typ", text: "nichts\n" },
  ];

  it("gruppiert nach Datei mit Zeile und Ausschnitt, ohne Groß/klein auch mit Umlauten", () => {
    const e = projektSuchen(dateien, anfrage("größe"));
    expect(e.dateien.map((d) => [d.datei, d.anzahl])).toEqual([
      ["main.typ", 1],
      ["kapitel/eins.typ", 2],
    ]);
    expect(e.gesamt).toBe(3);
    expect(e.begrenzt).toBe(false);
    const [t1] = e.dateien[0].treffer;
    expect(t1).toMatchObject({ zeile: 2, vorher: "Die ", mitte: "Größe", nachher: " der Straße.", text: "Größe" });
    expect(dateien[0].text.slice(t1.von, t1.bis)).toBe("Größe");
    // „GRÖSSE“ ist kein „größe“ (ß wird nicht zu SS), „größe“ schon.
    expect(e.dateien[1].treffer.map((x) => x.mitte)).toEqual(["Größe", "größe"]);
    expect(zusammenfassung(e)).toBe("3 Treffer in 2 Dateien");
  });

  it("Groß/klein und ganzes Wort wie in der Suchleiste", () => {
    const text = "Straße straße Straßenbahn Außenstraße";
    expect(fundeIn(text, anfrage("straße")).length).toBe(4);
    expect(fundeIn(text, anfrage("straße", { grossKlein: true })).length).toBe(2);
    expect(fundeIn(text, anfrage("straße", { ganzesWort: true })).map((f) => f.von)).toEqual([0, 7]);
    expect(fundeIn("Äpfel äpfel", anfrage("äpfel", { ganzesWort: true, grossKlein: true })).map((f) => f.von)).toEqual([6]);
  });

  it("Ersetzen mit \\n und regulärem Ausdruck mit $1, $& und $$", () => {
    expect(anwenden("a-b a-b", anfrage("-", { ersetzen: "\\n" }))).toBe("a\nb a\nb");
    const re = anfrage("(\\p{L}+)@(\\w+)", { regex: true, ersetzen: "$2 bei $1 ($&, $$)" });
    expect(anwenden("Jörg@praxis", re)).toBe("praxis bei Jörg (Jörg@praxis, $)");
    // `$12` mit nur einer Gruppe: $1, dann die „2“ als Text, wie CodeMirror.
    expect(ersatzFuer(anfrage("(a)", { regex: true, ersetzen: "$12" }), { von: 0, bis: 1, gruppen: ["a", "a"] })).toBe("a2");
    // Ohne regulären Ausdruck bleibt $1 stehen.
    expect(anwenden("x", anfrage("x", { ersetzen: "$1" }))).toBe("$1");
    // Zeilenanfänge
    expect(anwenden("eins\nzwei", anfrage("^", { regex: true, ersetzen: "- " }))).toBe("- eins\n- zwei");
  });

  it("Ersetzen eines Treffers nur, wenn die Stelle noch stimmt", () => {
    const a = anfrage("Größe");
    expect(fundBei("Die Größe", a, 4, 9)).toMatchObject({ von: 4, bis: 9 });
    expect(fundBei("Die neue Größe", a, 4, 9)).toBeNull();
  });

  it("begrenzt die Liste, zählt aber weiter", () => {
    const viele = [
      { datei: "a.typ", text: "x ".repeat(GRENZE - 10) },
      { datei: "b.typ", text: "x ".repeat(50) },
      { datei: "c.typ", text: "x" },
    ];
    const e = projektSuchen(viele, anfrage("x"));
    expect(e.gesamt).toBe(GRENZE + 41);
    expect(e.begrenzt).toBe(true);
    expect(e.dateien.map((d) => d.treffer.length)).toEqual([GRENZE - 10, 10, 0]);
    expect(e.dateien.map((d) => d.anzahl)).toEqual([GRENZE - 10, 50, 1]);
    expect(zusammenfassung(e)).toBe("1.041 Treffer in 3 Dateien · die Liste zeigt die ersten 1.000");
    expect(alleErsetzenFrage(e)).toBe("1.041 Treffer in 3 Dateien ersetzen?");
  });

  it("verständliche Meldung bei kaputtem regulärem Ausdruck, keine Treffer", () => {
    expect(anfrageFehler(anfrage("(ab", { regex: true }))).toBe(
      "Kein gültiger regulärer Ausdruck: runde Klammern passen nicht zusammen.",
    );
    expect(anfrageFehler(anfrage("[ab", { regex: true }))).toBe(
      "Kein gültiger regulärer Ausdruck: eine eckige Klammer [ ist nicht geschlossen.",
    );
    expect(anfrageFehler(anfrage("*a", { regex: true }))).toBe("Kein gültiger regulärer Ausdruck: *, + oder ? steht ohne etwas davor.");
    expect(anfrageFehler(anfrage("(ab", { regex: false }))).toBeNull();
    expect(anfrageFehler(anfrage("a(b)", { regex: true }))).toBeNull();
    const e = projektSuchen(dateien, anfrage("(ab", { regex: true }));
    expect(e.gesamt).toBe(0);
    expect(zusammenfassung(e)).toBe("Keine Treffer.");
  });

  it("Ausschnitt: lange Zeilen gekürzt, Treffer über mehrere Zeilen mit „…“", () => {
    const lang = "a".repeat(100) + "TREFFER" + "b".repeat(100);
    const s = ausschnitt(lang, 100, 107);
    expect(s.vorher).toBe("…" + "a".repeat(40));
    expect(s.mitte).toBe("TREFFER");
    expect(s.nachher).toBe("b".repeat(40) + "…");
    const m = ausschnitt("eins\n  zwei drei\nvier", 7, 18);
    expect(m).toEqual({ zeile: 2, vorher: "", mitte: "zwei drei", nachher: " …" });
    const re = projektSuchen([{ datei: "a.typ", text: "eins\nzwei" }], anfrage("s\\nz", { regex: true }));
    expect(re.gesamt).toBe(1);
    expect(re.dateien[0].treffer[0]).toMatchObject({ zeile: 1, mitte: "s", nachher: " …", text: "s\nz" });
  });

  it("Zeilenenden \\r\\n zählen wie im Editor als ein Zeichen", () => {
    expect(lf("a\r\nb\rc")).toBe("a\nb\nc");
    const e = projektSuchen([{ datei: "a.typ", text: "x\r\nGröße" }], anfrage("größe"));
    expect(e.dateien[0].treffer[0]).toMatchObject({ von: 2, bis: 7, zeile: 2 });
  });

  it("Surrogatpaare (Emoji) zählen wie im Editor in UTF-16", () => {
    const e = projektSuchen([{ datei: "a.typ", text: "😀 Größe" }], anfrage("Größe"));
    expect(e.dateien[0].treffer[0]).toMatchObject({ von: 3, bis: 8 });
  });
});

describe("Kürzel „Im Projekt suchen“", () => {
  const taste = (key: string, keyCode: number, mehr: Partial<{ metaKey: boolean; ctrlKey: boolean; shiftKey: boolean }> = {}) => ({
    key, code: "", keyCode, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, ...mehr,
  });
  const a = aktionen([], []);

  it("Cmd/Strg+Umschalt+H auf beiden Plattformen", () => {
    expect(aktionDerTaste(belegen(a, {}, "mac"), taste("H", 72, { metaKey: true, shiftKey: true }), "mac")).toBe("helfer/projektsuche");
    expect(aktionDerTaste(belegen(a, {}, "win"), taste("H", 72, { ctrlKey: true, shiftKey: true }), "win")).toBe("helfer/projektsuche");
  });

  it("ohne Konflikt mit System, Editor-Kürzeln und anderen Aktionen", () => {
    expect(konflikte(a, {}).filter((k) => k.aktion === "helfer/projektsuche")).toEqual([]);
  });
});
