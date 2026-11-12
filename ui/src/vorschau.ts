// Vorschau-Spalte (PLAN-M1 AP4, AP5): Seitenbilder über tftn://, nur sichtbare Seiten laden,
// letzter guter Stand mit Banner, Messung Taste → Bild gezeichnet, Klicksprung und Cursor-Marke.

import { t } from "./sprache";

export interface SeiteJs {
  hash: string;
  breite: number;
  hoehe: number;
}

export interface MeldungJs {
  fehler: boolean;
  text: string;
  ort: string | null;
  von: number | null;
  bis: number | null;
}

export interface VorschauEreignis {
  rev: number;
  gelungen: boolean;
  stand_rev: number | null;
  seiten: SeiteJs[];
  meldungen: MeldungJs[];
  dauer_ms: number;
}

/** Unter Windows (WebView2) laufen eigene Schemata als http://<schema>.localhost. */
const BASIS = navigator.userAgent.includes("Windows") ? "http://tftn.localhost/" : "tftn://localhost/";

/** Seite (0-basiert) und Punkt in pt. */
export type Stelle = [number, number, number];

export class Vorschau {
  private bilder: HTMLImageElement[] = [];
  private rahmen: HTMLDivElement[] = [];
  private seiten: SeiteJs[] = [];
  private sichtbar = new Set<number>();
  private beobachter: IntersectionObserver;
  /** Wartet auf das erste gezeichnete Bild einer Revision (für die Latenzmessung). */
  private wartet: { rev: number; offen: number } | null = null;

  constructor(
    private behaelter: HTMLElement,
    private liste: HTMLElement,
    private banner: HTMLElement,
    private gezeichnet: (rev: number) => void,
    klick: (seite: number, x: number, y: number) => void,
    /** Rechtsklick auf eine Seite (PLAN-M2 AP6): Kontextmenü an der Mausposition. */
    kontext?: (seite: number, x: number, y: number, maus: { x: number; y: number }) => void,
  ) {
    liste.addEventListener("click", (ev) => {
      const b = ev.target as HTMLElement;
      if (!(b instanceof HTMLImageElement)) return;
      const nr = Number(b.dataset.nr);
      const s = this.seiten[nr];
      if (!s) return;
      const m = this.massstab(nr);
      klick(nr, ev.offsetX / m, ev.offsetY / m);
    });
    liste.addEventListener("contextmenu", (ev) => {
      const b = ev.target as HTMLElement;
      if (!kontext || !(b instanceof HTMLImageElement)) return;
      const nr = Number(b.dataset.nr);
      if (!this.seiten[nr]) return;
      ev.preventDefault();
      const m = this.massstab(nr);
      kontext(nr, ev.offsetX / m, ev.offsetY / m, { x: ev.clientX, y: ev.clientY });
    });
    this.beobachter = new IntersectionObserver(
      (eintraege) => {
        for (const e of eintraege) {
          const nr = Number((e.target as HTMLElement).dataset.nr);
          if (e.isIntersecting) this.sichtbar.add(nr);
          else this.sichtbar.delete(nr);
          if (e.isIntersecting) this.laden(nr, null);
        }
      },
      { root: behaelter, rootMargin: "200px 0px" },
    );
    new ResizeObserver(() => this.groessen()).observe(behaelter);
  }

  /** Bildschirmpixel je Punkt bei der aktuellen Spaltenbreite. */
  private ppi(seite: SeiteJs): number {
    const breite = Math.max(200, this.behaelter.clientWidth - 32);
    const zoom = breite / seite.breite;
    return Math.round(72 * zoom * (window.devicePixelRatio || 1));
  }

  /** Bildschirmpixel je pt der Seite `nr`. */
  private massstab(nr: number): number {
    const b = this.bilder[nr];
    const s = this.seiten[nr];
    return b && s ? b.clientWidth / s.breite : 1;
  }

  /** Scrollt zur Stelle der Schreibmarke, wenn sie nicht zu sehen ist. */
  folgen(stellen: Stelle[]) {
    const ziel = stellen[0];
    if (!ziel || !this.rahmen[ziel[0]]) return;
    const [nr, , y] = ziel;
    const m = this.massstab(nr);
    const oben = this.rahmen[nr].offsetTop + (y - 10) * m;
    const sicht = this.behaelter;
    if (oben < sicht.scrollTop || oben > sicht.scrollTop + sicht.clientHeight - 40) {
      sicht.scrollTo({ top: oben - sicht.clientHeight / 3, behavior: "smooth" });
    }
  }

