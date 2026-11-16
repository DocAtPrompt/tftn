// Gliederung: Überschriften neben dem Editor, Klick springt hin, Ziehen oder Alt+↑/↓ verschiebt
// ganze Abschnitte. Gelesen und verschoben wird in Rust (tftn_edit::gliederung). Ein zweiter
// Reiter zeigt die Kommentare (`// …`, `/* … */`) nach Abschnitten (tftn_edit::kommentare).

import { schliessenKnopf } from "./dialoge";
import { type Zahlen, tausender } from "./zaehler";
import { t } from "./sprache";

export interface EintragJs {
  ebene: number;
  titel: string;
  /** UTF-16: Anfang der Überschrift */
  von: number;
  /** UTF-16: Ende des Abschnitts */
  bis: number;
  /** Wörter im Abschnitt samt Unterabschnitten */
  woerter?: number;
}

export interface KommentarJs {
  /** ohne Kommentarzeichen; Zeilen getrennt durch `\n` */
  text: string;
  /** UTF-16 */
  von: number;
  /** UTF-16 */
  bis: number;
}

export interface GliederungJs {
  rev: number;
  eintraege: EintragJs[];
  kommentare: KommentarJs[];
  gesamt: Zahlen;
}

export interface KommentarGruppe {
  /** Titel des Abschnitts; `null` vor der ersten Überschrift */
  titel: string | null;
  /** UTF-16: Anfang der Überschrift (Gruppe vor der ersten: -1) */
  von: number;
  kommentare: KommentarJs[];
}

/** Kommentare nach dem (Unter-)Abschnitt, in dem sie stehen; Abschnitte ohne Kommentar fehlen. */
export function kommentarGruppen(eintraege: readonly EintragJs[], kommentare: readonly KommentarJs[]): KommentarGruppe[] {
  const gruppen: KommentarGruppe[] = [];
  for (const k of kommentare) {
    const i = aktuell(eintraege, k.von);
    const von = i < 0 ? -1 : eintraege[i].von;
    const letzte = gruppen[gruppen.length - 1];
    if (letzte && letzte.von === von) letzte.kommentare.push(k);
    else gruppen.push({ titel: i < 0 ? null : eintraege[i].titel, von, kommentare: [k] });
  }
  return gruppen;
}

/** Index des Kommentars, in dem `pos` steht (Ränder eingeschlossen), sonst -1. */
export function kommentarAn(kommentare: readonly KommentarJs[], pos: number): number {
  return kommentare.findIndex((k) => k.von <= pos && pos <= k.bis);
}

export type Ansicht = "gliederung" | "kommentare";

/** Index des Abschnitts, in dem `pos` steht; -1 vor der ersten Überschrift. */
export function aktuell(liste: readonly EintragJs[], pos: number): number {
  let i = -1;
  for (let j = 0; j < liste.length && liste[j].von <= pos; j++) i = j;
  return i;
}

/** Erster Index nach dem Abschnitt `nr` samt Unterabschnitten. */
function nachTeilbaum(liste: readonly EintragJs[], nr: number): number {
  let j = nr + 1;
  while (j < liste.length && liste[j].ebene > liste[nr].ebene) j++;
  return j;
}

/** Darf Abschnitt `nr` vor `vor` (null: ans Ende)? Nicht in sich selbst, nicht an seine Stelle. */
export function zielErlaubt(liste: readonly EintragJs[], nr: number, vor: number | null): boolean {
  const ende = nachTeilbaum(liste, nr);
  const ziel = vor ?? liste.length;
  return ziel < nr || ziel > ende;
}

/**
 * Ziel für Alt+↑/↓: am Nachbarn gleicher Ebene vorbei, ohne das Kapitel zu verlassen.
 * Liefert den Index, vor den verschoben wird, `null` für „ans Ende“, `undefined` für „geht nicht“.
 */
