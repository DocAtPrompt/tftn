// Befehlspalette „Zitat einfügen“ (PLAN-M1 AP7): Suche in der .bib, optionale Seitenangabe,
// Einfügen an der Schreibmarke. Tastatur: ↑/↓ wählen, Enter einfügen, Esc schließen.

import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { type DoiJs, doiAusEingabe } from "./doi";
import { t } from "./sprache";

export interface Treffer {
  schluessel: string;
  autoren: string;
  jahr: number | null;
  titel: string;
  quelle: string;
  doi: string | null;
}

export interface ZitatSuche {
  treffer: Treffer[];
  anzahl: number;
  dauer_ms: number;
}

export class ZitatPalette {
  private huelle: HTMLDivElement;
  private suche: HTMLInputElement;
  private seite: HTMLInputElement;
  private liste: HTMLUListElement;
  private fuss: HTMLDivElement;
  private treffer: Treffer[] = [];
  /** Steht eine DOI in der Suche, die die Literatur noch nicht hat: erste Zeile „DOI nachschlagen“. */
  private doi: string | null = null;
  private gewaehlt = 0;
  private zeitgeber: number | undefined;
  /** Gesetzt, solange die Palette die Quelle eines vorhandenen Zitats wechselt (PLAN-M2 AP11). */
  private aendern: ((schluessel: string) => void) | null = null;

