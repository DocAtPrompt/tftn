// Werte der Widgets (PLAN-M2 AP6): Eingabefeld ↔ Wert in der Form, die Rust schickt und annimmt
// (`tftn_edit::Wert`, adjazent getaggt `{art, wert}`). Deutsche Eingaben: Dezimalkomma,
// Leerzeichen vor der Einheit, Prozent mit oder ohne `%`. Geschrieben wird nur, was hier gültig
// ist; die endgültige Prüfung macht Rust (Invarianten).

import { n, t, tn } from "./sprache";

/** Widget-Art wie in Rust (`{"art": "zahl", "mit": {...}}`). */
export type WidgetArt = { art: string; mit?: any };
/** Wert wie in Rust (`{"art": "laenge", "wert": {"betrag": 1, "einheit": "pt"}}`). */
export type Wert = { art: string; wert: any };

/** Ein Feld im Fach „Hier“ (Rust `FeldJs`). */
export interface FeldJs {
  parameter: string;
  titel: string;
  hilfe: string | null;
  standard: string | null;
  art: WidgetArt;
  wert: Wert | null;
  quelltext: string | null;
  gesperrt: string | null;
  fehlt: boolean;
}

export type Ergebnis = { wert: Wert } | { fehler: string };

const LEER = n("leer – Wert eingeben");
const EINHEITEN = ["pt", "mm", "cm", "in", "em"];
const ZAHL = /^-?\d+(?:[.,]\d+)?$/;
/** „1.000“: im Deutschen tausend, als Dezimalpunkt eins – nie still raten. */
const TAUSENDERPUNKT = /^-?\d{1,3}\.\d{3}$/;
const MEHRDEUTIG = n("mehrdeutig – Dezimalkomma (1,5) oder ohne Tausenderpunkt (1000) schreiben");

function zahlLesen(text: string): number | null {
  const s = text.trim();
  return ZAHL.test(s) ? Number(s.replace(",", ".")) : null;
}

function mehrdeutig(text: string): boolean {
  return TAUSENDERPUNKT.test(text.trim());
}

function zahlText(n: number): string {
  return String(n).replace(".", ",");
}

function grenzen(n: number, mit: { min?: number | null; max?: number | null }): string | null {
  if (mit.min != null && n < mit.min) return t("mindestens {min}", { min: zahlText(mit.min) });
  if (mit.max != null && n > mit.max) return t("höchstens {max}", { max: zahlText(mit.max) });
  return null;
}

function laengeLesen(text: string, einheiten: string[]): Ergebnis {
  const m = /^(-?\d+(?:[.,]\d+)?)\s*([a-zA-Z]*)$/.exec(text.trim());
  if (!m) return { fehler: t("keine Länge – etwa „12 pt“") };
  if (mehrdeutig(m[1])) return { fehler: tn(MEHRDEUTIG) };
  const erlaubt = einheiten.length ? einheiten : EINHEITEN;
  const einheit = m[2].toLowerCase();
  if (!erlaubt.includes(einheit)) {
    return {
      fehler: einheit
        ? t("Einheit „{einheit}“ nicht erlaubt (erlaubt: {erlaubt})", { einheit, erlaubt: erlaubt.join(", ") })
        : t("Einheit nicht erlaubt (erlaubt: {erlaubt})", { erlaubt: erlaubt.join(", ") }),
    };
  }
  return { wert: { art: "laenge", wert: { betrag: Number(m[1].replace(",", ".")), einheit } } };
}