export function nachbarZiel(
  liste: readonly EintragJs[],
  nr: number,
  richtung: "hoch" | "runter",
): number | null | undefined {
  const ebene = liste[nr].ebene;
  if (richtung === "hoch") {
    for (let j = nr - 1; j >= 0; j--) {
      if (liste[j].ebene < ebene) return undefined;
      if (liste[j].ebene === ebene) return j;
    }
    return undefined;
  }
  const naechster = nachTeilbaum(liste, nr);
  if (naechster >= liste.length || liste[naechster].ebene !== ebene) return undefined;
  const dahinter = nachTeilbaum(liste, naechster);
  return dahinter >= liste.length ? null : dahinter;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface GliederungAktionen {
  springen(von: number): void;
  verschieben(nr: number, vor: number | null): Promise<void>;
  schliessen(): void;
}

/** Merker in localStorage: zuletzt gewählter Reiter. */
const ANSICHT = "tftn-gliederung-ansicht";

function ansichtAnfang(): Ansicht {
  try {
    return localStorage.getItem(ANSICHT) === "kommentare" ? "kommentare" : "gliederung";
  } catch {
    return "gliederung";
  }
}

export class GliederungLeiste {
  private liste = el("div", "gl-liste");
  private eintraege: EintragJs[] = [];
  private kommentare: KommentarJs[] = [];
  private knoepfe: HTMLButtonElement[] = [];
  private gezogen: number | null = null;
  private markiert = -1;
  private pos = 0;
  private ende = el("div", "gl-ende", t("ans Ende"));
  private ansicht: Ansicht = ansichtAnfang();
  private reiter: Record<Ansicht, HTMLButtonElement>;

  constructor(
    ort: HTMLElement,
    private a: GliederungAktionen,
  ) {
    const kopf = el("div", "gl-kopf");
    const reiter = el("div", "gl-reiter");
    reiter.setAttribute("role", "tablist");
    this.reiter = {
      gliederung: this.reiterKnopf("gliederung", t("Gliederung")),
      kommentare: this.reiterKnopf("kommentare", t("Kommentare")),
    };
    reiter.append(this.reiter.gliederung, this.reiter.kommentare);
    kopf.append(reiter, schliessenKnopf(() => a.schliessen()));
    ort.append(kopf, this.liste);
    this.ende.addEventListener("dragover", (e) => this.ueber(e, null, this.ende));
    this.ende.addEventListener("dragleave", () => this.ende.classList.remove("gl-davor"));
    this.ende.addEventListener("drop", (e) => this.fallen(e, null));
    this.reiterZeigen();
  }

  private reiterKnopf(ansicht: Ansicht, text: string): HTMLButtonElement {
    const b = el("button", "gl-reiter-knopf", text);
    b.setAttribute("role", "tab");
    b.addEventListener("click", () => this.umschalten(ansicht));
    return b;
  }

  private reiterZeigen() {
    for (const [art, b] of Object.entries(this.reiter) as [Ansicht, HTMLButtonElement][]) {
      b.setAttribute("aria-selected", String(art === this.ansicht));
      b.classList.toggle("gl-reiter-an", art === this.ansicht);
    }
    const n = this.kommentare.length;
    this.reiter.kommentare.textContent = n ? `${t("Kommentare")} ${tausender(n)}` : t("Kommentare");
  }

  umschalten(ansicht: Ansicht) {
    if (ansicht === this.ansicht) return;
    this.ansicht = ansicht;
    try {
      localStorage.setItem(ANSICHT, ansicht);
    } catch {
      // ohne localStorage gilt die Wahl nur bis zum Neuladen
    }
    this.zeigen(this.eintraege, this.pos, this.kommentare);
  }

  zeigen(neu: readonly EintragJs[], pos: number, kommentare: readonly KommentarJs[] = []) {
    const hatteFokus = this.liste.contains(document.activeElement);
    this.eintraege = [...neu];
    this.kommentare = [...kommentare];
    this.markiert = -1;
    this.pos = pos;
    this.reiterZeigen();
    if (this.ansicht === "kommentare") {
      this.kommentareZeigen();
      return;
    }
    if (neu.length === 0) {
      this.knoepfe = [];
      this.liste.replaceChildren(
        el("p", "gl-leer", t("Noch keine Überschriften. Eine Zeile mit „= Titel“ beginnt ein Kapitel, „== Titel“ einen Abschnitt darin.")),
      );
      return;
    }
    this.knoepfe = neu.map((e, i) => this.knopf(e, i));
    this.liste.replaceChildren(...this.knoepfe, this.ende);
    this.cursor(pos);
    if (hatteFokus) (this.knoepfe[this.markiert] ?? this.knoepfe[0]).focus();
  }

  private kommentareZeigen() {
    if (this.kommentare.length === 0) {
      this.knoepfe = [];
      this.liste.replaceChildren(
        el("p", "gl-leer", t("Keine Kommentare. Was nach „//“ steht, ist eine Notiz für dich: Sie bleibt im Text, wird aber nicht gedruckt.")),
      );
      return;
    }
    this.knoepfe = [];
    const teile: HTMLElement[] = [];
    const gruppen = kommentarGruppen(this.eintraege, this.kommentare);
    for (const g of gruppen) {
      if (this.eintraege.length > 0)
        teile.push(el("div", "gl-gruppe", g.titel === null ? t("Vor der ersten Überschrift") : g.titel || t("(ohne Titel)")));
      for (const k of g.kommentare) {
        const b = el("button", "gl-eintrag gl-kommentar", k.text);
        b.title = k.text;
        b.addEventListener("click", () => this.a.springen(k.von));
        this.knoepfe.push(b);
        teile.push(b);
      }
    }
    this.liste.replaceChildren(...teile);
    this.cursor(this.pos);
  }

  /** Markiert den Abschnitt bzw. Kommentar, in dem der Cursor steht. */
  cursor(pos: number) {
    this.pos = pos;
    const i = this.ansicht === "kommentare" ? kommentarAn(this.kommentare, pos) : aktuell(this.eintraege, pos);
    if (i === this.markiert) return;
    this.knoepfe[this.markiert]?.classList.remove("gl-hier");
    this.knoepfe[i]?.classList.add("gl-hier");
    this.knoepfe[i]?.scrollIntoView({ block: "nearest" });
    this.markiert = i;
  }

  private knopf(e: EintragJs, i: number): HTMLButtonElement {
    const b = el("button", `gl-eintrag gl-ebene-${Math.min(e.ebene, 6)}`);
    b.append(el("span", "gl-titel", e.titel || t("(ohne Titel)")));
    if (e.woerter !== undefined) b.append(el("span", "gl-woerter", tausender(e.woerter)));
    b.style.paddingLeft = `${6 + (e.ebene - 1) * 14}px`;
    b.title = t("Klick springt hin. Ziehen oder Alt+↑/↓ verschiebt den ganzen Abschnitt.");
    b.draggable = true;
    b.addEventListener("click", () => this.a.springen(e.von));
    b.addEventListener("keydown", (k) => {
      if (!k.altKey || (k.key !== "ArrowUp" && k.key !== "ArrowDown")) return;
      k.preventDefault();
      const ziel = nachbarZiel(this.eintraege, i, k.key === "ArrowUp" ? "hoch" : "runter");
      if (ziel !== undefined) void this.a.verschieben(i, ziel);
    });
    b.addEventListener("dragstart", (d) => {
      this.gezogen = i;
      d.dataTransfer?.setData("application/x-tftn-abschnitt", String(i));
      if (d.dataTransfer) d.dataTransfer.effectAllowed = "move";
      this.liste.classList.add("gl-zieht");
    });
    b.addEventListener("dragend", () => {
      this.gezogen = null;
      this.liste.classList.remove("gl-zieht");
      for (const k of [...this.knoepfe, this.ende]) k.classList.remove("gl-davor");
    });
    b.addEventListener("dragover", (d) => this.ueber(d, i, b));
    b.addEventListener("dragleave", () => b.classList.remove("gl-davor"));
    b.addEventListener("drop", (d) => this.fallen(d, i));
    return b;
  }

  private ueber(d: DragEvent, vor: number | null, ziel: HTMLElement) {
    if (this.gezogen === null || !zielErlaubt(this.eintraege, this.gezogen, vor)) return;
    d.preventDefault();
    if (d.dataTransfer) d.dataTransfer.dropEffect = "move";
    ziel.classList.add("gl-davor");
  }

  private fallen(d: DragEvent, vor: number | null) {
    const nr = this.gezogen;
    if (nr === null || !zielErlaubt(this.eintraege, nr, vor)) return;
    d.preventDefault();
    void this.a.verschieben(nr, vor);
  }
}
