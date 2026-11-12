import { describe, expect, it } from "vitest";
import eingebautToml from "../../crates/tftn-katalog/eingebaut.toml?raw";
import type { EintragJs } from "./bausteine";
import type { ProfilJs } from "./export";
import { STANDARD, aktionDerTaste, aktionen, belegen, leistenSchritt, fehlerHinweis, keymapAus, konflikte, kuerzelAusTaste, normalisieren } from "./tasten";

/** Die eingebauten Bausteine wie aus `katalog` (nur Id und Herkunft zählen hier). */
function katalogAusEingebaut(): EintragJs[] {
  return [...eingebautToml.matchAll(/^id = "(.+)"$/gm)].map((m) => ({
    id: `typst/${m[1]}`,
    titel: m[1],
    gruppe: null,
    herkunft: "typst",
    modus: ["markup"],
    kuerzel: null,
  }));
}

const PROFILE: ProfilJs[] = [{ id: "druck", art: "pdf", standards: [] }];

const hinweis: EintragJs = {
  id: "tftn-fachartikel/hinweis",
  titel: "Hinweiskasten",
  gruppe: null,
  herkunft: "dokumenttyp",
  modus: ["markup"],
  kuerzel: "Mod-Shift-h",
};

const alle = (eigene: Record<string, string> = {}) => ({
  a: aktionen([...katalogAusEingebaut(), hinweis], PROFILE),
  eigene,
});

const taste = (t: Partial<Parameters<typeof kuerzelAusTaste>[0]>) => ({
  key: "",
  code: "",
  keyCode: 0,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  ...t,
});

describe("Standardbelegung", () => {
  it("standardbelegung ohne konflikt", () => {
    expect(konflikte(aktionen(katalogAusEingebaut(), PROFILE), {})).toEqual([]);
  });

  it("standard nennt nur vorhandene aktionen", () => {
    const ids = new Set(aktionen(katalogAusEingebaut(), PROFILE).map((a) => a.id));
    for (const id of Object.keys(STANDARD)) expect(ids, id).toContain(id);
    expect(ids).toContain("typst/fett");
    expect(ids).toContain("typst/kursiv");
    expect(ids).toContain("export/druck");
  });

  it("neu_und_oeffnen_haben_standard", () => {
    const ids = new Set(aktionen(katalogAusEingebaut(), PROFILE).map((a) => a.id));
    expect(ids).toContain("app/neu");
    expect(ids).toContain("app/oeffnen");
    expect(STANDARD["app/neu"]).toEqual({ mac: ["Mod-n"], win: ["Mod-n"] });
    expect(STANDARD["app/oeffnen"]).toEqual({ mac: ["Mod-o"], win: ["Mod-o"] });
  });

  it("fach_pakete_hat_standard", () => {
    const ids = new Set(aktionen(katalogAusEingebaut(), PROFILE).map((a) => a.id));
    expect(ids).toContain("helfer/fach-pakete");
    expect(ids).toContain("helfer/fach-formel");
    expect(STANDARD["helfer/fach-formel"]).toEqual({ mac: ["Mod-Shift-x"], win: ["Mod-Shift-x"] });
    expect(STANDARD["helfer/fach-pakete"]).toEqual({ mac: ["Mod-Shift-y"], win: ["Mod-Shift-y"] });
  });

  it("fach_dokument_hat_standard", () => {
    const ids = new Set(aktionen(katalogAusEingebaut(), PROFILE).map((a) => a.id));
    expect(ids).toContain("helfer/fach-dokument");
    expect(STANDARD["helfer/fach-dokument"]).toEqual({ mac: ["Mod-Shift-o"], win: ["Mod-Shift-o"] });
  });
});

describe("normalisieren", () => {
  it("löst Mod auf und ordnet", () => {
    expect(normalisieren("Mod-Shift-b", "mac")).toBe("Shift-Meta-b");
    expect(normalisieren("Shift-Mod-B", "win")).toBe("Ctrl-Shift-b");
    expect(normalisieren("Mod-Space", "win")).toBe("Ctrl-Space");
    expect(normalisieren("Cmd-k", "mac")).toBe("Meta-k");
    expect(normalisieren("Alt-A", "win")).toBe("Alt-Shift-a");
    expect(normalisieren("F5", "win")).toBe("F5");
  });

  it("lehnt Kürzel ohne Modifikator ab", () => {
    expect(normalisieren("Shift-a", "mac")).toBeNull();
    expect(normalisieren("a", "win")).toBeNull();
    expect(normalisieren("", "win")).toBeNull();
    expect(normalisieren("Mod-", "win")).toBeNull();
    expect(normalisieren("Hyper-a", "win")).toBeNull();
  });
});

