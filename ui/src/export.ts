// Export-Menü (PLAN-M1 AP8): Profile aus dem Manifest, „Probe“ nach _renders/, „Abnehmen“ nach
// export/ (setzt den Zeitstempel; zweiter Klick bestätigt). Vor dem Export wird gespeichert.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import { anAnker, knopfAnker, oberflaechenZoom } from "./platz";
import { groesse, t } from "./sprache";

export interface ProfilJs {
  id: string;
  art: string;
  standards: string[];
}

export interface ExportJs {
  pfad: string;
  relativ: string;
  gegenprobe: string;
  sha256: string | null;
  warnungen: string[];
}

export interface WordJs {
  pfad: string;
  datei: string;
  fehlt: string[];
}

export interface ArchivJs {
  pfad: string;
  relativ: string;
  pdf: string;
  standard: string;
  sha256: string;
  dateien: number;
  /** Byte */
  groesse: number;
  warnungen: string[];
}

/** „a-2b“ → „PDF/A-2b“, „ua-1“ → „PDF/UA-1“ */
export function standardName(s: string): string {
  const m = /^([a-z]+)-(.+)$/.exec(s);
  return m ? `PDF/${m[1].toUpperCase()}-${m[2]}` : s;
}

/** Name eines Export-Profils im Menü: die mitgelieferten mit Klartext, andere wie im Manifest. */
export function profilName(p: ProfilJs): string {
  const standards = p.standards.map(standardName).join(", ");
  if (p.art === "pdf" && p.id === "archiv") {
    return standards ? t("Archiv-PDF (Langzeit, {standard})", { standard: standards }) : t("Archiv-PDF (Langzeit)");
  }
  if (p.art === "pdf" && p.id === "vorschau" && !standards) return t("PDF");
  return `${p.id} · ${p.art}${standards ? ` · ${standards}` : ""}`;
}

/** Tooltip zum Profil, leer ohne. */
export function profilErklaerung(p: ProfilJs): string {
  if (p.art === "pdf" && p.id === "archiv") {
    return t("Nur das PDF, als PDF/A für die Langzeitablage. Alles, um es auch ohne TFTN neu zu setzen, steckt im Archivpaket (Zip).");
  }
  return "";
}

/** Die Zeilen der Erfolgsmeldung zum Archivpaket. */
export function archivZeilen(a: ArchivJs): string[] {
  return [
    t("Fertig: {datei} ({anzahl} Dateien, {groesse})", { datei: a.relativ, anzahl: a.dateien, groesse: groesse(a.groesse) }),
    t("Darin {pdf} als PDF/A ({standard}), SHA-256 {sha256}", { pdf: a.pdf, standard: a.standard, sha256: a.sha256 }),
    t("LIESMICH.txt im Zip sagt, wie das PDF ohne TFTN neu entsteht."),
  ];
}

export interface WordAktionen {
  exportieren(): Promise<WordJs>;
  zeigen(pfad: string): Promise<void>;
}

export class ExportMenue {
  private menue: HTMLDivElement;

  constructor(
    private knopf: HTMLElement,
    private profile: () => Promise<ProfilJs[]>,
    private exportieren: (profil: string, abnehmen: boolean) => Promise<ExportJs>,
    private word: WordAktionen,
    private serie?: () => void,
    private archiv?: () => Promise<ArchivJs>,
  ) {
    this.menue = document.createElement("div");
    this.menue.id = "export-menue";
    this.menue.hidden = true;
    document.body.append(this.menue);
    this.menue.setAttribute("role", "dialog");
    knopf.setAttribute("aria-haspopup", "dialog");
    knopf.setAttribute("aria-expanded", "false");
    knopf.addEventListener("click", () => (this.menue.hidden ? void this.oeffnen() : this.schliessen()));
    // Esc schließt über dialoge.ts (`escAktivieren`).
    dialogAnmelden(this, this.menue, this.knopf);
  }


  /** Unter dem Knopf, ganz im Fenster; nach jedem Wachsen neu. */
  private platzieren() {
    if (!this.menue.hidden) anAnker(this.menue, knopfAnker(this.knopf), oberflaechenZoom());
  }

