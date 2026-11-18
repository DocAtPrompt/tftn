// Inkognito für KI-Assistenten (MCP): Felder mit Personendaten gehen nur als Ersatztext hinaus,
// etwa „[Patientin]“ oder „[geb. 1970]“. Angekreuzt wird im Fach „Dokument“ (gespeichert in
// tftn.toml, nur Feldname und Ersatztext); was die KI sieht, zeigt eine Tabelle, die auf dem Rechner
// bleibt. Ersetzt und zurückgesetzt wird in Rust.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import type { Pfad } from "./formular";
import { t, zahl } from "./sprache";

/** Stand eines Felds (aus `FormularJs.inkognito`). */
export interface InkognitoFeld {
  an: boolean;
  /** eigener Ersatztext des Projekts */
  eigen: string | null;
  /** Ersatz der Vorlage in Worten */
  vorgabe: string;
  /** sensibel laut Vorlage (von Haus aus an) */
  sensibel: boolean;
}

/** Stand für die Statusleiste (Befehl `inkognito_stand`). */
export interface InkognitoJs {
  an: boolean;
  felder: string[];
  werte: number;
  fehler: string | null;
}

/** Schlüssel eines Felds in tftn.toml: Pfad ohne Eintragsnummern (`autoren.0.name` → `autoren.name`). */
export function inkognitoSchluessel(pfad: Pfad): string {
  return pfad.filter((s) => typeof s === "string").join(".");
}

/** Eigener Ersatztext aus dem Eingabefeld; `null`, wenn leer oder gleich der Vorgabe (dann gilt sie). */
export function eigenerErsatz(eingabe: string, vorgabe: string): string | null {
  const e = eingabe.replace(/[[\]]/g, "").replace(/\s+/g, " ").trim();
  return e === "" || e === vorgabe ? null : e;
}

/** Zusatz und Tooltip für das MCP-Abzeichen; leer, wenn nichts inkognito ist. */
export function inkognitoAnzeige(s: InkognitoJs | null): { zusatz: string; titel: string } {
  if (!s?.an) return { zusatz: "", titel: "" };
  if (s.fehler) return { zusatz: t("inkognito – gesperrt"), titel: s.fehler };
  return {
    zusatz: t("inkognito"),
    titel: t("Inkognito: {felder} gehen nur als Ersatztext an KI-Assistenten ({anzahl} Werte).", {
      felder: s.felder.join(", "),
      anzahl: zahl(s.werte),
    }),
  };
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface InkognitoAktionen {
  /** Ersatztext → echter Wert */
  tabelle(): Promise<[string, string][]>;
}

/** „Was die KI sieht“: Ersatztexte und echte Werte, nur zum Nachschlagen auf diesem Rechner. */
export class InkognitoDialog {
  private huelle = el("div", "neu-dialog inkognito-dialog");
  private liste = el("div", "netz-liste");

  constructor(
    private a: InkognitoAktionen,
    ausloeser: (Element | null)[] = [],
  ) {
    this.huelle.id = "inkognito-dialog";
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    this.huelle.setAttribute("aria-label", t("Was die KI sieht"));
    const hilfe = el(
      "p",
      "netz-hilfe",
      t(
        "So erscheinen die Felder, die inkognito sind, für KI-Assistenten über MCP. Schreibt die KI einen Ersatztext zurück, setzt TFTN den echten Wert ein. Diese Tabelle bleibt auf dem Rechner.",
      ),
    );
    this.huelle.append(dialogKopf(t("Was die KI sieht"), () => this.schliessen()), hilfe, this.liste);
    document.body.append(this.huelle);
    dialogAnmelden(this, this.huelle, ...ausloeser);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  schliessen() {
    this.huelle.hidden = true;
  }

  async zeigen() {
    this.huelle.hidden = false;
    try {
      const paare = await this.a.tabelle();
      if (paare.length === 0) {
        this.liste.replaceChildren(el("p", "leiste-leer", t("Die Felder, die inkognito sind, haben noch keine Werte.")));
        return;
      }
      const tabelle = el("table", "netz-tabelle");
      const kopf = el("tr");
      kopf.append(el("th", "", t("Die KI sieht")), el("th", "", t("Echter Wert")));
      tabelle.append(el("thead"), el("tbody"));
      tabelle.tHead!.append(kopf);
      for (const [ersatz, echt] of paare) {
        const z = el("tr");
        z.append(el("td", "inkognito-ersatz", ersatz), el("td", "", echt));
        tabelle.tBodies[0].append(z);
      }
      this.liste.replaceChildren(tabelle);
    } catch (fehler) {
      this.liste.replaceChildren(el("p", "dok-warnung", String(fehler)));
    }
  }
}
