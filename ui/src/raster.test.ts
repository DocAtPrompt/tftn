import { describe, expect, it } from "vitest";
import { type Tabelle, type Zelle, aktionFuer, auswahlNach, auswahlRechteck, bearbeitbar } from "./raster";

const z = (zeile: number, spalte: number, zeilen = 1, spalten = 1, text = ""): Zelle => ({
  zeile,
  spalte,
  zeilen,
  spalten,
  inhalt: { art: "markup", text },
  weitere: [],
});

/** 3×3 mit einer 2×2-Zelle bei (1,1) */
const mitBlock: Tabelle = {
  spalten: 3,
  zeilen: 3,
  spuren: null,
  weitere: [],
  zellen: [z(0, 0), z(0, 1), z(0, 2), z(1, 0), z(1, 1, 2, 2), z(2, 0)],
};

/** 3×2 ohne Spannen */
const einfach: Tabelle = {
  spalten: 2,
  zeilen: 3,
  spuren: null,
  weitere: [],
  zellen: [z(0, 0), z(0, 1), z(1, 0), z(1, 1), z(2, 0), z(2, 1)],
};

describe("auswahl_waechst_auf_ganze_zellen", () => {
  it("wächst über die verbundene Zelle", () => {
    expect(auswahlRechteck(mitBlock, [0, 0], [1, 1])).toEqual({ zeile: 0, spalte: 0, zeilen: 3, spalten: 3 });
  });
  it("Klick in die verbundene Zelle wählt sie ganz", () => {
    expect(auswahlRechteck(mitBlock, [2, 2], [2, 2])).toEqual({ zeile: 1, spalte: 1, zeilen: 2, spalten: 2 });
  });
  it("Richtung egal", () => {
    expect(auswahlRechteck(einfach, [2, 1], [1, 0])).toEqual({ zeile: 1, spalte: 0, zeilen: 2, spalten: 2 });
  });
});

describe("aktionen_aus_knoepfen", () => {
  const eins = { zeile: 1, spalte: 0, zeilen: 1, spalten: 1 };
  it("einfügen", () => {
    expect(aktionFuer("zeile-oben", eins, einfach)).toEqual({ art: "zeile-einfuegen", vor: 1 });
    expect(aktionFuer("zeile-unten", eins, einfach)).toEqual({ art: "zeile-einfuegen", vor: 2 });
    expect(aktionFuer("spalte-links", eins, einfach)).toEqual({ art: "spalte-einfuegen", vor: 0 });
    expect(aktionFuer("spalte-rechts", eins, einfach)).toEqual({ art: "spalte-einfuegen", vor: 1 });
    const block = { zeile: 1, spalte: 1, zeilen: 2, spalten: 2 };
    expect(aktionFuer("zeile-unten", block, mitBlock)).toEqual({ art: "zeile-einfuegen", vor: 3 });
    expect(aktionFuer("spalte-rechts", block, mitBlock)).toEqual({ art: "spalte-einfuegen", vor: 3 });
  });
  it("löschen", () => {
    expect(aktionFuer("zeile-loeschen", eins, einfach)).toEqual({ art: "zeile-loeschen", zeile: 1 });
    expect(aktionFuer("spalte-loeschen", eins, einfach)).toEqual({ art: "spalte-loeschen", spalte: 0 });
    expect(aktionFuer("zeile-loeschen", { zeile: 0, spalte: 0, zeilen: 2, spalten: 1 }, einfach)).toBeNull();
    const eineZeile: Tabelle = { ...einfach, zeilen: 1, zellen: [z(0, 0), z(0, 1)] };
    expect(aktionFuer("zeile-loeschen", { zeile: 0, spalte: 0, zeilen: 1, spalten: 1 }, eineZeile)).toBeNull();
  });
  it("verschieben", () => {
    expect(aktionFuer("zeile-hoch", eins, einfach)).toEqual({ art: "zeile-verschieben", von: 1, nach: 0 });
    expect(aktionFuer("zeile-runter", eins, einfach)).toEqual({ art: "zeile-verschieben", von: 1, nach: 2 });
    expect(aktionFuer("zeile-hoch", { ...eins, zeile: 0 }, einfach)).toBeNull();
    expect(aktionFuer("zeile-runter", { ...eins, zeile: 2 }, einfach)).toBeNull();
    expect(aktionFuer("spalte-zurueck", eins, einfach)).toEqual({ art: "spalte-verschieben", von: 0, nach: 1 });
    expect(aktionFuer("spalte-vor", eins, einfach)).toBeNull();
  });
  it("verbinden und trennen", () => {
    expect(aktionFuer("verbinden", eins, einfach)).toBeNull();
    expect(aktionFuer("verbinden", { zeile: 0, spalte: 0, zeilen: 2, spalten: 2 }, einfach)).toEqual({
      art: "verbinden",
      zeile: 0,
      spalte: 0,
      zeilen: 2,
      spalten: 2,
    });
    const block = { zeile: 1, spalte: 1, zeilen: 2, spalten: 2 };
    expect(aktionFuer("verbinden", block, mitBlock)).toBeNull();
    expect(aktionFuer("trennen", block, mitBlock)).toEqual({ art: "trennen", zeile: 1, spalte: 1 });
    expect(aktionFuer("trennen", eins, einfach)).toBeNull();
  });
});

describe("nach_der_aktion", () => {
  const a = { zeile: 1, spalte: 1, zeilen: 1, spalten: 1 };
  it("auswahl_folgt_dem_verschieben", () => {
    expect(auswahlNach({ art: "zeile-verschieben", von: 1, nach: 0 }, a)).toEqual({ ...a, zeile: 0 });
    expect(auswahlNach({ art: "spalte-verschieben", von: 1, nach: 2 }, a)).toEqual({ ...a, spalte: 2 });
  });
  it("einfuegen_davor_schiebt_die_auswahl", () => {
    expect(auswahlNach({ art: "zeile-einfuegen", vor: 1 }, a)).toEqual({ ...a, zeile: 2 });
    expect(auswahlNach({ art: "zeile-einfuegen", vor: 2 }, a)).toEqual(a);
    expect(auswahlNach({ art: "spalte-einfuegen", vor: 0 }, a)).toEqual({ ...a, spalte: 2 });
  });
  it("loeschen_hebt_die_auswahl_auf", () => {
    expect(auswahlNach({ art: "zeile-loeschen", zeile: 1 }, a)).toBeNull();
    expect(auswahlNach({ art: "spalte-loeschen", spalte: 0 }, a)).toBeNull();
  });
  it("verbinden_trennen_zelltext_lassen_sie", () => {
    expect(auswahlNach({ art: "verbinden", zeile: 1, spalte: 1, zeilen: 1, spalten: 1 }, a)).toEqual(a);
    expect(auswahlNach({ art: "zelltext", zeile: 1, spalte: 1, inhalt: "x" }, a)).toEqual(a);
  });
  it("ausdruck_zellen_sind_nicht_als_text_bearbeitbar", () => {
    expect(bearbeitbar(z(0, 0, 1, 1, "a"))).toBe(true);
    expect(bearbeitbar({ ...z(0, 0), inhalt: { art: "ausdruck", text: 'image("a.png")' } })).toBe(false);
  });
});
