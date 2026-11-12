// Fach „Formel“ (PLAN-M2 AP10): freies Tippen mit Vorschau daneben und eine Palette. Geschrieben wird
// erst auf „Übernehmen“ bzw. „Einfügen“, als ein Patch und ein Undo-Schritt. Die Logik steht in
// `formel.ts` (vitest); dies ist nur der DOM-Teil.

import type { EingesetztJs } from "./bausteine";
import { type Bereich, type FeldZustand, Folge, einsetzen, formelKnoepfe, nachEingabe, naechstesFeld, symbolEinsetzen } from "./formel";
import { t } from "./sprache";

export interface FormelStandJs {
  text: string | null;
  block: boolean;
  einfuegbar: boolean;
}

export interface FormelBildJs {
  bild: string | null;
  fehler: string[];
}

export interface PaletteJs {
  aufbau: { id: string; titel: string }[];
  symbole: { name: string; symbole: { text: string; zeichen: string }[] }[];
}

export interface FormelAktionen {
  stand(pos: number): Promise<FormelStandJs>;
  vorschau(text: string, block: boolean): Promise<FormelBildJs>;
  palette(): Promise<PaletteJs>;
  baustein(id: string, auswahl: string): Promise<EingesetztJs>;
  /** `true`, wenn der Patch im Editor angekommen ist. */
  setzen(pos: number, erwartet: string, text: string): Promise<boolean>;
  einfuegen(pos: number, text: string, block: boolean): Promise<boolean>;
  hinweis(text: string): void;
  zumEditor(): void;
  /** Aktuelle Position im Editor */
  cursor(): number;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class FormelFach {
  readonly huelle = el("div", "formel-fach");
  private feld = el("textarea", "formel-feld");
  private bild = el("img", "formel-bild");
  private fehler = el("p", "dok-fehler");
  private uebernehmen = el("button", "", t("Übernehmen"));
  private einfuegen = el("button", "", t("Einfügen"));
  private verwerfen = el("button", "", t("Verwerfen"));
  private block = el("input");
  private imSatz = el("input");
  private art = el("div", "formel-art");
  private felder: Bereich[] = [];
  private stand: FormelStandJs | null = null;
  private pos = 0;
  private laeuft = false;
  private zeitgeber: number | undefined;
  private bilder = new Folge();
  private staende = new Folge();
  private palette = el("div", "formel-palette");
  private paletteDa = false;

  constructor(private a: FormelAktionen) {
    this.feld.spellcheck = false;
    this.feld.rows = 4;
    this.feld.placeholder = t("Formel tippen oder aus der Palette wählen, z. B. x^2 + 1");
    this.bild.alt = t("Vorschau der Formel");
    this.bild.hidden = true;
    this.fehler.hidden = true;
    this.block.type = "radio";
    this.block.name = "formel-art";
    this.block.checked = true;
    this.imSatz.type = "radio";
    this.imSatz.name = "formel-art";
    const als = el("label");
    als.append(this.block, " ", t("Als Block"));
    const satz = el("label");
    satz.append(this.imSatz, " ", t("Im Satz"));
    this.art.append(als, satz);
    for (const r of [this.block, this.imSatz]) r.addEventListener("change", () => this.vorschauPlanen());

    const knoepfe = el("div", "formel-knoepfe");
    knoepfe.append(this.uebernehmen, this.einfuegen, this.verwerfen);
    this.uebernehmen.title = "Mod+Enter";
    this.uebernehmen.addEventListener("click", () => void this.schreiben());
    this.einfuegen.addEventListener("click", () => void this.schreiben());
    this.verwerfen.addEventListener("click", () => this.zuruecksetzen());
    this.huelle.append(this.feld, this.art, this.bild, this.fehler, knoepfe, this.palette);

    this.feld.addEventListener("input", () => {
      const z = nachEingabe(this.zustand(), this.feld.value, this.feld.selectionStart, this.feld.selectionEnd);
      this.felder = z.felder;
      this.knoepfe();
      this.vorschauPlanen();
    });
    this.feld.addEventListener("keydown", (e) => this.taste(e));
  }

  /** Der Text vor der letzten Eingabe zählt für `nachEingabe`; deshalb merkt sich `zuletzt` ihn. */
  private zuletzt = "";

  private zustand(): FeldZustand {
    const z = { text: this.zuletzt, von: this.feld.selectionStart, bis: this.feld.selectionEnd, felder: this.felder };
    this.zuletzt = this.feld.value;
    return z;
  }

  private anwenden(z: FeldZustand) {
    this.feld.value = z.text;
    this.zuletzt = z.text;
    this.felder = z.felder;
    this.feld.focus();
    this.feld.setSelectionRange(z.von, z.bis);
    this.knoepfe();
    this.vorschauPlanen();
  }

  private ungeschrieben(): boolean {
    return this.feld.value !== (this.stand?.text ?? "");
  }

  private knoepfe() {
    const k = formelKnoepfe({
      text: this.stand?.text ?? null,
      einfuegbar: this.stand?.einfuegbar ?? false,
      eingabe: this.feld.value,
      laeuft: this.laeuft,
    });
    const inFormel = this.stand?.text != null;
    this.uebernehmen.hidden = !inFormel;
    this.einfuegen.hidden = inFormel;
    this.art.hidden = inFormel;
    this.uebernehmen.disabled = !k.uebernehmen;
    this.einfuegen.disabled = !k.einfuegen;
    this.verwerfen.disabled = !this.ungeschrieben();
    this.einfuegen.title = k.titel;
    if (inFormel) this.uebernehmen.title = k.titel || "Mod+Enter";
  }

  private istBlock(): boolean {
    return this.stand?.text != null ? this.stand.block : this.block.checked;
  }

  private vorschauPlanen() {
    clearTimeout(this.zeitgeber);
    this.zeitgeber = window.setTimeout(() => void this.vorschau(), 200);
  }

  private async vorschau() {
    const nr = this.bilder.neu();
    try {
      const b = await this.a.vorschau(this.feld.value, this.istBlock());
      if (!this.bilder.aktuell(nr)) return;
      this.bild.hidden = !b.bild;
      if (b.bild) this.bild.src = b.bild;
      this.fehler.hidden = b.fehler.length === 0;
      this.fehler.textContent = b.fehler.join(" ");
    } catch (f) {
      if (this.bilder.aktuell(nr)) {
        this.bild.hidden = true;
        this.fehler.hidden = false;
        this.fehler.textContent = String(f);
      }
    }
  }

  /** Der Text im Editor hat sich geändert: Die gemerkte Stelle wandert mit. */
  aenderung(abbilden: (pos: number) => number) {
    this.pos = abbilden(this.pos);
  }

  /** Lädt den Stand am Cursor. Ungeschriebene Eingaben bleiben stehen. */
  async zeichnen(pos: number): Promise<void> {
    await this.paletteBauen();
    const gebunden = this.ungeschrieben() && this.stand?.text != null;
    if (gebunden) return;
    this.pos = pos;
    const nr = this.staende.neu();
    let s: FormelStandJs;
    try {
      s = await this.a.stand(pos);
    } catch {
      return; // veraltet: Der nächste Cursor lädt neu.
    }
    if (!this.staende.aktuell(nr)) return;
    if (this.ungeschrieben()) {
      // Mit neuer Eingabe wird die Formel am Cursor nicht übernommen, nur der Ort zählt.
      this.stand = { text: null, block: false, einfuegbar: s.text === null && s.einfuegbar };
      this.knoepfe();
      return;
    }
    this.stand = s;
    this.feld.value = s.text ?? "";
    this.zuletzt = this.feld.value;
    this.felder = [];
    this.knoepfe();
    void this.vorschau();
  }

  private zuruecksetzen() {
    this.feld.value = this.stand?.text ?? "";
    this.zuletzt = this.feld.value;
    this.felder = [];
    this.knoepfe();
    void this.zeichnen(this.a.cursor());
  }

  private async schreiben() {
    if (this.laeuft) return;
    const text = this.feld.value;
    const k = formelKnoepfe({
      text: this.stand?.text ?? null,
      einfuegbar: this.stand?.einfuegbar ?? false,
      eingabe: text,
      laeuft: false,
    });
    if (!k.uebernehmen && !k.einfuegen) return;
    this.laeuft = true;
    this.knoepfe();
    try {
      const ok = k.uebernehmen
        ? await this.a.setzen(this.pos, this.stand?.text ?? "", text)
        : await this.a.einfuegen(this.pos, text, this.block.checked);
      if (ok) {
        // Der Cursor-Lauf nach dem Patch lädt den neuen Stand.
        this.stand = { text: text.trim(), block: this.istBlock(), einfuegbar: false };
        this.feld.value = text.trim();
        this.zuletzt = this.feld.value;
        this.felder = [];
      }
    } catch (f) {
      this.a.hinweis(String(f));
    } finally {
      this.laeuft = false;
      this.knoepfe();
    }
  }

  private taste(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      void this.schreiben();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      this.zuruecksetzen();
      this.a.zumEditor();
    } else if (e.key === "Tab") {
      const z = naechstesFeld({ text: this.feld.value, von: this.feld.selectionStart, bis: this.feld.selectionEnd, felder: this.felder }, e.shiftKey);
      if (z) {
        // Ohne weiteres Feld geht Tab normal weiter, das Fach bleibt per Tastatur bedienbar.
        e.preventDefault();
        this.feld.setSelectionRange(z.von, z.bis);
      }
    }
  }

