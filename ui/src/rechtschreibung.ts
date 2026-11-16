// Rechtschreibung nach Dokumentsprache: Rust findet unbekannte Wörter im Fließtext, hier werden sie
// unterwellt; Rechtsklick zeigt Vorschläge, „Ins Wörterbuch“ und „Ignorieren“.

import { type EditorState, type Extension, StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView } from "@codemirror/view";
import { t } from "./sprache";
import { schwebeMenue } from "./schwebemenue";

export interface RechtschreibungJs {
  rev: number;
  sprache: string;
  falsch: [number, number][];
}

const falsch = Decoration.mark({ class: "cm-falsch" });

export const falschSetzen = StateEffect.define<[number, number][]>();

/** Unterwellte Wörter; folgen Änderungen, bis die nächste Prüfung sie ersetzt. */
export const falschFeld = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deko, tr) {
    deko = deko.map(tr.changes);
    for (const e of tr.effects) {
      if (e.is(falschSetzen)) {
        const laenge = tr.state.doc.length;
        deko = Decoration.set(
          e.value.filter(([von, bis]) => von < bis && bis <= laenge).map(([von, bis]) => falsch.range(von, bis)),
          true,
        );
      }
    }
    return deko;
  },
  provide: (f) => EditorView.decorations.from(f),
});

/** Bereiche ohne ignorierte Wörter (Groß-/Kleinschreibung zählt). */
export function ohneIgnorierte(state: EditorState, bereiche: [number, number][], ignoriert: Set<string>) {
  return bereiche.filter(([von, bis]) => bis <= state.doc.length && !ignoriert.has(state.sliceDoc(von, bis)));
}

/** Das unterwellte Wort an `pos`, sonst `null`. */
export function wortAn(state: EditorState, pos: number): { von: number; bis: number; wort: string } | null {
  let treffer: { von: number; bis: number; wort: string } | null = null;
  state.field(falschFeld).between(pos, pos, (von, bis) => {
    if (von <= pos && pos <= bis) {
      treffer = { von, bis, wort: state.sliceDoc(von, bis) };
      return false;
    }
  });
  return treffer;
}

export interface RechtschreibAktionen {
  vorschlaege(wort: string): Promise<string[]>;
  aufnehmen(wort: string): Promise<void>;
  ignorieren(wort: string): void;
}

/** Rechtsklick auf ein unterwelltes Wort: schwebendes Menü wie in der Vorschau. */
export function rechtschreibMenue(a: RechtschreibAktionen): Extension {
  const menue = schwebeMenue();
  const zeile = menue.zeile;

  return [
    falschFeld,
    EditorView.domEventHandlers({
      contextmenu(e, view) {
        const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
        const w = pos === null ? null : wortAn(view.state, pos);
        if (!w) return false;
        e.preventDefault();
        const ersetzen = (neu: string) => {
          // Steht noch dasselbe Wort da? Sonst nichts tun.
          if (view.state.sliceDoc(w.von, w.bis) !== w.wort) return;
          view.dispatch({ changes: { from: w.von, to: w.bis, insert: neu }, userEvent: "input.rechtschreibung" });
          view.focus();
        };
        const unten = [
          zeile(t("Ins Wörterbuch aufnehmen"), "menue-eintrag", () => void a.aufnehmen(w.wort)),
          zeile(t("Ignorieren"), "menue-eintrag", () => a.ignorieren(w.wort)),
        ];
        const warten = zeile(t("Vorschläge …"), "menue-gruppe");
        const anker = { x: e.clientX, oben: e.clientY, unten: e.clientY };
        menue.zeigen([zeile(w.wort, "menue-titel"), warten, ...unten], anker);
        a.vorschlaege(w.wort).then(
          (liste) => {
            if (!menue.offen()) return;
            const zeilen = liste.length
              ? liste.map((v) => zeile(v, "menue-eintrag", () => ersetzen(v)))
              : [zeile(t("Keine Vorschläge"), "menue-gruppe")];
            warten.replaceWith(...zeilen);
            menue.anpassen(anker); // gewachsen: wieder ganz ins Fenster
          },
          () => warten.replaceWith(zeile(t("Keine Vorschläge"), "menue-gruppe")),
        );
        return true;
      },
    }),
  ];
}
