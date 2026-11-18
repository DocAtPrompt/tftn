import { describe, expect, it } from "vitest";
import { ANTEIL_STANDARD, LEISTE_MAX, LEISTE_MIN, SPALTE_MIN, anteilLesen, leisteBreite, textAnteil } from "./breite";

describe("leisteBreite", () => {
  it("nach links ziehen macht die Leiste breiter", () => {
    expect(leisteBreite(340, 800, 700, 2000)).toBe(440);
    expect(leisteBreite(340, 800, 900, 2000)).toBe(LEISTE_MIN);
  });
  it("höchstens bis zur Obergrenze und zum verfügbaren Platz", () => {
    expect(leisteBreite(340, 800, 0, 2000)).toBe(LEISTE_MAX);
    expect(leisteBreite(340, 800, 0, 500)).toBe(500);
    expect(leisteBreite(340, 800, 0, 100)).toBe(LEISTE_MIN);
  });
});

describe("textAnteil", () => {
  it("folgt dem Zeiger, auf Zehntel", () => {
    expect(textAnteil(700, 100, 1200)).toBe(50);
    expect(textAnteil(460, 100, 1200)).toBe(30);
    expect(textAnteil(940, 100, 1200)).toBe(70);
  });
  it("beide Seiten behalten ihren Mindestplatz", () => {
    // 320 von 1200 Pixel sind 26,7 %.
    expect(textAnteil(100, 100, 1200)).toBe(26.7);
    expect(textAnteil(1300, 100, 1200)).toBe(73.3);
    expect(textAnteil(150, 100, 1200, SPALTE_MIN)).toBe(26.7);
  });
  it("zu schmal zum Verteilen: halb und halb", () => {
    expect(textAnteil(300, 0, 600)).toBe(ANTEIL_STANDARD);
  });
});

describe("anteilLesen", () => {
  it("nimmt nur brauchbare Werte", () => {
    expect(anteilLesen("62.5")).toBe(62.5);
    expect(anteilLesen(null)).toBeNull();
    expect(anteilLesen("")).toBeNull();
    expect(anteilLesen("abc")).toBeNull();
    expect(anteilLesen("5")).toBeNull();
    expect(anteilLesen("95")).toBeNull();
  });
});
