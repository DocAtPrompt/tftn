/**
 * Einfügen mit Umwandlung: Ob eingefügter Text nach Markdown bzw. formatiertem HTML aussieht und
 * das Hinweisband „In Typst umwandeln“ anbieten soll. Umgewandelt wird in Rust über Pandoc.
 */

/** Zeichen, die es in Typst so nicht gibt, in Markdown aber oft. Listen, `code` und ``` gelten
 *  in beiden und zählen deshalb nicht. */
const MARKDOWN = [
  /^#{1,6}[ \t]+\S/m, // Überschrift
  /\*\*[^*\n]+\*\*/, // fett
  /!?\[[^\]\n]+\]\([^)\s]+\)/, // Link, Bild
  /^\|?[ \t]*:?-{3,}:?[ \t]*(\|[ \t]*:?-{3,}:?[ \t]*)+\|?[ \t]*$/m, // Tabellenlinie
  /^>[ \t]?\S/m, // Zitat
];

/** Typst-Quelltext soll nicht „umgewandelt“ werden. */
const TYPST = /^[ \t]*#(import|include|set|show|let|figure|table)\b/m;

export function siehtNachMarkdownAus(text: string): boolean {
  if (TYPST.test(text)) return false;
  return MARKDOWN.some((m) => m.test(text));
}

/** HTML mit Aufbau (Überschriften, Listen, Tabellen, Auszeichnung, Links, Bilder), wie es ein
 *  Browser oder Word liefert; ein Code-Editor liefert nur `div`/`span`. */
const AUFBAU = /<(h[1-6]|ul|ol|table|strong|b|em|i|blockquote|img)\b|<a\b[^>]*\bhref=/i;

export type Angebot = "markdown" | "html";

/** Rechtsklick in die einzige, nicht leere Auswahl: deren Bereich (für „In Typst umwandeln“);
 *  sonst `null` (das Menü des Systems bzw. der Rechtschreibung). */
export function auswahlAmKlick(
  bereiche: readonly { from: number; to: number }[],
  pos: number | null,
): { von: number; bis: number } | null {
  if (bereiche.length !== 1 || pos === null) return null;
  const { from, to } = bereiche[0];
  return from < to && pos >= from && pos <= to ? { von: from, bis: to } : null;
}

/** Ein misslungener Typst-Lauf, während das Angebot noch gilt: kein Fehlerband, die Vorschau
 *  bleibt beim letzten Stand (sonst verdrängt das Fehlerband das Angebot). */
export function umwandelnWartet(lauf: { gelungen: boolean }, angebotOffen: boolean): boolean {
  return angebotOffen && !lauf.gelungen;
}

/** Was angeboten wird: HTML mit Aufbau vor Markdown im Text; `null`: einfach einfügen. */
export function umwandelnAngebot(text: string, html: string): Angebot | null {
  if (text.trim() === "") return null;
  if (AUFBAU.test(html)) return "html";
  return siehtNachMarkdownAus(text) ? "markdown" : null;
}
