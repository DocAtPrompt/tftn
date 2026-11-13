import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { type Completion, CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { type EintragJs, type Modus, abschnitt, kuerzelText, menue, slashAnfang, slashQuelle, typstQuelle } from "./bausteine";

const eintrag = (id: string, herkunft: EintragJs["herkunft"], modus: Modus[], titel = id): EintragJs => ({
  id,
  titel,
  gruppe: null,
  herkunft,
  modus,
  kuerzel: null,
});

const katalog: EintragJs[] = [
  eintrag("paket/a", "paket", ["markup"], "Kasten"),
  eintrag("typst/abbildung", "typst", ["markup"], "Abbildung"),
  eintrag("typst/x", "typst", ["code"], "Code"),
  eintrag("dokumenttyp/hinweis", "dokumenttyp", ["markup"], "Hinweiskasten"),
];

function quelle(modus: Modus | null, eingesetzt: [string, number, number][] = []) {
  return slashQuelle({
    modus: async () => modus,
    eintraege: () => katalog,
    einsetzen: (id, von, bis) => void eingesetzt.push([id, von, bis]),
  });
}

const ideQuelle = typstQuelle({
  vorschlaege: async () => ({ von: 0, vorschlaege: [] }),
  einsetzen: () => {},
});

function kontext(doc: string, explicit = false): CompletionContext {
  return new CompletionContext(EditorState.create({ doc }), doc.length, explicit);
}

describe("slashAnfang", () => {
  it("nur am Zeilenanfang und nur mit Wortzeichen dahinter", () => {
    expect(slashAnfang("/")).toBe(0);
    expect(slashAnfang("  /abb")).toBe(2);
    expect(slashAnfang("/Über")).toBe(0);
    for (const z of ["//", "/ ", "/ Begriff", "a /", "/abb/", "1/2"]) expect(slashAnfang(z)).toBeNull();
  });
});

describe("Quellen", () => {
  it("Tippen eines Absatzes öffnet nichts", () => {
    const satz = "Das ist ein Absatz mit 1/2 Liter, a/b und // nichts.";
    for (let i = 1; i <= satz.length; i++) {
      const ctx = kontext(satz.slice(0, i));
      expect(quelle("markup")(ctx)).toBeNull();
      expect(ideQuelle(ctx)).toBeNull();
    }
  });

  it("kein Menü außerhalb von Markup", async () => {
    expect(await quelle("code")(kontext("Text\n/"))).toBeNull();
    expect(await quelle(null)(kontext("Text\n/"))).toBeNull();
  });

  it("Menü nach / filtert und ersetzt das /", async () => {
    const eingesetzt: [string, number, number][] = [];
    const r = (await quelle("markup", eingesetzt)(kontext("/abb"))) as CompletionResult;
    expect(r.from).toBe(1);
    expect(r.options.map((o) => o.label)).toEqual(["Abbildung"]);
    const apply = r.options[0].apply as (v: unknown, c: Completion, from: number, to: number) => void;
    // Ersatz-Editor ohne offene Liste: apply schließt sie (closeCompletion) und setzt ein.
    apply({ state: EditorState.create({ doc: "/abb" }), dispatch: () => {} }, r.options[0], 1, 4);
    expect(eingesetzt).toEqual([["typst/abbildung", 0, 4]]);
  });

  it("Weitertippen filtert synchron, ohne neue Abfrage", async () => {
    const r = (await quelle("markup")(kontext("/"))) as CompletionResult;
    expect(r.update).toBeTypeOf("function");
    const neu = r.update!(r, 1, 4, kontext("/abb"));
    expect(neu?.options.map((o) => o.label)).toEqual(["Abbildung"]);
    expect(r.update!(r, 1, 2, kontext("//"))).toBeNull();
    expect(r.update!(r, 1, 2, kontext("/ "))).toBeNull();
  });

  it("Strg+Leertaste fragt die IDE", async () => {
    const q = typstQuelle({
      vorschlaege: async () => ({
        von: 1,
        vorschlaege: [{ titel: "figure", detail: null, art: "function", text: "figure(\n  \n)", felder: [], cursor: null }],
      }),
      einsetzen: () => {},
    });
    const r = (await q(kontext("#fig", true))) as CompletionResult;
    expect(r.from).toBe(1);
    expect(r.options[0]).toMatchObject({ label: "figure", type: "function" });
  });
});

describe("Strg+Leertaste", () => {
  it("einmal gerufen, beim Weitertippen nicht erneut", async () => {
    let gerufen = true;
    const q = typstQuelle({
      gerufen: () => {
        const g = gerufen;
        gerufen = false;
        return g;
      },
      vorschlaege: async () => ({
        von: 1,
        vorschlaege: [{ titel: "figure", detail: null, art: "function", text: "figure()", felder: [], cursor: null }],
      }),
      einsetzen: () => {},
    });
    expect(await q(kontext("#fig", true))).not.toBeNull();
    // CodeMirror fragt nach einem Zeichen außerhalb von validFor neu, weiter mit explicit = true.
    expect(q(kontext("#figure(", true))).toBeNull();
  });
});

describe("menue", () => {
  it("filtert nach Modus und Anfrage, ordnet nach Herkunft", () => {
    expect(menue(katalog, "markup", "").map((e) => e.id)).toEqual([
      "typst/abbildung",
      "dokumenttyp/hinweis",
      "paket/a",
    ]);
    expect(menue(katalog, "markup", "abb").map((e) => e.id)).toEqual(["typst/abbildung"]);
    expect(menue(katalog, "markup", "KASTEN").map((e) => e.id)).toEqual(["dokumenttyp/hinweis", "paket/a"]);
    expect(menue(katalog, "code", "").map((e) => e.id)).toEqual(["typst/x"]);
  });
});

describe("kuerzelText", () => {
  it("Mac-Symbole bzw. Windows-Namen", () => {
    expect(kuerzelText("Mod-Shift-t", true)).toBe("⇧⌘T");
    expect(kuerzelText("Mod-Shift-t", false)).toBe("Strg+Umschalt+T");
    expect(kuerzelText("Ctrl-Alt-f", true)).toBe("⌃⌥F");
    expect(kuerzelText("Mod-Shift-Enter", true)).toBe("⇧⌘↩");
    expect(kuerzelText("Mod-Shift-Enter", false)).toBe("Strg+Umschalt+Eingabe");
  });
});

describe("Pakete holen im Menü", () => {
  const e = (id: string, herkunft: EintragJs["herkunft"]): EintragJs => ({ id, titel: id, gruppe: null, herkunft, modus: ["markup"], kuerzel: null });

  it("holen_eintraege", () => {
    const m = menue([e("holen/x", "holen"), e("p", "paket"), e("t", "typst")], "markup", "");
    expect(m.map((x) => x.id)).toEqual(["t", "p", "holen/x"]);
    expect(abschnitt("holen").name).toBe("Pakete holen");
  });
});
