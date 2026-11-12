// Fach „Seite“: die Auswahllisten und die Umrechnung der Sprache.

import { n } from "./sprache";

export const FORMATE = [
  { wert: "a4", titel: "A4" },
  { wert: "a5", titel: "A5" },
  { wert: "a3", titel: "A3" },
  { wert: "iso-b5", titel: "B5" },
  { wert: "us-letter", titel: "US Letter" },
  { wert: "us-legal", titel: "US Legal" },
];

export const SPRACHEN = [
  { wert: "de-AT", titel: n("Deutsch (Österreich)") },
  { wert: "de-DE", titel: n("Deutsch (Deutschland)") },
  { wert: "de-CH", titel: n("Deutsch (Schweiz)") },
  { wert: "en-GB", titel: n("Englisch (GB)") },
  { wert: "en-US", titel: n("Englisch (USA)") },
  { wert: "fr", titel: n("Französisch") },
  { wert: "it", titel: n("Italienisch") },
  { wert: "es", titel: n("Spanisch") },
];

export function spracheWert(lang: string, region: string | null | undefined): string {
  return region ? `${lang}-${region}` : lang;
}

export function spracheLesen(wert: string): { lang: string; region: string | null } {
  const [lang, region] = wert.split("-");
  return { lang, region: region ?? null };
}

export interface Zeile {
  innen: string;
  mitte: string;
  aussen: string;
  erste: boolean;
}

export type SeitenzahlLage = "keine" | "unten-mitte" | "unten-aussen" | "oben-aussen" | "eigene";

export const SEITENZAHLEN: { wert: SeitenzahlLage; titel: string }[] = [
  { wert: "keine", titel: n("Keine") },
  { wert: "unten-mitte", titel: n("Unten Mitte") },
  { wert: "unten-aussen", titel: n("Unten außen") },
  { wert: "oben-aussen", titel: n("Oben außen") },
];

const SEITE = "{seite}";

/** Wo die Seitenzahl steht; „eigene“, wenn sie in einem längeren Text vorkommt. */
export function seitenzahlLage(kopf: Zeile, fuss: Zeile): SeitenzahlLage {
  const felder = [kopf.innen, kopf.mitte, kopf.aussen, fuss.innen, fuss.mitte, fuss.aussen];
  if (felder.some((f) => f.includes(SEITE) && f !== SEITE)) return "eigene";
  if (fuss.mitte === SEITE) return "unten-mitte";
  if (fuss.aussen === SEITE) return "unten-aussen";
  if (kopf.aussen === SEITE) return "oben-aussen";
  return felder.includes(SEITE) ? "eigene" : "keine";
}

/** Setzt die Seitenzahl an die gewählte Stelle; andere Texte bleiben. */
export function seitenzahlAnwenden(lage: SeitenzahlLage, kopf: Zeile, fuss: Zeile): { kopf: Zeile; fuss: Zeile } {
  const leer = (z: Zeile): Zeile => ({
    ...z,
    innen: z.innen === SEITE ? "" : z.innen,
    mitte: z.mitte === SEITE ? "" : z.mitte,
    aussen: z.aussen === SEITE ? "" : z.aussen,
  });
  const k = leer(kopf);
  const f = leer(fuss);
  if (lage === "unten-mitte") f.mitte = SEITE;
  if (lage === "unten-aussen") f.aussen = SEITE;
  if (lage === "oben-aussen") k.aussen = SEITE;
  return { kopf: k, fuss: f };
}
