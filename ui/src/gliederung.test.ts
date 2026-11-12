import { describe, expect, it } from "vitest";
import { type EintragJs, aktuell, nachbarZiel, zielErlaubt } from "./gliederung";

// = A (0) / == A1 (1) / == A2 (2) / = B (3) / = C (4) / == C1 (5)
const e = (ebene: number, von: number): EintragJs => ({ ebene, titel: "", von, bis: 0 });
const liste = [e(1, 10), e(2, 20), e(2, 30), e(1, 40), e(1, 50), e(2, 60)];

describe("aktuell", () => {
  it("der Abschnitt, in dem der Cursor steht", () => {
    expect(aktuell(liste, 5)).toBe(-1);
    expect(aktuell(liste, 10)).toBe(0);
    expect(aktuell(liste, 35)).toBe(2);
    expect(aktuell(liste, 999)).toBe(5);
  });
});

describe("zielErlaubt", () => {
  it("nicht in den eigenen Unterabschnitt, nicht an die eigene Stelle", () => {
    expect(zielErlaubt(liste, 0, 1)).toBe(false);
    expect(zielErlaubt(liste, 0, 2)).toBe(false);
    expect(zielErlaubt(liste, 0, 0)).toBe(false);
    expect(zielErlaubt(liste, 0, 3)).toBe(false); // schon dort
    expect(zielErlaubt(liste, 0, 4)).toBe(true);
    expect(zielErlaubt(liste, 3, 0)).toBe(true);
    expect(zielErlaubt(liste, 0, null)).toBe(true);
    expect(zielErlaubt(liste, 4, null)).toBe(false); // schon am Ende
  });
});

describe("nachbarZiel", () => {
  it("nach oben vor den vorigen Abschnitt gleicher Ebene", () => {
    expect(nachbarZiel(liste, 3, "hoch")).toBe(0);
    expect(nachbarZiel(liste, 2, "hoch")).toBe(1);
    expect(nachbarZiel(liste, 1, "hoch")).toBeUndefined(); // erster im Kapitel
    expect(nachbarZiel(liste, 0, "hoch")).toBeUndefined();
  });
  it("nach unten hinter den nächsten Abschnitt gleicher Ebene", () => {
    expect(nachbarZiel(liste, 0, "runter")).toBe(4);
    expect(nachbarZiel(liste, 3, "runter")).toBeNull(); // ans Ende
    expect(nachbarZiel(liste, 1, "runter")).toBe(3); // hinter A2, also vor B
    expect(nachbarZiel(liste, 2, "runter")).toBeUndefined(); // letzter im Kapitel
    expect(nachbarZiel(liste, 4, "runter")).toBeUndefined();
  });
});
