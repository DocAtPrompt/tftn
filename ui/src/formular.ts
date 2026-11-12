// Fach „Dokument“ (PLAN-M2 AP7): reine Logik des Formulars. Die Typen spiegeln die Rust-Seite
// (`tftn-formular` in kebab-case, die Antworten der App in snake_case). Geprüft wird endgültig in
// Rust; hier wird nur die Eingabe gelesen (deutsches Zahl- und Datumsformat) und angezeigt.

import { eingabeZuWert } from "./werte";
import { t } from "./sprache";

export type Schritt = string | number;
export type Pfad = Schritt[];
export type TextWidget = "einzeilig" | "mehrzeilig" | "markup";

export type Art =
  | {
      art: "text";
      widget: TextWidget;
      format: string | null;
      "min-laenge": number | null;
      "max-laenge": number | null;
      muster: string | null;
      standard: string | null;
    }
  | { art: "zahl"; ganzzahl: boolean; min: number | null; max: number | null; standard: number | null }
  | { art: "schalter"; standard: boolean | null }
  | { art: "datum"; standard: string | null }
  | { art: "wahl"; werte: unknown[]; beschriftungen: string[]; standard: unknown }
  | { art: "liste"; element: Art; min: number | null; max: number | null }
  | { art: "gruppe"; felder: Feld[] }
  | { art: "eintraege"; felder: Feld[]; min: number | null; max: number | null };

export interface Feld {
  name: string;
  titel: string;
  hilfe: string | null;
  pflicht: boolean;
  gruppe: string | null;
  sensibel: boolean;
  pandoc: string | null;
  art: Art;
}

export interface Formular {
  titel: string;
  felder: Feld[];
}

export interface Meldung {
  pfad: Pfad;
  text: string;
  fehler: boolean;
}

export interface FormularJs {
  stand: string;
  datei: string;
  formular: Formular | null;
  werte: Record<string, unknown>;
  meldungen: Meldung[];
  dateifehler: string | null;
  profilfehler: { ort: string; text: string }[];
  sensibel: string | null;
  rueck: boolean;
  vor: boolean;
  vorschlaege: Record<string, string[]>;
  abgelehnt: Meldung | null;
}

export interface ProjektJs {
  typ: string;
  version: string;
  typst: string;
  pakete: [string, string][];
  literatur_im_formular: boolean;
  /** Neuere Version des Dokumenttyps (PLAN-M2 AP8) */
  aktualisierung: { version: string; herkunft: "eigen" | "tftn" } | null;
}

/** Seitenvergleich alt/neu beim Aktualisieren des Dokumenttyps (PLAN-M2 AP8). */
export interface VergleichJs {
  alt: string;
  neu: string;
  seiten_alt: number;
  seiten_neu: number;
  geaendert: number[];
  bilder: { seite: number; alt: string | null; neu: string | null }[];
  fehler: string[];
  stellen: string[];
}

export type EintragAktion =
  | { art: "hinzufuegen" }
  | { art: "entfernen"; index: number }
  | { art: "verschieben"; index: number; nach: number };

export type Gelesen = { wert: unknown | null } | { fehler: string };

const DATUM_DE = /^(\d{1,2})\.(\d{1,2})\.(\d{4})$/;
const DATUM_ISO = /^(\d{4})-(\d{2})-(\d{2})$/;

function datumGueltig(j: number, m: number, tag: number): boolean {
  const d = new Date(Date.UTC(j, m - 1, tag));
  return j >= 1 && d.getUTCFullYear() === j && d.getUTCMonth() === m - 1 && d.getUTCDate() === tag;
}

const zwei = (n: number) => String(n).padStart(2, "0");

/** Eingabe → JSON-Wert; `null` = leer; `{fehler}` bei unlesbarer Eingabe (vor Rust). */
export function eingabeLesen(art: Art, roh: string | boolean): Gelesen {
  if (art.art === "schalter") return { wert: roh === true || roh === "true" };
  const s = typeof roh === "string" ? roh : String(roh);
  switch (art.art) {
    case "text": {
      if (s.trim() === "") return { wert: null };
      return { wert: art.widget === "einzeilig" ? s.trim() : s };
    }
    case "zahl": {
      if (s.trim() === "") return { wert: null };
      const e = eingabeZuWert({ art: "zahl", mit: { min: art.min, max: art.max, ganzzahl: false } }, s);
      if ("fehler" in e) return e;
      const z = (e.wert as { wert: number }).wert;
      if (art.ganzzahl && !Number.isInteger(z)) return { fehler: t("ganze Zahl erwartet") };
      return { wert: z };
    }
    case "datum": {
      const eing = s.trim();
      if (eing === "") return { wert: null };
      const de = DATUM_DE.exec(eing);
      const iso = DATUM_ISO.exec(eing);
      const [j, m, tag] = de ? [+de[3], +de[2], +de[1]] : iso ? [+iso[1], +iso[2], +iso[3]] : [0, 0, 0];
      if (!datumGueltig(j, m, tag)) return { fehler: t("kein gültiges Datum") };
      return { wert: `${String(j).padStart(4, "0")}-${zwei(m)}-${zwei(tag)}` };
    }
    case "wahl": {
      if (s === "") return { wert: null };
      const i = Number(s);
      if (!Number.isInteger(i) || i < 0 || i >= art.werte.length) return { fehler: t("kein Wert der Auswahl") };
      return { wert: art.werte[i] };
    }
    case "liste": {
      const zeilen = s
        .split("\n")
        .map((z) => z.trim())
        .filter((z) => z !== "");
      if (zeilen.length === 0) return { wert: null };
      const werte: unknown[] = [];
      for (const [i, z] of zeilen.entries()) {
        const e = eingabeLesen(art.element, z);
        if ("fehler" in e) return { fehler: t("Zeile {nr}: {fehler}", { nr: i + 1, fehler: e.fehler }) };
        werte.push(e.wert);
      }
      return { wert: werte };
    }
    default:
      return { fehler: t("nicht direkt bearbeitbar") };
  }
}

