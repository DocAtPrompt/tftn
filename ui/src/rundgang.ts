// Rundgänge: geführte Durchgänge durch die Oberfläche. Erklärt wird in Untertiteln (ein Band unten
// mittig), auf Wunsch mit einer Hervorhebung des Ziels (der Rest des Fensters leicht abgedunkelt).
// Zwei Arten:
// - „Ansehen“: Weiter/Zurück im Band (auch → und ←, solange der Fokus nicht im Text steht), Esc beendet.
// - „Vorführen“: läuft von selbst ab, führt die Handgriffe aus (tippen, Menüs öffnen, klicken), mit
//   festen Pausen; keine Knöpfe im Bild, damit man den Bildschirm als Video aufnehmen kann.
//   Tastatur und Maus des Benutzers bleiben an, bringen aber nichts durcheinander: Eine Schicht über
//   dem Fenster fängt Mausklicks ab, und ein Klick hält die Vorführung an bzw. setzt sie fort (statt
//   etwa die Schreibmarke zu versetzen); die Leertaste ebenso. Esc bricht jederzeit ab; alle anderen
//   Tasten wirken während der Vorführung nicht.
//
// Die Logik (`Lauf`) kennt kein DOM und ist in rundgang.test.ts geprüft; `RundgangBuehne` zeichnet.
// Die Handgriffe bindet main.ts an die vorhandenen Funktionen an; die Rundgänge selbst stehen als
// Daten in rundgaenge.ts.

import { type Dialog, dialogAnmelden } from "./dialoge";
import { t } from "./sprache";

export type Art = "ansehen" | "vorfuehren";

export type FensterName = "ueber" | "einstellungen" | "netzprotokoll" | "projektsuche";
export type FachName = "hier" | "einfuegen" | "formel" | "seite" | "dokument" | "pakete";

/**
 * Was eine Vorführung tun kann. Jeder Handgriff ist eine dünne Hülle um das, was ein Knopf oder eine
 * Taste ohnehin tut (main.ts, `handgriffe`). Alles ist abbrechbar: Nach Esc wirft der nächste Aufruf.
 * Handgriffe, die in den Text schreiben (`SCHREIBEND`), wirken nur in der Kopie einer Vorführung.
 */
export interface Handgriffe {
  /** Tippt `text` an der Schreibmarke, sichtbar Zeichen für Zeichen, wie echte Eingabe (Felder, „/“-Menü). */
  tippen(text: string, msJeZeichen?: number): Promise<void>;
  /** Ein Kürzel: eine Aktions-ID (`typst/fett`, `helfer/ueber`; gezeigt wird das geltende Kürzel)
   *  oder eine Taste wie `Mod-b`, `Enter`, `Escape`. Die Taste erscheint groß über dem Untertitel. */
  taste(kombination: string): Promise<void>;
  /** Markiert das erste Vorkommen von `text`. */
  markieren(text: string): Promise<void>;
  /** Schreibmarke hinter das erste Vorkommen von `nach`, ohne Angabe ans Ende des Texts. */
  schreibmarke(nach?: string): Promise<void>;
  /** Baustein nach ID (`typst/fussnote`) oder Titel: aus dem offenen „/“-Menü, sonst an der Schreibmarke. */
  bausteinWaehlen(titelOderId: string): Promise<void>;
  fachOeffnen(name: FachName): Promise<void>;
  exportMenue(offen: boolean): Promise<void>;
  dialogOeffnen(name: FensterName): Promise<void>;
  /** Klickt in der Vorschau auf die Stelle, an der `text` (aus dem Quelltext) gesetzt ist. */
  inVorschauKlicken(text: string): Promise<void>;
  /** Kapiteldatei als Reiter öffnen bzw. wählen. */
  reiterOeffnen(datei: string): Promise<void>;
  /** Skin für die Dauer des Rundgangs (`null`: wie das System); danach gilt wieder der eigene. */
  skinSetzen(kennung: string | null): Promise<void>;
  /** Editorschrift in Prozent für die Dauer des Rundgangs. */
  editorSchrift(prozent: number): Promise<void>;
  warten(ms: number): Promise<void>;
  /** Schließt offene Dialoge, Menüs, die Seitenleiste und das „/“-Menü. */
  schliessen(): Promise<void>;
}

