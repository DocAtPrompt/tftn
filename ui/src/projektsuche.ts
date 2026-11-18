// Im Projekt suchen und ersetzen: über alle `.typ`-Dateien des Projekts (Hauptdatei und Kapitel).
// Gesucht wird hier mit CodeMirrors `SearchQuery`, also genau wie in der Suchleiste des Editors
// (Groß/klein, ganzes Wort, regulärer Ausdruck, `\n` und `$1` im Ersatz). Die Texte kommen aus Rust
// (`reiter::projekt_texte`), für offene Reiter aus dem Editor. Ersetzt wird im Editor (main.ts), je
// Datei eine Transaktion, damit Cmd/Strg+Z je Reiter rückgängig macht.

import { EditorState } from "@codemirror/state";
import { SearchQuery } from "@codemirror/search";
import { dialogAnmelden, dialogKopf } from "./dialoge";
import { t, zahl } from "./sprache";

export interface SuchAnfrage {
  suche: string;
  ersetzen: string;
  grossKlein: boolean;
  ganzesWort: boolean;
  regex: boolean;
}

/** Eine Datei mit Text (`reiter::DateiTextJs`). */
export interface DateiText {
  datei: string;
  text: string;
}

export interface Fund {
  von: number;
  bis: number;
  /** Bei regulären Ausdrücken die Gruppen (für `$1` usw.). */
  gruppen: string[] | null;
}

export interface Treffer {
  datei: string;
  von: number;
  bis: number;
  /** Der gefundene Text (zum Prüfen, ob die Stelle noch stimmt). */
  text: string;
  /** Zeilennummer ab 1. */
  zeile: number;
  vorher: string;
  /** Der hervorgehobene Teil; bei Treffern über mehrere Zeilen nur die erste. */
  mitte: string;
  nachher: string;
}

export interface DateiTreffer {
  datei: string;
  treffer: Treffer[];
  /** Alle Treffer der Datei, auch die über der Grenze. */
  anzahl: number;
}

export interface SuchErgebnis {
  /** Nur Dateien mit Treffern, in der Reihenfolge der Eingabe. */
  dateien: DateiTreffer[];
  /** Gezählte Treffer insgesamt (höchstens `ZAEHLGRENZE`). */
  gesamt: number;
  /** Es gibt mehr Treffer, als die Liste zeigt. */
  begrenzt: boolean;
  /** Das Zählen hat bei `ZAEHLGRENZE` aufgehört. */
  mehr: boolean;
}

/** So viele Treffer zeigt die Liste höchstens. */
export const GRENZE = 1000;
/** So weit wird gezählt (und bei „Alle ersetzen“ genannt). */
export const ZAEHLGRENZE = 100_000;
/** Zeichen vor und nach dem Treffer im Ausschnitt. */
const RAND = 40;

/** Was die Engine über einen kaputten Ausdruck sagt, auf Deutsch (V8 unter Windows, WebKit am Mac). */
const GRUENDE: [RegExp, () => string][] = [
  [/unterminated group|missing \)|unmatched '?\)'?|unmatched parentheses/i, () => t("runde Klammern passen nicht zusammen")],
  [/unterminated character class|missing terminating \]/i, () => t("eine eckige Klammer [ ist nicht geschlossen")],
  [/nothing to repeat/i, () => t("*, + oder ? steht ohne etwas davor")],
  [/\\ at end of pattern/i, () => t("\\ steht am Ende")],
  [/lone quantifier|incomplete quantifier/i, () => t("geschweifte Klammern { } ohne Bedeutung (als Zeichen \\{ schreiben)")],
  [/numbers out of order/i, () => t("in {n,m} ist die erste Zahl größer als die zweite")],
  [/invalid (unicode |class |)escape|invalid escape/i, () => t("ungültige Folge mit \\")],
  [/group name|invalid group/i, () => t("ungültige Gruppe ( … )")],
];

