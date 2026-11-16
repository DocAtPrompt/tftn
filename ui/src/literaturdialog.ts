// Literaturverwaltung: Einträge der Literaturdatei als Liste (links, mit Suche) und als Formular
// (rechts). Speichern schreibt nur den einen Eintrag in die .bib; der Rest der Datei bleibt.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import {
  type BibEintrag,
  type Feld,
  type LitDateiJs,
  type LitEintragJs,
  PERSONEN,
  alsEintrag,
  arten,
  eintraegeFiltern,
  feldTitel,
  feldWert,
  formularFelder,
  personenZeilen,
  schluesselVorschlag,
} from "./literatur";
import { t } from "./sprache";

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface LiteraturAktionen {
  liste(): Promise<LitDateiJs[]>;
  /** `alt`: bisheriger Schlüssel; `null` legt neu an. */
  speichern(datei: string, alt: string | null, eintrag: BibEintrag): Promise<void>;
  loeschen(datei: string, schluessel: string): Promise<void>;
  /** `@schluessel` an der Schreibmarke einfügen. */
  zitieren(schluessel: string): void;
  geschlossen(): void;
}

/** Ein Feld im Formular: Wert aus seiner Eingabe. */
interface Zeile {
  name: string;
  roh: boolean;
  eingabe: HTMLInputElement | HTMLTextAreaElement;
}

const BEKANNT = [
  "author", "editor", "translator", "title", "year", "date", "journal", "volume", "number", "pages",
  "publisher", "address", "edition", "isbn", "issn", "doi", "url", "urldate", "booktitle", "school",
  "institution", "organization", "howpublished", "note", "series", "chapter", "month", "type",
  "abstract", "keywords", "language",
];

export class LiteraturDialog {
  private huelle = el("div", "neu-dialog literatur-dialog");
  private suche = el("input", "lit-suche");
  private dateiWahl = el("select", "lit-datei");
  private neuKnopf = el("button", "", t("Neuer Eintrag"));
  private liste = el("div", "lit-liste");
  private formular = el("div", "lit-formular");
  private meldung = el("div", "neu-meldung");
  private dateien: LitDateiJs[] = [];
  private datei = "";
  /** Bisheriger Schlüssel des Eintrags im Formular; `null`: neu; `undefined`: keiner gewählt. */
  private alt: string | null | undefined = undefined;
  private art = el("select", "lit-art");
  private schluessel = el("input", "lit-schluessel");
  private schluesselBeruehrt = false;
  private zeilen: Zeile[] = [];
  private ausgang = "";
  private warnte = false;

  constructor(private a: LiteraturAktionen) {
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    const kopf = dialogKopf(t("Literatur"), () => this.schliessen());
    const leiste = el("div", "lit-leiste");
    this.suche.placeholder = t("Suchen: Autor, Jahr, Titel, Schlüssel");
    this.suche.spellcheck = false;
    this.suche.addEventListener("input", () => this.listeZeigen());
    this.dateiWahl.title = t("Literaturdatei");
    this.dateiWahl.addEventListener("change", () => {
      if (!this.wechselnErlaubt()) {
        this.dateiWahl.value = this.datei;
        return;
      }
      this.datei = this.dateiWahl.value;
      this.alt = undefined;
      this.listeZeigen();
      this.formularZeigen(null);
    });
    this.neuKnopf.addEventListener("click", () => {
      if (this.wechselnErlaubt()) this.neu();
    });
    leiste.append(this.suche, this.dateiWahl, this.neuKnopf);
    const rumpf = el("div", "lit-rumpf");
    rumpf.append(this.liste, this.formular);
    this.huelle.append(kopf, leiste, rumpf, this.meldung);
    this.huelle.addEventListener("keydown", (e) => {
      // Cmd/Strg+S speichert hier den Eintrag, nicht das Dokument.
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        e.stopPropagation();
        void this.speichern();
      }
    });
    document.body.append(this.huelle);
    dialogAnmelden(this, this.huelle);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  /** Ungespeicherte Änderungen: Ein Klick daneben schließt nicht. */
  behalten(): boolean {
    return this.geaendert();
  }

  schliessen() {
    if (this.geaendert() && !this.warnte) {
      this.warnte = true;
      this.melden(t("Nicht gespeichert. Speichern, „Verwerfen“ – oder noch einmal schließen, dann gehen die Änderungen verloren."));
      return;
    }
    this.huelle.hidden = true;
    this.a.geschlossen();
  }

