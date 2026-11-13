import { describe, expect, it } from "vitest";
import { auszeichnungEntfernen } from "./auszeichnung";

/** Wendet das Ergebnis auf `text` an; `|` markieren Anfang und Ende der Auswahl. */
function probe(mitMarken: string, art: "fett" | "kursiv"): string | null {
  const von = mitMarken.indexOf("|");
  const bis = mitMarken.indexOf("|", von + 1) - 1;
  const text = mitMarken.replace(/\|/g, "");
  const a = auszeichnungEntfernen(text, von, bis, art);
  if (a === null) return null;
  const neu = text.slice(0, a.von) + a.einsetzen + text.slice(a.bis);
  return neu.slice(0, a.auswahlVon) + "|" + neu.slice(a.auswahlVon, a.auswahlBis) + "|" + neu.slice(a.auswahlBis);
}

describe("⌘B/⌘I schalten um", () => {
  it("Auswahl ist der ganze Aufruf", () => {
    expect(probe("Ein |#strong[Wort]| hier", "fett")).toBe("Ein |Wort| hier");
    expect(probe("Ein |#emph[Wort]| hier", "kursiv")).toBe("Ein |Wort| hier");
  });

  it("Auswahl ist der Inhalt des Aufrufs", () => {
    expect(probe("Ein #strong[|Wort|] hier", "fett")).toBe("Ein |Wort| hier");
    expect(probe("Ein #emph[|zwei Wörter|].", "kursiv")).toBe("Ein |zwei Wörter|.");
  });

  it("Kurzform *…* und _…_", () => {
    expect(probe("Ein |*Wort*| hier", "fett")).toBe("Ein |Wort| hier");
    expect(probe("Ein *|Wort|* hier", "fett")).toBe("Ein |Wort| hier");
    expect(probe("Ein _|Wort|_ hier", "kursiv")).toBe("Ein |Wort| hier");
  });

  it("nicht ausgezeichnet oder andere Art: nichts zu entfernen", () => {
    expect(probe("Ein |Wort| hier", "fett")).toBeNull();
    expect(probe("Ein #emph[|Wort|] hier", "fett")).toBeNull();
    expect(probe("Ein *|Wort|* hier", "kursiv")).toBeNull();
    expect(probe("Ein #strong[|Wo|rt] hier", "fett")).toBeNull();
    expect(probe("Ein ||Wort", "fett")).toBeNull();
  });
});
