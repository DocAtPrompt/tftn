// Kleines Menü unter einem Knopf der Kopfzeile (etwa „Neu“). Schließt mit Esc, Tab, einem Klick
// daneben oder nach einer Wahl; Pfeiltasten, Pos1 und Ende wandern durch die Punkte. Nach dem
// Schließen mit der Tastatur bekommt der Knopf den Fokus zurück.

import { dialogAnmelden } from "./dialoge";
import { ikoneSvg } from "./ikonen";
import { anAnker, knopfAnker, oberflaechenZoom } from "./platz";

export interface MenuePunkt {
  text: string;
  titel?: string;
  ikone?: string;
  aktion: () => void;
}

/** Was eine Taste im offenen Menü bewirkt: anderer Punkt, schließen oder nichts. */
export function menueTaste(taste: string, index: number, anzahl: number): { fokus: number } | "schliessen" | null {
  if (taste === "Tab") return "schliessen";
  if (anzahl === 0) return null;
  switch (taste) {
    case "ArrowDown":
      return { fokus: (index + 1) % anzahl };
    case "ArrowUp":
      return { fokus: index <= 0 ? anzahl - 1 : index - 1 };
    case "Home":
      return { fokus: 0 };
    case "End":
      return { fokus: anzahl - 1 };
    default:
      return null;
  }
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
    knopf.setAttribute("aria-expanded", "false");
    knopf.addEventListener("click", () => (this.offen ? this.schliessen() : this.oeffnen()));
    this.huelle.addEventListener("keydown", (e) => this.taste(e));
    dialogAnmelden(this, this.huelle, this.knopf);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }


  schliessen() {
    const hatteFokus = this.huelle.contains(document.activeElement);
    this.huelle.hidden = true;
    this.knopf.setAttribute("aria-expanded", "false");
    if (hatteFokus) this.knopf.focus();
  }

  private taste(e: KeyboardEvent) {
    const knoepfe = [...this.huelle.querySelectorAll<HTMLButtonElement>("button")];
    const was = menueTaste(e.key, knoepfe.indexOf(document.activeElement as HTMLButtonElement), knoepfe.length);
    if (was === "schliessen") {
      // Tab geht vom Knopf aus weiter (kein preventDefault).
      this.schliessen();
    } else if (was) {
      e.preventDefault();
      knoepfe[was.fokus].focus();
    }
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
    this.huelle.hidden = false;
    this.knopf.setAttribute("aria-expanded", "true");
    anAnker(this.huelle, knopfAnker(this.knopf), oberflaechenZoom());
    this.huelle.querySelector("button")?.focus();
  }
}