/** Alle Handgriffe (für den Test der Rundgänge). */
export const HANDGRIFFE: readonly (keyof Handgriffe)[] = [
  "tippen",
  "taste",
  "markieren",
  "schreibmarke",
  "bausteinWaehlen",
  "fachOeffnen",
  "exportMenue",
  "dialogOeffnen",
  "inVorschauKlicken",
  "reiterOeffnen",
  "skinSetzen",
  "editorSchrift",
  "warten",
  "schliessen",
];

/** Handgriffe, die den Text ändern können: nur in einem Rundgang mit `kopie`. */
export const SCHREIBEND: readonly (keyof Handgriffe)[] = ["tippen", "taste", "bausteinWaehlen"];

export interface Schritt {
  /** CSS-Auswahl (Elemente aus ui/index.html oder `DYNAMISCHE_ZIELE` in rundgaenge.ts) oder eine
   *  Funktion. Fehlt das Ziel oder ist es nicht zu sehen, zeigt der Schritt nur den Untertitel. */
  ziel?: string | (() => Element | null);
  /** Der Untertitel, schon übersetzt (über `t`). */
  text: string;
  /** Handgriff vor dem Untertitel (genauer: Der Text steht schon, die Hervorhebung kommt danach). */
  vorher?: (api: Handgriffe) => Promise<void>;
  /** Wann `vorher` läuft: nur beim Vorführen (Standard) oder in beiden Arten. */
  vorherIn?: "vorfuehren" | "beide";
  /** Vorführen: Standzeit nach dem Handgriff in ms; Standard aus der Textlänge (`dauerVon`). */
  dauer?: number;
}

export interface Rundgang {
  id: string;
  titel: string;
  beschreibung: string;
  /** Läuft in einer frischen Kopie mit erfundenen Daten, aus der Vorlage dieses Dokumenttyps. Pflicht,
   *  sobald ein Schritt schreibt. */
  kopie?: string;
  schritte: Schritt[];
}

/** Lesezeit je Zeichen des Untertitels beim Vorführen. */
export const MS_JE_ZEICHEN = 60;
export const MIN_DAUER = 3000;
/** Länger darf ein Handgriff nicht brauchen (angehaltene Zeit zählt nicht); danach geht es weiter. */
export const FRIST = 20000;
/** Vorführen: Ruhe vor dem ersten Schritt (Fenster frisch geladen, erste Vorschau). */
export const VORLAUF = 1200;

export function dauerVon(s: Pick<Schritt, "text" | "dauer">): number {
  return s.dauer ?? Math.max(MIN_DAUER, [...s.text].length * MS_JE_ZEICHEN);
}

/** Läuft der Handgriff des Schritts in dieser Art? */
export function laeuftIn(s: Pick<Schritt, "vorher" | "vorherIn">, art: Art): boolean {
  return !!s.vorher && (art === "vorfuehren" || s.vorherIn === "beide");
}

export type Befehl = "weiter" | "zurueck" | "ende";

/** Nächster Schritt beim Ansehen; `null`: Ende. Zurück am Anfang bleibt am Anfang. */
export function naechsterSchritt(nr: number, befehl: Befehl, anzahl: number): number | null {
  if (befehl === "ende") return null;
  if (befehl === "zurueck") return Math.max(0, nr - 1);
  return nr + 1 < anzahl ? nr + 1 : null;
}

export interface Anzeige {
  text: string;
  /** 0-basiert */
  nr: number;
  anzahl: number;
  art: Art;
  ziel: Element | null;
  angehalten: boolean;
}

