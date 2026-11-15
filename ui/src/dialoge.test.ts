import { describe, expect, it } from "vitest";
import { type Dialog, escZiel, klickZiel, mitBereich } from "./dialoge";

const d = (offen: boolean): Dialog => ({ offen, schliessen: () => {} });

describe("escZiel", () => {
  it("nimmt_den_zuletzt_geoeffneten", () => {
    const [a, b, c] = [d(true), d(true), d(false)];
    expect(escZiel([a, b, c])).toBe(b);
  });
  it("nichts_offen", () => {
    expect(escZiel([d(false)])).toBeNull();
    expect(escZiel([])).toBeNull();
  });
});

describe("klickZiel", () => {
  const innen = {} as Node;
  const aussen = {} as Node;
  const mit = (offen: boolean, behalten = false): Dialog => ({
    offen,
    schliessen: () => {},
    enthaelt: (n) => n === innen,
    behalten: () => behalten,
  });

  it("ein Klick daneben schließt den obersten offenen", () => {
    const [a, b] = [mit(true), mit(true)];
    expect(klickZiel([a, b, mit(false)], aussen)).toBe(b);
  });
  it("ein Klick hinein oder auf den Auslöser schließt nicht", () => {
    expect(klickZiel([mit(true)], innen)).toBeNull();
  });
  it("mit Eingaben, die verloren gingen, bleibt er offen", () => {
    expect(klickZiel([mit(true, true)], aussen)).toBeNull();
  });
  it("Dialoge ohne Angabe ihres Bereichs schließt nur Esc oder ×", () => {
    expect(klickZiel([d(true)], aussen)).toBeNull();
    expect(klickZiel([], aussen)).toBeNull();
  });
});

describe("mitBereich", () => {
  it("leiht dem Dialog ein `enthaelt` und reicht offen, schliessen und behalten durch", () => {
    let offen = true;
    let zu = 0;
    const innen = {} as Node;
    const roh = {
      get offen() {
        return offen;
      },
      schliessen: () => void zu++,
      behalten: () => false,
    };
    const d = mitBereich(roh, (z) => z === innen);
    expect(d.enthaelt!(innen)).toBe(true);
    expect(klickZiel([d], {} as Node)).toBe(d);
    d.schliessen();
    expect(zu).toBe(1);
    offen = false;
    expect(d.offen).toBe(false);
    expect(escZiel([d])).toBeNull();
  });
});
