// „Über TFTN“: Version, Lizenzen, feste Links und die geltenden Tastenkürzel zum Nachschlagen.
// Ein Link öffnet nur auf Klick den Standardbrowser; die Adresse kennt allein Rust, die Oberfläche
// schickt nur den Namen des Links. „Ganz offline“ sperrt die Links, die Adressen bleiben lesbar.

import { kuerzelText } from "./bausteine";
import { dialogAnmelden, dialogKopf } from "./dialoge";
import type { Art } from "./rundgang";
import { t } from "./sprache";
import type { Plattform, Zeile } from "./tasten";

export interface LinkJs {
  name: string;
  titel: string;
  adresse: string;
}

export interface UeberJs {
  version: string;
  typst_version: string;
  links: LinkJs[];
  /** Es gibt ein Ko-fi-Konto (dann zeigen die Einstellungen den Schalter). */
  kofi_vorhanden: boolean;
  kofi_ausblenden: boolean;
  ganz_offline: boolean;
}

export interface KuerzelGruppe {
  gruppe: string;
  eintraege: { titel: string; kuerzel: string }[];
}

/** Kürzel des Editors, die man nicht umbelegen kann, aber kennen sollte. */
export function editorKuerzel(p: Plattform): { titel: string; kuerzel: string }[] {
  const mac = p === "mac";
  return [
    { titel: t("Suchen und Ersetzen"), kuerzel: kuerzelText("Mod-f", mac) },
    { titel: t("Rückgängig"), kuerzel: kuerzelText("Mod-z", mac) },
    { titel: t("Wiederholen"), kuerzel: kuerzelText(mac ? "Mod-Shift-z" : "Mod-y", mac) },
    { titel: t("Vergrößern und verkleinern (Vorschau)"), kuerzel: mac ? t("⌘ + Mausrad") : t("Strg + Mausrad") },
  ];
}

/**
 * Die geltenden Kürzel, nach Gruppen in der Reihenfolge ihres ersten Auftretens; Aktionen ohne
 * Kürzel fehlen, ein überdecktes Kürzel ebenso (es wirkt nicht). Am Ende die Editor-Kürzel.
 */
export function kuerzelUebersicht(zeilen: readonly Zeile[], p: Plattform): KuerzelGruppe[] {
  const gruppen = new Map<string, KuerzelGruppe>();
  for (const z of zeilen) {
    const k = z.kuerzel[0];
    if (!k || z.ueberdecktVon) continue;
    let g = gruppen.get(z.aktion.gruppe);
    if (!g) gruppen.set(z.aktion.gruppe, (g = { gruppe: z.aktion.gruppe, eintraege: [] }));
    g.eintraege.push({ titel: z.aktion.titel, kuerzel: kuerzelText(k, p === "mac") });
  }
  return [...gruppen.values(), { gruppe: t("Editor"), eintraege: editorKuerzel(p) }];
}

/** Erste Zeile des Fensters. */
export function versionZeile(u: UeberJs): string {
  return t("Version {version} · typst {typst}", { version: u.version, typst: u.typst_version });
}

/** Was mitgeliefert wird und unter welcher Lizenz. */
export function beilagen(): string[] {
  return [
    t("typst (Satz und Kommandozeile): Apache-2.0"),
    t("Tauri und die Rust-Bibliotheken: MIT bzw. Apache-2.0"),
    t("CodeMirror (Editor): MIT"),
    t("Schriften der Dokumenttypen (Libertinus, Source Sans, DejaVu u. a.): freie Schriftlizenzen, vor allem die SIL Open Font License 1.1; die Texte liegen als LIZENZEN.txt beim Dokumenttyp"),
    t("Wörterbücher der Rechtschreibprüfung: je Sprache eine eigene freie Lizenz (etwa MPL, LGPL/GPL, MIT/BSD), als Datei „license“ beim Wörterbuch"),
  ];
}

