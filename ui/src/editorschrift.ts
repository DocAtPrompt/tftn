// Schriftgröße des Editors: Cmd/Strg + und − in Stufen, Cmd/Strg+0 zurück auf die Größe des Skins.
// Ein Faktor auf `--editor-groesse` (stil.css: `--editor-faktor`), gemerkt je Rechner. Die Vorschau
// und die Oberfläche bleiben, wie sie sind.

export const SCHRIFT_STUFEN = [60, 70, 80, 90, 100, 110, 125, 150, 175, 200, 250] as const;
const MERKER = "tftn-editor-schrift";

/** Die nächste Stufe über (`1`) bzw. unter (`-1`) `prozent`; an den Enden bleibt es stehen. */
export function schriftStufe(prozent: number, richtung: 1 | -1): number {
  if (richtung > 0) return SCHRIFT_STUFEN.find((s) => s > prozent) ?? SCHRIFT_STUFEN.at(-1)!;
  return [...SCHRIFT_STUFEN].reverse().find((s) => s < prozent) ?? SCHRIFT_STUFEN[0];
}

/** Gemerkter Wert, wenn er eine Stufe ist; sonst 100. */
export function schriftLesen(wert: string | null): number {
  const n = Number(wert);
  return (SCHRIFT_STUFEN as readonly number[]).includes(n) ? n : 100;
}

export function schriftGemerkt(): number {
  try {
    return schriftLesen(localStorage.getItem(MERKER));
  } catch {
    return 100;
  }
}

/** Setzt die Größe am Element (`:root`) und merkt sie; 100 nimmt den Faktor zurück. */
export function schriftSetzen(ziel: HTMLElement, prozent: number) {
  if (prozent === 100) ziel.style.removeProperty("--editor-faktor");
  else ziel.style.setProperty("--editor-faktor", String(prozent / 100));
  try {
    if (prozent === 100) localStorage.removeItem(MERKER);
    else localStorage.setItem(MERKER, String(prozent));
  } catch {
    // ohne localStorage gilt die Größe bis zum Neuladen
  }
}
