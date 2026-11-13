// Marken B im Editor (PLAN-M2 AP6): kleine Marken hinter jedem einstellbaren Wert bzw. „+ titel“
// an der Stelle, wo ein fehlender Wert eingefügt würde. Ein Klick öffnet ein Popover mit dem
// Editor des Felds. Wann die Marken sichtbar sind, entscheidet `MarkenSchalter` (marken.ts).

import { type Extension, StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { anAnker } from "./platz";

/** Eine Marke wie in Rust (`MarkeJs`); Positionen UTF-16 = CodeMirror-Positionen. */
export interface MarkeJs {
  pos: number;
  aufruf: number;
  parameter: string;
  baustein: string;
  titel: string;
  gesperrt: string | null;
  fehlt: boolean;
}

export const setzeMarken = StateEffect.define<MarkeJs[]>();

class MarkenWidget extends WidgetType {
  constructor(
    readonly m: MarkeJs,
    readonly klick: (m: MarkeJs, dom: HTMLElement) => void,
  ) {
    super();
  }

  eq(o: MarkenWidget) {
    return JSON.stringify(o.m) === JSON.stringify(this.m);
  }

  toDOM() {
    const s = document.createElement("span");
    s.className = "code-marke" + (this.m.gesperrt ? " code-marke-gesperrt" : "") + (this.m.fehlt ? " code-marke-fehlt" : "");
    s.textContent = this.m.fehlt ? `+ ${this.m.titel}` : this.m.titel;
    s.title = this.m.gesperrt ? `${this.m.baustein} · ${this.m.titel}: ${this.m.gesperrt}` : `${this.m.baustein} · ${this.m.titel}`;
    s.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.klick(this.m, s);
    });
    return s;
  }

  ignoreEvent() {
    return true;
  }
}

/** Die Erweiterung für den Editor; Marken kommen über `setzeMarken`, bei jeder Textänderung fallen sie weg. */
export function markenErweiterung(klick: (m: MarkeJs, dom: HTMLElement) => void): Extension {
  return StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(alt, tr) {
      for (const e of tr.effects) {
        if (e.is(setzeMarken)) {
          const laenge = tr.state.doc.length;
          return Decoration.set(
            e.value
              .filter((m) => m.pos >= 0 && m.pos <= laenge)
              .map((m) => Decoration.widget({ widget: new MarkenWidget(m, klick), side: 1 }).range(m.pos)),
            true,
          );
        }
      }
      return tr.docChanged ? Decoration.none : alt;
    },
    provide: (f) => EditorView.decorations.from(f),
  });
}

/** Ein Popover am Element `anker`; schließt mit Esc oder einem Klick daneben. */
export class Popover {
  private huelle: HTMLDivElement;
  private schliesser: ((e: Event) => void) | null = null;

  /** `zurueck`: wohin der Fokus nach Esc geht (der Editor). */
  constructor(private zurueck?: () => void) {
    this.huelle = document.createElement("div");
    this.huelle.className = "popover";
    this.huelle.hidden = true;
    document.body.append(this.huelle);
    dialogAnmelden(this);
    this.huelle.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this.schliessen();
        this.zurueck?.();
      }
    });
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  zeigen(anker: HTMLElement | { x: number; y: number }, titel: string, inhalt: HTMLElement) {
    const kopf = document.createElement("div");
    kopf.className = "popover-kopf";
    kopf.append(titel, schliessenKnopf(() => this.schliessen()));
    this.huelle.replaceChildren(kopf, inhalt);
    this.huelle.hidden = false;
    const r = anker instanceof HTMLElement ? anker.getBoundingClientRect() : { left: anker.x, top: anker.y, bottom: anker.y };
    anAnker(this.huelle, { x: r.left, oben: r.top, unten: r.bottom });
    this.huelle.querySelector<HTMLElement>("input, textarea, select")?.focus();
    if (this.schliesser) document.removeEventListener("mousedown", this.schliesser, true);
    this.schliesser = (e: Event) => {
      if (!this.huelle.contains(e.target as Node)) this.schliessen();
    };
    // erst nach diesem Klick lauschen
    setTimeout(() => this.schliesser && document.addEventListener("mousedown", this.schliesser, true));
  }

  schliessen() {
    this.huelle.hidden = true;
    if (this.schliesser) document.removeEventListener("mousedown", this.schliesser, true);
    this.schliesser = null;
  }
}
