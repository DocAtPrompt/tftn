// Gemeinsames aller Dialoge: Schließen-Knopf und Esc, das auch wirkt, wenn der Dialog den Fokus verloren hat.

import { t } from "./sprache";

export interface Dialog {
  readonly offen: boolean;
  schliessen(): void;
}

/** Der Dialog, den Esc schließt: der zuletzt angemeldete offene (er liegt oben). */
export function escZiel(dialoge: readonly Dialog[]): Dialog | null {
  for (let i = dialoge.length - 1; i >= 0; i--) if (dialoge[i].offen) return dialoge[i];
  return null;
}

const dialoge: Dialog[] = [];

export function dialogAnmelden(d: Dialog) {
  dialoge.push(d);
}

export function einDialogOffen(): boolean {
  return escZiel(dialoge) !== null;
}

/** Einmal beim Start. Was ein Dialog oder der Editor schon selbst behandelt hat, bleibt unberührt. */
export function escAktivieren() {
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    const d = escZiel(dialoge);
    if (d) {
      e.preventDefault();
      d.schliessen();
    }
  });
}

export function schliessenKnopf(schliessen: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = "neu-zu";
  b.textContent = "×";
  b.title = t("Schließen (Esc)");
  b.addEventListener("click", schliessen);
  return b;
}
