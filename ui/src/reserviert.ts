// Konfliktliste beider Plattformen (Entwurf 2026-11-04 Abschnitt 7, PLAN-M2 AP3): Kürzel, die das
// System, das Programm oder die Gewohnheit schon belegen, und die Editor-Kürzel, die TFTN installiert.
// Kürzel in CodeMirror-Schreibweise (`Mod` = ⌘ bzw. Strg); verglichen wird normalisiert je Plattform.

import type { KeyBinding } from "@codemirror/view";
import * as befehle from "@codemirror/commands";
import * as suche from "@codemirror/search";
import * as vervollstaendigung from "@codemirror/autocomplete";
import { t } from "./sprache";

const { defaultKeymap, historyKeymap, indentWithTab } = befehle;
const { searchKeymap } = suche;
const { completionKeymap, startCompletion } = vervollstaendigung;

export type Plattform = "mac" | "win";

export interface Reserviert {
  kuerzel: string;
  plattform: Plattform | "beide";
  was: string;
  /** Nur unter einer Bedingung belegt: Hinweis, kein Konflikt. */
  bedingt?: string;
  /** Für genau diese Aktion kein Konflikt (dieselbe Bedeutung). */
  entspricht?: string;
}

const b = (kuerzel: string, was: string, mehr: Partial<Reserviert> = {}): Reserviert => ({
  kuerzel,
  plattform: "beide",
  was,
  ...mehr,
});
const m = (kuerzel: string, was: string, mehr: Partial<Reserviert> = {}) => b(kuerzel, was, { plattform: "mac", ...mehr });
const w = (kuerzel: string, was: string, mehr: Partial<Reserviert> = {}) => b(kuerzel, was, { plattform: "win", ...mehr });

export const RESERVIERT: Reserviert[] = [
  // Gewohnt auf beiden Plattformen
  b("Mod-z", t("Rückgängig")),
  b("Mod-Shift-z", t("Wiederholen")),
  w("Mod-y", t("Wiederholen")),
  b("Mod-x", t("Ausschneiden")),
  b("Mod-c", t("Kopieren")),
  b("Mod-v", t("Einsetzen")),
  b("Mod-Shift-v", t("Einsetzen ohne Format")),
  b("Mod-a", t("Alles auswählen")),
  b("Mod-f", t("Suchen")),
  b("Mod-g", t("Weitersuchen")),
  b("Mod-Shift-g", t("Rückwärts suchen")),
  w("Mod-h", t("Ersetzen")),
  b("Mod-n", t("Neu"), { entspricht: "app/neu" }),
  b("Mod-Shift-n", t("Neues Fenster")),
  b("Mod-o", t("Öffnen"), { entspricht: "app/oeffnen" }),
  b("Mod-p", t("Drucken")),
  b("Mod-s", t("Speichern"), { entspricht: "app/speichern" }),
  b("Mod-Shift-s", t("Speichern unter")),
  b("Mod-t", t("Neuer Tab")),
  b("Mod-Shift-t", t("Geschlossenen Tab öffnen")),
  b("Mod-w", t("Fenster schließen")),
  b("Mod-q", t("Programm beenden")),
  b("Mod-r", t("Neu laden")),
  b("Mod-Shift-r", t("Neu laden ohne Zwischenspeicher")),
  b("Mod-=", t("Vergrößern")),
  b("Mod--", t("Verkleinern")),
  b("Mod-0", t("Originalgröße")),
  b("Mod-Tab", t("Nächster Tab")),
  b("Mod-Shift-Tab", t("Voriger Tab")),
  b("Mod-u", t("Unterstreichen")),
  // macOS
  m("Mod-h", t("Programm ausblenden")),
  m("Mod-Alt-h", t("Andere ausblenden")),
  m("Mod-m", t("Fenster minimieren")),
  m("Mod-,", t("Einstellungen"), { entspricht: "helfer/einstellungen" }),
  m("Mod-`", t("Fenster wechseln")),
  m("Mod-Space", t("Spotlight")),
  m("Mod-Alt-Space", t("Finder-Suche")),
  m("Mod-Shift-3", t("Bildschirmfoto")),
  m("Mod-Shift-4", t("Bildschirmfoto (Auswahl)")),
  m("Mod-Shift-5", t("Bildschirmfoto (Werkzeuge)")),
  m("Mod-Shift-q", t("Abmelden")),
  m("Ctrl-Mod-q", t("Bildschirm sperren")),
  m("Ctrl-Mod-f", t("Vollbild")),
  m("Ctrl-Mod-Space", t("Emoji und Symbole")),
  m("Mod-Alt-Escape", t("Sofort beenden")),
  m("Mod-Alt-d", t("Dock ein-/ausblenden")),
  m("Mod-e", t("Auswahl zum Suchen verwenden")),
  m("Mod-j", t("Zur Auswahl springen")),
  m("Ctrl-ArrowUp", t("Mission Control")),
  m("Ctrl-ArrowDown", t("Programmfenster")),
  m("Ctrl-ArrowLeft", t("Schreibtisch links")),
  m("Ctrl-ArrowRight", t("Schreibtisch rechts")),
  m("Mod-Shift-/", t("Hilfe")),
  m("Ctrl-Space", t("Eingabequelle wechseln"), { bedingt: t("nur bei mehreren Eingabequellen") }),
  m("Ctrl-Alt-Space", t("Nächste Eingabequelle"), { bedingt: t("nur bei mehreren Eingabequellen") }),
  // CodeMirrors Ersatz für Strg+Leertaste am Mac; tippt sonst ein Akzentzeichen.
  m("Alt-`", t("Akzent `"), { entspricht: "helfer/vervollstaendigen" }),
  // Windows
  w("Alt-F4", t("Programm beenden")),
  w("Alt-Tab", t("Programm wechseln")),
  w("Ctrl-Escape", t("Startmenü")),
  w("Ctrl-Shift-Escape", t("Task-Manager")),
  w("Ctrl-F4", t("Fenster schließen")),
  w("F1", t("Hilfe")),
  w("F5", t("Neu laden")),
  w("F7", t("Caret-Browsing")),
  w("F11", t("Vollbild")),
  w("F12", t("Entwicklerwerkzeuge")),
  w("Ctrl-Shift-i", t("Entwicklerwerkzeuge")),
  w("Ctrl-Shift-j", t("Konsole")),
  w("Ctrl-Shift-c", t("Element untersuchen")),
  w("Ctrl-Shift-Delete", t("Browserdaten löschen")),
  w("Alt-ArrowLeft", t("Zurück")),
  w("Alt-ArrowRight", t("Vorwärts")),
  w("Ctrl-Space", t("Eingabehilfe umschalten"), { bedingt: t("nur mit Eingabehilfe für ostasiatische Sprachen") }),
];

