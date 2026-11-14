// Seitenleiste C (PLAN-M2 AP6, Entwurf 4 C): Fächer „Hier“ (worin der Cursor steht, alle Werte als
// Formular, Markup-Knöpfe, Tabellenraster), „Einfügen“ (Bausteinliste) und „Dokument“ (Titelei,
// Literatur, Projekt, Export; PLAN-M2 AP7, `dokumentfach.ts`) und „Pakete“ (Universe-Pakete; PLAN-M2
// AP9, `paketfach.ts`). Die Leiste öffnet sich nur auf Knopf oder Kürzel und folgt dem Cursor nur,
// solange sie offen ist.

import type { EintragJs } from "./bausteine";
import type { DokumentFach } from "./dokumentfach";
import type { FormelFach } from "./formelfach";
import type { SeiteFach } from "./seitefach";
import type { PaketFach } from "./paketfach";
import { editor } from "./editoren";
import { breiteZiehbar } from "./breite";
import { type Auswahl, type Knopf, type Tabelle, aktionFuer, auswahlNach, auswahlRechteck, bearbeitbar } from "./raster";
import type { FeldJs, Wert } from "./werte";
import { t } from "./sprache";

export interface EbeneJs {
  baustein: string;
  funktion: string;
  von: number;
  bis: number;
  felder: FeldJs[];
}

export interface MarkupJs {
  art: "ueberschrift" | "liste" | "formel" | "zitat";
  ebene: number | null;
  aufzaehlung: boolean | null;
  block: boolean | null;
  schluessel?: string | null;
  zusatz?: string | null;
}

export interface TabelleJs {
  von: number;
  bis: number;
  tabelle: Tabelle | null;
  gesperrt: string | null;
}

export interface HierJs {
  rev: number;
  ebenen: EbeneJs[];
  markup: MarkupJs[];
  tabelle: TabelleJs | null;
}

export type MarkupAktion = "ebene-plus" | "ebene-minus" | "liste-umschalten" | "formel-umschalten";

export interface SeitenleisteAktionen {
  hier(pos: number): Promise<HierJs | null>;
  /** `rev` ist die Revision, zu der `aufruf` gehört (aus `HierJs`); Rust lehnt eine veraltete ab. */
  setzen(rev: number, aufruf: number, parameter: string, w: Wert, ende: boolean): void;
  /** `false`: abgelehnt, die Meldung ist schon gezeigt. */
  tabelle(rev: number, aufruf: number, aktion: object): Promise<boolean>;
  markup(aktion: MarkupAktion): void;
  /** Zitat am Cursor (PLAN-M2 AP11): Seitenangabe `null` = keine, Quelle `null` = bleibt. */
  zitat(rev: number, seite: string | null, schluessel: string | null): void;
  /** Zitat-Palette, um die Quelle zu wechseln; die Seitenangabe bleibt. */
  zitatQuelle(rev: number, seite: string | null): void;
  /** Bausteine, die am Cursor passen, gefiltert nach `anfrage`. */
  bausteine(anfrage: string): Promise<EintragJs[]>;
  einfuegen(id: string): void;
  geschlossen(): void;
  /** Zurück in den Text (Esc in einem Feld der Leiste). */
  zurueck?(): void;
}

export type Fach = "hier" | "einfuegen" | "formel" | "seite" | "dokument" | "pakete";