describe("kuerzelAusTaste", () => {
  it("baut die Schreibweise mit Mod", () => {
    expect(kuerzelAusTaste(taste({ metaKey: true, shiftKey: true, code: "KeyB", key: "B", keyCode: 66 }), "mac")).toBe(
      "Mod-Shift-b",
    );
    expect(kuerzelAusTaste(taste({ ctrlKey: true, shiftKey: true, code: "Period", key: ":", keyCode: 190 }), "win")).toBe(
      "Mod-Shift-.",
    );
    expect(kuerzelAusTaste(taste({ ctrlKey: true, code: "KeyK", key: "k", keyCode: 75 }), "mac")).toBe("Ctrl-k");
    expect(kuerzelAusTaste(taste({ code: "F5", key: "F5", keyCode: 116 }), "win")).toBe("F5");
  });

  it("deutsche Tastatur: gespeichert wird, was CodeMirror beim selben Druck sucht", () => {
    // QWERTZ: Die Taste Z liegt auf der physischen Position Y (code KeyY).
    expect(kuerzelAusTaste(taste({ metaKey: true, shiftKey: true, code: "KeyY", key: "z", keyCode: 90 }), "mac")).toBe(
      "Mod-Shift-z",
    );
    expect(kuerzelAusTaste(taste({ ctrlKey: true, code: "KeyY", key: "z", keyCode: 90 }), "win")).toBe("Mod-z");
    // Strg+- (deutsches Minus liegt auf US-Slash), Strg+ö (US-Semikolon)
    expect(kuerzelAusTaste(taste({ ctrlKey: true, code: "Slash", key: "-", keyCode: 189 }), "win")).toBe("Mod--");
    expect(kuerzelAusTaste(taste({ ctrlKey: true, code: "Semicolon", key: "ö", keyCode: 192 }), "win")).toBe("Mod-ö");
    // ⌘⇧. am Mac: key ist „:“, CodeMirror findet die Bindung über die Grundtaste „.“
    expect(kuerzelAusTaste(taste({ metaKey: true, shiftKey: true, code: "Period", key: ":", keyCode: 190 }), "mac")).toBe(
      "Mod-Shift-.",
    );
  });

  it("ignoriert Modifikatoren allein und ungültige Kürzel", () => {
    expect(kuerzelAusTaste(taste({ shiftKey: true, code: "ShiftLeft", key: "Shift" }), "mac")).toBeNull();
    expect(kuerzelAusTaste(taste({ metaKey: true, code: "MetaLeft", key: "Meta" }), "mac")).toBeNull();
    expect(kuerzelAusTaste(taste({ shiftKey: true, code: "KeyA", key: "A" }), "win")).toBeNull();
  });
});

describe("Vorrang", () => {
  it("Standard, Vorschlag, keine", () => {
    const { a } = alle();
    const z = belegen(a, {}, "win").zeilen;
    const fussnote = z.find((x) => x.aktion.id === "typst/fussnote")!;
    expect(fussnote.quelle).toBe("standard");
    expect(fussnote.kuerzel).toEqual(["Mod-Shift-f"]);
    expect(z.find((x) => x.aktion.id === hinweis.id)!.quelle).toBe("vorschlag");
    const ohne = belegen(a, { "typst/fussnote": "" }, "win").zeilen.find((x) => x.aktion.id === "typst/fussnote")!;
    expect(ohne.quelle).toBe("keine");
    expect(ohne.kuerzel).toEqual([]);
  });

  it("eigene Belegung überdeckt den Standard", () => {
    const { a } = alle();
    const eigene = { "helfer/zitat": "Mod-Shift-f" };
    const b = belegen(a, eigene, "win");
    expect(b.zeilen.find((x) => x.aktion.id === "helfer/zitat")!.quelle).toBe("eigene");
    expect(b.zeilen.find((x) => x.aktion.id === "typst/fussnote")!.ueberdecktVon).toBe("helfer/zitat");
    expect(b.taste.get("Ctrl-Shift-f")).toBe("helfer/zitat");
    const k = konflikte(a, eigene).filter((x) => x.art === "doppelt");
    expect(k.filter((x) => x.plattform === "mac")).toHaveLength(1);
    expect(k.filter((x) => x.plattform === "win")).toHaveLength(1);
  });

  it("ungültige eigene Belegung zählt als keine und wird gemeldet", () => {
    const { a } = alle();
    const b = belegen(a, { "typst/link": "k" }, "mac");
    expect(b.zeilen.find((x) => x.aktion.id === "typst/link")!.quelle).toBe("keine");
    expect(b.meldungen.join(" ")).toContain("typst/link");
  });
});

