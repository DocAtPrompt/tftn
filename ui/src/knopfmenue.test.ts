import { describe, expect, it } from "vitest";
import { menueTaste } from "./knopfmenue";

describe("menueTaste", () => {
  it("Pfeile wandern im Kreis, Pos1 und Ende springen", () => {
    expect(menueTaste("ArrowDown", 0, 3)).toEqual({ fokus: 1 });
    expect(menueTaste("ArrowDown", 2, 3)).toEqual({ fokus: 0 });
    expect(menueTaste("ArrowUp", 0, 3)).toEqual({ fokus: 2 });
    expect(menueTaste("ArrowUp", -1, 3)).toEqual({ fokus: 2 });
    expect(menueTaste("Home", 2, 3)).toEqual({ fokus: 0 });
    expect(menueTaste("End", 0, 3)).toEqual({ fokus: 2 });
  });
  it("Tab schließt, ohne die Taste zu schlucken; andere Tasten bleiben unberührt", () => {
    expect(menueTaste("Tab", 1, 3)).toBe("schliessen");
    expect(menueTaste("a", 1, 3)).toBeNull();
    expect(menueTaste("ArrowDown", 0, 0)).toBeNull();
  });
});
