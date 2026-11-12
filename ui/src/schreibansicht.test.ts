import { describe, expect, it } from "vitest";
import { escVerlaesst } from "./schreibansicht";

describe("escVerlaesst", () => {
  it("esc_verlaesst_die_schreibansicht", () => {
    expect(escVerlaesst(true, false, false)).toBe(true);
  });

  it("esc_ohne_schreibansicht_tut_nichts", () => {
    expect(escVerlaesst(false, false, false)).toBe(false);
  });

  it("esc_schliesst_zuerst_was_offen_ist", () => {
    // Vervollständigung, Suche oder Menü haben die Taste schon behandelt …
    expect(escVerlaesst(true, true, false)).toBe(false);
    // … oder ein Dialog liegt oben.
    expect(escVerlaesst(true, false, true)).toBe(false);
  });
});
