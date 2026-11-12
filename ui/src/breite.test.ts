import { describe, expect, it } from "vitest";
import { LEISTE_MAX, LEISTE_MIN, leisteBreite } from "./breite";

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
