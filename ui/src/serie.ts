// Serie: ein PDF je Datensatz (Ordner mit .toml-Dateien oder eine .toml mit [[serie]]). Erzeugt wird
// in Rust über die typst-CLI (tftn_export::serie); hier nur Dialog und Rechnung.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import { t } from "./sprache";

export interface SerieJs {
  namen: string[];
  zielordner: string;
  hauptdatei: "bereit" | "anpassbar" | "nein";
  daten: string;
  /** Dateiname des Sammel-PDFs (ohne Endung) */
  gesamt_name: string;
}

export interface StueckJs {
  name: string;
  pdf: string | null;
  fehler: string | null;
  gegenprobe: string;
  seiten: number;
}

export interface SerieErgebnisJs {
  stuecke: StueckJs[];
  gesamt: StueckJs | null;
}

/** Was entsteht: je Datensatz ein PDF, alle in einem (ein Drucklauf), oder beides. */
export type Ausgabe = "einzeln" | "gesamt" | "beides";

export interface SerieWahl {
  ausgabe: Ausgabe;
  beidseitig: boolean;
}

/** Gemerkte Wahl lesen; Unbekanntes fällt auf „einzeln“, einseitig zurück. */
export function wahlLesen(text: string | null): SerieWahl {
  try {
    const w = JSON.parse(text ?? "") as Partial<SerieWahl>;
    const ausgabe: Ausgabe = w.ausgabe === "gesamt" || w.ausgabe === "beides" ? w.ausgabe : "einzeln";
    return { ausgabe, beidseitig: w.beidseitig === true };
  } catch {
    return { ausgabe: "einzeln", beidseitig: false };
  }
}

/**
 * Die Stelle in der Hauptdatei, die für eine Serie `sys.inputs` lesen muss: `toml("<daten>")` genau
 * einmal. Ergebnis als Änderung für den Editor; `null`, wenn es die Stelle nicht (eindeutig) gibt.
 */
export function hauptdateiAnpassung(text: string, daten: string): { von: number; bis: number; neu: string } | null {
  const alt = `toml("${daten}")`;
  const von = text.indexOf(alt);
  if (von < 0 || text.indexOf(alt, von + 1) >= 0) return null;
  return { von, bis: von + alt.length, neu: `toml(sys.inputs.at("daten", default: "${daten}"))` };
}

/** „3 Dokumente: a, b, c“, bei vielen gekürzt. */
export function namenListe(namen: readonly string[], hoechstens = 6): string {
  const gezeigt = namen.slice(0, hoechstens).join(", ");
  const rest = namen.length - hoechstens;
  return rest > 0 ? t("{liste} und {anzahl} weitere", { liste: gezeigt, anzahl: rest }) : gezeigt;
}

/** Letzter Teil eines Pfads (mit / oder \). */
export function dateiname(pfad: string): string {
  return pfad.split(/[\\/]/).pop() ?? pfad;
}