export interface UeberAktionen {
  lesen(): Promise<UeberJs>;
  /** Öffnet den Link `name` im Standardbrowser (Rust kennt die Adresse). */
  oeffnen(name: string): Promise<void>;
  lizenzenZeigen(): Promise<void>;
  /** Die geltende Belegung (für die Kürzel-Übersicht). */
  zeilen(): readonly Zeile[];
  plattform: Plattform;
  einstellungen(): void;
  netzprotokoll(): void;
  /** Die Rundgänge (rundgaenge.ts); `kopie`: läuft in einer Kopie mit erfundenen Daten. */
  rundgaenge(): RundgangEintrag[];
  rundgang(id: string, art: Art): void;
}

export interface RundgangEintrag {
  id: string;
  titel: string;
  beschreibung: string;
  kopie: boolean;
}

/** Was unter einem Rundgang steht: seine Beschreibung, bei einer Kopie mit dem Hinweis darauf. */
export function rundgangZusatz(r: RundgangEintrag): string {
  return r.kopie ? t("{beschreibung} Läuft in einer Kopie mit erfundenen Daten.", { beschreibung: r.beschreibung }) : r.beschreibung;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class UeberDialog {
  private huelle = el("div", "neu-dialog ueber-dialog");
  private version = el("p", "ueber-version");
  private links = el("ul", "ueber-links");
  private offline = el("p", "netz-hilfe");
  private kuerzel = el("div", "ueber-kuerzel");
  private rundgaenge = el("ul", "ueber-rundgaenge");
  private meldung = el("p", "netz-meldung leiste-hilfe");

  constructor(
    private a: UeberAktionen,
    ausloeser: (Element | null)[] = [],
  ) {
    this.huelle.id = "ueber";
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    this.huelle.setAttribute("aria-label", t("Über TFTN"));
    const kopf = el("div", "ueber-kopf");
    kopf.append(
      el("p", "ueber-name", "TFTN"),
      el("p", "netz-hilfe", t("Offline schreiben mit Typst: ohne Konto, ohne Cloud, ohne Nutzungsstatistik.")),
      this.version,
    );
    const lizenz = el("section", "ueber-teil");
    const lizenzKnopf = el("button", "", t("Lizenztexte zeigen"));
    lizenzKnopf.title = t("Zeigt den Ordner mit den Lizenztexten von TFTN und typst im Finder bzw. Explorer");
    lizenzKnopf.addEventListener("click", () => void this.mitMeldung(() => this.a.lizenzenZeigen()));
    const liste = el("ul", "ueber-beilagen");
    for (const b of beilagen()) liste.append(el("li", "", b));
    lizenz.append(
      el("h3", "", t("Lizenz")),
      el("p", "", t("TFTN ist freie Software, nach Wahl unter der MIT- oder der Apache-2.0-Lizenz. Mitgeliefert werden:")),
      liste,
      lizenzKnopf,
    );
    const touren = el("section", "ueber-teil");
    touren.append(
      el("h3", "", t("Rundgänge")),
      el(
        "p",
        "netz-hilfe",
        t("„Ansehen“ führt Schritt für Schritt durch die Oberfläche. „Vorführen“ läuft von selbst ab, ohne Knöpfe im Bild – zum Aufnehmen eines Videos. Esc beendet beide."),
      ),
      this.rundgaenge,
    );
    this.rundgaenge.addEventListener("click", (e) => {
      const knopf = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-rundgang]");
      if (!knopf) return;
      this.schliessen();
      this.a.rundgang(knopf.dataset.rundgang!, knopf.dataset.art as Art);
    });
    const links = el("section", "ueber-teil");
    links.append(el("h3", "", t("Links")), this.offline, this.links);
    const kuerzel = el("section", "ueber-teil");
    const aendern = el("button", "", t("Kürzel ändern …"));
    aendern.addEventListener("click", () => {
      this.schliessen();
      this.a.einstellungen();
    });
    const netz = el("button", "", t("Netzprotokoll …"));
    netz.addEventListener("click", () => this.a.netzprotokoll());
    const knoepfe = el("div", "neu-ordnerzeile netz-unten");
    knoepfe.append(aendern, netz);
    kuerzel.append(el("h3", "", t("Tastenkürzel")), this.kuerzel, knoepfe);
    this.huelle.append(dialogKopf(t("Über TFTN"), () => this.schliessen()), kopf, touren, links, kuerzel, lizenz, this.meldung);
    this.links.addEventListener("click", (e) => {
      const knopf = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-link]");
      if (knopf) void this.mitMeldung(() => this.a.oeffnen(knopf.dataset.link!));
    });
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
    this.meldung.textContent = "";
    this.huelle.hidden = false;
    this.kuerzelZeichnen();
    this.rundgaengeZeichnen();
    try {
      this.zeichnen(await this.a.lesen());
    } catch (fehler) {
      this.meldung.textContent = String(fehler);
    }
  }

  private async mitMeldung(f: () => Promise<void>) {
    this.meldung.textContent = "";
    try {
      await f();
    } catch (fehler) {
      this.meldung.textContent = String(fehler);
    }
  }

  private zeichnen(u: UeberJs) {
    this.version.textContent = versionZeile(u);
    this.offline.textContent = u.ganz_offline
      ? t("„Ganz offline“ ist eingeschaltet: Die Links öffnen sich nicht, die Adressen lassen sich abschreiben.")
      : t("Ein Klick öffnet die Seite im Standardbrowser; TFTN selbst geht dabei nicht ins Netz. Jeder Klick steht im Netzprotokoll.");
    this.links.replaceChildren(
      ...u.links.map((l) => {
        const li = el("li", l.name === "kofi" ? "ueber-kofi" : "");
        const b = el("button", "ueber-link", l.titel);
        b.dataset.link = l.name;
        b.disabled = u.ganz_offline;
        li.append(b, el("span", "ueber-adresse", l.adresse));
        return li;
      }),
    );
  }

  private rundgaengeZeichnen() {
    this.rundgaenge.replaceChildren(
      ...this.a.rundgaenge().map((r) => {
        const li = el("li");
        const text = el("div", "ueber-rundgang-text");
        text.append(el("strong", "", r.titel), el("span", "ueber-adresse", rundgangZusatz(r)));
        const knoepfe = el("span", "ueber-rundgang-knoepfe");
        const knopf = (art: Art, titel: string, hilfe: string) => {
          const b = el("button", "", titel);
          b.dataset.rundgang = r.id;
          b.dataset.art = art;
          b.title = hilfe;
          return b;
        };
        knoepfe.append(
          knopf("ansehen", t("Ansehen"), t("Schritt für Schritt: Weiter, Zurück, Esc beendet")),
          knopf("vorfuehren", t("Vorführen"), t("Läuft von selbst ab, ohne Knöpfe im Bild (zum Aufnehmen, am Mac Cmd+Umschalt+5). Klick oder Leertaste hält an, Esc bricht ab.")),
        );
        li.append(text, knoepfe);
        return li;
      }),
    );
  }

  private kuerzelZeichnen() {
    const tabelle = el("table", "netz-tabelle");
    const rumpf = el("tbody");
    for (const g of kuerzelUebersicht(this.a.zeilen(), this.a.plattform)) {
      const kopf = el("tr", "ueber-gruppe");
      const th = el("th", "", g.gruppe);
      th.colSpan = 2;
      kopf.append(th);
      rumpf.append(kopf);
      for (const e of g.eintraege) {
        const tr = el("tr");
        tr.append(el("td", "", e.titel), el("td", "ueber-taste", e.kuerzel));
        rumpf.append(tr);
      }
    }
    tabelle.append(rumpf);
    this.kuerzel.replaceChildren(tabelle);
  }
}
