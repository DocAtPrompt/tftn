// Literaturverwaltung: Formularfelder je Art, Personen zeilenweise, Schlüsselvorschlag, Suche.
// Gelesen und geschrieben wird in Rust (tftn_literatur::verwaltung); hier nur Rechnung.

import { t } from "./sprache";

export interface Feld {
  name: string;
  wert: string;
  /** BibTeX-Quelltext (Zahl ohne Klammern, Abkürzung), wird so geschrieben */
  roh: boolean;
}

export interface BibEintrag {
  art: string;
  schluessel: string;
  felder: Feld[];
}

export interface LitEintragJs extends BibEintrag {
  zitiert: boolean;
}

export interface LitDateiJs {
  pfad: string;
  schreibbar: boolean;
  fehler: string | null;
  eintraege: LitEintragJs[];
}

/** Die Arten im Auswahlfeld, in dieser Reihenfolge. */
export function arten(): [string, string][] {
  return [
    ["article", t("Zeitschriftenartikel")],
    ["book", t("Buch")],
    ["incollection", t("Beitrag in einem Sammelband")],
    ["inproceedings", t("Konferenzbeitrag")],
    ["phdthesis", t("Dissertation")],
    ["mastersthesis", t("Abschlussarbeit")],
    ["report", t("Bericht")],
    ["online", t("Webseite")],
    ["misc", t("Sonstiges")],
  ];
}

const GRUND = ["author", "title", "year", "url", "note"];
const JE_ART: Record<string, string[]> = {
  article: ["author", "title", "journal", "year", "volume", "number", "pages", "doi", "url"],
  book: ["author", "editor", "title", "year", "edition", "publisher", "address", "isbn", "doi", "url"],
  incollection: ["author", "title", "booktitle", "editor", "year", "publisher", "address", "pages", "doi", "url"],
  inproceedings: ["author", "title", "booktitle", "editor", "year", "publisher", "address", "pages", "doi", "url"],
  phdthesis: ["author", "title", "school", "year", "address", "url"],
  mastersthesis: ["author", "title", "school", "year", "address", "url"],
  thesis: ["author", "title", "type", "institution", "year", "address", "url"],
  report: ["author", "title", "institution", "number", "year", "address", "url"],
  techreport: ["author", "title", "institution", "number", "year", "address", "url"],
  online: ["author", "title", "url", "urldate", "year", "organization"],
  misc: ["author", "title", "howpublished", "year", "url", "note"],
};

/** Gleichbedeutende Namen (BibTeX ↔ biblatex): Was da ist, nimmt den Platz ein. */
const GLEICH: Record<string, string[]> = {
  journal: ["journaltitle"],
  address: ["location"],
  year: ["date"],
};

/** Felder mit Personen: im Formular eine je Zeile. */
export const PERSONEN = new Set(["author", "editor", "translator"]);

/** Beschriftung eines Feldes; Unbekanntes unter seinem Namen. */
export function feldTitel(name: string): string {
  const titel: Record<string, string> = {
    author: t("Autorinnen und Autoren"),
    editor: t("Herausgabe"),
    translator: t("Übersetzung"),
    title: t("Titel"),
    year: t("Jahr"),
    date: t("Datum"),
    journal: t("Zeitschrift"),
    journaltitle: t("Zeitschrift"),
    volume: t("Band"),
    number: t("Nummer"),
    pages: t("Seiten"),
    publisher: t("Verlag"),
    address: t("Ort"),
    location: t("Ort"),
    edition: t("Auflage"),
    isbn: "ISBN",
    issn: "ISSN",
    doi: "DOI",
    url: "URL",
    urldate: t("Abgerufen am"),
    booktitle: t("Titel des Sammelbands"),
    school: t("Hochschule"),
    institution: t("Institution"),
    organization: t("Organisation"),
    howpublished: t("Erschienen"),
    note: t("Anmerkung"),
    series: t("Reihe"),
    chapter: t("Kapitel"),
    month: t("Monat"),
    type: t("Art der Arbeit"),
    abstract: t("Zusammenfassung"),
    keywords: t("Schlagwörter"),
    language: t("Sprache"),
  };
  return titel[name] ?? name;
}

/** Formularfelder: die üblichen der Art (leer, wo nichts steht), dann alle übrigen vorhandenen. */
export function formularFelder(art: string, vorhanden: readonly Feld[]): Feld[] {
  const da = new Map(vorhanden.map((f) => [f.name, f]));
  const genommen = new Set<string>();
  const aus: Feld[] = [];
  for (const name of JE_ART[art] ?? GRUND) {
    const name2 = [name, ...(GLEICH[name] ?? [])].find((n) => da.has(n)) ?? name;
    genommen.add(name2);
    aus.push(da.get(name2) ?? { name: name2, wert: "", roh: false });
  }
  for (const f of vorhanden) if (!genommen.has(f.name)) aus.push(f);
  return aus;
}

