// Wörter und Zeichen für die Statuszeile; gezählt wird in Rust (tftn_edit::zaehlen).

import { t, zahl } from "./sprache";

export interface Zahlen {
  woerter: number;
  zeichen: number;
  ohne_leerzeichen: number;
}

export function tausender(n: number): string {
  return zahl(n);
}

export function zahlText(gesamt: Zahlen, auswahl: Zahlen | null): string {
  if (auswahl) {
    return t("Auswahl: {auswahl} von {gesamt} Wörtern · {zeichen} Zeichen", {
      auswahl: tausender(auswahl.woerter),
      gesamt: tausender(gesamt.woerter),
      zeichen: tausender(auswahl.zeichen),
    });
  }
  const werte = { woerter: tausender(gesamt.woerter), zeichen: tausender(gesamt.zeichen) };
  return gesamt.woerter === 1 ? t("{woerter} Wort · {zeichen} Zeichen", werte) : t("{woerter} Wörter · {zeichen} Zeichen", werte);
}
