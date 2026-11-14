import { describe, expect, it } from "vitest";
import { type SkinJs, skinAnwenden, skinOptionen } from "./skin";

class Stil {
  werte = new Map<string, string>();
  setProperty(n: string, v: string) {
    this.werte.set(n, v);
  }
  removeProperty(n: string) {
    const v = this.werte.get(n) ?? "";
    this.werte.delete(n);
    return v;
  }
}

const papier: SkinJs = {
  kennung: "papier",
  name: "Papier",
  art: "hell",
  variablen: { "--grund": "#f4ecd8", "--syntax-key": "#8e2050" },
  warnungen: [],
};
const kontrast: SkinJs = { kennung: "kontrast", name: "Hoher Kontrast", art: "dunkel", variablen: { "--text": "#ffffff" }, warnungen: [] };

describe("skinAnwenden", () => {
  it("setzt die Variablen und color-scheme", () => {
    const s = new Stil();
    const gesetzt = skinAnwenden(s, [], papier);
    expect(s.werte.get("--grund")).toBe("#f4ecd8");
    expect(s.werte.get("color-scheme")).toBe("light");
    expect(gesetzt.sort()).toEqual(["--grund", "--syntax-key"]);
  });

  it("ein Wechsel entfernt, was der vorige Skin gesetzt hat", () => {
    const s = new Stil();
    const g = skinAnwenden(s, [], papier);
    skinAnwenden(s, g, kontrast);
    expect(s.werte.has("--grund")).toBe(false);
    expect(s.werte.get("--text")).toBe("#ffffff");
    expect(s.werte.get("color-scheme")).toBe("dark");
  });

  it("ohne Skin gilt wieder das System", () => {
    const s = new Stil();
    const g = skinAnwenden(s, [], papier);
    expect(skinAnwenden(s, g, null)).toEqual([]);
    expect(s.werte.size).toBe(0);
  });

  it("nur Variablen, kein beliebiges CSS", () => {
    const s = new Stil();
    skinAnwenden(s, [], { ...papier, variablen: { display: "none", "--grund": "#fff" } });
    expect(s.werte.has("display")).toBe(false);
  });
});

describe("skinOptionen", () => {
  it("„Wie das System“ zuerst, gewählter markiert, fehlender gewählter bleibt sichtbar", () => {
    const liste = [
      { kennung: "dunkel", name: "Dunkel" },
      { kennung: "papier", name: "Papier" },
    ];
    expect(skinOptionen(liste, null)[0]).toEqual({ wert: "", text: "Wie das System (hell/dunkel)", gewaehlt: true });
    expect(skinOptionen(liste, "papier").find((o) => o.gewaehlt)?.wert).toBe("papier");
    const weg = skinOptionen(liste, "geloescht");
    expect(weg.at(-1)).toEqual({ wert: "geloescht", text: "geloescht (nicht gefunden)", gewaehlt: true });
  });
});
