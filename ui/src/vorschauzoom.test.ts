import { describe, expect, it } from "vitest";
import { type Zoom, prozentAus, seitenBreite, stufe, zoomLesen } from "./vorschauzoom";

const a4 = { breite: 595, hoehe: 842 };
const spalte = { breite: 632, hoehe: 900 }; // nutzbar: 600 × 868

describe("seitenBreite", () => {
  it("Seitenbreite füllt die Spalte abzüglich Rand", () => {
    expect(seitenBreite({ art: "breite" }, spalte, a4)).toBe(600);
  });
  it("Ganze Seite: so groß, dass die Höhe passt, nie breiter als die Spalte", () => {
    expect(seitenBreite({ art: "seite" }, { breite: 632, hoehe: 800 }, a4)).toBeCloseTo((768 * 595) / 842, 5);
    expect(seitenBreite({ art: "seite" }, spalte, a4)).toBe(600); // Höhe passt schon bei voller Breite
    expect(seitenBreite({ art: "seite" }, { breite: 300, hoehe: 2000 }, a4)).toBe(268);
  });
  it("Prozent: 100 % ist die echte Größe (1 pt = 96/72 px)", () => {
    expect(seitenBreite({ art: "prozent", wert: 100 }, spalte, a4)).toBeCloseTo((595 * 96) / 72, 5);
    expect(seitenBreite({ art: "prozent", wert: 200 }, spalte, a4)).toBeCloseTo((595 * 96 * 2) / 72, 5);
  });
  it("nie kleiner als 100 px", () => {
    expect(seitenBreite({ art: "breite" }, { breite: 10, hoehe: 10 }, a4)).toBe(100);
  });
});

describe("prozentAus", () => {
  it("rechnet eine angezeigte Breite in Prozent der echten Größe um", () => {
    expect(prozentAus((595 * 96) / 72, a4)).toBe(100);
    expect(prozentAus(600, a4)).toBe(76);
  });
});

describe("stufe", () => {
  it("die nächste feste Stufe nach oben bzw. unten", () => {
    expect(stufe(100, 1)).toBe(110);
    expect(stufe(100, -1)).toBe(90);
    expect(stufe(76, 1)).toBe(80);
    expect(stufe(76, -1)).toBe(75);
  });
  it("bleibt in den Grenzen", () => {
    expect(stufe(400, 1)).toBe(400);
    expect(stufe(25, -1)).toBe(25);
    expect(stufe(1000, -1)).toBe(400);
  });
});

describe("zoomLesen", () => {
  it("liest den gemerkten Zoom, sonst Seitenbreite", () => {
    const z: Zoom[] = [{ art: "seite" }, { art: "prozent", wert: 150 }];
    for (const x of z) expect(zoomLesen(JSON.stringify(x))).toEqual(x);
    expect(zoomLesen(null)).toEqual({ art: "breite" });
    expect(zoomLesen("kaputt")).toEqual({ art: "breite" });
    expect(zoomLesen('{"art":"prozent","wert":9999}')).toEqual({ art: "prozent", wert: 400 });
  });
});