/** Was der Lauf vom Bildschirm braucht. */
export interface Buehne {
  zeigen(a: Anzeige): void;
  /** Ansehen: wartet auf Weiter, Zurück oder Beenden. */
  befehl(): Promise<Befehl>;
}

/** Esc (oder `abbrechen`) während eines Handgriffs oder einer Pause. */
export class Abbruch extends Error {
  constructor() {
    super("Rundgang abgebrochen");
  }
}

const schlafen = (ms: number) => new Promise<void>((weiter) => setTimeout(weiter, ms));

/** Handgriffe ohne `warten`; `warten` gibt der Lauf (abbrechbar, hält bei einer Pause an). */
export type HandgriffeBauen = (warten: (ms: number) => Promise<void>) => Omit<Handgriffe, "warten">;

export type Ergebnis = "fertig" | "abgebrochen";

/** Ein Durchgang durch einen Rundgang. Bleibt nie hängen: Fehlt ein Ziel, wird es nicht hervorgehoben;
 *  scheitert ein Handgriff oder dauert er länger als `FRIST`, geht es mit dem Untertitel weiter. */
export class Lauf {
  private abgebrochen = false;
  private angehalten = false;
  private melden: () => void = () => {};
  private signal: Promise<never>;
  private anzeige: Anzeige | null = null;
  /** Jeder Schritt bekommt eigene Handgriffe; die eines verworfenen Schritts werfen danach. */
  private generation = 0;

  constructor(
    readonly rundgang: Rundgang,
    readonly art: Art,
    private buehne: Buehne,
    private bauen: HandgriffeBauen,
    private zielSuchen: (ziel: NonNullable<Schritt["ziel"]>) => Element | null,
    /** Ansehen, vor einem Schritt zurück: schließt, was spätere Schritte geöffnet haben (Menüs,
     *  Seitenleiste); der frühere Schritt öffnet mit seinem Handgriff („beide“) wieder, was er braucht. */
    private zurueckRaeumen: () => void = () => {},
  ) {
    this.signal = new Promise<never>((_, nein) => {
      this.melden = () => nein(new Abbruch());
    });
    this.signal.catch(() => {});
  }

  get laeuft(): boolean {
    return !this.abgebrochen;
  }

  get istAngehalten(): boolean {
    return this.angehalten;
  }

  abbrechen() {
    if (this.abgebrochen) return;
    this.abgebrochen = true;
    this.melden();
  }

  /** Vorführen: anhalten bzw. fortsetzen (Klick, Leertaste). */
  anhalten(an = !this.angehalten) {
    if (this.art !== "vorfuehren" || this.abgebrochen || an === this.angehalten) return;
    this.angehalten = an;
    if (this.anzeige) this.zeigen({ ...this.anzeige, angehalten: an });
  }

  async starten(): Promise<Ergebnis> {
    const schritte = this.rundgang.schritte;
    try {
      if (this.art === "vorfuehren") await this.warten(VORLAUF);
      let nr = 0;
      while (nr < schritte.length) {
        const s = schritte[nr];
        this.zeigen({ text: s.text, nr, anzahl: schritte.length, art: this.art, ziel: null, angehalten: this.angehalten });
        if (laeuftIn(s, this.art)) await this.handgriff(s);
        this.pruefen();
        this.zeigen({ ...this.anzeige!, ziel: this.ziel(s) });
        if (this.art === "vorfuehren") {
          await this.warten(dauerVon(s));
          nr += 1;
        } else {
          const b = await this.mitAbbruch(this.buehne.befehl());
          const weiter = naechsterSchritt(nr, b, schritte.length);
          if (weiter === null) return b === "ende" ? "abgebrochen" : "fertig";
          if (b === "zurueck") this.zurueckRaeumen();
          nr = weiter;
        }
      }
      return "fertig";
    } catch (fehler) {
      if (fehler instanceof Abbruch) return "abgebrochen";
      throw fehler;
    } finally {
      this.abbrechen();
    }
  }

