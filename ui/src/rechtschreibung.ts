// Rechtschreibung nach Dokumentsprache: Rust findet unbekannte Wörter im Fließtext, hier werden sie
// unterwellt; Rechtsklick zeigt Vorschläge, „Ins Wörterbuch“ und „Ignorieren“.

import { type EditorState, type Extension, StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView } from "@codemirror/view";
import { t } from "./sprache";

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
  const huelle = document.createElement("ul");
  huelle.className = "vorschau-menue";
  huelle.hidden = true;
  document.body.append(huelle);
  const schliessen = () => {
    huelle.hidden = true;
    document.removeEventListener("mousedown", daneben, true);
  };
  const daneben = (e: Event) => {
    if (!huelle.contains(e.target as Node)) schliessen();
  };
  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || huelle.hidden) return;
    e.preventDefault(); // Esc schließt nur das Menü, nicht auch die Schreibansicht
    schliessen();
  });
  const zeile = (text: string, klasse: string, aktion?: () => void) => {
    const li = document.createElement("li");
    li.className = klasse;
    li.textContent = text;
    if (aktion)
      li.addEventListener("click", () => {
        schliessen();
        aktion();
      });
    return li;
  };

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
        huelle.replaceChildren(zeile(w.wort, "menue-titel"), warten, ...unten);
        huelle.hidden = false;
        huelle.style.left = `${Math.min(e.clientX, window.innerWidth - 240)}px`;
        huelle.style.top = `${Math.min(e.clientY, window.innerHeight - 220)}px`;
        setTimeout(() => document.addEventListener("mousedown", daneben, true));
        a.vorschlaege(w.wort).then(
          (liste) => {
            if (huelle.hidden) return;
            const zeilen = liste.length
              ? liste.map((v) => zeile(v, "menue-eintrag", () => ersetzen(v)))
              : [zeile(t("Keine Vorschläge"), "menue-gruppe")];
            warten.replaceWith(...zeilen);
          },
          () => warten.replaceWith(zeile(t("Keine Vorschläge"), "menue-gruppe")),
        );
        return true;
      },
    }),
  ];
}
