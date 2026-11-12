// Gliederung: Überschriften neben dem Editor, Klick springt hin, Ziehen oder Alt+↑/↓ verschiebt
// ganze Abschnitte. Gelesen und verschoben wird in Rust (tftn_edit::gliederung).

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

export interface GliederungJs {
  rev: number;
  eintraege: EintragJs[];
  gesamt: Zahlen;
}

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

export class GliederungLeiste {
  private liste = el("div", "gl-liste");
  private eintraege: EintragJs[] = [];
  private knoepfe: HTMLButtonElement[] = [];
  private gezogen: number | null = null;
  private markiert = -1;
  private ende = el("div", "gl-ende", t("ans Ende"));

  constructor(
    ort: HTMLElement,
    private a: GliederungAktionen,
  ) {
    const kopf = el("div", "gl-kopf");
    kopf.append(el("span", "", t("Gliederung")), schliessenKnopf(() => a.schliessen()));
    ort.append(kopf, this.liste);
    this.ende.addEventListener("dragover", (e) => this.ueber(e, null, this.ende));
    this.ende.addEventListener("dragleave", () => this.ende.classList.remove("gl-davor"));
    this.ende.addEventListener("drop", (e) => this.fallen(e, null));
  }

  zeigen(neu: readonly EintragJs[], pos: number) {
    const hatteFokus = this.liste.contains(document.activeElement);
    this.eintraege = [...neu];
    this.markiert = -1;
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

  /** Markiert den Abschnitt, in dem der Cursor steht. */
  cursor(pos: number) {
    const i = aktuell(this.eintraege, pos);
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
