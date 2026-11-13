import { describe, expect, it } from "vitest";
import { meldungMit } from "./meldung";

describe("meldungMit", () => {
  it("ohne doppelten Vorspann", () => {
    expect(meldungMit("Nicht geändert", "ungültiger Wert: Seitenangabe zu lang")).toBe("Nicht geändert: Seitenangabe zu lang");
  });
  it("andere Fehler unverändert", () => {
    expect(meldungMit("Nicht geändert", "Datei fehlt")).toBe("Nicht geändert: Datei fehlt");
  });
});