/**
 * Musterregeln auf dem normalisierten Kürzel: Am Mac tippt ⌥ mit einem Zeichen Sonderzeichen
 * (⌥L = @), unter Windows ist Strg+Alt die AltGr-Taste (AltGr+Q = @, AltGr+E = €).
 */
export function musterKonflikt(norm: string, p: Plattform): string | null {
  const teile = norm.split("-");
  const taste = teile.pop() ?? "";
  const zeichen = [...taste].length === 1;
  if (!zeichen) return null;
  const hat = (x: string) => teile.includes(x);
  if (p === "mac" && hat("Alt") && !hat("Ctrl") && !hat("Meta")) return t("tippt Sonderzeichen (z. B. ⌥L = @)");
  if (p === "win" && hat("Ctrl") && hat("Alt") && !hat("Meta"))
    return t("AltGr: tippt je nach Tastatur Zeichen wie @, € oder {");
  return null;
}

/** CodeMirror-Kürzel, die TFTN bewusst nicht installiert (Mod-i heißt kursiv, Mod-Shift-l Zitat). */
export const EDITOR_ENTFERNT = ["Mod-i", "Mod-Shift-l"];

/**
 * Ohne `EDITOR_ENTFERNT` und ohne `startCompletion`: Die Vervollständigung ist die TFTN-Aktion
 * „Vervollständigen“ mit eigener, einstellbarer Belegung.
 */
const ohne = (km: readonly KeyBinding[]) =>
  km.filter(
    (k) => !EDITOR_ENTFERNT.includes(k.key ?? "") && !EDITOR_ENTFERNT.includes(k.mac ?? "") && k.run !== startCompletion,
  );

/**
 * Genau die Editor-Keymaps, die `main.ts` installiert, in dieser Reihenfolge: `defaultKeymap`,
 * `historyKeymap`, `searchKeymap`, `completionKeymap` (Index 3, mit höchstem Vorrang, weil
 * `autocompletion` mit `defaultKeymap: false` läuft) und `indentWithTab`.
 */
export function editorKeymaps(): readonly (readonly KeyBinding[])[] {
  return [defaultKeymap, historyKeymap, searchKeymap, completionKeymap, [indentWithTab]].map(ohne);
}

/** Name des Editor-Befehls für die Anzeige; über die Exporte, weil der Bau Funktionsnamen kürzt. */
const NAMEN = new Map<unknown, string>(
  [befehle, suche, vervollstaendigung].flatMap((modul) =>
    Object.entries(modul).filter(([, f]) => typeof f === "function").map(([n, f]) => [f, n] as [unknown, string]),
  ),
);
export function befehlsname(k: KeyBinding): string {
  return (k.run && NAMEN.get(k.run)) ?? t("Editor-Befehl");
}
