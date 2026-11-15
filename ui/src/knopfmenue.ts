// Kleines Menü unter einem Knopf der Kopfzeile (etwa „Neu“). Schließt mit Esc, einem Klick daneben
// oder nach einer Wahl.

import { bereich, dialogAnmelden } from "./dialoge";
import { ikoneSvg } from "./ikonen";

export interface MenuePunkt {
  text: string;
  titel?: string;
  ikone?: string;
  aktion: () => void;
}

/** Vergrößerung der Oberfläche (Einstellungen); Lage und Maße rechnen sich in ungezoomten Pixeln. */
export function oberflaechenZoom(): number {
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--ui-zoom")) || 1;
}

export class KnopfMenue {
  private huelle = document.createElement("div");

  constructor(
    private knopf: HTMLElement,
    private punkte: () => MenuePunkt[],
  ) {
    this.huelle.className = "knopf-menue";
    this.huelle.setAttribute("role", "menu");
    this.huelle.hidden = true;
    document.body.append(this.huelle);
    knopf.setAttribute("aria-haspopup", "menu");
    knopf.addEventListener("click", () => (this.offen ? this.schliessen() : this.oeffnen()));
    dialogAnmelden(this);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  enthaelt(ziel: Node): boolean {
    return bereich(this.huelle, this.knopf)(ziel);
  }

  schliessen() {
    this.huelle.hidden = true;
  }

  private oeffnen() {
    this.huelle.replaceChildren(
      ...this.punkte().map((p) => {
        const b = document.createElement("button");
        b.setAttribute("role", "menuitem");
        b.innerHTML = ikoneSvg(p.ikone ?? "");
        b.append(p.text);
        if (p.titel) b.title = p.titel;
        b.addEventListener("click", () => {
          this.schliessen();
          p.aktion();
        });
        return b;
      }),
    );
    const zoom = oberflaechenZoom();
    const r = this.knopf.getBoundingClientRect();
    this.huelle.style.top = `${(r.bottom + 4) / zoom}px`;
    this.huelle.style.left = `${Math.max(8, r.left) / zoom}px`;
    this.huelle.hidden = false;
    this.huelle.querySelector("button")?.focus();
  }
}
