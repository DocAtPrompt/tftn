// Palette „Baustein einfügen“ (PLAN-M2 AP2): über das Kürzel bzw. den Knopf, mit eigenem Suchfeld,
// damit eine Markierung im Editor erhalten bleibt und in `${auswahl}` wandert. Gesucht und geordnet
// wird wie im Menü nach „/“ (`menue`). Tastatur: ↑/↓ wählen, Enter einfügen, Esc schließen.

import { type EintragJs, type Modus, abschnitt, kuerzelText, menue } from "./bausteine";
import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { t } from "./sprache";

export class BausteinPalette {
  private huelle: HTMLDivElement;
  private suche: HTMLInputElement;
  private liste: HTMLUListElement;
  private fuss: HTMLDivElement;
  private treffer: EintragJs[] = [];
  private gewaehlt = 0;
  private modus: Modus = "markup";

  constructor(
    private eintraege: () => EintragJs[],
    private waehlen: (id: string) => void,
    private geschlossen: () => void,
  ) {
    this.huelle = document.createElement("div");
    this.huelle.id = "baustein-palette";
    this.huelle.hidden = true;
    this.huelle.innerHTML = `
      <div class="neu-kopf"><div class="palette-kopf">${t("Baustein einfügen")}</div></div>
      <div class="palette-felder palette-einzeln">
        <input class="palette-suche" placeholder="${t("Abbildung, Tabelle, Fußnote …")}" spellcheck="false" />
      </div>
      <ul class="palette-liste"></ul>
      <div class="palette-fuss"></div>`;
    document.body.append(this.huelle);
    this.huelle.querySelector(".neu-kopf")!.append(schliessenKnopf(() => this.schliessen()));
    dialogAnmelden(this);
    this.suche = this.huelle.querySelector(".palette-suche")!;
    this.liste = this.huelle.querySelector(".palette-liste")!;
    this.fuss = this.huelle.querySelector(".palette-fuss")!;
    this.suche.addEventListener("input", () => this.aktualisieren());
    this.huelle.addEventListener("keydown", (e) => this.taste(e));
    this.liste.addEventListener("click", (e) => {
      const li = (e.target as HTMLElement).closest("li[data-nr]") as HTMLElement | null;
      if (li) {
        this.gewaehlt = Number(li.dataset.nr);
        this.uebernehmen();
      }
    });
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  oeffnen(modus: Modus) {
    this.modus = modus;
    this.huelle.hidden = false;
    this.suche.value = "";
    this.suche.focus();
    this.aktualisieren();
  }

  schliessen() {
    this.huelle.hidden = true;
    this.geschlossen();
  }

  private aktualisieren() {
    this.treffer = menue(this.eintraege(), this.modus, this.suche.value);
    this.gewaehlt = 0;
    const zeilen: HTMLLIElement[] = [];
    let letzter = "";
    this.treffer.forEach((e, i) => {
      const name = abschnitt(e.herkunft).name;
      if (name !== letzter) {
        const kopf = document.createElement("li");
        kopf.className = "palette-abschnitt";
        kopf.textContent = name;
        zeilen.push(kopf);
        letzter = name;
      }
      const li = document.createElement("li");
      li.dataset.nr = String(i);
      li.className = "baustein-zeile";
      const titel = document.createElement("span");
      titel.textContent = e.titel;
      li.append(titel);
      if (e.kuerzel) {
        const k = document.createElement("code");
        k.textContent = kuerzelText(e.kuerzel);
        li.append(k);
      }
      zeilen.push(li);
    });
    this.liste.replaceChildren(...zeilen);
    this.markieren();
    const anzahl = this.treffer.length;
    this.fuss.textContent = anzahl
      ? anzahl === 1
        ? t("1 Baustein · Enter fügt ein · markierter Text wandert in den Baustein")
        : t("{anzahl} Bausteine · Enter fügt ein · markierter Text wandert in den Baustein", { anzahl })
      : t("Kein Baustein passt hierher ({modus}).", {
          modus: this.modus === "markup" ? t("Text") : this.modus === "code" ? t("Code") : t("Formel"),
        });
  }

  private markieren() {
    this.liste.querySelectorAll<HTMLElement>("li[data-nr]").forEach((li) => {
      const an = Number(li.dataset.nr) === this.gewaehlt;
      li.classList.toggle("gewaehlt", an);
      if (an) li.scrollIntoView({ block: "nearest" });
    });
  }

  private taste(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      this.schliessen();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = this.treffer.length;
      if (n) this.gewaehlt = (this.gewaehlt + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      this.markieren();
    } else if (e.key === "Enter") {
      e.preventDefault();
      this.uebernehmen();
    }
  }

  private uebernehmen() {
    const t = this.treffer[this.gewaehlt];
    if (!t) return;
    this.huelle.hidden = true;
    this.waehlen(t.id);
  }
}
