import { describe, expect, it } from "vitest";
import { SPRACHEN, seitenzahlAnwenden, seitenzahlLage, spracheLesen, spracheWert } from "./seite";

describe("sprache", () => {
  it("wert_und_zurueck", () => {
    expect(spracheWert("de", "AT")).toBe("de-AT");
    expect(spracheWert("fr", null)).toBe("fr");
    expect(spracheLesen("de-AT")).toEqual({ lang: "de", region: "AT" });
    expect(spracheLesen("fr")).toEqual({ lang: "fr", region: null });
  });
  it("oesterreich_steht_zuerst", () => {
    expect(SPRACHEN[0].wert).toBe("de-AT");
  });
});

const z = (innen = "", mitte = "", aussen = "") => ({ innen, mitte, aussen, erste: true });

describe("seitenzahl", () => {
  it("erkennt_die_lage", () => {
    expect(seitenzahlLage(z(), z())).toBe("keine");
    expect(seitenzahlLage(z(), z("", "{seite}"))).toBe("unten-mitte");
    expect(seitenzahlLage(z(), z("", "", "{seite}"))).toBe("unten-aussen");
    expect(seitenzahlLage(z("", "", "{seite}"), z())).toBe("oben-aussen");
    expect(seitenzahlLage(z(), z("", "Seite {seite}"))).toBe("eigene");
  });
  it("setzt_und_entfernt", () => {
    const { kopf, fuss } = seitenzahlAnwenden("unten-mitte", z("Titel", "", "{seite}"), z("Links"));
    expect(kopf.aussen).toBe("");
    expect(kopf.innen).toBe("Titel");
    expect(fuss.mitte).toBe("{seite}");
    expect(fuss.innen).toBe("Links");
  });
});
