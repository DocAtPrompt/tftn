// Breite der Seitenleiste: am linken Rand ziehen, gemerkt je Rechner (localStorage).

import { t } from "./sprache";

export const LEISTE_MIN = 280;
export const LEISTE_MAX = 720;
const MERKER = "tftn-leiste-breite";

/** Neue Breite beim Ziehen von `startX` nach `x`; höchstens `platz`, nie unter dem Minimum. */
export function leisteBreite(start: number, startX: number, x: number, platz: number): number {
  const ziel = start + (startX - x);
  return Math.round(Math.max(LEISTE_MIN, Math.min(ziel, LEISTE_MAX, platz)));
}

function gemerkt(): number | null {
  try {
    const n = Number(localStorage.getItem(MERKER));
    return n >= LEISTE_MIN && n <= LEISTE_MAX ? n : null;
  } catch {
    return null;
  }
}

/** Griff am linken Rand von `leiste`; Doppelklick setzt die Breite zurück. */
export function breiteZiehbar(leiste: HTMLElement) {
  const b = gemerkt();
  if (b !== null) leiste.style.width = `${b}px`;
  const griff = document.createElement("div");
  griff.className = "leiste-griff";
  griff.title = t("Ziehen ändert die Breite, Doppelklick setzt sie zurück");
  leiste.append(griff);
  griff.addEventListener("pointerdown", (d) => {
    d.preventDefault();
    griff.setPointerCapture(d.pointerId);
    const start = leiste.getBoundingClientRect().width;
    const platz = (leiste.parentElement?.getBoundingClientRect().width ?? window.innerWidth) - 240;
    const ziehen = (m: PointerEvent) => {
      leiste.style.width = `${leisteBreite(start, d.clientX, m.clientX, platz)}px`;
    };
    const ende = () => {
      griff.removeEventListener("pointermove", ziehen);
      griff.removeEventListener("pointerup", ende);
      griff.removeEventListener("pointercancel", ende);
      try {
        localStorage.setItem(MERKER, String(Math.round(leiste.getBoundingClientRect().width)));
      } catch {
        // ohne localStorage gilt die Breite bis zum Neuladen
      }
    };
    griff.addEventListener("pointermove", ziehen);
    griff.addEventListener("pointerup", ende);
    griff.addEventListener("pointercancel", ende);
  });
  griff.addEventListener("dblclick", () => {
    leiste.style.width = "";
    try {
      localStorage.removeItem(MERKER);
    } catch {
      // nichts gemerkt
    }
  });
}