  /** Scrollt zu einem Punkt (interner Verweis). */
  scrollen(nr: number, y: number) {
    const r = this.rahmen[nr];
    if (r) this.behaelter.scrollTo({ top: r.offsetTop + y * this.massstab(nr) - 40, behavior: "smooth" });
  }

  private groessen() {
    const breite = Math.max(200, this.behaelter.clientWidth - 32);
    this.bilder.forEach((b, i) => {
      const s = this.seiten[i];
      if (!s) return;
      b.style.width = `${breite}px`;
      b.style.height = `${(breite * s.hoehe) / s.breite}px`;
    });
  }

  private laden(nr: number, rev: number | null) {
    const b = this.bilder[nr];
    const s = this.seiten[nr];
    if (!b || !s) return;
    const url = `${BASIS}seite/${nr}/${s.hash}?ppi=${this.ppi(s)}`;
    if (b.dataset.url === url) return;
    b.dataset.url = url;
    if (rev !== null && this.wartet?.rev === rev) this.wartet.offen += 1;
    b.onload = () => {
      if (rev !== null && this.wartet?.rev === rev && --this.wartet.offen === 0) this.fertig(rev);
    };
    b.src = url;
  }

  private fertig(rev: number) {
    this.wartet = null;
    requestAnimationFrame(() => this.gezeichnet(rev));
  }

  aktualisieren(e: VorschauEreignis) {
    this.behaelter.classList.toggle("veraltet", !e.gelungen);
    const erster = e.meldungen.find((m) => m.fehler);
    if (e.gelungen) {
      this.banner.hidden = true;
    } else {
      this.banner.hidden = false;
      // Erste Zeile fett: der deutsche Satz; darunter Tipps und Wortlaut von typst.
      const [satz, ...rest] = (erster?.text ?? t("Fehler im Dokument.")).split("\n");
      const ort = erster?.ort ?? null;
      const zeile = ort ? /^(.*):(\d+):\d+$/.exec(ort) : null;
      const wo = zeile
        ? t("Fehler in {datei}, Zeile {zeile}: {satz}", { datei: zeile[1], zeile: zeile[2], satz })
        : ort
          ? t("Fehler in {ort}: {satz}", { ort, satz })
          : t("Fehler: {satz}", { satz });
      const stand = e.stand_rev === null ? t("Noch keine Vorschau.") : t("Die Vorschau zeigt den letzten Stand ohne Fehler.");
      const kopf = document.createElement("strong");
      kopf.textContent = wo;
      const unten = document.createElement("div");
      unten.className = "banner-rest";
      unten.textContent = [...rest.map((z) => z.trim()), stand].join("\n");
      this.banner.replaceChildren(kopf, unten);
    }
    if (!e.gelungen) return;

    // Seitenliste angleichen
    while (this.bilder.length < e.seiten.length) {
      const r = document.createElement("div");
      r.className = "seite-rahmen";
      const b = document.createElement("img");
      b.className = "seite";
      b.alt = t("Seite {nr}", { nr: this.bilder.length + 1 });
      b.dataset.nr = String(this.bilder.length);
      b.draggable = false;
      r.append(b);
      this.liste.append(r);
      this.beobachter.observe(b);
      this.bilder.push(b);
      this.rahmen.push(r);
    }
    while (this.bilder.length > e.seiten.length) {
      const b = this.bilder.pop()!;
      this.beobachter.unobserve(b);
      this.sichtbar.delete(Number(b.dataset.nr));
      this.rahmen.pop()!.remove();
    }
    this.seiten = e.seiten;
    this.groessen();

    this.wartet = { rev: e.rev, offen: 0 };
    for (const nr of this.sichtbar) this.laden(nr, e.rev);
    // Keine sichtbare Seite hat sich geändert: Vorschau ist sofort aktuell.
    if (this.wartet?.offen === 0) this.fertig(e.rev);
  }
}
