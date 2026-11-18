import { describe, expect, it } from "vitest";
import { SCHRIFT_STUFEN, schriftLesen, schriftStufe } from "./editorschrift";

describe("Schriftgröße des Editors", () => {
  it("geht in Stufen und bleibt an den Enden stehen", () => {
    expect(schriftStufe(100, 1)).toBe(110);
    expect(schriftStufe(100, -1)).toBe(90);
    expect(schriftStufe(115, 1)).toBe(125);
    expect(schriftStufe(115, -1)).toBe(110);
    expect(schriftStufe(SCHRIFT_STUFEN.at(-1)!, 1)).toBe(SCHRIFT_STUFEN.at(-1));
    expect(schriftStufe(SCHRIFT_STUFEN[0], -1)).toBe(SCHRIFT_STUFEN[0]);
  });

  it("liest nur gültige Stufen", () => {
    expect(schriftLesen("125")).toBe(125);
    for (const w of [null, "", "abc", "123", "-10", "1e3"]) expect(schriftLesen(w)).toBe(100);
  });
});
