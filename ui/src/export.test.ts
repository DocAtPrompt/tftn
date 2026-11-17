import { describe, expect, it } from "vitest";
import { archivZeilen, profilErklaerung, profilName, standardName } from "./export";
import { groesse } from "./sprache";

describe("Archivpaket", () => {
  it("meldet Zip, Umfang, PDF/A und Prüfsumme", () => {
    const z = archivZeilen({
      pfad: "/x/export/Befund-archiv-2026-11-17.zip",
      relativ: "export/Befund-archiv-2026-11-17.zip",
      pdf: "Befund.pdf",
      standard: "a-2b",
      sha256: "ab12",
      dateien: 13,
      groesse: 3_383_028,
      warnungen: [],
    });
    expect(z[0]).toBe("Fertig: export/Befund-archiv-2026-11-17.zip (13 Dateien, 3,4 MB)");
    expect(z[1]).toBe("Darin Befund.pdf als PDF/A (a-2b), SHA-256 ab12");
    expect(z).toHaveLength(3);
  });

  it("schreibt Größen kurz", () => {
    expect(groesse(3_000_000)).toBe("3 MB");
    expect(groesse(1_234_567)).toBe("1,2 MB");
    expect(groesse(40_000)).toBe("40 kB");
    expect(groesse(10)).toBe("1 kB");
  });
});

describe("Namen der Export-Profile", () => {
  const p = (id: string, standards: string[] = []) => ({ id, art: "pdf", standards });
  it("nennt die mitgelieferten im Klartext", () => {
    expect(profilName(p("vorschau"))).toBe("PDF");
    expect(profilName(p("archiv", ["a-2b"]))).toBe("Archiv-PDF (Langzeit, PDF/A-2b)");
    expect(profilErklaerung(p("archiv", ["a-2b"]))).toContain("Archivpaket (Zip)");
    expect(profilErklaerung(p("vorschau"))).toBe("");
  });
  it("lässt eigene Profile, wie sie sind", () => {
    expect(profilName(p("druckerei", ["x-4"]))).toBe("druckerei · pdf · PDF/X-4");
    expect(profilName({ id: "bilder", art: "png", standards: [] })).toBe("bilder · png");
  });
  it("schreibt Standards aus", () => {
    expect(standardName("a-2b")).toBe("PDF/A-2b");
    expect(standardName("ua-1")).toBe("PDF/UA-1");
    expect(standardName("1.7")).toBe("1.7");
  });
});
