// Export-Menü (PLAN-M1 AP8): Profile aus dem Manifest, „Probe“ nach _renders/, „Abnehmen“ nach
// export/ (setzt den Zeitstempel; zweiter Klick bestätigt). Vor dem Export wird gespeichert.

import { dialogAnmelden } from "./dialoge";
import { t } from "./sprache";

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

export interface WordAktionen {
  exportieren(): Promise<WordJs>;
  zeigen(pfad: string): Promise<void>;
}

export class ExportMenue {
  private menue: HTMLDivElement;

  constructor(
    knopf: HTMLElement,
    private profile: () => Promise<ProfilJs[]>,
    private exportieren: (profil: string, abnehmen: boolean) => Promise<ExportJs>,
    private word: WordAktionen,
  ) {
    this.menue = document.createElement("div");
    this.menue.id = "export-menue";
    this.menue.hidden = true;
    document.body.append(this.menue);
    knopf.addEventListener("click", () => (this.menue.hidden ? void this.oeffnen() : this.schliessen()));
    this.menue.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.schliessen();
      }
    });
    dialogAnmelden(this);
  }

  get offen(): boolean {
    return !this.menue.hidden;
  }

  schliessen() {
    this.menue.hidden = true;
  }

  private async oeffnen() {
    this.menue.hidden = false;
    this.menue.textContent = t("Lade Export-Profile …");
    let liste: ProfilJs[];
    try {
      liste = await this.profile();
    } catch (fehler) {
      this.menue.textContent = String(fehler);
      return;
    }
    this.menue.replaceChildren();
    const w = document.createElement("div");
    w.className = "export-zeile";
    const wName = document.createElement("span");
    wName.textContent = t("Word (.docx) zum Gegenlesen");
    wName.title = t(
      "Text, Überschriften, Fußnoten, Tabellen, Bilder, Formeln, Zitate und Literatur; ohne Seitenlayout. Über Pandoc.",
    );
    const wKnopf = document.createElement("button");
    wKnopf.textContent = t("Exportieren");
    wKnopf.onclick = () => void this.wordLos();
    w.append(wName, wKnopf);
    this.menue.append(w);
    for (const p of liste) {
      const zeile = document.createElement("div");
      zeile.className = "export-zeile";
      const name = document.createElement("span");
      name.textContent = `${p.id} · ${p.art}${p.standards.length ? ` · ${p.standards.join(", ")}` : ""}`;
      const probe = document.createElement("button");
      probe.textContent = t("Probe");
      probe.title = t("nach _renders/ (nicht synchronisiert)");
      probe.onclick = () => void this.los(p.id, false);
      const abnehmen = document.createElement("button");
      abnehmen.textContent = t("Abnehmen …");
      abnehmen.title = t("nach export/, Zeitstempel wird auf jetzt gesetzt");
      abnehmen.onclick = () => {
        if (abnehmen.dataset.sicher) void this.los(p.id, true);
        else {
          abnehmen.dataset.sicher = "1";
          abnehmen.textContent = t("Wirklich abnehmen?");
        }
      };
      zeile.append(name, probe, abnehmen);
      this.menue.append(zeile);
    }
    const ergebnis = document.createElement("div");
    ergebnis.className = "export-ergebnis";
    this.menue.append(ergebnis);
  }

  private async wordLos() {
    const ergebnis = this.menue.querySelector(".export-ergebnis")!;
    ergebnis.textContent = t("Exportiere nach Word …");
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
  }

  private async los(profil: string, abnehmen: boolean) {
    const ergebnis = this.menue.querySelector(".export-ergebnis")!;
    ergebnis.textContent = t("Exportiere …");
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
  }
}
