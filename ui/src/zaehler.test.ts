import { describe, expect, it } from "vitest";
import { tausender, zahlText } from "./zaehler";

const z = (woerter: number, zeichen: number) => ({ woerter, zeichen, ohne_leerzeichen: zeichen });

describe("zahlText", () => {
  it("ganzes Dokument", () => {
    expect(zahlText(z(1234, 7890), null)).toBe("1.234 Wörter · 7.890 Zeichen");
    expect(zahlText(z(1, 4), null)).toBe("1 Wort · 4 Zeichen");
    expect(zahlText(z(0, 0), null)).toBe("0 Wörter · 0 Zeichen");
  });
  it("mit Auswahl", () => {
    expect(zahlText(z(1234, 7890), z(120, 640))).toBe("Auswahl: 120 von 1.234 Wörtern · 640 Zeichen");
  });
  it("Tausenderpunkte", () => {
    expect(tausender(999)).toBe("999");
    expect(tausender(1234567)).toBe("1.234.567");
  });
});