/** Teilt an „and“ außerhalb geschweifter Klammern. */
function personenTeilen(wert: string): string[] {
  const teile: string[] = [];
  let tiefe = 0;
  let start = 0;
  for (let i = 0; i < wert.length; i++) {
    const c = wert[i];
    if (c === "{") tiefe++;
    else if (c === "}") tiefe--;
    else if (tiefe === 0 && /\s/.test(c)) {
      const m = /^\s+and\s+/i.exec(wert.slice(i));
      if (m) {
        teile.push(wert.slice(start, i));
        i += m[0].length - 1;
        start = i + 1;
      }
    }
  }
  teile.push(wert.slice(start));
  return teile.map((s) => s.trim()).filter((s) => s !== "");
}

/** `A and B` → eine Person je Zeile. */
export function personenZeilen(wert: string): string {
  return personenTeilen(wert).join("\n");
}

/** Eine Person je Zeile → `A and B`. */
export function personenWert(zeilen: string): string {
  return zeilen
    .split("\n")
    .map((z) => z.trim())
    .filter((z) => z !== "")
    .join(" and ");
}

/** Aus dem Formular (Personen zeilenweise) ein Eintrag; leere Felder fallen weg. */
export function alsEintrag(art: string, schluessel: string, felder: readonly Feld[]): BibEintrag {
  return {
    art,
    schluessel: schluessel.trim(),
    felder: felder
      .map((f) => ({ ...f, wert: !f.roh && PERSONEN.has(f.name) ? personenWert(f.wert) : f.wert.trim() }))
      .filter((f) => f.wert !== ""),
  };
}

const KEIN_NAME = new Set(["the", "der", "die", "das", "von", "van", "de", "del", "della", "la", "le", "du", "di", "da", "den", "ter"]);

function ascii(wort: string): string {
  return wort
    .toLowerCase()
    .replace(/ä/g, "ae")
    .replace(/ö/g, "oe")
    .replace(/ü/g, "ue")
    .replace(/ß/g, "ss")
    .normalize("NFD")
    .replace(/[^a-z]/g, "");
}

/** `<nachname><jahr>` wie bei „Literatur aus DOI“, mit a, b, … frei gemacht. */
export function schluesselVorschlag(personen: string, jahr: string, vergeben: ReadonlySet<string>): string {
  const erste = personenTeilen(personen)[0] ?? "";
  const geklammert = /^\{.*\}$/.test(erste);
  const ohne = erste.replace(/[{}]/g, "");
  const nachname = ohne.includes(",") ? ohne.split(",")[0] : geklammert ? ohne : (ohne.split(/\s+/).pop() ?? "");
  const name = nachname
    .split(/[\s-]+/)
    .map(ascii)
    .find((w) => w.length >= 2 && !KEIN_NAME.has(w));
  const stamm = `${name ?? "quelle"}${/\d{4}/.exec(jahr)?.[0] ?? ""}`;
  if (!vergeben.has(stamm)) return stamm;
  for (const z of "abcdefghijklmnopqrstuvwxyz") if (!vergeben.has(stamm + z)) return stamm + z;
  return stamm;
}

/** Kleinbuchstaben, Umlaute einfach und doppelt (ü → u und ue). */
function falten(text: string): string {
  const klein = text.toLowerCase();
  const einfach = klein.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/ß/g, "ss");
  const doppelt = klein.replace(/ä/g, "ae").replace(/ö/g, "oe").replace(/ü/g, "ue").replace(/ß/g, "ss");
  return `${klein} ${einfach} ${doppelt}`;
}

/** Einträge, in denen jedes Wort der Suche vorkommt (Schlüssel oder ein Feld). */
export function eintraegeFiltern<T extends BibEintrag>(liste: readonly T[], suche: string): T[] {
  const woerter = suche
    .split(/\s+/)
    .filter((w) => w !== "")
    .map((w) => falten(w).split(" ")[1]);
  if (woerter.length === 0) return [...liste];
  return liste.filter((e) => {
    const heu = falten([e.schluessel, ...e.felder.map((f) => f.wert)].join(" "));
    return woerter.every((w) => heu.includes(w));
  });
}

/** Wert eines Feldes (oder seines Gleichnamigen), ohne Klammern; leer, wenn es fehlt. */
export function feldWert(e: BibEintrag, name: string): string {
  const f = e.felder.find((x) => x.name === name || (GLEICH[name] ?? []).includes(x.name));
  return (f?.wert ?? "").replace(/[{}]/g, "");
}