function datumGueltig(j: number, m: number, tag: number): boolean {
  if (j < 1 || j > 9999 || m < 1 || m > 12 || tag < 1) return false;
  const tage = [31, j % 4 === 0 && (j % 100 !== 0 || j % 400 === 0) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  return tag <= tage[m - 1];
}

/** Text aus einem Eingabefeld → Wert der Art, oder eine Meldung. */
export function eingabeZuWert(art: WidgetArt, text: string): Ergebnis {
  const mit = art.mit ?? {};
  switch (art.art) {
    case "text":
      if (!mit.mehrzeilig && /[\r\n]/.test(text)) return { fehler: t("kein Zeilenumbruch erlaubt") };
      return { wert: { art: "text", wert: text } };
    case "inhalt":
      return { wert: { art: "inhalt", wert: text } };
    case "liste": {
      const zeilen = text.split("\n").filter((z, i, a) => z !== "" || i < a.length - 1);
      if (text === "") return { wert: { art: "liste", wert: [] } };
      const werte: Wert[] = [];
      for (const z of zeilen) {
        const e = eingabeZuWert(mit.element, z);
        if ("fehler" in e) return { fehler: t("„{eintrag}“: {fehler}", { eintrag: z, fehler: e.fehler }) };
        werte.push(e.wert);
      }
      return { wert: { art: "liste", wert: werte } };
    }
  }
  if (text.trim() === "") return { fehler: tn(LEER) };
  switch (art.art) {
    case "wahl": {
      const werte: string[] = mit.werte ?? [];
      if (werte.length && !werte.includes(text)) return { fehler: t("erlaubt: {werte}", { werte: werte.join(", ") }) };
      return { wert: { art: "text", wert: text } };
    }
    case "datei": {
      const endungen: string[] = mit.endungen ?? [];
      const endung = text.includes(".") ? text.slice(text.lastIndexOf(".") + 1).toLowerCase() : "";
      if (endungen.length && !endungen.map((e) => e.toLowerCase()).includes(endung)) {
        return { fehler: t("Endung nicht erlaubt (erlaubt: {endungen})", { endungen: endungen.join(", ") }) };
      }
      return { wert: { art: "text", wert: text } };
    }
    case "label":
      if (!/^[\p{L}\p{N}_.:-]+$/u.test(text)) return { fehler: t("ungültiges Label – nur Buchstaben, Ziffern, _ - . :") };
      return { wert: { art: "label", wert: text } };
    case "zahl": {
      if (mehrdeutig(text)) return { fehler: tn(MEHRDEUTIG) };
      const n = zahlLesen(text);
      if (n === null) return { fehler: t("keine Zahl") };
      if (mit.ganzzahl && !Number.isInteger(n)) return { fehler: t("nur eine ganze Zahl") };
      const g = grenzen(n, mit);
      return g ? { fehler: g } : { wert: { art: "zahl", wert: n } };
    }
    case "laenge":
      return laengeLesen(text, mit.einheiten ?? []);
    case "prozent": {
      if (mehrdeutig(text.trim().replace(/\s*%$/, ""))) return { fehler: tn(MEHRDEUTIG) };
      const n = zahlLesen(text.trim().replace(/\s*%$/, ""));
      if (n === null) return { fehler: t("keine Prozentzahl – etwa „70 %“") };
      const g = grenzen(n, mit);
      return g ? { fehler: g } : { wert: { art: "prozent", wert: n } };
    }
    case "schalter":
      if (text === "true" || text === "false") return { wert: { art: "schalter", wert: text === "true" } };
      return { fehler: t("ja oder nein") };
    case "farbe": {
      if (mit.raum === "cmyk") {
        const teile = text.split(";").map((x) => zahlLesen(x));
        if (teile.length !== 4 || teile.some((x) => x === null || !Number.isInteger(x))) {
          return { fehler: t("vier ganze Zahlen, getrennt durch ;") };
        }
        if (teile.some((x) => x! < 0 || x! > 100)) return { fehler: t("jeder Anteil 0 bis 100") };
        return { wert: { art: "farbe", wert: { cmyk: teile } } };
      }
      const m = /^#([0-9a-fA-F]{6})$/.exec(text.trim());
      if (!m) return { fehler: t("Farbe als #rrggbb") };
      const rgb = [0, 2, 4].map((i) => parseInt(m[1].slice(i, i + 2), 16));
      return { wert: { art: "farbe", wert: { rgb } } };
    }
    case "datum": {
      const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text.trim());
      if (!m) return { fehler: t("kein Datum (JJJJ-MM-TT)") };
      const [j, mo, tag] = [Number(m[1]), Number(m[2]), Number(m[3])];
      if (!datumGueltig(j, mo, tag)) return { fehler: t("kein gültiges Datum") };
      return { wert: { art: "datum", wert: { jahr: j, monat: mo, tag } } };
    }
    case "punkt": {
      const teile = text.split(";");
      if (teile.length !== 2) return { fehler: t("zwei Werte, getrennt durch ;") };
      const werte: Wert[] = [];
      for (const teil of teile) {
        if (mehrdeutig(teil.trim().replace(/\s*[a-zA-Z]+$/, ""))) return { fehler: tn(MEHRDEUTIG) };
        const zahl = zahlLesen(teil);
        if (zahl !== null) {
          werte.push({ art: "zahl", wert: zahl });
          continue;
        }
        const e = laengeLesen(teil, []);
        if ("fehler" in e) return e;
        werte.push(e.wert);
      }
      return { wert: { art: "punkt", wert: werte } };
    }
  }
  return { fehler: t("Widget-Art {art} hier nicht änderbar", { art: art.art }) };
}

function hex2(n: number): string {
  return n.toString(16).padStart(2, "0");
}

/** Wert → Text für das Eingabefeld. Ohne Wert ein leerer Text. */
export function wertZuEingabe(art: WidgetArt, w: Wert | null): string {
  if (!w) return "";
  switch (w.art) {
    case "text":
    case "inhalt":
    case "label":
      return w.wert;
    case "zahl":
    case "prozent":
      return zahlText(w.wert);
    case "laenge":
      return `${zahlText(w.wert.betrag)} ${w.wert.einheit}`;
    case "schalter":
      return String(w.wert);
    case "farbe": {
      const f = w.wert;
      if (f.cmyk) return f.cmyk.join("; ");
      const rgb: number[] = f.rgb ?? [f.luma, f.luma, f.luma];
      return `#${rgb.map(hex2).join("")}`;
    }
    case "datum": {
      const d = w.wert;
      return `${String(d.jahr).padStart(4, "0")}-${String(d.monat).padStart(2, "0")}-${String(d.tag).padStart(2, "0")}`;
    }
    case "punkt":
      return w.wert.map((x: Wert) => wertZuEingabe(art, x)).join("; ");
    case "liste":
      return w.wert.map((x: Wert) => wertZuEingabe(art.mit?.element ?? art, x)).join("\n");
  }
  return "";
}