  private async paletteBauen() {
    if (this.paletteDa) return;
    this.paletteDa = true;
    let p: PaletteJs;
    try {
      p = await this.a.palette();
    } catch (f) {
      this.paletteDa = false;
      this.a.hinweis(String(f));
      return;
    }
    const knopf = (text: string, titel: string, klasse: string, aktion: () => void) => {
      const b = el("button", klasse, text);
      b.title = titel;
      // Die Auswahl im Textfeld bleibt, wenn der Knopf den Fokus nicht nimmt.
      b.addEventListener("mousedown", (e) => e.preventDefault());
      b.addEventListener("click", aktion);
      return b;
    };
    const aufbau = el("div", "formel-gruppe");
    aufbau.append(el("h4", "", t("Aufbau")));
    const reihe = el("div", "formel-reihe");
    for (const e of p.aufbau) {
      reihe.append(knopf(e.titel, e.titel, "formel-aufbau", () => void this.bausteinEinfuegen(e.id)));
    }
    aufbau.append(reihe);
    this.palette.replaceChildren(aufbau);
    for (const g of p.symbole) {
      const d = el("div", "formel-gruppe");
      d.append(el("h4", "", g.name));
      const zeile = el("div", "formel-reihe");
      for (const s of g.symbole) {
        zeile.append(
          knopf(s.zeichen, s.text, "formel-symbol", () => {
            const z = this.aktuell();
            this.anwenden(symbolEinsetzen(z, s.text));
          }),
        );
      }
      d.append(zeile);
      this.palette.append(d);
    }
  }

  private aktuell(): FeldZustand {
    return { text: this.feld.value, von: this.feld.selectionStart, bis: this.feld.selectionEnd, felder: this.felder };
  }

  private async bausteinEinfuegen(id: string) {
    const z = this.aktuell();
    try {
      const e = await this.a.baustein(id, z.text.slice(z.von, z.bis));
      this.anwenden(einsetzen(this.aktuell(), e));
    } catch (f) {
      this.a.hinweis(String(f));
    }
  }
}