export interface SerieAktionen {
  pruefen(quelle: string): Promise<SerieJs>;
  erzeugen(quelle: string, wahl: SerieWahl): Promise<SerieErgebnisJs>;
  /** Hauptdatei im Editor anpassen und speichern; `false`, wenn es nicht ging. */
  anpassen(daten: string): Promise<boolean>;
  /** `start`: die bisher eingetragene Quelle (leer: Projektordner). */
  dateiWaehlen(start: string): Promise<string | null>;
  ordnerWaehlen(start: string): Promise<string | null>;
  zeigen(pfad: string): Promise<void>;
  /** Schlüssel für die zuletzt benutzte Quelle (je Projekt). */
  merker(): string;
  geschlossen(): void;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class SerieDialog {
  private huelle = el("div", "neu-dialog serie-dialog");
  private quelle = el("input", "serie-quelle");
  private uebersicht = el("div", "serie-uebersicht");
  private los = el("button", "serie-los", t("PDFs erzeugen"));
  private ausgabe = el("select", "serie-ausgabe");
  private beidseitig = el("input");
  private beidseitigZeile = el("label", "serie-beidseitig");
  private ergebnis = el("div", "serie-ergebnis");
  private stand: SerieJs | null = null;
  private laeuft = false;
  private uhr: ReturnType<typeof setTimeout> | undefined;

  constructor(private a: SerieAktionen) {
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    const kopf = dialogKopf(t("Serie"), () => this.schliessen());
    const hilfe = el(
      "p",
      "serie-hilfe",
      t(
        "Ein PDF je Datensatz, etwa für Serienbriefe und Rechnungen. Datensätze: ein Ordner mit .toml-Dateien (je Datei ein Dokument) oder eine .toml mit Abschnitten [[serie]]. Jeder Datensatz enthält nur, was sich ändert; der Rest kommt aus dem Fach „Dokument“. Die PDFs landen bei den Daten.",
      ),
    );
    const zeile = el("div", "serie-zeile");
    this.quelle.placeholder = t("Pfad zu Ordner oder .toml-Datei");
    this.quelle.spellcheck = false;
    this.quelle.addEventListener("input", () => {
      clearTimeout(this.uhr);
      this.uhr = setTimeout(() => void this.pruefen(), 300);
    });
    const datei = el("button", "", t("Datei …"));
    datei.onclick = () => void this.a.dateiWaehlen(this.start()).then((p) => p && this.setzen(p));
    const ordner = el("button", "", t("Ordner …"));
    ordner.onclick = () => void this.a.ordnerWaehlen(this.start()).then((p) => p && this.setzen(p));
    zeile.append(this.quelle, datei, ordner);
    this.los.disabled = true;
    this.los.onclick = () => void this.erzeugen();
    // Ausgabe: einzeln, ein Sammel-PDF (ein Drucklauf) oder beides; beidseitig nur fürs Sammel-PDF.
    for (const [wert, text] of [
      ["einzeln", t("je Datensatz ein PDF")],
      ["gesamt", t("alle in einem PDF (ein Drucklauf)")],
      ["beides", t("beides")],
    ] as const) {
      const o = el("option", "", text);
      o.value = wert;
      this.ausgabe.append(o);
    }
    this.ausgabe.onchange = () => this.wahlGeaendert();
    this.beidseitig.type = "checkbox";
    this.beidseitig.onchange = () => this.wahlGeaendert();
    this.beidseitigZeile.append(this.beidseitig, document.createTextNode(t("beidseitig drucken: jedes Dokument beginnt auf einer Vorderseite")));
    const wahlZeile = el("div", "serie-wahl");
    wahlZeile.append(el("span", "", t("Ausgabe:")), this.ausgabe);
    const knoepfe = el("div", "serie-knoepfe");
    knoepfe.append(this.los);
    this.huelle.append(kopf, hilfe, zeile, this.uebersicht, wahlZeile, this.beidseitigZeile, knoepfe, this.ergebnis);
    document.body.append(this.huelle);
    dialogAnmelden(this, this.huelle);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  /** Während des Laufs: Ein Klick daneben schließt nicht. */
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
    this.ergebnis.replaceChildren();
    let gemerkt = "";
    try {
      gemerkt = localStorage.getItem(this.a.merker()) ?? "";
    } catch {
      // ohne localStorage bleibt das Feld leer
    }
    if (gemerkt && !this.quelle.value) this.quelle.value = gemerkt;
    let wahl: SerieWahl = wahlLesen(null);
    try {
      wahl = wahlLesen(localStorage.getItem(`${this.a.merker()}:wahl`));
    } catch {
      // ohne localStorage die Vorgabe
    }
    this.ausgabe.value = wahl.ausgabe;
    this.beidseitig.checked = wahl.beidseitig;
    this.wahlZeigen();
    void this.pruefen();
    this.quelle.focus();
  }

  private wahl(): SerieWahl {
    return { ausgabe: this.ausgabe.value as Ausgabe, beidseitig: this.beidseitig.checked };
  }

  private wahlZeigen() {
    this.beidseitigZeile.hidden = this.ausgabe.value === "einzeln";
    this.uebersichtZeigen();
  }

  private wahlGeaendert() {
    try {
      localStorage.setItem(`${this.a.merker()}:wahl`, JSON.stringify(this.wahl()));
    } catch {
      // ohne localStorage wird nichts gemerkt
    }
    this.wahlZeigen();
  }

  /** Wo die Auswahl öffnet: bei der Quelle, wenn sie taugt; sonst (leer) im Projektordner. */
  private start(): string {
    return this.stand ? this.quelle.value.trim() : "";
  }

  private setzen(pfad: string) {
    this.quelle.value = pfad;
    void this.pruefen();
  }

  private async pruefen() {
    const quelle = this.quelle.value.trim();
    this.stand = null;
    this.los.disabled = true;
    if (!quelle) {
      this.uebersicht.replaceChildren();
      return;
    }
    try {
      const s = await this.a.pruefen(quelle);
      if (quelle !== this.quelle.value.trim()) return; // inzwischen weitergetippt
      this.stand = s;
      try {
        localStorage.setItem(this.a.merker(), quelle);
      } catch {
        // ohne localStorage wird nichts gemerkt
      }
      this.uebersichtZeigen();
    } catch (fehler) {
      if (quelle !== this.quelle.value.trim()) return;
      // Eine unbrauchbare Quelle nicht beim nächsten Öffnen wieder anbieten.
      try {
        if (localStorage.getItem(this.a.merker()) === quelle) localStorage.removeItem(this.a.merker());
      } catch {
        // ohne localStorage nichts zu vergessen
      }
      this.uebersicht.replaceChildren(el("div", "serie-fehler", String(fehler)));
    }
  }

  private uebersichtZeigen() {
    const s = this.stand;
    if (!s) return;
    const teile: HTMLElement[] = [
      el(
        "div",
        "",
        t("{anzahl} Dokumente: {namen}", { anzahl: s.namen.length, namen: namenListe(s.namen) }),
      ),
      el(
        "div",
        "serie-ziel",
        this.ausgabe.value === "einzeln"
          ? t("PDFs nach {ordner} (gleichnamige werden ersetzt)", { ordner: s.zielordner })
          : this.ausgabe.value === "gesamt"
            ? t("Ein PDF {datei} nach {ordner} (wird ersetzt)", { datei: `${s.gesamt_name}.pdf`, ordner: s.zielordner })
            : t("PDFs und {datei} nach {ordner} (gleichnamige werden ersetzt)", { datei: `${s.gesamt_name}.pdf`, ordner: s.zielordner }),
      ),
    ];
    if (s.hauptdatei === "anpassbar") {
      const box = el("div", "serie-hinweis");
      box.append(
        el(
          "span",
          "",
          t("Die Hauptdatei liest ihre Daten noch fest aus {daten}. Für eine Serie muss sie sie über sys.inputs lesen – eine kleine Änderung in einer Zeile.", { daten: s.daten }),
        ),
      );
      const k = el("button", "", t("Hauptdatei anpassen"));
      k.onclick = async () => {
        k.disabled = true;
        if (await this.a.anpassen(s.daten)) await this.pruefen();
        else k.disabled = false;
      };
      box.append(k);
      teile.push(box);
    } else if (s.hauptdatei === "nein") {
      teile.push(
        el(
          "div",
          "serie-fehler",
          t("Die Hauptdatei liest keine Formulardaten ({daten}); dieser Dokumenttyp eignet sich nicht für eine Serie.", { daten: s.daten }),
        ),
      );
    }
    this.uebersicht.replaceChildren(...teile);
    this.los.disabled = s.hauptdatei !== "bereit" || s.namen.length === 0;
  }

  /** Fortschritt (Ereignis `serie-fortschritt`). */
  fortschritt(fertig: number, gesamt: number, name: string) {
    if (this.laeuft) this.ergebnis.textContent = t("{fertig} von {gesamt}: {name}", { fertig, gesamt, name });
  }

  private async erzeugen() {
    const quelle = this.quelle.value.trim();
    if (!quelle || this.laeuft) return;
    this.laeuft = true;
    this.los.disabled = true;
    this.ergebnis.textContent = t("Erzeuge …");
    try {
      const wahl = this.wahl();
      const e = await this.a.erzeugen(quelle, wahl);
      const inhalt = ergebnisInhalt(e, wahl, this.stand?.zielordner ?? "");
      this.ergebnis.replaceChildren(...ergebnisTeile(inhalt, (p) => this.a.zeigen(p)));
    } catch (fehler) {
      this.ergebnis.replaceChildren(el("div", "serie-fehler", String(fehler)));
    } finally {
      this.laeuft = false;
      this.los.disabled = this.stand?.hauptdatei !== "bereit";
    }
  }
}

/** Was das Ergebnis zeigt (ohne DOM, damit testbar). */
export interface ErgebnisInhalt {
  kopf: string;
  /** „in <ordner>:“ und die Dateinamen der Einzelnen, wenn sie bei den Daten liegen */
  ordner: string | null;
  dateien: string | null;
  /** „Sammel-PDF x.pdf: n Seiten“ */
  gesamt: string | null;
  gesamtFehler: string | null;
  /** welche Datei „Im Ordner zeigen“ markiert */
  zeigen: string | null;
  zeilen: string[];
  fehler: { name: string; text: string }[];
}

export function ergebnisInhalt(e: SerieErgebnisJs, wahl: SerieWahl, zielordner: string): ErgebnisInhalt {
  const gut = e.stuecke.filter((s) => s.pdf);
  const einzeln = wahl.ausgabe !== "gesamt";
  const g = e.gesamt;
  return {
    kopf: einzeln
      ? t("{gut} von {gesamt} PDFs erzeugt.", { gut: gut.length, gesamt: e.stuecke.length })
      : t("{gut} von {gesamt} Dokumenten erzeugt.", { gut: gut.length, gesamt: e.stuecke.length }),
    ordner: einzeln && gut.length ? t("in {ordner}:", { ordner: zielordner }) : null,
    dateien: einzeln && gut.length ? namenListe(gut.map((s) => dateiname(s.pdf!)), 12) : null,
    gesamt: g?.pdf ? t("Sammel-PDF {datei}: {seiten} Seiten", { datei: dateiname(g.pdf), seiten: g.seiten }) : null,
    gesamtFehler: g && !g.pdf ? (g.fehler ?? "") : null,
    zeigen: g?.pdf ?? (einzeln ? (gut[0]?.pdf ?? null) : null),
    // Auch bei „nur gesamt“ alle Zeilen: Das Sammel-PDF setzt die Einzelnen aus _lokal/serie/pdf zusammen.
    zeilen: [...gut.map((s) => s.gegenprobe), ...(g?.pdf ? [g.gegenprobe] : [])],
    fehler: e.stuecke.filter((s) => !s.pdf).map((s) => ({ name: s.name, text: s.fehler ?? "" })),
  };
}

/** Die Ergebnisanzeige: Einzelne (wenn verlangt), Sammel-PDF, Fehler je Datensatz. */
export function ergebnisTeile(i: ErgebnisInhalt, zeigen: (pfad: string) => Promise<void>): HTMLElement[] {
  const kopf = el("div", "serie-fertig", i.kopf);
  const teile: HTMLElement[] = [kopf];
  if (i.ordner && i.dateien) teile.push(el("div", "serie-ziel", i.ordner), el("div", "serie-dateien", i.dateien));
  if (i.gesamt) teile.push(el("div", "serie-dateien", i.gesamt));
  if (i.gesamtFehler !== null) {
    const f = el("div", "serie-fehler");
    f.append(el("strong", "", t("Sammel-PDF: ")), document.createTextNode(i.gesamtFehler));
    teile.push(f);
  }
  const ziel = i.zeigen;
  if (ziel) {
    const z = el("button", "", t("Im Ordner zeigen"));
    z.onclick = () => void zeigen(ziel).catch((f) => (kopf.textContent = String(f)));
    teile.push(z);
  }
  if (i.zeilen.length) {
    const gp = el("details", "serie-gegenprobe");
    gp.append(el("summary", "", t("Gegenprobe: dieselben PDFs ohne TFTN")), el("code", "", i.zeilen.join("\n")));
    teile.push(gp);
  }
  for (const f of i.fehler) {
    const d = el("div", "serie-fehler");
    d.append(el("strong", "", `${f.name}: `), document.createTextNode(f.text));
    teile.push(d);
  }
  return teile;
}
