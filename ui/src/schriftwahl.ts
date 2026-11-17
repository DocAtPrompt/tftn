// Dialog „Weitere Schrift“: eine installierte Schrift oder eine Schriftdatei fürs Dokument. Die
// Familie wird nach fonts/ kopiert; fehlende Zeichen der Dokumentsprache stehen gleich dabei.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import { t } from "./sprache";

export interface FamilieJs {
  name: string;
  schnitte: number;
  /** Byte */
  groesse: number;
  /** Zeichen, die der Schrift für die Dokumentsprache fehlen */
  fehlend: string;
  /** Das Dokument kennt die Familie schon. */
  vorhanden: boolean;
  /** Schriftdatei; null: installiert */
  datei: string | null;
}

export interface EingebundenJs {
  dateien: string[];
  groesse: number;
}

export interface SchriftwahlAktionen {
  suchen(neu: boolean): Promise<FamilieJs[]>;
  /** Systemdialog (Mac); null bei Abbruch */
  dateiWaehlen(): Promise<string | null>;
  dateiLesen(pfad: string): Promise<FamilieJs[]>;
  hinzufuegen(f: FamilieJs): Promise<EingebundenJs>;
  /** Nach dem Einbinden: Schrift im Dokument setzen. */
  gewaehlt(name: string, eingebunden: EingebundenJs): Promise<void>;
  geschlossen(): void;
  /** "dialog": Systemdialog für Dateien; "pfad": Pfadfeld (Windows). */
  dateiArt: "dialog" | "pfad";
}

/** Höchstens so viele Zeilen; der Rest über die Suche. */
export const ZEILEN_HOECHSTENS = 150;

/** Familien, deren Name alle Wörter der Suche enthält (ohne Groß/klein); mit `vollstaendig` nur
 *  solche ohne fehlende Zeichen (Familien aus einer gewählten Datei bleiben immer). */
export function filtern(liste: FamilieJs[], suche: string, vollstaendig = false): FamilieJs[] {
  const woerter = suche.toLowerCase().split(/\s+/).filter(Boolean);
  return liste.filter(
    (f) => woerter.every((w) => f.name.toLowerCase().includes(w)) && (!vollstaendig || !f.fehlend || f.datei !== null),
  );
}

/** „4 Schnitte · 1,2 MB“ */
export function umfang(f: FamilieJs, sprache = "de"): string {
  const mb = f.groesse / 1_000_000;
  const groesse =
    mb >= 0.1
      ? `${mb.toLocaleString(sprache, { maximumFractionDigits: 1 })} MB`
      : `${Math.max(1, Math.round(f.groesse / 1000)).toLocaleString(sprache)} kB`;
  const schnitte = f.schnitte === 1 ? t("1 Schnitt") : t("{n} Schnitte", { n: f.schnitte });
  return `${schnitte} · ${groesse}`;
}