  async zeigen() {
    this.huelle.hidden = false;
    this.warnte = false;
    this.melden("");
    this.suche.value = "";
    await this.laden(undefined);
    this.suche.focus();
  }

  private melden(text: string) {
    this.meldung.textContent = text;
  }

  private aktuelleDatei(): LitDateiJs | undefined {
    return this.dateien.find((d) => d.pfad === this.datei);
  }

  /** Liste neu lesen; danach `waehlen` (Schlüssel) zeigen, sonst den ersten Eintrag. */
  private async laden(waehlen: string | undefined) {
    try {
      this.dateien = await this.a.liste();
    } catch (fehler) {
      this.dateien = [];
      this.melden(String(fehler));
    }
    if (!this.aktuelleDatei()) this.datei = (this.dateien.find((d) => d.schreibbar) ?? this.dateien[0])?.pfad ?? "";
    this.dateiWahl.replaceChildren(...this.dateien.map((d) => new Option(d.pfad, d.pfad)));
    this.dateiWahl.value = this.datei;
    this.dateiWahl.hidden = this.dateien.length < 2;
    const d = this.aktuelleDatei();
    this.neuKnopf.disabled = !d?.schreibbar;
    const e = d?.eintraege.find((x) => x.schluessel === waehlen) ?? d?.eintraege[0];
    // vor der Liste setzen, sonst ist der gewählte Eintrag dort nicht hervorgehoben
    this.alt = e ? e.schluessel : undefined;
    this.listeZeigen();
    this.formularZeigen(e ?? null);
  }

  private listeZeigen() {
    const d = this.aktuelleDatei();
    if (!d) {
      this.liste.replaceChildren(el("p", "lit-leer", t("Dieses Dokument hat keine Literaturdatei.")));
      return;
    }
    if (d.fehler) {
      this.liste.replaceChildren(el("p", "lit-leer", d.fehler));
      return;
    }
    if (d.eintraege.length === 0) {
      this.liste.replaceChildren(
        el("p", "lit-leer", t("Noch keine Einträge in {datei}. „Neuer Eintrag“ legt einen an; im Zitat-Dialog geht es auch über eine DOI.", { datei: d.pfad })),
      );
      return;
    }
    const treffer = eintraegeFiltern(d.eintraege, this.suche.value);
    if (treffer.length === 0) {
      this.liste.replaceChildren(el("p", "lit-leer", t("Nichts gefunden.")));
      return;
    }
    this.liste.replaceChildren(...treffer.map((e) => this.listenEintrag(e)));
  }

  private listenEintrag(e: LitEintragJs): HTMLButtonElement {
    const b = el("button", "lit-eintrag");
    b.classList.toggle("gewaehlt", e.schluessel === this.alt);
    const kopf = el("span", "lit-kopf");
    kopf.append(el("code", "", e.schluessel));
    if (!e.zitiert) kopf.append(el("span", "lit-marke", t("nicht zitiert")));
    const autoren = feldWert(e, "author") || feldWert(e, "editor");
    const jahr = feldWert(e, "year").slice(0, 4);
    b.append(kopf, el("span", "lit-zeile", [autoren.split(/\s+and\s+/i)[0], jahr].filter(Boolean).join(" · ")), el("span", "lit-zeile lit-titel", feldWert(e, "title")));
    b.addEventListener("click", () => {
      if (e.schluessel === this.alt || !this.wechselnErlaubt()) return;
      this.alt = e.schluessel;
      for (const k of this.liste.querySelectorAll(".lit-eintrag")) k.classList.toggle("gewaehlt", k === b);
      this.formularZeigen(e);
    });
    return b;
  }

  /** Mit ungespeicherten Änderungen nicht wegwechseln. */
  private wechselnErlaubt(): boolean {
    if (!this.geaendert()) return true;
    this.melden(t("Erst speichern oder „Verwerfen“."));
    return false;
  }

  private neu() {
    this.alt = null;
    for (const k of this.liste.querySelectorAll(".lit-eintrag")) k.classList.remove("gewaehlt");
    this.formularZeigen({ art: "article", schluessel: "", felder: [] });
    this.formular.querySelector<HTMLElement>(".lit-feld textarea, .lit-feld input")?.focus();
  }

