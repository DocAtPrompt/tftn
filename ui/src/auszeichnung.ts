// ⌘B/⌘I schalten um (AP3): Ist die Auswahl schon fett bzw. kursiv, wird die Auszeichnung entfernt
// statt erneut eingewickelt. Erkannt werden `#strong[…]`/`#emph[…]` und die Kurzform `*…*`/`_…_`,
// die Auswahl als ganzer Ausdruck oder als sein Inhalt.

export interface Entfernen {
  /** Ersetzt wird `von..bis` durch `einsetzen`. */
  von: number;
  bis: number;
  einsetzen: string;
  /** Neue Auswahl im geänderten Text. */
  auswahlVon: number;
  auswahlBis: number;
}

const FORMEN = {
  fett: [["#strong[", "]"], ["*", "*"]],
  kursiv: [["#emph[", "]"], ["_", "_"]],
} as const;

export function auszeichnungEntfernen(text: string, von: number, bis: number, art: "fett" | "kursiv"): Entfernen | null {
  if (bis <= von) return null;
  for (const [auf, zu] of FORMEN[art]) {
    const auswahl = text.slice(von, bis);
    // Auswahl ist der ganze Ausdruck.
    if (auswahl.length > auf.length + zu.length && auswahl.startsWith(auf) && auswahl.endsWith(zu)) {
      const innen = auswahl.slice(auf.length, auswahl.length - zu.length);
      if (ausgeglichen(innen)) return { von, bis, einsetzen: innen, auswahlVon: von, auswahlBis: von + innen.length };
    }
    // Auswahl ist der Inhalt.
    if (text.slice(von - auf.length, von) === auf && text.slice(bis, bis + zu.length) === zu && ausgeglichen(auswahl)) {
      const a = von - auf.length;
      return { von: a, bis: bis + zu.length, einsetzen: auswahl, auswahlVon: a, auswahlBis: a + auswahl.length };
    }
  }
  return null;
}

/** Eckige Klammern ausgeglichen: `#strong[a] b [c]` ist kein einzelner Aufruf. */
function ausgeglichen(s: string): boolean {
  let tiefe = 0;
  for (const c of s) {
    if (c === "[") tiefe++;
    else if (c === "]" && --tiefe < 0) return false;
  }
  return tiefe === 0;
}
