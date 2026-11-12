// Protokoll zwischen Editor und Rust-Spiegel (KONZEPT 4.1). Reine Logik, ohne Tauri, für vitest.

import type { ChangeSet } from "@codemirror/state";

/** Eine Änderung in UTF-16-Positionen des Texts vor der Transaktion (wie `tftn_core::spiegel::Aenderung`). */
export interface Aenderung {
  von: number;
  bis: number;
  text: string;
}

/** Jede wievielte Revision eine Prüfsumme mitgeschickt wird (PLAN-M1 AP3). */
export const PRUEFSUMME_ALLE = 50;

/** Änderungen eines ChangeSets, aufsteigend und in Positionen des alten Dokuments. */
export function aenderungenAus(changes: ChangeSet): Aenderung[] {
  const liste: Aenderung[] = [];
  changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
    // CodeMirror führt Zeilen getrennt; als Text immer mit "\n" verbunden.
    liste.push({ von: fromA, bis: toA, text: inserted.sliceString(0, inserted.length, "\n") });
  });
  return liste;
}

const FNV_START = 0xcbf29ce484222325n;
const FNV_PRIM = 0x100000001b3n;
const MASKE = 0xffffffffffffffffn;
const kodierer = new TextEncoder();

/** FNV-1a 64 über die UTF-8-Bytes, hexadezimal mit 16 Stellen (wie `tftn_core::spiegel::pruefsumme`). */
export function pruefsumme(text: string): string {
  let h = FNV_START;
  for (const b of kodierer.encode(text)) {
    h ^= BigInt(b);
    h = (h * FNV_PRIM) & MASKE;
  }
  return h.toString(16).padStart(16, "0");
}
