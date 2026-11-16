import { describe, expect, it } from "vitest";
import {
  type BibEintrag,
  type Feld,
  alsEintrag,
  eintraegeFiltern,
  formularFelder,
  personenWert,
  personenZeilen,
  schluesselVorschlag,
} from "./literatur";

const f = (name: string, wert: string, roh = false): Feld => ({ name, wert, roh });

describe("formularFelder", () => {
  it("die üblichen Felder der Art in fester Reihenfolge, dann die übrigen vorhandenen", () => {
    const felder = formularFelder("article", [f("title", "T"), f("keywords", "x"), f("author", "A")]);
    expect(felder.map((x) => x.name)).toEqual([
      "author",
      "title",
      "journal",
      "year",
      "volume",
      "number",
      "pages",
      "doi",
      "url",
      "keywords",
    ]);
    expect(felder.find((x) => x.name === "title")?.wert).toBe("T");
    expect(felder.find((x) => x.name === "journal")?.wert).toBe("");
  });
  it("biblatex-Namen ersetzen den üblichen Namen an seiner Stelle", () => {
    const felder = formularFelder("article", [f("journaltitle", "Nature"), f("date", "2015-05")]);
    const namen = felder.map((x) => x.name);
    expect(namen).toContain("journaltitle");
    expect(namen).not.toContain("journal");
    expect(namen.indexOf("date")).toBe(3);
  });
  it("unbekannte Art: Grundfelder", () => {
    expect(formularFelder("patent", []).map((x) => x.name)).toEqual(["author", "title", "year", "url", "note"]);
  });
  it("rohe Werte bleiben roh", () => {
    expect(formularFelder("article", [f("year", "2015", true)]).find((x) => x.name === "year")).toEqual(f("year", "2015", true));
  });
});

describe("Personen", () => {
  it("eine Person je Zeile", () => {
    expect(personenZeilen("LeCun, Yann and Bengio, Yoshua AND Hinton, G.")).toBe("LeCun, Yann\nBengio, Yoshua\nHinton, G.");
    expect(personenWert("LeCun, Yann\n\n  Bengio, Yoshua  \n")).toBe("LeCun, Yann and Bengio, Yoshua");
  });
  it("„and“ in geschweiften Klammern trennt nicht", () => {
    expect(personenZeilen("{Barnes and Noble} and Smith, J.")).toBe("{Barnes and Noble}\nSmith, J.");
  });
});

describe("alsEintrag", () => {
  it("Personen zurück mit „and“, leere Felder fallen weg", () => {
    const e = alsEintrag("book", " knuth1984 ", [f("author", "Knuth, Donald\nLevy, Silvio"), f("title", "Buch"), f("note", " ")]);
    expect(e).toEqual<BibEintrag>({
      art: "book",
      schluessel: "knuth1984",
      felder: [f("author", "Knuth, Donald and Levy, Silvio"), f("title", "Buch")],
    });
  });
});

describe("schluesselVorschlag", () => {
  it("Nachname der ersten Person und Jahr, frei gemacht mit a, b, …", () => {
    expect(schluesselVorschlag("Müller, Anna and Weber, Jo", "2026", new Set())).toBe("mueller2026");
    expect(schluesselVorschlag("Anna Müller", "2026", new Set())).toBe("mueller2026");
    expect(schluesselVorschlag("van der Berg, Jan", "1999", new Set(["berg1999"]))).toBe("berg1999a");
    expect(schluesselVorschlag("", "", new Set())).toBe("quelle");
    expect(schluesselVorschlag("{World Health Organization}", "2020-03-01", new Set())).toBe("world2020");
  });
});

describe("eintraegeFiltern", () => {
  const liste: BibEintrag[] = [
    { art: "article", schluessel: "lecun2015", felder: [f("author", "LeCun, Yann"), f("title", "Deep learning"), f("year", "2015")] },
    { art: "book", schluessel: "mueller2026", felder: [f("author", "Müller, Anna"), f("title", "Über Typst")] },
  ];
  it("jedes Wort muss vorkommen, Umlaute auch ohne Punkte", () => {
    expect(eintraegeFiltern(liste, "").length).toBe(2);
    expect(eintraegeFiltern(liste, "deep 2015").map((e) => e.schluessel)).toEqual(["lecun2015"]);
    expect(eintraegeFiltern(liste, "muller uber").map((e) => e.schluessel)).toEqual(["mueller2026"]);
    expect(eintraegeFiltern(liste, "Müller").map((e) => e.schluessel)).toEqual(["mueller2026"]);
    expect(eintraegeFiltern(liste, "nichts")).toEqual([]);
  });
});
