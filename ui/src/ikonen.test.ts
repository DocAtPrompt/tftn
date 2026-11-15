import { describe, expect, it } from "vitest";
import { IKONEN, ikoneSvg } from "./ikonen";
import html from "../index.html?raw";

describe("ikonen", () => {
  it("jedes Symbol aus index.html gibt es", () => {
    const namen = [...html.matchAll(/data-ikone="([^"]+)"/g)].map((m) => m[1]);
    expect(namen.length).toBeGreaterThan(8);
    for (const n of namen) expect(IKONEN, n).toHaveProperty(n);
  });

  it("Strichzeichnung in Textfarbe, für Vorleser unsichtbar", () => {
    const s = ikoneSvg("zahnrad");
    expect(s).toMatch(/^<svg [^>]*viewBox="0 0 24 24"/);
    expect(s).toContain('stroke="currentColor"');
    expect(s).toContain('aria-hidden="true"');
    expect(ikoneSvg("gibt-es-nicht")).toBe("");
  });
});
