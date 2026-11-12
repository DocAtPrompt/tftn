// Dialog „Neues Projekt“ (PLAN-M2 AP8, Spec 9): Dokumenttypen mit Titel, Beschreibung, Herkunft und
// Vorschaubild; Elternordner und Projektname; Anlegen. Die Logik (Name prüfen, Pfad, Reihenfolge)
// steht in neuprojekt.ts, angelegt wird in Rust.

import { type TypJs, type TypenJs, namePruefen, typenOrdnen, zielPfad, brauchtListe } from "./neuprojekt";
import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { t } from "./sprache";

export interface NeuAktionen {
  typen(): Promise<TypenJs>;
  vorschaubild(name: string): Promise<string>;
  /** Systemdialog (nur am Mac); null bei Abbruch oder ohne Dialog. */
  ordnerWaehlen(): Promise<string | null>;
  /** Gibt es einen Systemdialog zur Ordnerwahl? */
  ordnerdialog: boolean;
  /** Speichert das offene Projekt, legt dann an; scheitert das Speichern, wird nichts angelegt. */
  anlegen(typ: string, eltern: string, name: string, schriften: boolean): Promise<string>;
  /** Lädt das Fenster neu; das Ziel hat Rust beim Anlegen gemerkt (gespeichert wurde vorher). */
  wechseln(): Promise<void>;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export const SCHRIFTEN_HINWEIS =
  t("Schriften einbinden (nur die benutzten, meist 1 bis 3 MB): Das Dokument bringt seine Schriften mit und sieht auf jedem Rechner und auch in Jahren genau gleich aus – empfohlen zum Weitergeben und Archivieren. Ohne Häkchen nutzt es die Schriften von TFTN.");

export class NeuDialog {
  private huelle: HTMLDivElement;
  private gewaehlt: TypJs | null = null;
  private eltern = el("input", "neu-eltern");
  private name = el("input", "neu-name");
  private braucht = el("ul", "neu-braucht");
  private schriften = el("input");
  private meldung = el("div", "neu-meldung");
  private anlegenKnopf = el("button", "neu-anlegen", t("Anlegen"));
  private laeuft = false;

  constructor(private a: NeuAktionen) {
    this.huelle = el("div", "neu-dialog");
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    this.huelle.setAttribute("aria-label", t("Neues Projekt"));
    document.body.append(this.huelle);
    this.huelle.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.schliessen();
      }
    });
    dialogAnmelden(this);
    this.eltern.type = "text";
    this.eltern.placeholder = t("Ordner, in dem das Projekt entsteht");
    this.name.type = "text";
    this.name.placeholder = t("Projektname, z. B. Artikel Reproduzierbarkeit");
    for (const f of [this.eltern, this.name]) f.addEventListener("input", () => this.pruefen());
    this.name.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void this.anlegen();
    });
    this.anlegenKnopf.addEventListener("click", () => void this.anlegen());
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  async oeffnen() {
    this.huelle.hidden = false;
    this.huelle.replaceChildren(el("p", "", t("Lade Dokumenttypen …")));
    let daten: TypenJs;
    try {
      daten = await this.a.typen();
    } catch (fehler) {
      this.huelle.replaceChildren(el("p", "dok-fehler", String(fehler)), this.zuKnopf());
      return;
    }
    const typen = typenOrdnen(daten.typen);
    this.gewaehlt = typen[0] ?? null;
    if (!this.eltern.value) this.eltern.value = daten.vorgabe_ordner ?? "";

    const kopf = el("div", "neu-kopf");
    kopf.append(el("h2", "", t("Neu aus Vorlage")), this.zuKnopf());
    const teile: HTMLElement[] = [kopf];
    for (const w of daten.warnungen) teile.push(el("p", "dok-warnung", w));

    const liste = el("ul", "neu-typen");
    for (const typ of typen) liste.append(this.typZeile(typ, liste));
    if (!typen.length) liste.append(el("li", "leiste-leer", t("Keine Dokumenttypen gefunden.")));
    const links = el("div", "neu-links");
    links.append(liste, el("p", "leiste-hilfe", t("Eigene Dokumenttypen: {ordner}", { ordner: daten.eigener_ordner })));

    const rechts = el("div", "neu-rechts");
    const ordnerZeile = el("div", "neu-ordnerzeile");
    ordnerZeile.append(this.eltern);
    if (this.a.ordnerdialog) {
      const w = el("button", "", t("Wählen …"));
      w.addEventListener("click", async () => {
        const o = await this.a.ordnerWaehlen();
        if (o) {
          this.eltern.value = o;
          this.pruefen();
        }
      });
      ordnerZeile.append(w);
    }
    rechts.append(
      el("label", "leiste-titel", t("Ordner")),
      ordnerZeile,
      el("label", "leiste-titel", t("Name")),
      this.name,
      this.schriftenZeile(),
      this.meldung,
      this.anlegenKnopf,
      el("label", "leiste-titel", t("Das brauchst du")),
      this.braucht,
    );
    this.brauchtZeigen();
    const rumpf = el("div", "neu-rumpf");
    rumpf.append(links, rechts);
    teile.push(rumpf);
    this.huelle.replaceChildren(...teile);
    this.pruefen();
    this.name.focus();
  }

  schliessen() {
    if (this.laeuft) return;
    this.huelle.hidden = true;
  }

  private schriftenZeile(): HTMLElement {
    this.schriften.type = "checkbox";
    const z = el("label", "schriften-haken");
    z.append(this.schriften, " ", SCHRIFTEN_HINWEIS);
    return z;
  }

  private brauchtZeigen() {
    this.braucht.replaceChildren(...brauchtListe(this.gewaehlt?.braucht).map((b) => el("li", "", b)));
  }

  private zuKnopf(): HTMLButtonElement {
    return schliessenKnopf(() => this.schliessen());
  }

  private typZeile(typ: TypJs, liste: HTMLUListElement): HTMLLIElement {
    const li = el("li", "neu-typ");
    li.tabIndex = 0;
    const bild = el("img", "neu-bild");
    bild.alt = "";
    const text = el("div", "neu-typtext");
    text.append(
      el("div", "neu-typtitel", `${typ.titel} ${typ.version}`),
      el("div", "leiste-hilfe", `${typ.herkunft === "eigen" ? t("eigen") : "TFTN"} · ${typ.name}`),
    );
    if (typ.beschreibung) text.append(el("div", "", typ.beschreibung));
    li.append(bild, text);
    li.classList.toggle("gewaehlt", typ === this.gewaehlt);
    const waehlen = () => {
      this.gewaehlt = typ;
      for (const x of liste.children) x.classList.toggle("gewaehlt", x === li);
      this.brauchtZeigen();
      this.pruefen();
    };
    li.addEventListener("click", waehlen);
    li.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        waehlen();
      }
    });
    // Vorschaubild nachladen (Rust legt den Typ dafür kurz im Temp-Ordner an).
    void this.a.vorschaubild(typ.name).then(
      (url) => (bild.src = url),
      () => bild.classList.add("neu-bild-fehlt"),
    );
    return li;
  }

  private pruefen(): boolean {
    const fehler = !this.gewaehlt
      ? t("Dokumenttyp wählen")
      : this.eltern.value.trim() === ""
        ? t("Ordner fehlt")
        : namePruefen(this.name.value);
    this.meldung.textContent = fehler ?? t("Wird angelegt in {pfad}", { pfad: zielPfad(this.eltern.value.trim(), this.name.value) });
    this.meldung.classList.toggle("dok-meldung-fehler", fehler !== null && this.name.value !== "");
    this.anlegenKnopf.disabled = fehler !== null || this.laeuft;
    return fehler === null;
  }

  private async anlegen() {
    if (!this.pruefen() || !this.gewaehlt || this.laeuft) return;
    this.laeuft = true;
    this.anlegenKnopf.disabled = true;
    this.meldung.textContent = t("Lege an …");
    try {
      await this.a.anlegen(this.gewaehlt.name, this.eltern.value.trim(), this.name.value.trim(), this.schriften.checked);
      this.meldung.textContent = t("Angelegt; öffne das neue Projekt …");
      await this.a.wechseln();
    } catch (fehler) {
      this.laeuft = false;
      this.anlegenKnopf.disabled = false;
      // Die Meldung aus Rust bleibt stehen, bis man etwas ändert.
      this.meldung.textContent = String(fehler);
      this.meldung.classList.add("dok-meldung-fehler");
      return;
    }
    this.laeuft = false;
  }
}

