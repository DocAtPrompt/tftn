import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { falschFeld, falschSetzen, ohneIgnorierte, wortAn } from "./rechtschreibung";

function mit(text: string, bereiche: [number, number][]) {
  const s = EditorState.create({ doc: text, extensions: [falschFeld] });
  return s.update({ effects: falschSetzen.of(bereiche) }).state;
}

describe("Rechtschreibung", () => {
  it("findet das unterwellte Wort an einer Stelle", () => {
    const s = mit("Ein Fehlr hier", [[4, 9]]);
    expect(wortAn(s, 6)).toEqual({ von: 4, bis: 9, wort: "Fehlr" });
    expect(wortAn(s, 4)?.wort).toBe("Fehlr");
    expect(wortAn(s, 9)?.wort).toBe("Fehlr");
    expect(wortAn(s, 1)).toBeNull();
    expect(wortAn(s, 12)).toBeNull();
  });

  it("Markierungen wandern mit, bis die nächste Prüfung kommt", () => {
    let s = mit("Ein Fehlr hier", [[4, 9]]);
    s = s.update({ changes: { from: 0, insert: "Noch " } }).state;
    expect(wortAn(s, 11)).toEqual({ von: 9, bis: 14, wort: "Fehlr" });
    s = s.update({ effects: falschSetzen.of([]) }).state;
    expect(wortAn(s, 11)).toBeNull();
  });

  it("Bereiche außerhalb des Texts fallen weg", () => {
    const s = mit("kurz", [
      [0, 4],
      [3, 99],
      [2, 2],
    ]);
    expect(s.field(falschFeld).size).toBe(1);
  });

  it("Ignorierte Wörter bleiben unmarkiert", () => {
    const s = EditorState.create({ doc: "Fehlr und Lüddecke" });
    expect(
      ohneIgnorierte(
        s,
        [
          [0, 5],
          [10, 18],
        ],
        new Set(["Lüddecke"]),
      ),
    ).toEqual([[0, 5]]);
  });
});
