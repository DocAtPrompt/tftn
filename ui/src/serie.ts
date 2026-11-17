// Serie: ein PDF je Datensatz (Ordner mit .toml-Dateien oder eine .toml mit [[serie]]). Erzeugt wird
// in Rust über die typst-CLI (tftn_export::serie); hier nur Dialog und Rechnung.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import { t } from "./sprache";

export interface SerieJs {
  namen: string[];
  zielordner: string;
  hauptdatei: "bereit" | "anpassbar" | "nein";
  daten: string;
}

export interface StueckJs {
  name: string;
  pdf: string | null;
  fehler: string | null;
  gegenprobe: string;
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
  erzeugen(quelle: string): Promise<StueckJs[]>;
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
    const knoepfe = el("div", "serie-knoepfe");
    knoepfe.append(this.los);
    this.huelle.append(kopf, hilfe, zeile, this.uebersicht, knoepfe, this.ergebnis);
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
    void this.pruefen();
    this.quelle.focus();
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
      el("div", "serie-ziel", t("PDFs nach {ordner} (gleichnamige werden ersetzt)", { ordner: s.zielordner })),
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
      const stuecke = await this.a.erzeugen(quelle);
      const gut = stuecke.filter((s) => s.pdf);
      const schlecht = stuecke.filter((s) => !s.pdf);
      const kopf = el("div", "serie-fertig", t("{gut} von {gesamt} PDFs erzeugt.", { gut: gut.length, gesamt: stuecke.length }));
      const teile: HTMLElement[] = [kopf];
      if (gut.length) {
        const ordner = this.stand?.zielordner ?? "";
        teile.push(
          el("div", "serie-ziel", t("in {ordner}:", { ordner })),
          el("div", "serie-dateien", namenListe(gut.map((s) => dateiname(s.pdf!)), 12)),
        );
        const zeigen = el("button", "", t("Im Ordner zeigen"));
        zeigen.onclick = () => void this.a.zeigen(gut[0].pdf!).catch((f) => (kopf.textContent = String(f)));
        teile.push(zeigen);
        const gp = el("details", "serie-gegenprobe");
        gp.append(el("summary", "", t("Gegenprobe: dieselben PDFs ohne TFTN")));
        const code = el("code", "", gut.map((s) => s.gegenprobe).join("\n"));
        gp.append(code);
        teile.push(gp);
      }
      for (const s of schlecht) {
        const f = el("div", "serie-fehler");
        f.append(el("strong", "", `${s.name}: `), document.createTextNode(s.fehler ?? ""));
        teile.push(f);
      }
      this.ergebnis.replaceChildren(...teile);
    } catch (fehler) {
      this.ergebnis.replaceChildren(el("div", "serie-fehler", String(fehler)));
    } finally {
      this.laeuft = false;
      this.los.disabled = this.stand?.hauptdatei !== "bereit";
    }
  }
}