describe("Konflikte", () => {
  it("alt und altgr", () => {
    const { a } = alle();
    const alt = konflikte(a, { "typst/link": "Alt-l" });
    expect(alt.some((k) => k.plattform === "mac" && k.art === "system" && k.mit.includes("Sonderzeichen"))).toBe(true);
    expect(alt.some((k) => k.plattform === "win" && k.art === "system")).toBe(false);
    const altgr = konflikte(a, { "typst/link": "Mod-Alt-q" });
    expect(altgr.some((k) => k.plattform === "win" && k.art === "system" && k.mit.includes("AltGr"))).toBe(true);
  });

  it("bewusst gesetzt", () => {
    const { a } = alle();
    const k = konflikte(a, { "typst/link": "Mod-f" });
    for (const p of ["mac", "win"]) {
      expect(k.some((x) => x.plattform === p && x.art === "system" && x.aktion === "typst/link")).toBe(true);
    }
    expect(belegen(a, { "typst/link": "Mod-f" }, "win").taste.get("Ctrl-f")).toBe("typst/link");
  });

  it("bedingt ist kein konflikt", () => {
    const { a } = alle();
    expect(konflikte(a, {}).filter((k) => k.aktion === "helfer/vervollstaendigen")).toEqual([]);
  });

  it("entfernte Editor-Kürzel sind frei", () => {
    const { a } = alle();
    expect(konflikte(a, { "typst/kursiv": "Mod-i" })).toEqual([]);
  });

  it("Editor-Kürzel werden gemeldet", () => {
    const { a } = alle();
    const k = konflikte(a, { "typst/link": "Mod-Shift-k" });
    expect(k.some((x) => x.art === "editor" && x.plattform === "win")).toBe(true);
  });
});

describe("keymapAus", () => {
  it("baut Einträge aus der Belegung", () => {
    const { a } = alle();
    const gerufen: string[] = [];
    const km = keymapAus(belegen(a, { "typst/fussnote": "" }, "win"), (id) => (gerufen.push(id), true));
    const b = km.find((k) => k.key === "Ctrl-Shift-b")!;
    expect(b.preventDefault).toBe(true);
    expect(b.run!({} as never)).toBe(true);
    expect(gerufen).toEqual(["helfer/baustein"]);
    expect(km.some((k) => k.key === "Ctrl-Shift-f")).toBe(false);
  });

  it("überdeckte Kürzel bindet nur der Halter", () => {
    const { a } = alle();
    const km = keymapAus(belegen(a, { "helfer/zitat": "Mod-Shift-f" }, "mac"), () => true);
    expect(km.filter((k) => k.key === "Shift-Meta-f")).toHaveLength(1);
  });
});

describe("fehlerHinweis", () => {
  it("meldet einen neuen Fehler und seine Behebung", () => {
    expect(fehlerHinweis(null, "tasten.toml, Zeile 3: x")).toContain("Zeile 3");
    expect(fehlerHinweis("tasten.toml, Zeile 3: x", null)).toContain("wieder");
    expect(fehlerHinweis("a", "a")).toBeNull();
    expect(fehlerHinweis(null, null)).toBeNull();
  });
});

describe("neue Helfer (PLAN-M2 AP6)", () => {
  it("Werte, Seitenleiste und Fach Einfügen sind Aktionen", () => {
    const ids = aktionen([], []).map((a) => a.id);
    for (const id of ["helfer/werte", "helfer/seitenleiste", "helfer/fach-einfuegen"]) expect(ids).toContain(id);
  });
  it("Standardkürzel", () => {
    expect(STANDARD["helfer/werte"]).toEqual({ mac: ["Mod-Shift-m"], win: ["Mod-Shift-m"] });
    expect(STANDARD["helfer/seitenleiste"]).toEqual({ mac: ["Mod-Shift-p"], win: ["Mod-Shift-p"] });
    expect(STANDARD["helfer/fach-einfuegen"]).toBeUndefined();
  });
});

describe("Schreibansicht", () => {
  it("schreibansicht_hat_standard", () => {
    expect(STANDARD["helfer/schreibansicht"]).toEqual({ mac: ["Mod-Shift-Enter"], win: ["Mod-Shift-Enter"] });
  });

  it("schreibansicht_taste_ausserhalb_des_editors", () => {
    const e = { key: "Enter", code: "Enter", keyCode: 13, altKey: false, ctrlKey: false, metaKey: true, shiftKey: true };
    expect(aktionDerTaste(belegen(alle().a, {}, "mac"), e, "mac")).toBe("helfer/schreibansicht");
    const w = { ...e, metaKey: false, ctrlKey: true };
    expect(aktionDerTaste(belegen(alle().a, {}, "win"), w, "win")).toBe("helfer/schreibansicht");
  });
});

describe("leistenSchritt", () => {
  it("kuerzel_oeffnet_springt_schliesst", () => {
    expect(leistenSchritt(false, false)).toBe("oeffnen");
    expect(leistenSchritt(true, false)).toBe("fokus");
    expect(leistenSchritt(true, true)).toBe("schliessen");
  });

  it("taste_ausserhalb_des_editors", () => {
    const b = belegen(alle().a, {}, "mac");
    const e = (mit: Partial<KeyboardEvent>) => ({
      key: "P",
      code: "KeyP",
      keyCode: 80,
      altKey: false,
      ctrlKey: false,
      metaKey: true,
      shiftKey: true,
      ...mit,
    });
    expect(aktionDerTaste(b, e({}), "mac")).toBe("helfer/seitenleiste");
    expect(aktionDerTaste(b, e({ metaKey: false }), "mac")).not.toBe("helfer/seitenleiste");
  });
});