/** `null`, wenn die Anfrage gültig ist; sonst eine verständliche Meldung (nur bei regulären Ausdrücken). */
export function anfrageFehler(a: SuchAnfrage): string | null {
  if (!a.regex || !a.suche) return null;
  try {
    // Dieselben Flags wie CodeMirror (`gm`, `u`).
    new RegExp(a.suche, "gmu");
    return null;
  } catch (e) {
    const roh = e instanceof Error ? e.message : String(e);
    const treffer = GRUENDE.find(([muster]) => muster.test(roh));
    const grund = treffer ? treffer[1]() : roh.split(": ").pop() || roh;
    return t("Kein gültiger regulärer Ausdruck: {grund}.", { grund });
  }
}

function abfrage(a: SuchAnfrage): SearchQuery {
  return new SearchQuery({
    search: a.suche,
    replace: a.ersetzen,
    caseSensitive: a.grossKlein,
    wholeWord: a.ganzesWort,
    regexp: a.regex,
  });
}

/** Zeilenenden wie im Editor (CodeMirror zählt `\r\n` als ein Zeichen). */
export function lf(text: string): string {
  return text.replace(/\r\n?/g, "\n");
}

/** Alle Fundstellen in `text` (Zeilenenden `\n`), höchstens `grenze`. Leer bei ungültiger Anfrage. */
export function fundeIn(text: string, a: SuchAnfrage, grenze = Infinity): Fund[] {
  const q = abfrage(a);
  if (!q.valid) return [];
  const aus: Fund[] = [];
  const cursor = q.getCursor(EditorState.create({ doc: text }));
  for (let r = cursor.next(); !r.done && aus.length < grenze; r = cursor.next()) {
    const v = r.value as { from: number; to: number; match?: RegExpExecArray };
    aus.push({ von: v.from, bis: v.to, gruppen: v.match ? [...v.match].map((g) => g ?? "") : null });
  }
  return aus;
}

/** `\n`, `\r`, `\t`, `\\` wie in CodeMirror (ohne `literal`). */
function entschluesseln(text: string): string {
  return text.replace(/\\([nrt\\])/g, (_, z) => (z === "n" ? "\n" : z === "r" ? "\r" : z === "t" ? "\t" : "\\"));
}

/** Der Ersatztext für einen Fund, genau wie CodeMirror ihn bildet (`$&`, `$$`, `$1` …). */
export function ersatzFuer(a: SuchAnfrage, f: Fund): string {
  const ersatz = entschluesseln(a.ersetzen);
  if (!a.regex || !f.gruppen) return ersatz;
  const g = f.gruppen;
  return ersatz.replace(/\$([$&]|\d+)/g, (m, i: string) => {
    if (i === "&") return g[0];
    if (i === "$") return "$";
    for (let l = i.length; l > 0; l--) {
      const n = +i.slice(0, l);
      if (n > 0 && n < g.length) return g[n] + i.slice(l);
    }
    return m;
  });
}

/** Alle Änderungen für „Alle ersetzen“ in einem Text (für eine Transaktion). */
export function ersetzungen(text: string, a: SuchAnfrage): { from: number; to: number; insert: string }[] {
  return fundeIn(text, a).map((f) => ({ from: f.von, to: f.bis, insert: ersatzFuer(a, f) }));
}

/** Der Fund an genau dieser Stelle im jetzigen Text, sonst `null` (die Stelle hat sich verschoben). */
export function fundBei(text: string, a: SuchAnfrage, von: number, bis: number): Fund | null {
  for (const f of fundeIn(text, a)) {
    if (f.von === von && f.bis === bis) return f;
    if (f.von > von) break;
  }
  return null;
}

/** Zeile und Ausschnitt um einen Treffer, gekürzt mit „…“. */
export function ausschnitt(text: string, von: number, bis: number): Pick<Treffer, "zeile" | "vorher" | "mitte" | "nachher"> {
  const anfang = text.lastIndexOf("\n", von - 1) + 1;
  const ende0 = text.indexOf("\n", von);
  const ende = ende0 < 0 ? text.length : ende0;
  let zeile = 1;
  for (let i = text.indexOf("\n"); i >= 0 && i < von; i = text.indexOf("\n", i + 1)) zeile++;
  const mitteBis = Math.min(bis, ende);
  let vorher = text.slice(anfang, von).replace(/^\s+/, "");
  if (vorher.length > RAND) vorher = "…" + vorher.slice(vorher.length - RAND);
  let nachher = bis > ende ? " …" : text.slice(mitteBis, ende);
  if (nachher.length > RAND) nachher = nachher.slice(0, RAND) + "…";
  return { zeile, vorher, mitte: text.slice(von, mitteBis), nachher };
}

