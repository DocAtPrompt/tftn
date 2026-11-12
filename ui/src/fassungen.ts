// Frühere Fassungen: Liste, Vorschau, Zurückholen. Abgelegt werden sie in Rust beim Speichern.

import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { t, zahl } from "./sprache";

export interface FassungJs {
  name: string;
  /** Unix-Sekunden */
  zeit: number;
  zeichen: number;
  wie_jetzt: boolean;
}

const zwei = (n: number) => String(n).padStart(2, "0");

function gleicherTag(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

/** „heute 09:05“, „gestern 23:59“, „Di 3.11. 08:00“, aus anderem Jahr mit Jahreszahl. */
export function fassungZeit(zeit: number, jetzt = new Date()): string {
  const d = new Date(zeit * 1000);
  const uhr = `${zwei(d.getHours())}:${zwei(d.getMinutes())}`;
  if (gleicherTag(d, jetzt)) return t("heute {uhr}", { uhr });
  const gestern = new Date(jetzt.getFullYear(), jetzt.getMonth(), jetzt.getDate() - 1);
  if (gleicherTag(d, gestern)) return t("gestern {uhr}", { uhr });
  const wochentag = [t("So"), t("Mo"), t("Di"), t("Mi"), t("Do"), t("Fr"), t("Sa")][d.getDay()];
  const tag = d.getDate();
  const monat = d.getMonth() + 1;
  if (d.getFullYear() === jetzt.getFullYear()) return t("{wochentag} {tag}.{monat}. {uhr}", { wochentag, tag, monat, uhr });
  return t("{wochentag} {tag}.{monat}.{jahr} {uhr}", { wochentag, tag, monat, jahr: d.getFullYear(), uhr });
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface FassungenAktionen {
  liste(): Promise<FassungJs[]>;
  text(name: string): Promise<string>;
  /** Ersetzt den Text im Editor (ein Undo-Schritt). */
  zurueckholen(name: string): Promise<void>;
}

export class FassungenDialog {
  private huelle: HTMLDivElement;
  private liste = el("div", "fassungen-liste");
  private vorschau = el("pre", "fassungen-vorschau");
  private knopf = el("button", "", t("Diese Fassung zurückholen"));
  private gewaehlt: string | null = null;

  constructor(private a: FassungenAktionen) {
    this.huelle = el("div", "neu-dialog fassungen-dialog");
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    const kopf = el("div", "neu-kopf");
    kopf.append(el("h2", "", t("Frühere Fassungen")), schliessenKnopf(() => this.schliessen()));
    const hilfe = el(
      "p",
      "fassungen-hilfe",
      t(
        "Beim Sichern hält TFTN höchstens alle zehn Minuten eine Fassung fest: vom letzten Tag alle, davor eine je Tag, 60 Tage lang. Zurückholen ersetzt den Text; Rückgängig (Cmd/Strg+Z) bringt den jetzigen wieder.",
      ),
    );
    const rumpf = el("div", "fassungen-rumpf");
    rumpf.append(this.liste, this.vorschau);
    const unten = el("div", "neu-ordnerzeile");
    unten.append(this.knopf);
    this.knopf.disabled = true;
    this.knopf.addEventListener("click", () => void this.zurueck());
    this.huelle.append(kopf, hilfe, rumpf, unten);
    document.body.append(this.huelle);
    dialogAnmelden(this);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  schliessen() {
    this.huelle.hidden = true;
  }

  async zeigen() {
    this.gewaehlt = null;
    this.knopf.disabled = true;
    this.vorschau.textContent = "";
    this.liste.replaceChildren(el("p", "fassungen-hilfe", t("Lade …")));
    this.huelle.hidden = false;
    let alle: FassungJs[];
    try {
      alle = await this.a.liste();
    } catch (fehler) {
      this.liste.replaceChildren(el("p", "fassungen-hilfe", String(fehler)));
      return;
    }
    if (alle.length === 0) {
      this.liste.replaceChildren(
        el("p", "fassungen-hilfe", t("Noch keine Fassung. Die erste entsteht beim nächsten Sichern.")),
      );
      return;
    }
    const knoepfe = alle.map((f) => {
      const b = el("button", "fassung");
      b.append(el("span", "", fassungZeit(f.zeit)), el("span", "fassung-zahl", t("{zahl} Zeichen", { zahl: zahl(f.zeichen) })));
      if (f.wie_jetzt) b.append(el("span", "fassung-zahl", t("wie jetzt")));
      b.addEventListener("click", () => void this.waehlen(f, b, knoepfe));
      return b;
    });
    this.liste.replaceChildren(...knoepfe);
    knoepfe[0].focus();
  }

  private async waehlen(f: FassungJs, b: HTMLButtonElement, alle: HTMLButtonElement[]) {
    for (const k of alle) k.classList.toggle("gewaehlt", k === b);
    this.gewaehlt = f.name;
    this.knopf.disabled = f.wie_jetzt;
    try {
      this.vorschau.textContent = await this.a.text(f.name);
    } catch (fehler) {
      this.vorschau.textContent = String(fehler);
      this.knopf.disabled = true;
    }
  }

  private async zurueck() {
    if (!this.gewaehlt) return;
    try {
      await this.a.zurueckholen(this.gewaehlt);
      this.schliessen();
    } catch (fehler) {
      this.vorschau.textContent = String(fehler);
    }
  }
}