  private zeigen(a: Anzeige) {
    this.anzeige = a;
    this.buehne.zeigen(a);
  }

  private pruefen() {
    if (this.abgebrochen) throw new Abbruch();
  }

  private mitAbbruch<T>(p: Promise<T>): Promise<T> {
    return Promise.race([p, this.signal]);
  }

  /** Wartet `ms`; angehaltene Zeit zählt nicht mit. Wirft bei Abbruch. */
  private async warten(ms: number, gen = this.generation) {
    let rest = ms;
    while (rest > 0) {
      this.pruefenFuer(gen);
      const stueck = Math.min(rest, 100);
      await this.mitAbbruch(schlafen(stueck));
      if (!this.angehalten) rest -= stueck;
    }
    this.pruefenFuer(gen);
  }

  private pruefenFuer(gen: number) {
    this.pruefen();
    if (gen !== this.generation) throw new Abbruch();
  }

  /** Handgriffe für einen Schritt: Jeder Aufruf prüft vorher und nachher, ob es noch weitergeht. */
  private handgriffe(gen: number): Handgriffe {
    const warten = (ms: number) => this.warten(ms, gen);
    const roh = this.bauen(warten);
    const api = { warten } as Handgriffe;
    for (const name of HANDGRIFFE) {
      if (name === "warten") continue;
      const f = roh[name] as (...a: unknown[]) => Promise<void>;
      (api as unknown as Record<string, unknown>)[name] = async (...a: unknown[]) => {
        this.pruefenFuer(gen);
        await this.mitAbbruch(f(...a));
        this.pruefenFuer(gen);
      };
    }
    return api;
  }

  private async handgriff(s: Schritt) {
    const gen = ++this.generation;
    const frist = this.warten(FRIST, gen).then(() => {
      throw new Error(`Handgriff nach ${FRIST} ms verworfen`);
    });
    frist.catch(() => {});
    try {
      await Promise.race([s.vorher!(this.handgriffe(gen)), frist]);
    } catch (fehler) {
      if (this.abgebrochen) throw new Abbruch();
      if (!(fehler instanceof Abbruch)) console.warn("Rundgang: Handgriff fehlgeschlagen", fehler);
    } finally {
      // Ein verworfener Handgriff (Frist) darf danach nichts mehr tun.
      if (this.generation === gen) this.generation += 1;
    }
  }

  private ziel(s: Schritt): Element | null {
    if (!s.ziel) return null;
    try {
      return this.zielSuchen(s.ziel);
    } catch {
      return null;
    }
  }
}

/** Ziel nach Auswahl oder Funktion; ein verstecktes oder leeres Element zählt als fehlend. */
export function zielImDokument(ziel: NonNullable<Schritt["ziel"]>): Element | null {
  const e = typeof ziel === "string" ? document.querySelector(ziel) : ziel();
  if (!e || !e.isConnected || e.closest("[hidden]")) return null;
  const r = e.getBoundingClientRect();
  return r.width > 0 && r.height > 0 ? e : null;
}

