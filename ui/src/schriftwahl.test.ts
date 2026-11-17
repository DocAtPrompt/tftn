import { describe, expect, it } from "vitest";
import { type FamilieJs, fehlendText, filtern, umfang } from "./schriftwahl";

const f = (name: string, mehr: Partial<FamilieJs> = {}): FamilieJs => ({
  name,
  schnitte: 4,
  groesse: 1_234_567,
  fehlend: "",
  vorhanden: false,
  datei: null,
  ...mehr,
});

describe("schriftwahl", () => {
  it("filtert nach allen wörtern, ohne groß/klein", () => {
    const liste = [f("Source Serif 4"), f("Source Sans 3"), f("Helvetica Neue")];
    expect(filtern(liste, "source serif").map((x) => x.name)).toEqual(["Source Serif 4"]);
    expect(filtern(liste, "  SANS ").map((x) => x.name)).toEqual(["Source Sans 3"]);
    expect(filtern(liste, "")).toHaveLength(3);
  });

  it("nur vollständige, dateien bleiben", () => {
    const liste = [f("Voll"), f("Lücke", { fehlend: "ß" }), f("Datei", { fehlend: "ß", datei: "/x.otf" })];
    expect(filtern(liste, "", true).map((x) => x.name)).toEqual(["Voll", "Datei"]);
    expect(filtern(liste, "", false)).toHaveLength(3);
  });

  it("umfang mit schnitten und größe", () => {
    expect(umfang(f("A"))).toBe("4 Schnitte · 1,2 MB");
    expect(umfang(f("A", { schnitte: 1, groesse: 40_000 }))).toBe("1 Schnitt · 40 kB");
    expect(umfang(f("A", { groesse: 10 }))).toBe("4 Schnitte · 1 kB");
  });

  it("fehlende zeichen, lange listen gekürzt", () => {
    expect(fehlendText(f("A"))).toBe("");
    expect(fehlendText(f("A", { fehlend: "ß€" }))).toBe("Es fehlen: ß €");
    const lang = fehlendText(f("A", { fehlend: "abcdefghijklmnopqrstu" }));
    expect(lang.endsWith("p …")).toBe(true);
  });
});
