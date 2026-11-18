import { describe, expect, it } from "vitest";
import { kapitelPunkte, nachSchliessen, reiterName, reiterTitel } from "./reiter";

describe("Reiter", () => {
  it("Name ohne Ordner, Titel mit Pfad und Zustand", () => {
    expect(reiterName("kapitel/eins.typ")).toBe("eins.typ");
    expect(reiterName("main.typ")).toBe("main.typ");
    expect(reiterTitel({ datei: "main.typ", haupt: true, ungespeichert: false })).toBe("main.typ · Hauptdatei");
    expect(reiterTitel({ datei: "kapitel/eins.typ", haupt: false, ungespeichert: true })).toBe(
      "kapitel/eins.typ · ungespeichert",
    );
  });

  it("nach dem Schließen: rechter Nachbar, am Ende der linke", () => {
    const liste = ["main.typ", "a.typ", "b.typ"];
    expect(nachSchliessen(liste, "a.typ", "a.typ")).toBe("b.typ");
    expect(nachSchliessen(liste, "b.typ", "b.typ")).toBe("a.typ");
    expect(nachSchliessen(liste, "a.typ", "main.typ")).toBe("main.typ");
  });

  it("„+“ bietet nur Kapitel an, die noch nicht offen sind", () => {
    const gewaehlt: string[] = [];
    const p = kapitelPunkte(["a.typ", "kapitel/b.typ"], ["main.typ", "a.typ"], (d) => gewaehlt.push(d));
    expect(p.map((x) => x.text)).toEqual(["kapitel/b.typ"]);
    p[0].aktion();
    expect(gewaehlt).toEqual(["kapitel/b.typ"]);
    expect(kapitelPunkte(["a.typ"], ["a.typ"], () => {})[0].text).toBe("Keine weiteren Kapiteldateien");
  });
});
