import { describe, expect, it } from "vitest";
import { type MarkenEreignis, MarkenSchalter } from "./marken";

const ab = (key: string, zeit: number, m: Partial<{ alt: boolean; ctrl: boolean; meta: boolean; shift: boolean }> = {}): MarkenEreignis => ({
  art: "taste-ab",
  key,
  zeit,
  alt: m.alt ?? key === "Alt",
  ctrl: m.ctrl ?? key === "Control",
  meta: m.meta ?? key === "Meta",
  shift: m.shift ?? key === "Shift",
});
const auf = (key: string, zeit: number): MarkenEreignis => ({ art: "taste-auf", key, zeit });
const zeit = (z: number): MarkenEreignis => ({ art: "zeit", zeit: z });

/** Führt die Folge aus und meldet, ob nach irgendeinem Ereignis Marken sichtbar waren. */
function jemalsSichtbar(folge: MarkenEreignis[]): boolean {
  const s = new MarkenSchalter();
  return folge.some((e) => s.ereignis(e).sichtbar);
}

describe("MarkenSchalter", () => {
  it("tippen_oeffnet_nichts", () => {
    const folge: MarkenEreignis[] = [];
    let t = 0;
    const tippe = (key: string, m: Parameters<typeof ab>[2] = {}, halten = 60) => {
      folge.push(ab(key, t, m), zeit(t + 10));
      t += halten;
      folge.push(zeit(t), auf(key, t));
      t += 40;
    };
    for (const c of "Die Rücklaufquote lag bei 64 %. ") tippe(c);
    // Mac: ⌥5 = [, ⌥L = @ (Alt gedrückt, dann das Zeichen, dann beide los)
    for (const [taste, zeichen] of [
      ["5", "["],
      ["l", "@"],
    ]) {
      folge.push(ab("Alt", t), zeit(t + 100));
      t += 150;
      folge.push(ab(zeichen, t, { alt: true }), zeit(t + 200));
      t += 250;
      folge.push(auf(taste, t), zeit(t + 300), auf("Alt", t + 350));
      t += 400;
    }
    // Umschalt für Großbuchstaben
    folge.push(ab("Shift", t), zeit(t + 500));
    t += 520;
    tippe("D", { shift: true });
    folge.push(auf("Shift", t));
    t += 40;
    // Windows AltGr+Q = @ (Strg+Alt)
    folge.push(ab("Control", t), ab("Alt", t + 5, { ctrl: true }), zeit(t + 450));
    t += 500;
    folge.push(ab("@", t, { alt: true, ctrl: true }), zeit(t + 600), auf("Alt", t + 650), auf("Control", t + 660));
    t += 700;
    tippe("Backspace");
    tippe("Enter");
    folge.push(zeit(t + 5000));
    expect(jemalsSichtbar(folge)).toBe(false);
  });

  it("alt_halten_zeigt_und_loslassen_verbirgt", () => {
    const s = new MarkenSchalter();
    expect(s.ereignis(ab("Alt", 0)).sichtbar).toBe(false);
    expect(s.ereignis(zeit(399)).sichtbar).toBe(false);
    expect(s.ereignis(zeit(400)).sichtbar).toBe(true);
    expect(s.ereignis(zeit(900)).sichtbar).toBe(true);
    expect(s.ereignis(auf("Alt", 950)).sichtbar).toBe(false);
  });

  it("alt_mit_taste_bricht_ab", () => {
    const s = new MarkenSchalter();
    s.ereignis(ab("Alt", 0));
    expect(s.ereignis(ab("e", 100, { alt: true })).sichtbar).toBe(false);
    expect(s.ereignis(zeit(600)).sichtbar).toBe(false);
    expect(s.ereignis(auf("e", 650)).sichtbar).toBe(false);
    expect(s.ereignis(zeit(1200)).sichtbar).toBe(false);
    // Erst ein neues Drücken von Alt zählt wieder.
    s.ereignis(auf("Alt", 1300));
    s.ereignis(ab("Alt", 2000));
    expect(s.ereignis(zeit(2400)).sichtbar).toBe(true);
  });

  it("maus_bricht_ab_und_altgr_zeigt_nie", () => {
    const s = new MarkenSchalter();
    s.ereignis(ab("Alt", 0));
    s.ereignis({ art: "maus" });
    expect(s.ereignis(zeit(500)).sichtbar).toBe(false);
    const w = new MarkenSchalter();
    w.ereignis(ab("Alt", 0, { ctrl: true }));
    expect(w.ereignis(zeit(800)).sichtbar).toBe(false);
  });

  it("kuerzel_bis_escape", () => {
    const s = new MarkenSchalter();
    expect(s.ereignis({ art: "kuerzel" }).sichtbar).toBe(true);
    expect(s.ereignis(zeit(5000)).sichtbar).toBe(true);
    expect(s.ereignis({ art: "escape" }).sichtbar).toBe(false);
    expect(s.ereignis({ art: "kuerzel" }).sichtbar).toBe(true);
    expect(s.ereignis({ art: "dokument-geaendert" }).sichtbar).toBe(false);
    s.ereignis({ art: "kuerzel" });
    expect(s.ereignis({ art: "maus" }).sichtbar).toBe(false);
  });
});
