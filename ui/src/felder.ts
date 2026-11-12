// Felder eingefügter Bausteine (PLAN-M2 AP2, Entwurf 4 „A“): Tab springt zum nächsten Feld,
// Umschalt+Tab zurück, Escape beendet. Eigenes StateField statt der Snippet-Felder von CodeMirror,
// deren `${…}`/`#{…}`-Syntax mit Typst kollidiert (PLAN-M2, Risiko 1).

import {
  EditorSelection,
  type EditorState,
  type Extension,
  Prec,
  type Range,
  StateEffect,
  StateField,
  type TransactionSpec,
} from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType, keymap } from "@codemirror/view";
import { isolateHistory } from "@codemirror/commands";
import type { EingesetztJs } from "./bausteine";

/** Benutzerereignis einer Baustein-Einfügung. */
export const BAUSTEIN_EREIGNIS = "input.tftn-baustein";

export interface FeldBereich {
  name: string;
  von: number;
  bis: number;
}

interface Zustand {
  bereiche: FeldBereich[];
  aktiv: number;
  /** Wohin der letzte Tab führt: `${cursor}` oder das Ende des Schnipsels. */
  ende: number;
}

const setzeFelder = StateEffect.define<Zustand | null>();
const waehleFeld = StateEffect.define<number>();

const feldZustand = StateField.define<Zustand | null>({
  create: () => null,
  update(z, tr) {
    let eigene = false;
    for (const e of tr.effects) {
      if (e.is(setzeFelder)) {
        z = e.value;
        eigene = true;
      }
    }
    if (!eigene && z && tr.docChanged) {
      // Undo/Redo und eine Änderung, die mehr als ein ganzes Feld überdeckt, beenden die Felder;
      // sonst landeten alle Felder auf dem ersetzten Stück und Tab bliebe gekapert.
      const ueberdeckt = (b: FeldBereich) => {
        let ja = false;
        tr.changes.iterChangedRanges((von, bis) => {
          if (von <= b.von && bis >= b.bis && bis - von > b.bis - b.von) ja = true;
        });
        return ja;
      };
      if (tr.isUserEvent("undo") || tr.isUserEvent("redo") || z.bereiche.some(ueberdeckt)) return null;
      // An einer Feldgrenze Getipptes wächst ins Feld hinein.
      z = {
        ...z,
        bereiche: z.bereiche.map((b) => ({
          name: b.name,
          von: tr.changes.mapPos(b.von, -1),
          bis: tr.changes.mapPos(b.bis, 1),
        })),
        ende: tr.changes.mapPos(z.ende, 1),
      };
    }
    for (const e of tr.effects) {
      if (e.is(waehleFeld) && z) {
        z = { ...z, aktiv: e.value };
        eigene = true;
      }
    }
    if (!eigene && z && (tr.selection || tr.docChanged)) {
      const kopf = tr.state.selection.main.head;
      if (!z.bereiche.some((b) => b.von <= kopf && kopf <= b.bis)) z = null;
    }
    return z;
  },
});

class LeeresFeld extends WidgetType {
  constructor(readonly name: string) {
    super();
  }
  eq(other: LeeresFeld) {
    return other.name === this.name;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "tftn-feld-leer";
    s.textContent = this.name;
    return s;
  }
  ignoreEvent() {
    return false;
  }
}

const feldDeko = EditorView.decorations.compute([feldZustand], (state): DecorationSet => {
  const z = state.field(feldZustand);
  if (!z) return Decoration.none;
  const liste: Range<Decoration>[] = z.bereiche.map((b, i) => {
    const klasse = i === z.aktiv ? "tftn-feld aktiv" : "tftn-feld";
    return b.von === b.bis
      ? Decoration.widget({ widget: new LeeresFeld(b.name), side: 1 }).range(b.von)
      : Decoration.mark({ class: klasse }).range(b.von, b.bis);
  });
  return Decoration.set(liste, true);
});

/** Die Felder der letzten Einfügung, solange sie aktiv sind. */
export function aktiveFelder(state: EditorState): FeldBereich[] | null {
  return state.field(feldZustand, false)?.bereiche ?? null;
}

/**
 * Eine Transaktion: Importzeile (falls nötig) und Schnipsel statt `von..bis`, ein Undo-Schritt.
 * Ausgewählt wird das erste Feld, sonst `cursor`, sonst das Ende des Schnipsels.
 */
export function einsetzenSpec(
  state: EditorState,
  von: number,
  bis: number,
  e: EingesetztJs,
  imp: { pos: number; text: string } | null,
): TransactionSpec {
  const changes = state.changes([
    ...(imp ? [{ from: imp.pos, insert: imp.text }] : []),
    { from: von, to: bis, insert: e.text },
  ]);
  // Liegt der Import vor (oder an) der Einfügestelle, verschiebt er sie um seine Länge.
  const start = von + (imp && imp.pos <= von ? imp.text.length : 0);
  const bereiche = e.felder.map((f) => ({ name: f.name, von: start + f.von, bis: start + f.bis }));
  const ende = start + (e.cursor ?? e.text.length);
  const erstes = bereiche[0];
  return {
    changes,
    selection: erstes ? EditorSelection.range(erstes.von, erstes.bis) : EditorSelection.cursor(ende),
    effects: setzeFelder.of(bereiche.length ? { bereiche, aktiv: 0, ende } : null),
    annotations: isolateHistory.of("full"),
    userEvent: BAUSTEIN_EREIGNIS,
    scrollIntoView: true,
  };
}

/** Zum nächsten (`1`) bzw. vorigen (`-1`) Feld; `null` ohne aktive Felder. */
export function feldWechsel(state: EditorState, richtung: 1 | -1): TransactionSpec | null {
  const z = state.field(feldZustand, false);
  if (!z) return null;
  const ziel = Math.max(0, z.aktiv + richtung);
  if (ziel >= z.bereiche.length) {
    return { selection: EditorSelection.cursor(z.ende), effects: setzeFelder.of(null), scrollIntoView: true };
  }
  const b = z.bereiche[ziel];
  return { selection: EditorSelection.range(b.von, b.bis), effects: waehleFeld.of(ziel), scrollIntoView: true };
}

function wechseln(view: EditorView, richtung: 1 | -1): boolean {
  const spec = feldWechsel(view.state, richtung);
  if (!spec) return false;
  view.dispatch(spec);
  return true;
}

export const felder: Extension = [
  feldZustand,
  feldDeko,
  Prec.high(
    keymap.of([
      { key: "Tab", run: (v) => wechseln(v, 1) },
      { key: "Shift-Tab", run: (v) => wechseln(v, -1) },
      {
        key: "Escape",
        run: (v) => {
          if (!v.state.field(feldZustand, false)) return false;
          v.dispatch({ effects: setzeFelder.of(null) });
          return true;
        },
      },
    ]),
  ),
];
