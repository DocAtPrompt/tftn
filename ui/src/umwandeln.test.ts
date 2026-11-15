import { describe, expect, it } from "vitest";
import { siehtNachMarkdownAus, umwandelnAngebot } from "./umwandeln";

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