/** Warnung zu fehlenden Zeichen, leer ohne. Lange Listen werden gekürzt. */
export function fehlendText(f: FamilieJs): string {
  const z = [...f.fehlend];
  if (z.length === 0) return "";
  const gezeigt = z.length > 16 ? `${z.slice(0, 16).join(" ")} …` : z.join(" ");
  return t("Es fehlen: {zeichen}", { zeichen: gezeigt });
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class SchriftwahlDialog {
  private huelle = el("div", "neu-dialog schriftwahl-dialog");
  private suche = el("input", "schriftwahl-suche");
  private liste = el("div", "schriftwahl-liste");
  private meldung = el("p", "schriftwahl-meldung");
  private nehmen = el("button", "", t("Schrift übernehmen"));
  private vollstaendig = el("input");
  private familien: FamilieJs[] = [];
  private gewaehlt: FamilieJs | null = null;
  private laeuft = false;

  constructor(private a: SchriftwahlAktionen) {
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    const kopf = dialogKopf(t("Weitere Schrift"), () => this.schliessen());
    const hilfe = el(
      "p",
      "schriftwahl-hilfe",
      t(
        "Die gewählte Schrift wird samt Fett und Kursiv in den Ordner fonts/ des Dokuments kopiert. So setzen Vorschau, Export und jeder andere Rechner mit derselben Schrift. Schriften, die mit dem System kamen (etwa von Apple oder Microsoft), nur für eigene Dokumente verwenden und das Dokument samt fonts/ nicht weitergeben.",
      ),
    );
    const zeile = el("div", "schriftwahl-zeile");
    this.suche.type = "search";
    this.suche.placeholder = t("Schrift suchen");
    this.suche.spellcheck = false;
    this.suche.addEventListener("input", () => this.zeigenListe());
    const neu = el("button", "", t("Neu suchen"));
    neu.title = t("Die installierten Schriften noch einmal lesen, etwa nach dem Installieren einer Schrift");
    neu.onclick = () => void this.laden(true);
    zeile.append(this.suche, neu);
    const voll = el("label", "schriftwahl-voll");
    this.vollstaendig.type = "checkbox";
    this.vollstaendig.checked = true;
    this.vollstaendig.onchange = () => this.zeigenListe();
    voll.append(this.vollstaendig, document.createTextNode(t("Nur Schriften mit allen Zeichen der Dokumentsprache")));
    const dateiZeile = el("div", "schriftwahl-zeile");
    if (a.dateiArt === "dialog") {
      const datei = el("button", "", t("Schriftdatei …"));
      datei.onclick = () =>
        void a.dateiWaehlen().then((p) => {
          if (p) void this.dateiLaden(p);
        });
      dateiZeile.append(datei);
    } else {
      const pfad = el("input", "schriftwahl-suche");
      pfad.placeholder = t("Pfad zu einer Schriftdatei (.otf, .ttf)");
      pfad.spellcheck = false;
      const lesen = el("button", "", t("Lesen"));
      lesen.onclick = () => {
        if (pfad.value.trim()) void this.dateiLaden(pfad.value.trim());
      };
      dateiZeile.append(pfad, lesen);
    }
    this.nehmen.disabled = true;
    this.nehmen.onclick = () => void this.uebernehmen();
    const knoepfe = el("div", "schriftwahl-knoepfe");
    knoepfe.append(this.nehmen);
    this.huelle.append(kopf, hilfe, zeile, voll, dateiZeile, this.liste, this.meldung, knoepfe);
    document.body.append(this.huelle);
    dialogAnmelden(this, this.huelle);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  behalten(): boolean {
    return this.laeuft;
  }

  schliessen() {
    if (this.laeuft) return;
    this.huelle.hidden = true;
    this.a.geschlossen();
  }

  zeigen() {
    this.huelle.hidden = false;
    this.meldung.textContent = "";
    this.gewaehlt = null;
    this.nehmen.disabled = true;
    this.suche.focus();
    void this.laden(false);
  }

  private async laden(neu: boolean) {
    this.liste.replaceChildren(el("p", "schriftwahl-leise", t("Suche die Schriften auf diesem Rechner …")));
    try {
      this.familien = await this.a.suchen(neu);
      this.zeigenListe();
    } catch (f) {
      this.liste.replaceChildren();
      this.meldung.textContent = String(f);
    }
  }

  private async dateiLaden(pfad: string) {
    this.meldung.textContent = "";
    try {
      const aus = await this.a.dateiLesen(pfad);
      // Familien aus der Datei oben, die installierten darunter.
      this.familien = [...aus, ...this.familien.filter((f) => f.datei === null)];
      this.suche.value = "";
      this.zeigenListe();
      this.waehlen(aus[0]);
    } catch (f) {
      this.meldung.textContent = String(f);
    }
  }

  private zeigenListe() {
    const treffer = filtern(this.familien, this.suche.value, this.vollstaendig.checked);
    const ausgeblendet = filtern(this.familien, this.suche.value).length - treffer.length;
    const zeilen = treffer.slice(0, ZEILEN_HOECHSTENS).map((f) => this.zeile(f));
    if (treffer.length > ZEILEN_HOECHSTENS)
      zeilen.push(
        el("p", "schriftwahl-leise", t("… und {n} weitere; die Suche grenzt ein.", { n: treffer.length - ZEILEN_HOECHSTENS })),
      );
    if (treffer.length === 0) zeilen.push(el("p", "schriftwahl-leise", t("Keine Schrift gefunden.")));
    if (ausgeblendet > 0)
      zeilen.push(el("p", "schriftwahl-leise", t("{n} Schriften mit fehlenden Zeichen ausgeblendet.", { n: ausgeblendet })));
    this.liste.replaceChildren(...zeilen);
  }

  private zeile(f: FamilieJs): HTMLElement {
    const z = el("button", "schriftwahl-eintrag");
    z.type = "button";
    if (this.gewaehlt === f) z.classList.add("gewaehlt");
    const name = el("span", "schriftwahl-name", f.name);
    const probe = el("span", "schriftwahl-probe", "Straße, Äpfel – „Grüße“ 0123 €");
    // Installierte Schriften kennt auch die Oberfläche; eine Datei nicht (dann die Ersatzschrift).
    if (f.datei === null) probe.style.fontFamily = `"${f.name.replace(/"/g, "")}", system-ui`;
    const info = el("span", "schriftwahl-info", f.vorhanden ? `${umfang(f)} · ${t("schon im Dokument")}` : umfang(f));
    z.append(name, probe, info);
    const fehlt = fehlendText(f);
    if (fehlt) z.append(el("span", "schriftwahl-fehlt", fehlt));
    if (f.datei) z.append(el("span", "schriftwahl-info", f.datei));
    z.onclick = () => this.waehlen(f);
    z.ondblclick = () => void this.uebernehmen();
    return z;
  }

  private waehlen(f: FamilieJs | undefined) {
    if (!f) return;
    this.gewaehlt = f;
    this.nehmen.disabled = false;
    this.meldung.textContent = f.fehlend
      ? t("Achtung: {fehlt}. Diese Zeichen kämen aus einer anderen Schrift oder als leeres Kästchen.", { fehlt: fehlendText(f) })
      : "";
    for (const z of this.liste.querySelectorAll(".schriftwahl-eintrag.gewaehlt")) z.classList.remove("gewaehlt");
    this.zeigenListe();
  }

  private async uebernehmen() {
    const f = this.gewaehlt;
    if (!f || this.laeuft) return;
    this.laeuft = true;
    this.nehmen.disabled = true;
    this.meldung.textContent = t("Kopiere „{name}“ ins Dokument …", { name: f.name });
    try {
      const e = await this.a.hinzufuegen(f);
      await this.a.gewaehlt(f.name, e);
      this.laeuft = false;
      this.schliessen();
    } catch (fehler) {
      this.meldung.textContent = String(fehler);
      this.laeuft = false;
      this.nehmen.disabled = false;
    }
  }
}