/** Durchsucht alle Dateien; die Liste hat höchstens `grenze` Treffer, gezählt wird weiter. */
export function projektSuchen(dateien: DateiText[], a: SuchAnfrage, grenze = GRENZE): SuchErgebnis {
  const aus: SuchErgebnis = { dateien: [], gesamt: 0, begrenzt: false, mehr: false };
  if (!a.suche || anfrageFehler(a)) return aus;
  for (const d of dateien) {
    if (aus.gesamt >= ZAEHLGRENZE) {
      aus.mehr = true;
      break;
    }
    const text = lf(d.text);
    const funde = fundeIn(text, a, ZAEHLGRENZE - aus.gesamt + 1);
    if (!funde.length) continue;
    if (aus.gesamt + funde.length > ZAEHLGRENZE) {
      funde.length = ZAEHLGRENZE - aus.gesamt;
      aus.mehr = true;
    }
    const platz = Math.max(0, grenze - aus.dateien.reduce((s, x) => s + x.treffer.length, 0));
    const treffer = funde.slice(0, platz).map((f) => ({
      datei: d.datei,
      von: f.von,
      bis: f.bis,
      text: text.slice(f.von, f.bis),
      ...ausschnitt(text, f.von, f.bis),
    }));
    if (treffer.length < funde.length) aus.begrenzt = true;
    aus.dateien.push({ datei: d.datei, treffer, anzahl: funde.length });
    aus.gesamt += funde.length;
  }
  return aus;
}

/** „3 Treffer in 2 Dateien“, mit Hinweis auf die Grenze. */
export function zusammenfassung(e: SuchErgebnis): string {
  if (!e.gesamt) return t("Keine Treffer.");
  const treffer = e.mehr ? t("mehr als {zahl}", { zahl: zahl(e.gesamt) }) : zahl(e.gesamt);
  const dateien = e.dateien.length;
  let s =
    dateien === 1
      ? t("{treffer} Treffer in 1 Datei", { treffer })
      : t("{treffer} Treffer in {dateien} Dateien", { treffer, dateien: zahl(dateien) });
  if (e.begrenzt) s += " · " + t("die Liste zeigt die ersten {grenze}", { grenze: zahl(GRENZE) });
  return s;
}

/** Text des Knopfs „Alle ersetzen“ nach dem ersten Klick (der zweite bestätigt). */
export function alleErsetzenFrage(e: SuchErgebnis): string {
  const treffer = e.mehr ? t("mehr als {zahl}", { zahl: zahl(e.gesamt) }) : zahl(e.gesamt);
  return e.dateien.length === 1
    ? t("{treffer} Treffer in 1 Datei ersetzen?", { treffer })
    : t("{treffer} Treffer in {dateien} Dateien ersetzen?", { treffer, dateien: zahl(e.dateien.length) });
}

// --- Fenster ----------------------------------------------------------------------------------

export interface ProjektSucheAktionen {
  /** Alle `.typ`-Dateien mit Text; offene Reiter mit dem Stand im Editor. */
  texte(): Promise<DateiText[]>;
  /** Öffnet die Datei als Reiter und wählt den Treffer aus. */
  zeigen(t: Treffer): Promise<void>;
  /** Ersetzt einen Treffer im Editor; `false`, wenn die Stelle nicht mehr passt. */
  ersetzen(t: Treffer, a: SuchAnfrage): Promise<boolean>;
  /** Ersetzt alle Treffer in diesen Dateien im Editor; liefert die Zahl der Ersetzungen. */
  alleErsetzen(dateien: string[], a: SuchAnfrage): Promise<number>;
  /** Die Auswahl im Editor, wenn sie in einer Zeile liegt (Vorbelegung des Suchfelds). */
  auswahl(): string;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

function schalter(text: string, titel: string): [HTMLLabelElement, HTMLInputElement] {
  const l = el("label", "psuche-schalter");
  const k = el("input");
  k.type = "checkbox";
  l.title = titel;
  l.append(k, " ", text);
  return [l, k];
}

export class ProjektSuche {
  private huelle = el("div", "neu-dialog psuche-dialog");
  private suche = el("input", "psuche-feld");
  private ersatz = el("input", "psuche-feld");
  private grossKlein: HTMLInputElement;
  private ganzesWort: HTMLInputElement;
  private regex: HTMLInputElement;
  private alle = el("button", "", t("Alle ersetzen …"));
  private stand = el("p", "psuche-stand");
  private liste = el("div", "psuche-liste");
  private ergebnis: SuchErgebnis | null = null;
  private zeitgeber: number | undefined;
  /** Zählt Suchläufe; ein älterer, später fertiger Lauf zeigt nichts mehr. */
  private lauf = 0;
  private beschaeftigt = false;

