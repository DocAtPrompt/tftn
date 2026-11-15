import { describe, expect, it } from "vitest";
import { systemMenueLassen } from "./kontextmenue";

/** Ein Element, das für die genannten Auswahlen `closest` erfüllt. */
const el = (...treffer: string[]) => ({
  closest: (auswahl: string) => (auswahl.split(",").some((s) => treffer.includes(s.trim())) ? {} : null),
});

describe("systemMenueLassen", () => {
  it("in Eingabefeldern und im Editor bleibt das Menü des Systems (Ausschneiden, Kopieren, Einsetzen)", () => {
    expect(systemMenueLassen(el("input"))).toBe(true);
    expect(systemMenueLassen(el("textarea"))).toBe(true);
    expect(systemMenueLassen(el(`[contenteditable="true"]`))).toBe(true);
  });
  it("sonst (Leisten, Knöpfe, Vorschau-Rand, Dialoge) kein Menü mit „Neu laden“", () => {
    expect(systemMenueLassen(el())).toBe(false);
    expect(systemMenueLassen(el("[contenteditable=false]"))).toBe(false);
    expect(systemMenueLassen(null)).toBe(false);
    expect(systemMenueLassen({})).toBe(false);
  });
});