const KNOEPFE: [Knopf, string, string][] = [
  ["zeile-oben", t("＋ Zeile oben"), t("Zeile über der Auswahl einfügen")],
  ["zeile-unten", t("＋ Zeile unten"), t("Zeile unter der Auswahl einfügen")],
  ["spalte-links", t("＋ Spalte links"), t("Spalte links der Auswahl einfügen")],
  ["spalte-rechts", t("＋ Spalte rechts"), t("Spalte rechts der Auswahl einfügen")],
  ["zeile-loeschen", t("− Zeile"), t("Zeile löschen")],
  ["spalte-loeschen", t("− Spalte"), t("Spalte löschen")],
  ["zeile-hoch", "↑", t("Zeile nach oben")],
  ["zeile-runter", "↓", t("Zeile nach unten")],
  ["spalte-vor", "←", t("Spalte nach links")],
  ["spalte-zurueck", "→", t("Spalte nach rechts")],
  ["verbinden", t("Verbinden"), t("Ausgewählte Zellen verbinden (Umschalt+Klick wählt einen Bereich)")],
  ["trennen", t("Trennen"), t("Verbundene Zelle trennen")],
];

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class Seitenleiste {
  private huelle: HTMLElement;
  private inhalt: HTMLElement;
  private reiter = new Map<Fach, HTMLButtonElement>();
  private fach: Fach = "hier";
  private pos = 0;
  private zeitgeber: number | undefined;
  private anfrage = 0;
  /** Auswahl im Raster; gilt für die Tabelle, die bei `rasterVon` beginnt. */
  private auswahl: Auswahl | null = null;
  private anker: [number, number] | null = null;
  private rasterVon = -1;
  private suche = "";
  /** Revision des gezeigten Stands; die Positionen der Leiste gehören zu ihr. */
  private rev = -1;
  /** Ein Neuladen wartete, weil ein Editor der Leiste den Fokus hatte. */
  private ausstehend = false;

  constructor(
    ort: HTMLElement,
    private a: SeitenleisteAktionen,
    private dokument: DokumentFach,
    private pakete: PaketFach,
    private formel: FormelFach,
    private seite: SeiteFach,
  ) {
    this.huelle = el("aside");
    this.huelle.id = "seitenleiste";
    this.huelle.hidden = true;
    const kopf = el("div", "leiste-kopf");
    const fach = (f: Fach, titel: string) => {
      const b = el("button", "leiste-reiter", titel);
      b.addEventListener("click", () => this.oeffnen(f));
      this.reiter.set(f, b);
      kopf.append(b);
    };
    fach("hier", t("Hier"));
    fach("einfuegen", t("Einfügen"));
    fach("formel", t("Formel"));
    fach("seite", t("Seite"));
    fach("dokument", t("Dokument"));
    fach("pakete", t("Pakete"));
    const zu = el("button", "leiste-zu", "×");
    zu.title = t("Seitenleiste schließen");
    zu.addEventListener("click", () => this.schliessen());
    kopf.append(zu);
    this.inhalt = el("div", "leiste-inhalt");
    this.huelle.append(kopf, this.inhalt);
    breiteZiehbar(this.huelle);
    ort.append(this.huelle);
    // Wer die Leiste verlässt, bekommt den neuen Stand.
    this.huelle.addEventListener("focusout", (e) => {
      if (this.ausstehend && !this.huelle.contains(e.relatedTarget as Node | null)) void this.laden();
    });
  }

  /**
   * Neu laden, auch wenn gerade ein Editor der Leiste den Fokus hat (etwa nach „veraltet“ oder nach
   * einem Patch aus der Leiste, sonst trägt das nächste Feld eine alte Revision). `pos` = neuer Cursor.
   */
  nachladen(pos?: number) {
    if (pos !== undefined) this.pos = pos;
    if (this.offen) void this.laden(true);
  }

  get fokusInnen(): boolean {
    return this.huelle.contains(document.activeElement);
  }

  /** Fokus aufs erste bedienbare Element des Fachs. */
  fokussieren() {
    this.inhalt
      .querySelector<HTMLElement>("input:not(:disabled), textarea:not(:disabled), select:not(:disabled), button:not(:disabled)")
      ?.focus();
  }

  /** Ein Eingabefeld der Leiste hat den Fokus: dann nicht neu zeichnen (M1: Farbwähler, Tippen). */
  private editiert(): boolean {
    const a = document.activeElement;
    return !!a && this.inhalt.contains(a) && /^(INPUT|TEXTAREA|SELECT)$/.test(a.tagName);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  oeffnen(fach: Fach) {
    this.fach = fach;
    this.huelle.hidden = false;
    for (const [f, b] of this.reiter) b.classList.toggle("aktiv", f === fach);
    void this.laden();
  }

  schliessen() {
    this.huelle.hidden = true;
    this.a.geschlossen();
  }

  /** Fach „Dokument“ öffnen und das Formularfeld `feld` zeigen (Klick auf die Titelei). */
  feldZeigen(feld: string | null) {
    this.oeffnen("dokument");
    if (feld) void this.dokument.feldZeigen(feld);
  }

  /** Cursor bewegt oder Text geändert; nur wenn offen, mit 150 ms Verzögerung. */
  cursor(pos: number) {
    this.pos = pos;
    // Die Fächer „Dokument“ und „Pakete“ hängen nicht am Cursor.
    if (!this.offen || this.fach === "dokument" || this.fach === "pakete" || this.fach === "seite") return;
    clearTimeout(this.zeitgeber);
    this.zeitgeber = window.setTimeout(() => void this.laden(), 150);
  }

  private async laden(erzwingen = false) {
    if (this.fach === "hier" && !erzwingen && this.editiert()) {
      this.ausstehend = true;
      return;
    }
    this.ausstehend = false;
    const nr = ++this.anfrage;
    if (this.fach === "dokument") {
      if (this.dokument.huelle.parentElement !== this.inhalt) this.inhalt.replaceChildren(this.dokument.huelle);
      await this.dokument.zeichnen();
      return;
    }
    if (this.fach === "formel") {
      if (this.formel.huelle.parentElement !== this.inhalt) this.inhalt.replaceChildren(this.formel.huelle);
      await this.formel.zeichnen(this.pos);
      return;
    }
    if (this.fach === "seite") {
      if (this.seite.huelle.parentElement !== this.inhalt) this.inhalt.replaceChildren(this.seite.huelle);
      await this.seite.zeichnen();
      return;
    }
    if (this.fach === "pakete") {
      if (this.pakete.huelle.parentElement !== this.inhalt) this.inhalt.replaceChildren(this.pakete.huelle);
      await this.pakete.zeichnen();
      return;
    }
    if (this.fach === "einfuegen") {
      const liste = await this.a.bausteine(this.suche);
      if (nr === this.anfrage) this.einfuegenZeichnen(liste);
      return;
    }
    try {
      const h = await this.a.hier(this.pos);
      if (nr === this.anfrage && h) this.hierZeichnen(h);
    } catch {
      // z. B. Position außerhalb nach einer großen Änderung; der nächste Cursor lädt neu
    }
  }

  // --- Fach „Hier“ -------------------------------------------------------------------------

  private hierZeichnen(h: HierJs) {
    // Fokus im Editor der Leiste merken und nach dem Neuzeichnen zurückgeben.
    const fokus = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>("[data-schluessel]")?.dataset.schluessel;
    this.rev = h.rev;
    const teile: HTMLElement[] = [];
    h.ebenen.forEach((e, i) => teile.push(this.ebene(e, i === 0)));
    if (h.markup.length) teile.push(this.markupKnoepfe(h.markup));
    if (h.tabelle) teile.push(this.raster(h.tabelle));
    if (!teile.length) teile.push(el("p", "leiste-leer", t("Hier gibt es nichts einzustellen.")));
    this.inhalt.replaceChildren(...teile);
    if (fokus) {
      const ziel = this.inhalt.querySelector<HTMLElement>(`[data-schluessel="${CSS.escape(fokus)}"]`);
      ziel?.querySelector<HTMLElement>("input, textarea, select")?.focus();
    }
  }

  private ebene(e: EbeneJs, offen: boolean): HTMLElement {
    const d = el("details", "leiste-ebene");
    d.open = offen;
    d.append(el("summary", "", e.baustein));
    for (const f of e.felder) {
      const zeile = el("div", "leiste-feld");
      zeile.dataset.schluessel = `${e.von}/${f.parameter}`;
      zeile.append(el("label", "leiste-titel", f.titel));
      const rev = this.rev;
      zeile.append(editor(f, (w, ende) => this.a.setzen(rev, e.von, f.parameter, w, ende)));
      if (f.fehlt && !f.gesperrt) {
        zeile.append(el("div", "leiste-hilfe", f.standard ? t("nicht gesetzt (Standard: {standard})", { standard: f.standard }) : t("nicht gesetzt")));
      }
      if (f.hilfe) zeile.append(el("div", "leiste-hilfe", f.hilfe));
      d.append(zeile);
    }
    return d;
  }

  private markupKnoepfe(markup: MarkupJs[]): HTMLElement {
    const d = el("div", "leiste-markup");
    const knopf = (titel: string, aktion: MarkupAktion) => {
      const b = el("button", "", titel);
      b.addEventListener("click", () => this.a.markup(aktion));
      d.append(b);
    };
    for (const m of markup) {
      if (m.art === "ueberschrift") {
        d.append(el("span", "leiste-titel", t("Überschrift, Ebene {ebene}", { ebene: m.ebene ?? "" })));
        knopf(t("Ebene höher"), "ebene-minus");
        knopf(t("Ebene tiefer"), "ebene-plus");
      } else if (m.art === "liste") {
        d.append(el("span", "leiste-titel", m.aufzaehlung ? t("Aufzählung") : t("Liste")));
        knopf(m.aufzaehlung ? t("Zur Liste") : t("Zur Aufzählung"), "liste-umschalten");
      } else if (m.art === "zitat") {
        d.append(this.zitatZeile(m));
      } else {
        d.append(el("span", "leiste-titel", m.block ? t("Formel als Block") : t("Formel im Satz")));
        knopf(m.block ? t("Im Satz") : t("Als Block"), "formel-umschalten");
        const feld = el("button", "", t("Im Formelfeld ändern"));
        feld.addEventListener("click", () => this.oeffnen("formel"));
        d.append(feld);
      }
    }
    return d;
  }

  /** Quelle und Seitenangabe eines Zitats (PLAN-M2 AP11): Enter schreibt, leer entfernt die Angabe. */
  private zitatZeile(m: MarkupJs): HTMLElement {
    const z = el("div", "leiste-zitat");
    z.dataset.schluessel = "zitat-seite";
    const seite = el("input");
    seite.placeholder = t("Seitenangabe");
    seite.title = t("Enter oder Verlassen übernimmt, Esc verwirft; beginnt die Angabe mit einer Zahl, kommt „S.“ davor; leer entfernt sie");
    seite.spellcheck = false;
    let vorher = m.zusatz ?? "";
    seite.value = vorher;
    const uebernehmen = () => {
      const neu = seite.value.trim();
      if (neu === vorher) return;
      vorher = neu; // Enter, danach Verlassen: nur einmal schreiben
      this.a.zitat(this.rev, neu || null, null);
    };
    seite.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        uebernehmen();
      } else if (e.key === "Escape") {
        // verwerfen und zurück in den Text, ohne die Leiste zu schließen
        e.preventDefault();
        e.stopPropagation();
        seite.value = vorher;
        this.a.zurueck?.();
      }
    });
    // Verlassen übernimmt wie Enter (wie die übrigen Felder der Leiste).
    seite.addEventListener("blur", uebernehmen);
    const quelle = el("button", "", t("Andere Quelle …"));
    quelle.addEventListener("click", () => this.a.zitatQuelle(this.rev, m.zusatz ?? null));
    z.append(el("span", "leiste-titel", t("Zitat: {schluessel}", { schluessel: m.schluessel ?? "" })), seite, quelle);
    return z;
  }

  // --- Tabellenraster ----------------------------------------------------------------------

  private raster(tj: TabelleJs): HTMLElement {
    const d = el("div", "leiste-tabelle");
    d.append(el("div", "leiste-titel", t("Tabelle")));
    if (!tj.tabelle) {
      d.append(el("div", "feld-gesperrt", tj.gesperrt ?? t("nicht bearbeitbar")));
      return d;
    }
    const tab = tj.tabelle;
    const rev = this.rev;
    if (this.rasterVon !== tj.von) {
      this.rasterVon = tj.von;
      this.auswahl = null;
      this.anker = null;
    }
    if (this.auswahl && (this.auswahl.zeile + this.auswahl.zeilen > tab.zeilen || this.auswahl.spalte + this.auswahl.spalten > tab.spalten)) {
      this.auswahl = null;
    }
    const hilfe = el("div", "leiste-hilfe");
    const HILFE = t("Zelle anklicken, Umschalt+Klick wählt einen Bereich, Doppelklick ändert den Text.");
    hilfe.textContent = HILFE;
    const gitter = el("div", "raster");
    gitter.style.gridTemplateColumns = `repeat(${tab.spalten}, minmax(3em, 1fr))`;
    const zellen: [HTMLElement, (typeof tab.zellen)[number]][] = [];
    const knoepfe: [HTMLButtonElement, Knopf][] = [];
    // Auswahl ohne Neuaufbau, damit ein Doppelklick dieselbe Zelle trifft.
    const auffrischen = () => {
      const a = this.auswahl;
      for (const [c, z] of zellen) {
        c.classList.toggle(
          "gewaehlt",
          !!a && z.zeile >= a.zeile && z.zeile < a.zeile + a.zeilen && z.spalte >= a.spalte && z.spalte < a.spalte + a.spalten,
        );
      }
      for (const [b, k] of knoepfe) b.disabled = !a || aktionFuer(k, a, tab) === null;
    };
    for (const z of tab.zellen) {
      const c = el("div", "raster-zelle", z.inhalt.text || " ");
      if (!bearbeitbar(z)) c.classList.add("raster-ausdruck");
      c.style.gridRow = `${z.zeile + 1} / span ${z.zeilen}`;
      c.style.gridColumn = `${z.spalte + 1} / span ${z.spalten}`;
      c.addEventListener("click", (ev) => {
        const hier: [number, number] = [z.zeile, z.spalte];
        const ende: [number, number] = [z.zeile + z.zeilen - 1, z.spalte + z.spalten - 1];
        if (ev.shiftKey && this.anker) {
          this.auswahl = auswahlRechteck(tab, this.anker, hier);
        } else {
          this.anker = hier;
          this.auswahl = auswahlRechteck(tab, hier, ende);
        }
        hilfe.textContent = HILFE;
        auffrischen();
      });
      c.addEventListener("dblclick", () => {
        if (bearbeitbar(z)) this.zelleBearbeiten(c, rev, tj.von, z.zeile, z.spalte, z.inhalt.text);
        else hilfe.textContent = t("Diese Zelle ist berechnet – im Text bearbeiten.");
      });
      zellen.push([c, z]);
      gitter.append(c);
    }
    d.append(gitter);
    const leiste = el("div", "raster-knoepfe");
    for (const [k, titel, text] of KNOEPFE) {
      const b = el("button", "", titel);
      b.title = text;
      b.addEventListener("click", () => {
        const a = this.auswahl;
        const aktion = a ? aktionFuer(k, a, tab) : null;
        if (!a || !aktion) return;
        this.auswahl = auswahlNach(aktion, a);
        void this.a.tabelle(rev, tj.von, aktion);
      });
      knoepfe.push([b, k]);
      leiste.append(b);
    }
    auffrischen();
    d.append(leiste, hilfe);
    return d;
  }

  private zelleBearbeiten(zelle: HTMLElement, rev: number, aufruf: number, zeile: number, spalte: number, text: string) {
    const e = el("input", "raster-eingabe");
    e.value = text;
    let fertig = false;
    const ende = (setzen: boolean) => {
      if (fertig) return;
      fertig = true;
      const neu = setzen && e.value !== text;
      zelle.textContent = (neu ? e.value : text) || " ";
      // Abgelehnt: wieder der alte Text, nicht bis zur nächsten Cursorbewegung der abgelehnte.
      if (neu)
        void this.a.tabelle(rev, aufruf, { art: "zelltext", zeile, spalte, inhalt: e.value }).then((ok) => {
          if (!ok) zelle.textContent = text || " ";
        });
    };
    e.addEventListener("keydown", (ev) => {
      ev.stopPropagation();
      if (ev.key === "Enter") ende(true);
      else if (ev.key === "Escape") ende(false);
    });
    e.addEventListener("blur", () => ende(true));
    zelle.replaceChildren(e);
    e.focus();
    e.select();
  }

  // --- Fach „Einfügen“ ---------------------------------------------------------------------

  private einfuegenZeichnen(liste: EintragJs[]) {
    const suche = el("input", "leiste-suche");
    suche.placeholder = t("Baustein suchen …");
    suche.value = this.suche;
    suche.addEventListener("input", () => {
      this.suche = suche.value;
      void this.laden();
    });
    const ul = el("ul", "leiste-bausteine");
    let gruppe: string | null = null;
    for (const e of liste) {
      if (e.gruppe !== gruppe) {
        gruppe = e.gruppe;
        if (gruppe) ul.append(el("li", "palette-abschnitt", gruppe));
      }
      const li = el("li", "baustein-zeile", e.titel);
      li.addEventListener("click", () => this.a.einfuegen(e.id));
      ul.append(li);
    }
    if (!liste.length) ul.append(el("li", "leiste-leer", t("Hier passt kein Baustein.")));
    const hatteFokus = document.activeElement?.classList.contains("leiste-suche");
    this.inhalt.replaceChildren(suche, ul);
    if (hatteFokus) {
      suche.focus();
      suche.setSelectionRange(suche.value.length, suche.value.length);
    }
  }
}