/**
 * Pfad eintippen, wo es keinen Systemdialog gibt (Windows): „Öffnen …“ fragt so nach dem
 * Projektordner (Abschlussprüfung AP8). Liefert den Pfad oder null bei Abbruch.
 */
export class PfadDialog {
  private huelle: HTMLDivElement;
  private abbrechen: (() => void) | null = null;

  constructor() {
    this.huelle = el("div", "neu-dialog pfad-dialog");
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    document.body.append(this.huelle);
    dialogAnmelden(this);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  /** Wie „Abbrechen“. */
  schliessen() {
    this.abbrechen?.();
  }

  /** Name und „Schriften einbinden“ beim ersten Speichern. */
  async namenFragen(titel: string, vorgabe = ""): Promise<{ name: string; schriften: boolean } | null> {
    const haken = el("input");
    haken.type = "checkbox";
    const zeile = el("label", "schriften-haken");
    zeile.append(haken, " ", SCHRIFTEN_HINWEIS);
    const name = await this.fragen(titel, vorgabe, { platzhalter: t("Name, z. B. Mein Artikel"), knopf: t("Speichern") }, zeile);
    return name === null ? null : { name, schriften: haken.checked };
  }

  fragen(
    titel: string,
    vorgabe = "",
    mit?: { platzhalter: string; knopf: string },
    zusatz?: HTMLElement,
  ): Promise<string | null> {
    return new Promise((fertig) => {
      const feld = el("input", "neu-eltern");
      feld.type = "text";
      feld.value = vorgabe;
      feld.placeholder = mit?.platzhalter ?? t("z. B. C:\\Users\\…\\Tresorit\\Büro\\Artikel");
      const ok = el("button", "", mit?.knopf ?? t("Öffnen"));
      const ab = el("button", "", t("Abbrechen"));
      const ende = (wert: string | null) => {
        this.huelle.hidden = true;
        this.abbrechen = null;
        fertig(wert);
      };
      this.abbrechen = () => ende(null);
      ok.addEventListener("click", () => ende(feld.value.trim() || null));
      ab.addEventListener("click", () => ende(null));
      feld.addEventListener("keydown", (e) => {
        if (e.key === "Enter") ende(feld.value.trim() || null);
        if (e.key === "Escape") {
          e.preventDefault();
          ende(null);
        }
      });
      const knoepfe = el("div", "neu-ordnerzeile");
      knoepfe.append(ok, ab);
      const kopf = el("div", "neu-kopf");
      kopf.append(el("h2", "", titel), schliessenKnopf(() => ende(null)));
      this.huelle.replaceChildren(kopf, feld, ...(zusatz ? [zusatz] : []), knoepfe);
      this.huelle.hidden = false;
      feld.focus();
    });
  }
}
