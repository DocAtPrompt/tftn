// Marken B (PLAN-M2 AP6, Entwurf 4 B): wann Marken im Code sichtbar sind, als reiner Zustand.
// ⌥ bzw. Alt allein 400 ms halten zeigt sie, Loslassen verbirgt sie. Jede andere Taste während des
// Haltens, ein Mausklick oder Strg/Meta/Umschalt dazu (AltGr = Strg+Alt) bricht ab – so öffnet das
// Tippen von ⌥- und AltGr-Zeichen nie etwas. Das Kürzel „Werte an dieser Stelle“ zeigt sie, bis Esc,
// ein Klick in den Text oder eine Textänderung kommt.

export type MarkenEreignis =
  | { art: "taste-ab"; key: string; alt: boolean; ctrl: boolean; meta: boolean; shift: boolean; zeit: number }
  | { art: "taste-auf"; key: string; zeit: number }
  | { art: "zeit"; zeit: number }
  | { art: "maus" }
  | { art: "kuerzel" }
  | { art: "escape" }
  | { art: "dokument-geaendert" };

/** So lange muss Alt allein gehalten werden. */
export const HALTEN_MS = 400;

export class MarkenSchalter {
  private altUnten = false;
  /** Anfang des Haltens; `null`, wenn kein gültiges Halten läuft. */
  private seit: number | null = null;
  private gehalten = false;
  private dauerhaft = false;

  /** Neuer Zustand nach dem Ereignis; `sichtbar` sagt, ob Marken jetzt gezeigt werden. */
  ereignis(e: MarkenEreignis): { sichtbar: boolean } {
    switch (e.art) {
      case "taste-ab":
        if (e.key === "Alt") {
          if (!this.altUnten) {
            // Wiederholungen beim Halten zählen nicht als neues Drücken.
            this.altUnten = true;
            this.seit = e.ctrl || e.meta || e.shift ? null : e.zeit;
          }
        } else {
          this.abbrechen();
        }
        break;
      case "taste-auf":
        if (e.key === "Alt") {
          this.altUnten = false;
          this.abbrechen();
        }
        break;
      case "zeit":
        if (this.altUnten && this.seit !== null && e.zeit - this.seit >= HALTEN_MS) this.gehalten = true;
        break;
      case "maus":
        this.abbrechen();
        this.dauerhaft = false;
        break;
      case "kuerzel":
        this.dauerhaft = true;
        break;
      case "escape":
        this.abbrechen();
        this.dauerhaft = false;
        break;
      case "dokument-geaendert":
        this.dauerhaft = false;
        break;
    }
    return { sichtbar: this.gehalten || this.dauerhaft };
  }

  private abbrechen() {
    this.seit = null;
    this.gehalten = false;
  }
}