/** Wo das Band steht: unten mittig; liegt das Ziel im unteren Teil des Fensters, darüber. */
export function bandAbstandUnten(ziel: { oben: number; unten: number } | null, fensterHoehe: number, normal = 48): number {
  if (!ziel || ziel.oben < fensterHoehe * 0.55) return normal;
  return Math.max(normal, fensterHoehe - ziel.oben + 16);
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

/**
 * Untertitel, Hervorhebung, Schutzschicht (Vorführen) und kleine Einblendungen (Taste, Klick).
 * Bei dialoge.ts angemeldet (Esc); Esc fängt sie zusätzlich ganz vorn ab, damit es auch dann den
 * Rundgang beendet, wenn ein Handgriff gerade ein Menü geöffnet hat.
 */
export class RundgangBuehne implements Buehne, Dialog {
  private huelle = el("div", "rundgang");
  private band = el("div", "rundgang-band");
  private text = el("p", "rundgang-text");
  private fuss = el("div", "rundgang-fuss");
  private zaehler = el("span", "rundgang-zaehler");
  private pause = el("span", "rundgang-pause");
  private zurueck = el("button", "", t("Zurück"));
  private weiter = el("button", "rundgang-weiter", t("Weiter"));
  private beenden = el("button", "", t("Beenden"));
  private taste = el("div", "rundgang-taste");
  private ring = el("div", "rundgang-ring");
  private schutz = el("div", "rundgang-schutz");
  private lauf: Lauf | null = null;
  private antwort: ((b: Befehl) => void) | null = null;
  private ziel: Element | null = null;
  private folgen = 0;
  private tasteUhr: number | undefined;
  private tastenHoerer = (e: KeyboardEvent) => this.tastatur(e);

  constructor() {
    for (const e of [this.huelle, this.ring, this.schutz]) e.hidden = true;
    this.band.setAttribute("role", "status");
    this.band.setAttribute("aria-live", "polite");
    this.text.id = "rundgang-text";
    this.zurueck.addEventListener("click", () => this.antworten("zurueck"));
    this.weiter.addEventListener("click", () => this.antworten("weiter"));
    this.beenden.addEventListener("click", () => this.antworten("ende"));
    this.beenden.title = t("Rundgang beenden (Esc)");
    const knoepfe = el("span", "rundgang-knoepfe");
    knoepfe.append(this.zurueck, this.weiter, this.beenden);
    this.fuss.append(this.zaehler, this.pause, knoepfe);
    this.band.append(this.text, this.fuss);
    this.huelle.append(this.taste, this.band);
    // Ohne `title`: Ein Hinweis unter dem ruhenden Mauszeiger stünde sonst im Video.
    this.schutz.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.lauf?.anhalten();
    });
    document.body.append(this.schutz, this.ring, this.huelle);
    dialogAnmelden(this);
  }

  get offen(): boolean {
    return this.lauf?.laeuft ?? false;
  }

  /** Esc über dialoge.ts. */
  schliessen() {
    this.lauf?.abbrechen();
  }

  /** Vor `lauf.starten()`: Band einblenden, Tastatur abfangen. */
  verbinden(lauf: Lauf) {
    this.lauf = lauf;
    this.huelle.hidden = false;
    // Das Band erscheint mit dem ersten Untertitel (beim Vorführen erst nach dem Vorlauf).
    this.band.hidden = true;
    this.huelle.classList.toggle("vorfuehren", lauf.art === "vorfuehren");
    this.schutz.hidden = lauf.art !== "vorfuehren";
    window.addEventListener("keydown", this.tastenHoerer, true);
    this.folgenStarten();
  }

  /** Nach dem Lauf: alles weg. */
  trennen() {
    this.lauf = null;
    this.antwort = null;
    this.ziel = null;
    cancelAnimationFrame(this.folgen);
    for (const e of [this.huelle, this.ring, this.schutz]) e.hidden = true;
    this.taste.classList.remove("sichtbar");
    window.removeEventListener("keydown", this.tastenHoerer, true);
  }

  zeigen(a: Anzeige) {
    this.band.hidden = false;
    this.text.textContent = a.text;
    this.zaehler.textContent = t("{nr} von {anzahl}", { nr: a.nr + 1, anzahl: a.anzahl });
    const ansehen = a.art === "ansehen";
    this.fuss.hidden = !ansehen && !a.angehalten;
    this.zurueck.hidden = this.weiter.hidden = this.beenden.hidden = !ansehen;
    this.zaehler.hidden = !ansehen;
    this.zurueck.disabled = a.nr === 0;
    this.weiter.textContent = a.nr + 1 < a.anzahl ? t("Weiter") : t("Fertig");
    this.pause.textContent = a.angehalten ? t("Angehalten – Klick oder Leertaste setzt fort, Esc beendet.") : "";
    this.pause.hidden = !a.angehalten;
    this.ziel = a.ziel;
    this.ringSetzen();
    if (ansehen) this.weiter.focus({ preventScroll: true });
  }

  befehl(): Promise<Befehl> {
    return new Promise((weiter) => (this.antwort = weiter));
  }

  /** Große Taste über dem Untertitel (Vorführen und Ansehen), blendet sich aus. */
  tasteZeigen(text: string) {
    clearTimeout(this.tasteUhr);
    this.taste.textContent = text;
    this.taste.classList.add("sichtbar");
    this.tasteUhr = window.setTimeout(() => this.taste.classList.remove("sichtbar"), 1600);
  }

  /** Ein Kreis an der Stelle eines vorgeführten Klicks (Bildschirmpunkt). */
  klickZeigen(x: number, y: number) {
    const k = el("div", "rundgang-klick");
    k.style.left = `${x}px`;
    k.style.top = `${y}px`;
    document.body.append(k);
    window.setTimeout(() => k.remove(), 900);
  }

  private antworten(b: Befehl) {
    const a = this.antwort;
    this.antwort = null;
    a?.(b);
  }

  private tastatur(e: KeyboardEvent) {
    if (!this.lauf?.laeuft || !e.isTrusted) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      this.lauf.abbrechen();
      return;
    }
    if (this.lauf.art === "vorfuehren") {
      // Nichts soll in den vorgeführten Text geraten; die Leertaste hält an.
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.key === " " && !e.repeat) this.lauf.anhalten();
      return;
    }
    // Ansehen: Pfeile blättern, solange nicht in einem Eingabefeld oder im Editor getippt wird.
    const ort = e.target as HTMLElement | null;
    const tippt = !!ort?.closest?.("input, textarea, select, [contenteditable='true'], .cm-editor");
    if (tippt && !this.band.contains(ort)) return;
    if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
      e.preventDefault();
      this.antworten(e.key === "ArrowRight" ? "weiter" : "zurueck");
    }
  }

  /** Die Hervorhebung folgt dem Ziel (Scrollen, Größe, Oberflächengröße), je Bild neu gemessen. */
  private folgenStarten() {
    const schritt = () => {
      this.ringSetzen();
      this.folgen = requestAnimationFrame(schritt);
    };
    cancelAnimationFrame(this.folgen);
    this.folgen = requestAnimationFrame(schritt);
  }

  private ringSetzen() {
    const z = this.ziel;
    const sichtbar = !!z && z.isConnected && !z.closest("[hidden]");
    const r = sichtbar ? z!.getBoundingClientRect() : null;
    if (!r || r.width === 0 || r.height === 0) {
      this.ring.hidden = true;
      this.huelle.style.bottom = "";
      return;
    }
    // getBoundingClientRect liefert sichtbare Pixel, auch unter `zoom: var(--ui-zoom)`; der Ring
    // selbst ist nicht gezoomt, also gelten die Werte unverändert (vgl. `anAnker` in platz.ts).
    const rand = 4;
    const links = Math.max(2, r.left - rand);
    const oben = Math.max(2, r.top - rand);
    const rechts = Math.min(window.innerWidth - 2, r.right + rand);
    const unten = Math.min(window.innerHeight - 2, r.bottom + rand);
    this.ring.hidden = false;
    const stil = this.ring.style;
    const neu = [`${links}px`, `${oben}px`, `${rechts - links}px`, `${unten - oben}px`];
    if (stil.left !== neu[0] || stil.top !== neu[1] || stil.width !== neu[2] || stil.height !== neu[3]) {
      [stil.left, stil.top, stil.width, stil.height] = neu;
    }
    const abstand = bandAbstandUnten({ oben: r.top, unten: r.bottom }, window.innerHeight);
    this.huelle.style.bottom = abstand === 48 ? "" : `${abstand}px`;
  }
}
