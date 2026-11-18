import { describe, expect, it } from "vitest";
import { phrasen } from "./phrasen";

describe("Phrasen für CodeMirror", () => {
  it("deckt die Suchleiste ab und behält das $", () => {
    const p = phrasen();
    for (const k of ["Find", "Replace", "next", "previous", "all", "match case", "by word", "regexp", "replace", "replace all", "close"])
      expect(p[k], k).toBeTruthy();
    expect(p["replaced $ matches"]).toContain("$");
    expect(p["replaced match on line $"]).toContain("$");
    expect(p.Find).toBe("Suchen");
  });
});
