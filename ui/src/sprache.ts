// Sprache der Oberfläche. Deutsch steht im Code: t("…") übersetzt einen festen Text, n("…") markiert
// einen Text in einer Tabelle (bleibt deutsch, wird aber in die Sprachdatei aufgenommen) und tn(wert)
// übersetzt so einen markierten Wert später. Platzhalter heißen {name}. Die Übersetzungen kommen aus
// sprachen/<kennung>.toml über Rust (start.ts lädt sie, bevor main.ts läuft). Fehlt eine, bleibt der
// deutsche Text.

let texte: Record<string, string> = {};
let kennung = "de";

export interface OberflaecheJs {
  kennung: string;
  texte: Record<string, string>;
}

export function katalogSetzen(neu: Record<string, string>, sprache = "de"): void {
  texte = neu;
  kennung = sprache;
}

/** Ganze Zahl mit Tausendertrennern der Oberflächensprache (Deutsch: Punkt). */
export function zahl(wert: number): string {
  const ganz = Math.round(wert);
  if (kennung === "de") return String(ganz).replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  try {
    return ganz.toLocaleString(kennung);
  } catch {
    return String(ganz);
  }
}

function einsetzen(vorlage: string, werte?: Record<string, string | number>): string {
  if (!werte) return vorlage;
  return vorlage.replace(/\{([\p{L}_][\p{L}\p{N}_]*)\}/gu, (ganz, name: string) =>
    name in werte ? String(werte[name]) : ganz,
  );
}

/** Fester Text in der geltenden Sprache; `{name}` wird aus `werte` gefüllt. */
export function t(quelle: string, werte?: Record<string, string | number>): string {
  return einsetzen(texte[quelle] || quelle, werte);
}

/** Markiert einen Text für die Sprachdatei, ohne ihn zu übersetzen (für Tabellen, Konstanten). */
export function n(quelle: string): string {
  return quelle;
}

/** Übersetzt einen Wert, der mit n("…") markiert wurde. */
export function tn(quelle: string, werte?: Record<string, string | number>): string {
  return einsetzen(texte[quelle] || quelle, werte);
}

const ATTRIBUTE = ["title", "placeholder", "aria-label"];

/** Ersetzt den Text ohne den Leerraum drumherum; `null`, wenn es keine Übersetzung gibt. */
export function textErsetzen(roh: string): string | null {
  const kern = roh.trim();
  const z = kern && texte[kern];
  return z ? roh.replace(kern, () => z) : null;
}

/** Übersetzt die festen Texte einer Seite (index.html): Textknoten und sichtbare Attribute. */
export function seiteUebersetzen(wurzel: Element): void {
  const gang = document.createTreeWalker(wurzel, NodeFilter.SHOW_TEXT);
  for (let k = gang.nextNode(); k; k = gang.nextNode()) {
    const neu = textErsetzen(k.nodeValue ?? "");
    if (neu !== null) k.nodeValue = neu;
  }
  for (const el of [wurzel, ...wurzel.querySelectorAll("*")]) {
    for (const a of ATTRIBUTE) {
      const wert = el.getAttribute(a);
      const neu = wert && textErsetzen(wert);
      if (neu) el.setAttribute(a, neu);
    }
  }
}
