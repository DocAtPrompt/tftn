// Zoom der Vorschau: Seitenbreite (Standard), ganze Seite oder feste Prozent der echten Größe.
// Reine Rechnung; Bedienung in main.ts, Anwendung in vorschau.ts.

export type Zoom = { art: "breite" } | { art: "seite" } | { art: "prozent"; wert: number };

export interface Masse {
  breite: number;
  hoehe: number;
}

/** Rand um die Seiten (Polsterung von #seiten, je Seite 16 px). */
const RAND = 32;
/** Bildschirmpixel je pt bei 100 % (CSS: 96 px je Zoll, Typst: 72 pt je Zoll). */
const PX_JE_PT = 96 / 72;

export const STUFEN = [25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400];
const MIN = STUFEN[0];
const MAX = STUFEN[STUFEN.length - 1];

/** Angezeigte Breite einer Seite (`seite` in pt) in einer Spalte (`spalte` in px). */
export function seitenBreite(zoom: Zoom, spalte: Masse, seite: Masse): number {
  const frei = spalte.breite - RAND;
  let b: number;
  if (zoom.art === "prozent") b = (seite.breite * PX_JE_PT * zoom.wert) / 100;
  else if (zoom.art === "seite") b = Math.min(frei, ((spalte.hoehe - RAND) * seite.breite) / seite.hoehe);
  else b = frei;
  return Math.max(100, b);
}

/** Angezeigte Breite in Prozent der echten Größe, gerundet. */
export function prozentAus(breite: number, seite: Masse): number {
  return Math.round((breite / (seite.breite * PX_JE_PT)) * 100);
}

/** Nächste feste Stufe über (`richtung` 1) bzw. unter (-1) `prozent`. */
export function stufe(prozent: number, richtung: 1 | -1): number {
  if (richtung > 0) return STUFEN.find((s) => s > prozent) ?? MAX;
  return [...STUFEN].reverse().find((s) => s < prozent) ?? MIN;
}

/** Gemerkten Zoom lesen (localStorage); Unlesbares ergibt Seitenbreite. */
export function zoomLesen(text: string | null): Zoom {
  try {
    const z = JSON.parse(text ?? "") as Zoom;
    if (z.art === "breite" || z.art === "seite") return { art: z.art };
    if (z.art === "prozent" && typeof z.wert === "number" && Number.isFinite(z.wert))
      return { art: "prozent", wert: Math.min(MAX, Math.max(MIN, Math.round(z.wert))) };
  } catch {
    // unlesbar
  }
  return { art: "breite" };
}
