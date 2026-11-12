// Literatur aus DOI: erkennt eine DOI in der Suche des Zitat-Dialogs (wie `doi_lesen` im Holer).

export interface DoiJs {
  schluessel: string;
  neu: boolean;
  datei: string;
  /** Fehlt `#bibliography(…)`: Text fürs Ende von main.typ */
  verzeichnis: string | null;
}

const VORSAETZE = ["https://doi.org/", "http://doi.org/", "https://www.doi.org/", "https://dx.doi.org/", "http://dx.doi.org/"];

/** Die DOI in der Eingabe, sonst `null`. */
export function doiAusEingabe(eingabe: string): string | null {
  let t = eingabe.trim();
  for (const v of VORSAETZE) if (t.startsWith(v)) t = t.slice(v.length);
  if (/^doi:/i.test(t)) t = t.slice(4).trimStart();
  return /^10\.\d{4,}(\.\d+)*\/\S+$/.test(t) ? t : null;
}
