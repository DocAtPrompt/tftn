/** Meldungen in der Statuszeile. */

import { t } from "./sprache";

/**
 * „Was: Fehler“ – ohne den Vorspann „ungültiger Wert:“, den Rust bei abgelehnten Eingaben setzt;
 * sonst stünde „Nicht geändert: ungültiger Wert: …“ da (Abschlussprüfung AP11).
 */
export function meldungMit(was: string, fehler: string): string {
  const vorspann = t("ungültiger Wert: {grund}", { grund: "" });
  const rest = vorspann && fehler.startsWith(vorspann) ? fehler.slice(vorspann.length) : fehler;
  return `${was}: ${rest}`;
}