  get offen(): boolean {
    return !this.menue.hidden;
  }

  schliessen() {
    this.menue.hidden = true;
    this.knopf.setAttribute("aria-expanded", "false");
  }

  private async oeffnen() {
    this.menue.hidden = false;
    this.knopf.setAttribute("aria-expanded", "true");
    this.menue.textContent = t("Lade Export-Profile …");
    this.platzieren();
    let liste: ProfilJs[];
    try {
      liste = await this.profile();
    } catch (fehler) {
      this.menue.textContent = String(fehler);
      return;
    }
    this.menue.replaceChildren(dialogKopf(t("Export"), () => this.schliessen()));
    for (const p of liste) {
      const zeile = document.createElement("div");
      zeile.className = "export-zeile";
      const name = document.createElement("span");
      name.textContent = profilName(p);
      const erklaerung = profilErklaerung(p);
      if (erklaerung) name.title = erklaerung;
      const probe = document.createElement("button");
      probe.textContent = t("Probe");
      probe.title = t("nach _renders/ (nicht synchronisiert)");
      probe.onclick = () => void this.los(p.id, false, zeile);
      const abnehmen = document.createElement("button");
      abnehmen.textContent = t("Abnehmen …");
      abnehmen.title = t("nach export/, Zeitstempel wird auf jetzt gesetzt");
      abnehmen.onclick = () => {
        if (abnehmen.dataset.sicher) void this.los(p.id, true, zeile);
        else {
          abnehmen.dataset.sicher = "1";
          abnehmen.textContent = t("Wirklich abnehmen?");
        }
      };
      zeile.append(name, probe, abnehmen);
      this.menue.append(zeile);
    }
    if (this.archiv) {
      const a = document.createElement("div");
      a.className = "export-zeile";
      const aName = document.createElement("span");
      aName.textContent = t("Archivpaket (Zip)");
      aName.title = t(
        "Für die Ablage: ein Zip nach export/ mit dem PDF/A und allem, um es neu zu setzen – Quelltext, Daten, Bilder, alle benutzten Schriften und Pakete und eine Anleitung. Damit entsteht dasselbe PDF auch in Jahren und ohne TFTN, Byte für Byte.",
      );
      const aKnopf = document.createElement("button");
      aKnopf.textContent = t("Archivieren");
      aKnopf.onclick = () => void this.archivLos(a);
      a.append(aName, aKnopf);
      this.menue.append(a);
    }
    const w = document.createElement("div");
    w.className = "export-zeile";
    const wName = document.createElement("span");
    wName.textContent = t("Word (.docx) zum Gegenlesen");
    wName.title = t(
      "Text, Überschriften, Fußnoten, Tabellen, Bilder, Formeln, Zitate und Literatur; ohne Seitenlayout. Über Pandoc.",
    );
    const wKnopf = document.createElement("button");
    wKnopf.textContent = t("Exportieren");
    wKnopf.onclick = () => void this.wordLos(w);
    w.append(wName, wKnopf);
    this.menue.append(w);
    if (this.serie) {
      const s = document.createElement("div");
      s.className = "export-zeile";
      const sName = document.createElement("span");
      sName.textContent = t("Serie: ein PDF je Datensatz");
      sName.title = t("Serienbriefe, Rechnungen: Datensätze aus einem Ordner oder einer .toml, über die typst-CLI");
      const sKnopf = document.createElement("button");
      sKnopf.textContent = t("Serie …");
      sKnopf.onclick = () => {
        this.schliessen();
        this.serie?.();
      };
      s.append(sName, sKnopf);
      this.menue.append(s);
    }
    this.platzieren();
  }

  /** Das Ergebnis steht direkt unter der Zeile, die es ausgelöst hat (ein früheres verschwindet). */
  private ergebnisBei(zeile: HTMLElement): HTMLDivElement {
    this.menue.querySelector(".export-ergebnis")?.remove();
    this.menue.querySelectorAll(".export-zeile-aktiv").forEach((z) => z.classList.remove("export-zeile-aktiv"));
    zeile.classList.add("export-zeile-aktiv");
    const ergebnis = document.createElement("div");
    ergebnis.className = "export-ergebnis";
    ergebnis.setAttribute("role", "status");
    zeile.after(ergebnis);
    return ergebnis;
  }