  constructor(
    private suchen: (anfrage: string) => Promise<ZitatSuche>,
    private einfuegen: (schluessel: string, seite: string | null, anhang?: string | null) => void,
    private geschlossen: () => void,
    private doiHolen: (doi: string) => Promise<DoiJs>,
  ) {
    this.huelle = document.createElement("div");
    this.huelle.id = "palette";
    this.huelle.hidden = true;
    this.huelle.innerHTML = `
      <div class="neu-kopf"><div class="palette-kopf">${t("Zitat einfügen")}</div></div>
      <div class="palette-felder">
        <input class="palette-suche" placeholder="${t("Autor, Jahr, Titel, Schlüssel oder DOI")}" spellcheck="false" />
        <input class="palette-seite" placeholder="${t("Seite")}" spellcheck="false" />
      </div>
      <ul class="palette-liste"></ul>
      <div class="palette-fuss"></div>`;
    document.body.append(this.huelle);
    this.huelle.querySelector(".neu-kopf")!.append(schliessenKnopf(() => this.schliessen()));
    dialogAnmelden(this);
    this.suche = this.huelle.querySelector(".palette-suche")!;
    this.seite = this.huelle.querySelector(".palette-seite")!;
    this.liste = this.huelle.querySelector(".palette-liste")!;
    this.fuss = this.huelle.querySelector(".palette-fuss")!;

    this.suche.addEventListener("input", () => {
      clearTimeout(this.zeitgeber);
      this.zeitgeber = window.setTimeout(() => void this.aktualisieren(), 40);
    });
    this.huelle.addEventListener("keydown", (e) => this.taste(e));
    this.liste.addEventListener("click", (e) => {
      const li = (e.target as HTMLElement).closest("li");
      if (li) {
        this.gewaehlt = Number(li.dataset.nr);
        this.uebernehmen();
      }
    });
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  oeffnen() {
    this.anzeigen(null);
  }

  /** Quelle eines vorhandenen Zitats wählen; das Feld „Seite“ entfällt, die Angabe bleibt. */
  oeffnenZumAendern(ziel: (schluessel: string) => void) {
    this.anzeigen(ziel);
  }

  private anzeigen(aendern: ((schluessel: string) => void) | null) {
    this.aendern = aendern;
    this.huelle.querySelector(".palette-kopf")!.textContent = aendern ? t("Quelle ändern") : t("Zitat einfügen");
    this.seite.hidden = aendern !== null;
    this.huelle.hidden = false;
    this.seite.value = "";
    this.suche.select();
    this.suche.focus();
    void this.aktualisieren();
  }

  schliessen() {
    this.huelle.hidden = true;
    this.geschlossen();
  }

  private async aktualisieren() {
    const anfrage = this.suche.value;
    const s = await this.suchen(anfrage);
    if (anfrage !== this.suche.value) return; // inzwischen weitergetippt
    this.treffer = s.treffer;
    const doi = doiAusEingabe(anfrage);
    this.doi = doi && !s.treffer.some((x) => x.doi?.toLowerCase() === doi.toLowerCase()) ? doi : null;
    this.gewaehlt = 0;
    const doiZeile: HTMLLIElement[] = [];
    if (this.doi) {
      const li = document.createElement("li");
      li.dataset.nr = "0";
      li.className = "zitat-doi";
      const kopf = document.createElement("div");
      kopf.className = "zitat-kopf";
      kopf.textContent = t("DOI nachschlagen und zitieren");
      const titel = document.createElement("div");
      titel.className = "zitat-titel";
      titel.textContent = t("{doi} · bei Crossref bzw. DataCite (Netz), Eintrag kommt in die Literaturdatei", { doi: this.doi });
      li.append(kopf, titel);
      doiZeile.push(li);
    }
    const versatz = doiZeile.length;
    this.liste.replaceChildren(
      ...doiZeile,
      ...s.treffer.map((tr, i) => {
        const li = document.createElement("li");
        li.dataset.nr = String(i + versatz);
        const kopf = document.createElement("div");
        kopf.className = "zitat-kopf";
        kopf.textContent = `${tr.autoren || t("o. A.")} ${tr.jahr ?? t("o. J.")}`;
        const titel = document.createElement("div");
        titel.className = "zitat-titel";
        titel.textContent = tr.quelle ? t("{titel}. In: {quelle}", { titel: tr.titel, quelle: tr.quelle }) : tr.titel;
        const schluessel = document.createElement("code");
        schluessel.textContent = tr.schluessel;
        li.append(kopf, titel, schluessel);
        return li;
      }),
    );
    this.markieren();
    this.fuss.textContent =
      s.anzahl === 0
        ? t("Keine Literatur geladen (literatur.bib fehlt oder ist fehlerhaft).")
        : t("{treffer} von {anzahl} Einträgen · Suche {ms} ms · Enter fügt ein", {
            treffer: s.treffer.length,
            anzahl: s.anzahl,
            ms: s.dauer_ms.toFixed(1),
          });
  }

  private markieren() {
    this.liste.querySelectorAll("li").forEach((li, i) => li.classList.toggle("gewaehlt", i === this.gewaehlt));
    this.liste.children[this.gewaehlt]?.scrollIntoView({ block: "nearest" });
  }

  private taste(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      this.schliessen();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = this.treffer.length + (this.doi ? 1 : 0);
      if (n) this.gewaehlt = (this.gewaehlt + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      this.markieren();
    } else if (e.key === "Enter") {
      e.preventDefault();
      this.uebernehmen();
    }
  }

  private uebernehmen() {
    if (this.doi && this.gewaehlt === 0) {
      void this.doiUebernehmen(this.doi);
      return;
    }
    const tr = this.treffer[this.gewaehlt - (this.doi ? 1 : 0)];
    if (!tr) return;
    const seite = this.seite.value.trim();
    this.huelle.hidden = true;
    if (this.aendern) this.aendern(tr.schluessel);
    else this.einfuegen(tr.schluessel, seite === "" ? null : seite);
  }

  /** DOI über den Holer nachschlagen; der Dialog bleibt bei einem Fehler offen und nennt ihn. */
  private async doiUebernehmen(doi: string) {
    this.fuss.textContent = t("Schlage {doi} nach …", { doi });
    try {
      const d = await this.doiHolen(doi);
      if (this.huelle.hidden) return;
      const seite = this.seite.value.trim();
      this.huelle.hidden = true;
      if (this.aendern) this.aendern(d.schluessel);
      else this.einfuegen(d.schluessel, seite === "" ? null : seite, d.verzeichnis);
    } catch (fehler) {
      this.fuss.textContent = String(fehler);
    }
  }
}
