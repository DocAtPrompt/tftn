import { describe, expect, it } from "vitest";
import { abbildungText, blockEinfuegen, istBild, eingefuegterName } from "./bildeinfuegen";

describe("abbildungText", () => {
  it("Abbildung mit Breite, Cursor in der Unterschrift", () => {
    const a = abbildungText("bilder/a.png");
    expect(a.text).toBe('#figure(\n  image("bilder/a.png", width: 80%),\n  caption: [],\n)');
    expect(a.text.slice(a.cursor, a.cursor + 1)).toBe("]");
  });
});

describe("blockEinfuegen", () => {
  it("in eine leere Zeile direkt", () => {
    expect(blockEinfuegen({ from: 10, to: 10, text: "" }, "X")).toEqual({ von: 10, text: "X", versatz: 0 });
    expect(blockEinfuegen({ from: 10, to: 12, text: "  " }, "X")).toEqual({ von: 10, text: "X", versatz: 0 });
  });
  it("hinter eine Zeile mit Text, mit Leerzeile davor und danach", () => {
    expect(blockEinfuegen({ from: 0, to: 5, text: "Hallo" }, "X")).toEqual({ von: 5, text: "\n\nX\n", versatz: 2 });
  });
});

describe("istBild und eingefuegterName", () => {
  it("nach Endung oder Typ", () => {
    expect(istBild("Foto.JPG", "")).toBe(true);
    expect(istBild("x", "image/png")).toBe(true);
    expect(istBild("brief.docx", "application/msword")).toBe(false);
  });
  it("Bildschirmfotos aus der Zwischenablage bekommen Datum und Zeit", () => {
    const t = new Date(2026, 10, 11, 9, 5, 7);
    expect(eingefuegterName("image.png", "image/png", t)).toBe("bildschirmfoto-2026-11-11-090507.png");
    expect(eingefuegterName("", "image/jpeg", t)).toBe("bildschirmfoto-2026-11-11-090507.jpg");
    expect(eingefuegterName("Plan.svg", "image/svg+xml", t)).toBe("Plan.svg");
  });
});