  /** Während eine Aufgabe läuft, sind alle Knöpfe des Menüs gesperrt (bis auf ×). */
  private sperren(an: boolean) {
    this.menue.querySelectorAll<HTMLButtonElement>(".export-zeile button").forEach((k) => (k.disabled = an));
  }

  private async lauf(zeile: HTMLElement, text: string, aufgabe: (ergebnis: HTMLDivElement) => Promise<void>) {
    const ergebnis = this.ergebnisBei(zeile);
    ergebnis.textContent = text;
    this.sperren(true);
    this.platzieren();
    try {
      await aufgabe(ergebnis);
    } finally {
      this.sperren(false);
      this.platzieren();
    }
  }

  private wordLos(zeile: HTMLElement) {
    return this.lauf(zeile, t("Exportiere nach Word …"), async (ergebnis) => {
      try {
        const e = await this.word.exportieren();
        ergebnis.replaceChildren();
        const kopf = document.createElement("div");
        kopf.textContent = t("Fertig: „{datei}“ im Dokumentordner. Seitenlayout (Ränder, Kopfzeilen, Schrift) kommt nicht mit.", { datei: e.datei });
        const zeigen = document.createElement("button");
        zeigen.textContent = t("Im Ordner zeigen");
        zeigen.onclick = () => void this.word.zeigen(e.pfad).catch((f) => (kopf.textContent = String(f)));
        ergebnis.append(kopf, zeigen);
        if (e.fehlt.length) {
          const d = document.createElement("div");
          d.textContent = t("In Word fehlt: {liste}", { liste: e.fehlt.join(", ") });
          ergebnis.append(d);
        }
      } catch (fehler) {
        ergebnis.textContent = t("Export nach Word fehlgeschlagen: {fehler}", { fehler: String(fehler) });
      }
    });
  }

  private archivLos(zeile: HTMLElement) {
    return this.lauf(zeile, t("Archiviere …"), async (ergebnis) => {
      try {
        const a = await this.archiv!();
        ergebnis.replaceChildren();
        for (const z of archivZeilen(a)) {
          const d = document.createElement("div");
          d.textContent = z;
          ergebnis.append(d);
        }
        const zeigen = document.createElement("button");
        zeigen.textContent = t("Im Ordner zeigen");
        zeigen.onclick = () => void this.word.zeigen(a.pfad).catch((f) => (ergebnis.textContent = String(f)));
        ergebnis.append(zeigen);
        for (const w of a.warnungen) {
          const d = document.createElement("div");
          d.textContent = w;
          ergebnis.append(d);
        }
      } catch (fehler) {
        ergebnis.textContent = t("Archivieren fehlgeschlagen: {fehler}", { fehler: String(fehler) });
      }
    });
  }

  private los(profil: string, abnehmen: boolean, zeile: HTMLElement) {
    return this.lauf(zeile, t("Exportiere …"), async (ergebnis) => {
      try {
        const e = await this.exportieren(profil, abnehmen);
        ergebnis.replaceChildren();
        const kopf = document.createElement("div");
        kopf.textContent = t("Fertig: {datei}", { datei: e.relativ });
        const gp = document.createElement("code");
        gp.textContent = e.gegenprobe;
        gp.title = t("Gegenprobe: dieselbe Datei ohne TFTN");
        ergebnis.append(kopf, gp);
        if (e.sha256) {
          const h = document.createElement("div");
          h.textContent = `SHA-256 ${e.sha256}`;
          ergebnis.append(h);
        }
        for (const w of e.warnungen) {
          const d = document.createElement("div");
          d.textContent = w;
          ergebnis.append(d);
        }
      } catch (fehler) {
        ergebnis.textContent = t("Export fehlgeschlagen: {fehler}", { fehler: String(fehler) });
      }
    });
  }
}
