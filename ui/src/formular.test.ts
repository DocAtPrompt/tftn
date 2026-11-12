import { describe, expect, it } from "vitest";
import {
  type Art,
  type Feld,
  type Formular,
  eingabeLesen,
  eingabeZeigen,
  gruppen,
  historieTaste,
  Reihe,
  zustandUebertragen,
  pfadText,
  wertAn,
} from "./formular";

const text: Art = { art: "text", widget: "einzeilig", format: null, "min-laenge": null, "max-laenge": null, muster: null, standard: null };
const zahl = (ganzzahl = false): Art => ({ art: "zahl", ganzzahl, min: null, max: null, standard: null });
const datum: Art = { art: "datum", standard: null };
const liste: Art = { art: "liste", element: text, min: null, max: null };
const wahl: Art = { art: "wahl", werte: ["a", "b"], beschriftungen: ["A", "B"], standard: null };

function wert(art: Art, roh: string | boolean): unknown {
  const e = eingabeLesen(art, roh);
  if ("fehler" in e) throw new Error(`unerwarteter Fehler: ${e.fehler}`);
  return e.wert;
}

function fehler(art: Art, roh: string | boolean): string {
  const e = eingabeLesen(art, roh);
  if ("wert" in e) throw new Error(`unerwartet gültig: ${JSON.stringify(e.wert)}`);
  return e.fehler;
}

const feld = (name: string, art: Art, gruppe: string | null = null): Feld => ({
  name,
  titel: name,
  hilfe: null,
  pflicht: false,
  gruppe,
  sensibel: false,
  pandoc: null,
  art,
});

describe("eingaben_formular", () => {
  it("zahl", () => {
    expect(wert(zahl(), "1,5")).toBe(1.5);
    expect(wert(zahl(), "1.5")).toBe(1.5);
    expect(fehler(zahl(), "abc")).toContain("Zahl");
    expect(wert(zahl(), "")).toBeNull();
    expect(wert(zahl(), "  ")).toBeNull();
    expect(fehler(zahl(true), "2,5")).toBe("ganze Zahl erwartet");
    expect(wert(zahl(true), "3")).toBe(3);
  });

  it("datum", () => {
    expect(wert(datum, "31.12.2026")).toBe("2026-12-31");
    expect(wert(datum, "1.2.2026")).toBe("2026-02-01");
    expect(wert(datum, "2026-12-31")).toBe("2026-12-31");
    expect(fehler(datum, "31.02.2026")).toBe("kein gültiges Datum");
    expect(fehler(datum, "morgen")).toBe("kein gültiges Datum");
    expect(wert(datum, "")).toBeNull();
  });

  it("liste", () => {
    expect(wert(liste, "a\n\n b \n")).toEqual(["a", "b"]);
    expect(wert(liste, "")).toBeNull();
    expect(wert(liste, "\n \n")).toBeNull();
  });

  it("text", () => {
    expect(wert(text, " Titel ")).toBe("Titel");
    expect(wert(text, "   ")).toBeNull();
    const markup: Art = { ...text, widget: "markup" } as Art;
    expect(wert(markup, "Zeile 1\nZeile 2")).toBe("Zeile 1\nZeile 2");
  });

  it("schalter_und_wahl", () => {
    expect(wert({ art: "schalter", standard: null }, true)).toBe(true);
    expect(wert(wahl, "1")).toBe("b");
    expect(wert(wahl, "")).toBeNull();
  });
});

describe("zeigen_ist_umkehrung", () => {
  it("zahl, datum, liste, wahl", () => {
    expect(eingabeZeigen(zahl(), 1.5)).toBe("1,5");
    expect(wert(zahl(), eingabeZeigen(zahl(), 1.5))).toBe(1.5);
    expect(eingabeZeigen(datum, "2026-12-31")).toBe("31.12.2026");
    expect(wert(datum, eingabeZeigen(datum, "2026-12-31"))).toBe("2026-12-31");
    expect(eingabeZeigen(liste, ["a", "b"])).toBe("a\nb");
    expect(wert(liste, eingabeZeigen(liste, ["a", "b"]))).toEqual(["a", "b"]);
    expect(eingabeZeigen(wahl, "b")).toBe("1");
    expect(eingabeZeigen(text, undefined)).toBe("");
    expect(eingabeZeigen({ art: "schalter", standard: true }, undefined)).toBe(true);
  });
});

