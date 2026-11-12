// Logik des Fachs „Formel“ (PLAN-M2 AP10): das Textfeld als Zustand mit Auswahl und Feldern (Tab
// springt wie bei den Bausteinen im Editor). Der DOM-Teil steht in `formelfach.ts`.

import type { EingesetztJs } from "./bausteine";
import { t } from "./sprache";

export interface Bereich {
  von: number;
  bis: number;
}

export interface FeldZustand extends Bereich {
  text: string;
  felder: Bereich[];
}

/** Ersetzt die Auswahl durch das Schnipsel und wählt sein erstes Feld. */
export function einsetzen(z: FeldZustand, e: EingesetztJs): FeldZustand {
  const text = z.text.slice(0, z.von) + e.text + z.text.slice(z.bis);
  const felder = e.felder.map((f) => ({ von: z.von + f.von, bis: z.von + f.bis }));
  const ziel = felder[0] ?? { von: z.von + (e.cursor ?? e.text.length), bis: z.von + (e.cursor ?? e.text.length) };
  return { text, ...ziel, felder };
}

const BUCHSTABE = /[\p{L}\p{N}]/u;

/** Fügt ein Symbol ein; ein Leerzeichen nur, wo sonst Buchstaben zusammenwüchsen. */
export function symbolEinsetzen(z: FeldZustand, symbol: string): FeldZustand {
  const davor = z.von > 0 && BUCHSTABE.test(z.text[z.von - 1]) ? " " : "";
  const danach = z.bis < z.text.length && BUCHSTABE.test(z.text[z.bis]) ? " " : "";
  const text = z.text.slice(0, z.von) + davor + symbol + danach + z.text.slice(z.bis);
  const stelle = z.von + davor.length + symbol.length;
  return nachEingabe(z, text, stelle, stelle);
}

/** Neuer Text aus dem Feld: Felder wandern mit, ein Feld, in dem getippt wurde, wächst mit. */
export function nachEingabe(z: FeldZustand, text: string, von: number, bis: number): FeldZustand {
  const alt = z.text;
  let vorne = 0;
  while (vorne < alt.length && vorne < text.length && alt[vorne] === text[vorne]) vorne++;
  let hinten = 0;
  while (
    hinten < alt.length - vorne &&
    hinten < text.length - vorne &&
    alt[alt.length - 1 - hinten] === text[text.length - 1 - hinten]
  )
    hinten++;
  const altEnde = alt.length - hinten;
  const delta = text.length - alt.length;
  const felder = z.felder.map((f) => {
    if (f.von >= altEnde && f.von > vorne) return { von: f.von + delta, bis: f.bis + delta };
    if (f.bis < vorne) return f;
    return { von: Math.min(f.von, vorne), bis: Math.max(f.bis + delta, vorne) };
  });
  return { text, von, bis, felder };
}

/** Das nächste bzw. vorige Feld, von der Auswahl aus; `null`, wenn es keines mehr gibt. */
export function naechstesFeld(z: FeldZustand, rueckwaerts: boolean): FeldZustand | null {
  const ziel = rueckwaerts
    ? [...z.felder].reverse().find((f) => f.bis <= z.von)
    : z.felder.find((f) => f.von >= z.bis);
  return ziel ? { ...z, von: ziel.von, bis: ziel.bis } : null;
}

export function formelKnoepfe(s: { text: string | null; einfuegbar: boolean; eingabe: string; laeuft: boolean }): {
  uebernehmen: boolean;
  einfuegen: boolean;
  titel: string;
} {
  const eingabe = s.eingabe.trim();
  if (s.text !== null) {
    const geaendert = eingabe !== "" && eingabe !== s.text;
    return { uebernehmen: geaendert && !s.laeuft, einfuegen: false, titel: geaendert ? "" : t("Nichts geändert.") };
  }
  if (!s.einfuegbar) return { uebernehmen: false, einfuegen: false, titel: t("Eine neue Formel geht nur im Text.") };
  if (eingabe === "") return { uebernehmen: false, einfuegen: false, titel: t("Erst eine Formel eingeben.") };
  return { uebernehmen: false, einfuegen: !s.laeuft, titel: "" };
}

/** Zählt Anfragen, damit eine späte Antwort keine neuere überschreibt. */
export class Folge {
  private nr = 0;
  neu(): number {
    return ++this.nr;
  }
  aktuell(nr: number): boolean {
    return nr === this.nr;
  }
}