  /** Das Formular für `e`; `null`: leer (nichts gewählt). */
  private formularZeigen(e: BibEintrag | null) {
    this.warnte = false;
    if (!e || this.alt === undefined) {
      this.zeilen = [];
      this.ausgang = "";
      this.formular.replaceChildren();
      return;
    }
    const schreibbar = this.aktuelleDatei()?.schreibbar ?? false;
    this.schluesselBeruehrt = this.alt !== null;
    this.art.replaceChildren(...arten().map(([wert, titel]) => new Option(titel, wert)));
    if (![...this.art.options].some((o) => o.value === e.art)) this.art.append(new Option(e.art, e.art));
    this.art.value = e.art;
    // Andere Art: Ausgefülltes bleibt, leere Felder der alten Art fallen weg.
    this.art.onchange = () => this.felderZeigen(this.art.value, this.felderJetzt().filter((f) => f.wert.trim() !== ""));
    this.schluessel.value = e.schluessel;
    this.schluessel.spellcheck = false;
    this.schluessel.oninput = () => {
      this.schluesselBeruehrt = true;
      this.knoepfeZeigen();
    };
    this.felderZeigen(e.art, e.felder);
    this.ausgang = JSON.stringify(alsEintrag(e.art, e.schluessel, this.felderJetzt()));
    this.knoepfeZeigen();
    for (const x of this.formular.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | HTMLButtonElement>("input, textarea, select, .lit-feld-dazu button")) {
      x.disabled = !schreibbar;
    }
  }

  /** Felder der Art; Personen in `vorhanden` als `A and B` oder schon zeilenweise. */
  private felderZeigen(art: string, vorhanden: readonly Feld[]) {
    const kopf = el("div", "lit-feldkopf");
    const artFeld = el("label", "lit-feld");
    artFeld.append(el("span", "", t("Art")), this.art);
    const schluesselFeld = el("label", "lit-feld");
    schluesselFeld.append(el("span", "", t("Schlüssel")), this.schluessel);
    this.schluessel.title = t("Damit wird zitiert: @schluessel");
    kopf.append(artFeld, schluesselFeld);
    const felder = el("div", "lit-felder");
    this.zeilen = formularFelder(art, vorhanden).map((f) => {
      const z = this.feldZeile(f);
      felder.append(z.huelle);
      return z.zeile;
    });
    const dazu = el("div", "lit-feld-dazu");
    const name = el("input");
    name.placeholder = t("Weiteres Feld, z. B. series");
    name.spellcheck = false;
    name.setAttribute("list", "lit-feldnamen");
    const liste = el("datalist");
    liste.id = "lit-feldnamen";
    liste.append(...BEKANNT.map((n) => new Option(feldTitel(n), n)));
    const knopf = el("button", "", t("Feld hinzufügen"));
    knopf.addEventListener("click", () => {
      const n = name.value.trim().toLowerCase();
      if (!n) return;
      const da = this.zeilen.find((z) => z.name === n);
      if (da) {
        da.eingabe.focus();
        return;
      }
      const z = this.feldZeile({ name: n, wert: "", roh: false });
      felder.append(z.huelle);
      this.zeilen.push(z.zeile);
      name.value = "";
      z.zeile.eingabe.focus();
    });
    dazu.append(name, liste, knopf);
    const knoepfe = el("div", "lit-knoepfe");
    this.formular.replaceChildren(kopf, felder, dazu, knoepfe);
    this.knoepfeZeigen();
  }

  private feldZeile(f: Feld): { huelle: HTMLElement; zeile: Zeile } {
    const huelle = el("label", "lit-feld");
    huelle.append(el("span", "", feldTitel(f.name)));
    const personen = PERSONEN.has(f.name) && !f.roh;
    const lang = personen || ["title", "booktitle", "abstract", "note"].includes(f.name);
    let eingabe: HTMLInputElement | HTMLTextAreaElement;
    if (lang) {
      const ta = el("textarea");
      ta.value = personen ? personenZeilen(f.wert) : f.wert;
      ta.rows = Math.max(f.name === "abstract" ? 4 : 2, ta.value.split("\n").length);
      if (personen) ta.placeholder = t("Nachname, Vorname – eine Person je Zeile");
      eingabe = ta;
    } else {
      eingabe = el("input");
      eingabe.value = f.wert;
    }
    eingabe.spellcheck = false;
    if (f.roh) {
      eingabe.classList.add("lit-roh");
      eingabe.title = t("BibTeX-Ausdruck ohne Klammern (Zahl, Abkürzung); wird so geschrieben");
    }
    eingabe.addEventListener("input", () => {
      if (this.alt === null && !this.schluesselBeruehrt && (f.name === "author" || f.name === "editor" || f.name === "year" || f.name === "date")) {
        this.schluesselVorschlagen();
      }
      this.knoepfeZeigen();
    });
    huelle.append(eingabe);
    return { huelle, zeile: { name: f.name, roh: f.roh, eingabe } };
  }