  constructor(private a: ProjektSucheAktionen) {
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    this.huelle.setAttribute("aria-label", t("Im Projekt suchen"));
    const kopf = dialogKopf(t("Im Projekt suchen"), () => this.schliessen());
    this.suche.type = "search";
    this.suche.placeholder = t("Suchen");
    this.suche.setAttribute("aria-label", t("Suchen"));
    this.ersatz.placeholder = t("Ersetzen durch");
    this.ersatz.setAttribute("aria-label", t("Ersetzen durch"));
    const [gk, gkK] = schalter(t("Groß/klein"), t("Groß- und Kleinschreibung beachten"));
    const [gw, gwK] = schalter(t("ganzes Wort"), t("Nur ganze Wörter finden"));
    const [re, reK] = schalter(
      t("regulärer Ausdruck"),
      t("Regulärer Ausdruck wie in JavaScript; im Ersatz stehen $1, $2 … für die Gruppen, $& für den ganzen Treffer"),
    );
    this.grossKlein = gkK;
    this.ganzesWort = gwK;
    this.regex = reK;
    const schalterZeile = el("div", "psuche-zeile");
    schalterZeile.append(gk, gw, re);
    const ersatzZeile = el("div", "psuche-zeile");
    this.alle.title = t("Ersetzt im Editor; Cmd/Strg+Z macht es je Datei rückgängig. Gespeichert wird wie gewohnt.");
    ersatzZeile.append(this.ersatz, this.alle);
    this.stand.setAttribute("role", "status");
    this.huelle.append(kopf, this.suche, schalterZeile, ersatzZeile, this.stand, this.liste);
    document.body.append(this.huelle);
    dialogAnmelden(this);

    this.suche.addEventListener("input", () => this.spaeterSuchen());
    this.suche.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        void this.suchen();
      }
    });
    this.ersatz.addEventListener("input", () => this.alleZuruecksetzen());
    for (const k of [gkK, gwK, reK]) k.addEventListener("change", () => void this.suchen());
    this.alle.addEventListener("click", () => void this.alleKlick());
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  schliessen() {
    this.huelle.hidden = true;
  }

  /** Öffnen (vorbelegt mit der Auswahl im Editor) bzw. ins Suchfeld springen. */
  oeffnen() {
    const auswahl = this.a.auswahl();
    if (auswahl) this.suche.value = auswahl;
    this.huelle.hidden = false;
    this.suche.focus();
    this.suche.select();
    void this.suchen();
  }

  private anfrage(): SuchAnfrage {
    return {
      suche: this.suche.value,
      ersetzen: this.ersatz.value,
      grossKlein: this.grossKlein.checked,
      ganzesWort: this.ganzesWort.checked,
      regex: this.regex.checked,
    };
  }

  private spaeterSuchen() {
    clearTimeout(this.zeitgeber);
    this.zeitgeber = window.setTimeout(() => void this.suchen(), 250);
  }

  private alleZuruecksetzen() {
    delete this.alle.dataset.sicher;
    this.alle.textContent = t("Alle ersetzen …");
  }

  /** Sucht neu über alle Dateien und zeigt die Liste. */
  async suchen(): Promise<SuchErgebnis | null> {
    clearTimeout(this.zeitgeber);
    this.alleZuruecksetzen();
    const a = this.anfrage();
    const meiner = ++this.lauf;
    const fehler = anfrageFehler(a);
    this.suche.classList.toggle("psuche-fehler", fehler !== null);
    if (!a.suche || fehler) {
      this.ergebnis = null;
      this.stand.textContent = fehler ?? "";
      this.liste.replaceChildren();
      this.alle.disabled = true;
      return null;
    }
    let texte: DateiText[];
    try {
      texte = await this.a.texte();
    } catch (f) {
      if (meiner === this.lauf) this.stand.textContent = String(f);
      return null;
    }
    if (meiner !== this.lauf) return null;
    const e = projektSuchen(texte, a);
    this.ergebnis = e;
    this.stand.textContent = zusammenfassung(e);
    this.alle.disabled = e.gesamt === 0 || this.beschaeftigt;
    this.listeZeigen(e, a);
    return e;
  }

  private listeZeigen(e: SuchErgebnis, a: SuchAnfrage) {
    const teile: HTMLElement[] = [];
    for (const d of e.dateien) {
      const gruppe = el("div", "psuche-datei");
      const kopf = el("div", "psuche-dateikopf");
      kopf.append(el("span", "psuche-dateiname", d.datei), el("span", "psuche-zahl", zahl(d.anzahl)));
      gruppe.append(kopf);
      for (const tr of d.treffer) {
        const zeile = el("div", "psuche-treffer");
        const knopf = el("button", "psuche-stelle");
        knopf.title = t("{datei}, Zeile {zeile}", { datei: d.datei, zeile: tr.zeile });
        const text = el("span", "psuche-text");
        text.append(tr.vorher, el("mark", "", tr.mitte), tr.nachher);
        knopf.append(el("span", "psuche-nr", String(tr.zeile)), text);
        knopf.addEventListener("click", () => void this.a.zeigen(tr));
        const ersetzen = el("button", "psuche-ersetzen", t("Ersetzen"));
        ersetzen.title = t("Diesen Treffer ersetzen");
        ersetzen.addEventListener("click", () => void this.einenErsetzen(tr, a));
        zeile.append(knopf, ersetzen);
        gruppe.append(zeile);
      }
      teile.push(gruppe);
    }
    this.liste.replaceChildren(...teile);
  }

  private async einenErsetzen(tr: Treffer, a: SuchAnfrage) {
    if (this.beschaeftigt) return;
    this.beschaeftigt = true;
    try {
      const ok = await this.a.ersetzen(tr, { ...a, ersetzen: this.ersatz.value });
      await this.suchen();
      if (!ok) this.stand.textContent = t("Die Stelle hat sich geändert; die Liste ist neu.") + " " + this.stand.textContent;
    } catch (f) {
      this.stand.textContent = String(f);
    } finally {
      this.beschaeftigt = false;
    }
  }

  /** Erster Klick: neu suchen und nennen, wie viel ersetzt würde; zweiter Klick: ersetzen. */
  private async alleKlick() {
    if (this.beschaeftigt) return;
    if (!this.alle.dataset.sicher) {
      const e = await this.suchen();
      if (!e || !e.gesamt) return;
      this.alle.dataset.sicher = "1";
      this.alle.textContent = alleErsetzenFrage(e);
      return;
    }
    const e = this.ergebnis;
    if (!e) return;
    this.beschaeftigt = true;
    this.alle.disabled = true;
    try {
      const n = await this.a.alleErsetzen(
        e.dateien.map((d) => d.datei),
        this.anfrage(),
      );
      await this.suchen();
      this.stand.textContent =
        e.dateien.length === 1
          ? t("{zahl} Treffer in 1 Datei ersetzt; Cmd/Strg+Z macht es im Reiter rückgängig.", { zahl: zahl(n) })
          : t("{zahl} Treffer in {dateien} Dateien ersetzt; Cmd/Strg+Z macht es je Reiter rückgängig.", {
              zahl: zahl(n),
              dateien: zahl(e.dateien.length),
            });
    } catch (f) {
      this.stand.textContent = String(f);
    } finally {
      this.beschaeftigt = false;
      this.alleZuruecksetzen();
      this.alle.disabled = !this.ergebnis?.gesamt;
    }
  }
}
