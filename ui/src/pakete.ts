// Fach „Pakete“ (PLAN-M2 AP9, Entwurf 8.1): reine Logik für Knöpfe und Anzeigen, ohne DOM.

import { t } from "./sprache";

/** Ein Treffer der Suche (`pakete_suchen`, `TrefferJs` in `src-tauri/src/pakete.rs`). */
export interface Treffer {
  name: string;
  version: string;
  beschreibung: string | null;
  lizenz: string | null;
  im_projekt: boolean;
  /** samt allen Paketen, die es importiert */
  im_zwischenspeicher: boolean;
  /** eine Paketbeschreibung des Katalogs passt */
  helfer: boolean;
}

export interface IndexStand {
  datum: string;
  pakete: number;
}

export interface Knopf {
  an: boolean;
  titel: string;
}

const LAEUFT = t("Es wird gerade geholt …");

export function hinzufuegenKnopf(tr: Treffer, offline: boolean, laeuft: boolean): Knopf {
  if (laeuft) return { an: false, titel: LAEUFT };
  if (tr.im_projekt) return { an: false, titel: t("Schon im Projekt") };
  if (tr.im_zwischenspeicher) return { an: true, titel: t("Aus dem Zwischenspeicher ins Projekt übernehmen") };
  if (offline) return { an: false, titel: t("Ganz offline – nur Pakete im Zwischenspeicher") };
  return { an: true, titel: t("Von packages.typst.org holen (samt Abhängigkeiten) und ins Projekt übernehmen") };
}

export function indexKnopf(index: IndexStand | null, offline: boolean, laeuft: boolean): Knopf & { text: string } {
  const text = index ? t("Index aktualisieren") : t("Index holen");
  if (offline) return { an: false, text, titel: t("Ganz offline – TFTN holt nichts aus dem Netz") };
  if (laeuft) return { an: false, text, titel: LAEUFT };
  return { an: true, text, titel: t("Paketindex von packages.typst.org holen") };
}

/** Statuszeile: Zieladresse, solange der Holer läuft (Festlegung 10). */
export function verbindungText(laeuft: boolean): string {
  return laeuft ? "⇅ packages.typst.org" : "";
}

export function indexText(index: IndexStand | null): string {
  if (!index) return t("Noch kein Index – „Index holen“");
  const [jahr, monat, tag] = index.datum.split("-").map(Number);
  const zahl = index.pakete === 1 ? t("1 Paket") : t("{anzahl} Pakete", { anzahl: index.pakete });
  return t("Index vom {tag}. {monat}. {jahr} · {zahl}", { tag, monat, jahr, zahl });
}

/** „Holen“ bei einem Paket mit Beschreibung, das im Projekt fehlt (PLAN-M2 AP10). */
export function holbarKnopf(h: { im_zwischenspeicher: boolean }, offline: boolean, laeuft: boolean): Knopf {
  if (laeuft) return { an: false, titel: LAEUFT };
  if (h.im_zwischenspeicher) return { an: true, titel: t("Aus dem Zwischenspeicher ins Projekt übernehmen") };
  if (offline) return { an: false, titel: t("Ganz offline – nur Pakete im Zwischenspeicher") };
  return { an: true, titel: t("Von packages.typst.org holen (samt Abhängigkeiten) und ins Projekt übernehmen") };
}

export function entfernenKnopf(p: { benutzt: boolean }, laeuft: boolean): Knopf {
  if (laeuft) return { an: false, titel: LAEUFT };
  if (p.benutzt) return { an: false, titel: t("Paket wird noch importiert – erst die Importe entfernen") };
  return { an: true, titel: t("Aus dem Projekt entfernen, samt Abhängigkeiten, die sonst niemand braucht") };
}

export function wechselKnopf(offline: boolean, laeuft: boolean): Knopf {
  if (laeuft) return { an: false, titel: LAEUFT };
  if (offline) return { an: false, titel: t("Ganz offline – nur Versionen im Zwischenspeicher") };
  return { an: true, titel: t("Speichert, holt die Version, baut eine Probe und zeigt die geänderten Seiten") };
}

/** Hinweis, wenn es Bausteine gibt, aber nicht für diese Version (PLAN-M2, Risiken). */
export function helferText(p: { helfer: boolean }, hatBeschreibung: boolean): string | null {
  return !p.helfer && hatBeschreibung ? t("Für diese Version keine Helfer") : null;
}
