import { describe, expect, it } from "vitest";
import { belegen, aktionen } from "./tasten";
import { beilagen, editorKuerzel, kuerzelUebersicht, rundgangZusatz, versionZeile, type UeberJs } from "./ueber";

const stand: UeberJs = {
  version: "0.7.0",
  typst_version: "0.15.1",
  links: [],
  kofi_vorhanden: false,
  kofi_ausblenden: false,
  ganz_offline: false,
};

describe("Über TFTN", () => {
  it("nennt bei einem Rundgang in der Kopie die erfundenen Daten", () => {
    const r = { id: "x", titel: "X", beschreibung: "Tippt.", kopie: false };
    expect(rundgangZusatz(r)).toBe("Tippt.");
    expect(rundgangZusatz({ ...r, kopie: true })).toBe("Tippt. Läuft in einer Kopie mit erfundenen Daten.");
  });

  it("nennt Version und typst", () => {
    const z = versionZeile(stand);
    expect(z).toContain("0.7.0");
    expect(z).toContain("typst 0.15.1");
  });

  it("listet die Beilagen mit Lizenz", () => {
    const b = beilagen().join("\n");
    for (const w of ["typst", "Apache-2.0", "CodeMirror", "Open Font License", "Wörterbücher"]) expect(b).toContain(w);
  });

  it("zeigt nur geltende Kürzel, nach Gruppen, Editor zuletzt", () => {
    const a = aktionen([], []);
    const b = belegen(a, { "helfer/zitat": "", "helfer/baustein": "Mod-Alt-b" }, "mac");
    const g = kuerzelUebersicht(b.zeilen, "mac");
    expect(g.at(-1)!.eintraege).toEqual(editorKuerzel("mac"));
    const alle = g.flatMap((x) => x.eintraege.map((e) => e.titel));
    expect(alle).not.toContain("Zitat einfügen");
    const baustein = g.flatMap((x) => x.eintraege).find((e) => e.titel === "Baustein einfügen")!;
    expect(baustein.kuerzel).toBe("⌥⌘B");
    const ueber = g.flatMap((x) => x.eintraege).find((e) => e.titel === "Über TFTN und Tastenkürzel")!;
    expect(ueber.kuerzel).toBe("F1");
    // jede Gruppe nur einmal
    const namen = g.map((x) => x.gruppe);
    expect(new Set(namen).size).toBe(namen.length);
  });

  it("lässt überdeckte Kürzel weg", () => {
    const a = aktionen([], []);
    // Zitat bekommt das Kürzel des Bausteins: Der Baustein (Standard) wird überdeckt.
    const b = belegen(a, { "helfer/zitat": "Mod-Shift-b" }, "win");
    const titel = kuerzelUebersicht(b.zeilen, "win").flatMap((x) => x.eintraege.map((e) => e.titel));
    expect(titel).toContain("Zitat einfügen");
    expect(titel).not.toContain("Baustein einfügen");
  });

  it("Editor-Kürzel je Plattform", () => {
    expect(editorKuerzel("win").map((e) => e.kuerzel)).toContain("Strg+Y");
    expect(editorKuerzel("mac").map((e) => e.kuerzel)).toContain("⇧⌘Z");
  });
});