  /** Neuer Eintrag: Schlüssel aus erster Person und Jahr, solange man ihn nicht selbst tippt. */
  private schluesselVorschlagen() {
    const wert = (n: string) => this.zeilen.find((z) => z.name === n)?.eingabe.value ?? "";
    const personen = (wert("author") || wert("editor")).split("\n").join(" and ");
    const vergeben = new Set(this.dateien.flatMap((d) => d.eintraege.map((e) => e.schluessel)));
    this.schluessel.value = schluesselVorschlag(personen, wert("year") || wert("date"), vergeben);
  }

  /** Die Felder, wie sie im Formular stehen (Personen zeilenweise). */
  private felderJetzt(): Feld[] {
    return this.zeilen.map((z) => ({ name: z.name, wert: z.eingabe.value, roh: z.roh }));
  }

  private eintragJetzt(): BibEintrag {
    return alsEintrag(this.art.value, this.schluessel.value, this.felderJetzt());
  }

  private geaendert(): boolean {
    return this.alt !== undefined && JSON.stringify(this.eintragJetzt()) !== this.ausgang;
  }

  private knoepfeZeigen() {
    const ort = this.formular.querySelector(".lit-knoepfe");
    if (!ort || this.alt === undefined) return;
    const schreibbar = this.aktuelleDatei()?.schreibbar ?? false;
    const geaendert = this.geaendert() || this.alt === null;
    const speichern = el("button", "lit-speichern", this.alt === null ? t("Anlegen") : t("Speichern"));
    speichern.disabled = !schreibbar || !geaendert;
    speichern.title = t("Cmd/Strg+S");
    speichern.addEventListener("click", () => void this.speichern());
    const knoepfe: HTMLElement[] = [speichern];
    if (this.geaendert() || this.alt === null) {
      const verwerfen = el("button", "", t("Verwerfen"));
      verwerfen.addEventListener("click", () => {
        this.melden("");
        if (this.alt === null) {
          this.alt = undefined;
          void this.laden(undefined);
          return;
        }
        const e = this.aktuelleDatei()?.eintraege.find((x) => x.schluessel === this.alt) ?? null;
        this.formularZeigen(e);
      });
      knoepfe.push(verwerfen);
    }
    if (this.alt !== null) {
      const alt = this.alt;
      const zitieren = el("button", "", t("Zitieren"));
      zitieren.title = t("@{schluessel} an der Schreibmarke einfügen", { schluessel: alt });
      zitieren.disabled = this.geaendert();
      zitieren.addEventListener("click", () => {
        this.huelle.hidden = true;
        this.a.zitieren(alt);
      });
      const loeschen = el("button", "lit-loeschen", t("Löschen"));
      loeschen.disabled = !schreibbar;
      loeschen.addEventListener("click", () => {
        if (loeschen.dataset.sicher !== "1") {
          loeschen.dataset.sicher = "1";
          loeschen.textContent = t("Wirklich löschen?");
          return;
        }
        void this.loeschen(alt);
      });
      knoepfe.push(zitieren, loeschen);
    }
    ort.replaceChildren(...knoepfe);
  }

  private async speichern() {
    if (this.alt === undefined || !(this.geaendert() || this.alt === null)) return;
    const e = this.eintragJetzt();
    try {
      await this.a.speichern(this.datei, this.alt, e);
    } catch (fehler) {
      this.melden(String(fehler));
      return;
    }
    this.melden(this.alt === null ? t("„{schluessel}“ angelegt.", { schluessel: e.schluessel }) : t("Gespeichert."));
    this.alt = e.schluessel;
    await this.laden(e.schluessel);
  }

  private async loeschen(schluessel: string) {
    try {
      await this.a.loeschen(this.datei, schluessel);
    } catch (fehler) {
      this.melden(String(fehler));
      return;
    }
    this.melden(t("„{schluessel}“ gelöscht.", { schluessel }));
    this.alt = undefined;
    await this.laden(undefined);
  }
}
