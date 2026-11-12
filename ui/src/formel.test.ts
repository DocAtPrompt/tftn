import { describe, expect, it } from "vitest";
import { type FeldZustand, Folge, einsetzen, formelKnoepfe, nachEingabe, naechstesFeld, symbolEinsetzen } from "./formel";

const z = (text: string, von = text.length, bis = von, felder: FeldZustand["felder"] = []): FeldZustand => ({ text, von, bis, felder });

describe("einsetzen", () => {
  const bruch = { text: "frac(a, b)", felder: [{ name: "Zähler", von: 5, bis: 6 }, { name: "Nenner", von: 8, bis: 9 }], cursor: null };

  it("einsetzen_waehlt_erstes_feld", () => {
    const n = einsetzen(z(""), bruch);
    expect(n.text).toBe("frac(a, b)");
    expect([n.von, n.bis]).toEqual([5, 6]);
    expect(n.felder).toEqual([{ von: 5, bis: 6 }, { von: 8, bis: 9 }]);
  });

  it("einsetzen_verschiebt_die_felder", () => {
    const n = einsetzen(z("x + MARK + y", 4, 8), bruch);
    expect(n.text).toBe("x + frac(a, b) + y");
    expect([n.von, n.bis]).toEqual([9, 10]);
    expect(n.felder[1]).toEqual({ von: 12, bis: 13 });
  });

  it("ohne_felder_zum_cursor_oder_ans_ende", () => {
    const n = einsetzen(z("a "), { text: "sum_i ", felder: [], cursor: 6 });
    expect([n.von, n.bis]).toEqual([8, 8]);
    const m = einsetzen(z(""), { text: "& ", felder: [], cursor: null });
    expect([m.von, m.bis]).toEqual([2, 2]);
  });
});

describe("symbolEinsetzen", () => {
  it("symbol_trennt_buchstaben", () => {
    expect(symbolEinsetzen(z("beta"), "alpha").text).toBe("beta alpha");
    expect(symbolEinsetzen(z("2"), "alpha").text).toBe("2 alpha");
    expect(symbolEinsetzen(z("("), "alpha").text).toBe("(alpha");
    expect(symbolEinsetzen(z("a "), "alpha").text).toBe("a alpha");
    expect(symbolEinsetzen(z("x", 0), "alpha").text).toBe("alpha x");
    const n = symbolEinsetzen(z("x", 0), "alpha");
    expect([n.von, n.bis]).toEqual([5, 5]);
  });

  it("symbol_ersetzt_die_auswahl", () => {
    expect(symbolEinsetzen(z("a b c", 2, 3), "pi").text).toBe("a pi c");
  });

  it("symbol_verschiebt_spaetere_felder", () => {
    const n = symbolEinsetzen(z("frac(a, b)", 0, 0, [{ von: 5, bis: 6 }]), "x");
    expect(n.felder).toEqual([{ von: 7, bis: 8 }]);
  });
});

describe("nachEingabe", () => {
  const felder = [{ von: 5, bis: 6 }, { von: 8, bis: 9 }];

  it("tippen_im_feld_dehnt_es", () => {
    const n = nachEingabe(z("frac(a, b)", 5, 6, felder), "frac(xyz, b)", 8, 8);
    expect(n.felder).toEqual([{ von: 5, bis: 8 }, { von: 10, bis: 11 }]);
    expect([n.von, n.bis]).toEqual([8, 8]);
  });

  it("tippen_davor_verschiebt", () => {
    const n = nachEingabe(z("frac(a, b)", 0, 0, felder), "1 frac(a, b)", 2, 2);
    expect(n.felder).toEqual([{ von: 7, bis: 8 }, { von: 10, bis: 11 }]);
  });

  it("tippen_dahinter_laesst_stehen", () => {
    const n = nachEingabe(z("frac(a, b)", 10, 10, felder), "frac(a, b) c", 12, 12);
    expect(n.felder).toEqual(felder);
  });
});

describe("naechstesFeld", () => {
  const felder = [{ von: 0, bis: 1 }, { von: 3, bis: 4 }];

  it("naechstes_feld", () => {
    const n = naechstesFeld(z("a, b", 0, 1, felder), false)!;
    expect([n.von, n.bis]).toEqual([3, 4]);
    const z2 = naechstesFeld(n, true)!;
    expect([z2.von, z2.bis]).toEqual([0, 1]);
  });

  it("am_ende_null", () => {
    expect(naechstesFeld(z("a, b", 3, 4, felder), false)).toBeNull();
    expect(naechstesFeld(z("a, b", 0, 1, felder), true)).toBeNull();
    expect(naechstesFeld(z("a"), false)).toBeNull();
  });

  it("ausserhalb_der_felder_zum_naechsten_danach", () => {
    const n = naechstesFeld(z("a, b", 2, 2, felder), false)!;
    expect([n.von, n.bis]).toEqual([3, 4]);
  });
});

describe("formelKnoepfe", () => {
  const s = { text: null as string | null, einfuegbar: true, eingabe: "x", laeuft: false };

  it("knoepfe_faelle", () => {
    expect(formelKnoepfe(s)).toMatchObject({ einfuegen: true, uebernehmen: false });
    expect(formelKnoepfe({ ...s, eingabe: "  " })).toMatchObject({ einfuegen: false, titel: "Erst eine Formel eingeben." });
    expect(formelKnoepfe({ ...s, einfuegbar: false })).toMatchObject({ einfuegen: false, titel: "Eine neue Formel geht nur im Text." });
    expect(formelKnoepfe({ ...s, text: "x", einfuegbar: false })).toMatchObject({ uebernehmen: false, einfuegen: false, titel: "Nichts geändert." });
    expect(formelKnoepfe({ ...s, text: "x", einfuegbar: false, eingabe: "y" })).toMatchObject({ uebernehmen: true });
    expect(formelKnoepfe({ ...s, text: "x", eingabe: "y", laeuft: true }).uebernehmen).toBe(false);
  });
});

describe("Folge", () => {
  it("spaete_antwort_verworfen", () => {
    const f = new Folge();
    const a = f.neu();
    const b = f.neu();
    expect(f.aktuell(a)).toBe(false);
    expect(f.aktuell(b)).toBe(true);
  });
});