/** JSON-Wert → Inhalt des Eingabefelds (Zahl mit Komma, Datum TT.MM.JJJJ, Liste je Zeile). */
export function eingabeZeigen(art: Art, wert: unknown): string | boolean {
  if (art.art === "schalter") return typeof wert === "boolean" ? wert : (art.standard ?? false);
  if (wert === undefined || wert === null) return "";
  switch (art.art) {
    case "zahl":
      return typeof wert === "number" ? String(wert).replace(".", ",") : String(wert);
    case "datum": {
      const iso = typeof wert === "string" ? DATUM_ISO.exec(wert) : null;
      return iso ? `${iso[3]}.${iso[2]}.${iso[1]}` : String(wert);
    }
    case "liste":
      return Array.isArray(wert) ? wert.map((w) => String(eingabeZeigen(art.element, w))).join("\n") : "";
    case "wahl": {
      const i = art.werte.findIndex((w) => JSON.stringify(w) === JSON.stringify(wert));
      return i < 0 ? "" : String(i);
    }
    case "text":
      return String(wert);
    default:
      return "";
  }
}

export function pfadText(p: Pfad): string {
  return p.join(".");
}

export function wertAn(werte: unknown, p: Pfad): unknown {
  let w: unknown = werte;
  for (const s of p) {
    if (w === null || typeof w !== "object") return undefined;
    w = (w as Record<string | number, unknown>)[s];
  }
  return w;
}

/** Gruppen für die Anzeige: Felder ohne `gruppe` unter „Titelei“, die übrigen nach `gruppe`. */
export function gruppen(f: Formular): { titel: string; felder: Feld[] }[] {
  // Die Titelei erkennt man am Schlüssel; angezeigt wird sie in der Sprache der Oberfläche.
  const liste: { schluessel: string; titel: string; felder: Feld[] }[] = [];
  for (const feld of f.felder) {
    const schluessel = feld.gruppe ?? "Titelei";
    let g = liste.find((x) => x.schluessel === schluessel);
    if (!g) {
      g = { schluessel, titel: schluessel === "Titelei" ? t("Titelei") : schluessel, felder: [] };
      if (schluessel === "Titelei") liste.unshift(g);
      else liste.push(g);
    }
    g.felder.push(feld);
  }
  return liste.map(({ titel, felder }) => ({ titel, felder }));
}

/**
 * Taste im Fach: Rückgängig bzw. Wiederholen der Historie des Fachs. `mod` ist ⌘ am Mac bzw. Strg
 * unter Windows; hat ein Textfeld ungeschriebene Eingaben, bleibt es beim Rückgängig des Felds.
 */
export function historieTaste(
  e: { key: string; mod: boolean; shift: boolean; ctrl: boolean },
  mac: boolean,
  feldGeaendert: boolean,
): "rueck" | "vor" | null {
  if (feldGeaendert) return null;
  const k = e.key.toLowerCase();
  if (e.mod && k === "z") return e.shift ? "vor" : "rueck";
  if (!mac && e.ctrl && !e.shift && k === "y") return "vor";
  return null;
}

/**
 * Schreibanfragen der Reihe nach (Abschlussprüfung AP7): Jede bekommt den Stand erst beim Absenden,
 * also nach der Antwort der vorigen. Sonst trüge eine zweite schnelle Änderung den alten Stand und
 * scheiterte als „veraltet“. Ein Fehler hält die Reihe nicht an.
 */
export class Reihe<S> {
  private letzte: Promise<unknown> = Promise.resolve();

  constructor(private stand: () => S) {}

  einreihen<T>(anfrage: (stand: S) => Promise<T>): Promise<T> {
    const p = this.letzte.then(() => anfrage(this.stand()));
    this.letzte = p.catch(() => undefined);
    return p;
  }
}

/** Was ein Eingabefeld über das Neuzeichnen hinweg behalten soll. */
export interface FeldZustand {
  value: string;
  selectionStart: number | null;
  selectionEnd: number | null;
  scrollTop: number;
  dataset: { gezeigt?: string };
}

/**
 * Überträgt ungeschriebene Eingaben, Cursor und Scrollstand des fokussierten Felds auf seinen
 * Nachfolger nach dem Neuzeichnen (Abschlussprüfung AP7). Ungeschriebenes bleibt nur, solange der
 * gezeigte Wert darunter derselbe ist; Cursor und Scrollstand nur, wenn der Text gleich bleibt.
 */
export function zustandUebertragen(alt: FeldZustand, neu: FeldZustand) {
  const getippt = alt.value !== alt.dataset.gezeigt;
  if (getippt && alt.dataset.gezeigt === neu.dataset.gezeigt) neu.value = alt.value;
  if (neu.value !== alt.value) return;
  neu.selectionStart = alt.selectionStart;
  neu.selectionEnd = alt.selectionEnd;
  neu.scrollTop = alt.scrollTop;
}
