import { describe, expect, it } from "vitest";
import { auswahlAmKlick, siehtNachMarkdownAus, umwandelnAngebot, umwandelnWartet } from "./umwandeln";

describe("siehtNachMarkdownAus", () => {
  it("erkennt typisches Markdown", () => {
    for (const t of [
      "# Titel\n\nText",
      "Ein **fetter** Satz",
      "Siehe [Seite](https://example.org).",
      "![Bild](bild.png)",
      "| A | B |\n|---|---|\n| 1 | 2 |",
      "> Ein Zitat\n> geht weiter",
      "## Abschnitt",
    ]) {
      expect(siehtNachMarkdownAus(t), t).toBe(true);
    }
  });

  it("lässt gewöhnlichen Text und Typst in Ruhe", () => {
    for (const t of [
      "Einfacher Satz ohne Auszeichnung.",
      "- Liste\n- geht auch in Typst",
      "1. erstens\n2. zweitens",
      "`code` und ```raw```",
      "= Überschrift\n\n*fett* und _kursiv_",
      "#import \"@local/x:0.1.0\": *\n# Titel",
      "#set text(lang: \"de\")\n**",
      "#Hashtag ohne Leerzeichen",
      "",
    ]) {
      expect(siehtNachMarkdownAus(t), t).toBe(false);
    }
  });
});

describe("auswahlAmKlick", () => {
  const r = (from: number, to: number) => ({ from, to });
  it("eine Auswahl, Klick darin", () => {
    expect(auswahlAmKlick([r(5, 20)], 5)).toEqual({ von: 5, bis: 20 });
    expect(auswahlAmKlick([r(5, 20)], 12)).toEqual({ von: 5, bis: 20 });
    expect(auswahlAmKlick([r(5, 20)], 20)).toEqual({ von: 5, bis: 20 });
  });
  it("sonst das Menü des Systems", () => {
    expect(auswahlAmKlick([r(5, 20)], 21)).toBe(null);
    expect(auswahlAmKlick([r(5, 20)], null)).toBe(null);
    expect(auswahlAmKlick([r(7, 7)], 7)).toBe(null);
    expect(auswahlAmKlick([r(1, 3), r(5, 9)], 2)).toBe(null);
  });
});

describe("umwandelnWartet", () => {
  it("hält nur Fehler zurück, solange das Angebot gilt", () => {
    expect(umwandelnWartet({ gelungen: false }, true)).toBe(true);
    expect(umwandelnWartet({ gelungen: true }, true)).toBe(false);
    expect(umwandelnWartet({ gelungen: false }, false)).toBe(false);
  });
});

describe("umwandelnAngebot", () => {
  it("HTML mit Aufbau geht vor", () => {
    expect(umwandelnAngebot("Titel\nText", "<h1>Titel</h1><p>Text</p>")).toBe("html");
    expect(umwandelnAngebot("fett", "<meta charset='utf-8'><b>fett</b>")).toBe("html");
    expect(umwandelnAngebot("a", "<ul><li>a</li></ul>")).toBe("html");
    expect(umwandelnAngebot("Link", "<a href=\"https://x.at\">Link</a>")).toBe("html");
  });

  it("HTML ohne Aufbau zählt nicht (etwa aus einem Code-Editor)", () => {
    const html = "<div style=\"color: #000\"><span>let x = 1;</span><br></div>";
    expect(umwandelnAngebot("let x = 1;", html)).toBe(null);
    expect(umwandelnAngebot("# Titel", html)).toBe("markdown");
    expect(umwandelnAngebot("Text", "<body><p>Text</p></body>")).toBe(null);
  });

  it("ohne HTML entscheidet der Text", () => {
    expect(umwandelnAngebot("Ein **fetter** Satz", "")).toBe("markdown");
    expect(umwandelnAngebot("Ein Satz", "")).toBe(null);
  });

  it("ohne Text kein Angebot", () => {
    expect(umwandelnAngebot("", "<h1>x</h1>")).toBe(null);
  });
});
