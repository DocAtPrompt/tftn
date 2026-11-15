import { describe, expect, it } from "vitest";
import { lageImStil, platzieren } from "./platz";

const fenster = { breite: 800, hoehe: 600 };

describe("platzieren", () => {
  it("unter dem Anker, wenn Platz ist", () => {
    expect(platzieren({ x: 100, oben: 90, unten: 110 }, { breite: 200, hoehe: 100 }, fenster)).toEqual({ links: 100, oben: 114 });
  });
  it("rechts und unten nicht über den Rand", () => {
    const p = platzieren({ x: 750, oben: 560, unten: 580 }, { breite: 280, hoehe: 300 }, fenster);
    expect(p.links + 280).toBeLessThanOrEqual(800 - 8);
    // unten kein Platz: über dem Anker
    expect(p.oben + 300).toBeLessThanOrEqual(556);
  });
  it("höher als das Fenster: oben bündig mit Rand", () => {
    expect(platzieren({ x: 10, oben: 300, unten: 320 }, { breite: 100, hoehe: 900 }, fenster).oben).toBe(8);
  });
  it("links nie unter den Rand", () => {
    expect(platzieren({ x: -50, oben: 0, unten: 10 }, { breite: 100, hoehe: 50 }, fenster).links).toBe(8);
  });
});

describe("lageImStil", () => {
  it("rechnet die Lage in Pixel des gezoomten Elements um", () => {
    // Gemessen (sichtbar) 260 × 130 bei Zoom 1,3; unter dem Knopf rechts im Fenster.
    const l = lageImStil({ x: 700, oben: 10, unten: 40 }, { breite: 260, hoehe: 130 }, fenster, 1.3);
    // sichtbar: links = 800 - 260 - 8 = 532, oben = 44 → im Stil durch 1,3
    expect(l.links).toBeCloseTo(532 / 1.3);
    expect(l.oben).toBeCloseTo(44 / 1.3);
  });
  it("ohne Zoom wie platzieren", () => {
    const a = { x: 20, oben: 10, unten: 40 };
    const g = { breite: 100, hoehe: 50 };
    expect(lageImStil(a, g, fenster, 1)).toEqual(platzieren(a, g, fenster));
  });
});