describe("pfade", () => {
  it("pfadText und wertAn", () => {
    expect(pfadText(["autoren", 0, "name"])).toBe("autoren.0.name");
    const w = { autoren: [{ name: "A" }], literatur: { stil: "ieee" } };
    expect(wertAn(w, ["autoren", 0, "name"])).toBe("A");
    expect(wertAn(w, ["literatur", "stil"])).toBe("ieee");
    expect(wertAn(w, ["autoren", 3, "name"])).toBeUndefined();
    expect(wertAn(w, ["fehlt", "x"])).toBeUndefined();
  });
});

describe("gruppen_reihenfolge", () => {
  it("titelei zuerst, dann nach gruppe", () => {
    const f: Formular = {
      titel: "F",
      felder: [
        feld("titel", text),
        feld("literatur", { art: "gruppe", felder: [] }, "Literatur"),
        feld("datum", datum),
      ],
    };
    const g = gruppen(f);
    expect(g.map((x) => x.titel)).toEqual(["Titelei", "Literatur"]);
    expect(g[0].felder.map((x) => x.name)).toEqual(["titel", "datum"]);
    expect(g[1].felder.map((x) => x.name)).toEqual(["literatur"]);
  });
});

describe("historie_taste", () => {
  const t = (key: string, mod: boolean, shift = false, ctrl = false) => ({ key, mod, shift, ctrl });
  it("mac", () => {
    expect(historieTaste(t("z", true), true, false)).toBe("rueck");
    expect(historieTaste(t("Z", true, true), true, false)).toBe("vor");
    expect(historieTaste(t("y", false, false, true), true, false)).toBeNull();
    expect(historieTaste(t("z", false), true, false)).toBeNull();
  });
  it("windows", () => {
    expect(historieTaste(t("z", true, false, true), false, false)).toBe("rueck");
    expect(historieTaste(t("y", true, false, true), false, false)).toBe("vor");
    expect(historieTaste(t("z", true, true, true), false, false)).toBe("vor");
  });
  it("feld mit ungeschriebener eingabe", () => {
    expect(historieTaste(t("z", true), true, true)).toBeNull();
  });
});

describe("abschlusspruefung", () => {
  it("schreiben_der_reihe_nach", async () => {
    // Zwei Änderungen kurz hintereinander: die zweite trägt den Stand aus der ersten Antwort.
    let stand = "S0";
    const gesendet: string[] = [];
    const reihe = new Reihe(() => stand);
    const setzen = (s: string) =>
      new Promise<string>((ok) => {
        gesendet.push(s);
        setTimeout(() => {
          stand = `${s}+`;
          ok(stand);
        }, 5);
      });
    const a = reihe.einreihen(setzen);
    const b = reihe.einreihen(setzen);
    expect(await a).toBe("S0+");
    expect(await b).toBe("S0++");
    expect(gesendet).toEqual(["S0", "S0+"]);
  });

  it("reihe_laeuft_nach_fehler_weiter", async () => {
    const reihe = new Reihe(() => "S");
    const a = reihe.einreihen(() => Promise.reject(new Error("veraltet")));
    const b = reihe.einreihen((s) => Promise.resolve(s));
    await expect(a).rejects.toThrow("veraltet");
    expect(await b).toBe("S");
  });

  it("neuzeichnen_behaelt_eingabe_und_cursor", () => {
    const alt = { value: "Tippfehlr korrigiert", selectionStart: 5, selectionEnd: 5, scrollTop: 40, dataset: { gezeigt: "Tippfehler" } };
    const neu = { value: "Tippfehler", selectionStart: 0, selectionEnd: 0, scrollTop: 0, dataset: { gezeigt: "Tippfehler" } };
    zustandUebertragen(alt, neu);
    expect(neu.value).toBe("Tippfehlr korrigiert");
    expect([neu.selectionStart, neu.selectionEnd, neu.scrollTop]).toEqual([5, 5, 40]);
  });

  it("neuzeichnen_nimmt_neuen_wert_wenn_nichts_getippt", () => {
    // Eigener Wert geschrieben (oder von außen geändert): der neue Wert gilt, Cursor bleibt, wo möglich.
    const alt = { value: "Alt", selectionStart: 2, selectionEnd: 3, scrollTop: 0, dataset: { gezeigt: "Alt" } };
    const neu = { value: "Neu", selectionStart: 0, selectionEnd: 0, scrollTop: 0, dataset: { gezeigt: "Neu" } };
    zustandUebertragen(alt, neu);
    expect(neu.value).toBe("Neu");
    expect([neu.selectionStart, neu.selectionEnd]).toEqual([0, 0]);
  });
});
