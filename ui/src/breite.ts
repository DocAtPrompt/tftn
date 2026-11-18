// Breite der Seitenleiste (am linken Rand ziehen) und Aufteilung zwischen Text und Vorschau (Trenner
// dazwischen), beide gemerkt je Rechner (localStorage).

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

// Trenner zwischen Text und Vorschau: Anteil des Textes in Prozent, gemerkt je Rechner.
export const ANTEIL_STANDARD = 50;
/** Text und Vorschau behalten beim Ziehen mindestens so viel Platz (Pixel). */
export const SPALTE_MIN = 320;
const ANTEIL_MERKER = "tftn-text-anteil";

/** Anteil des Textes in Prozent (auf Zehntel), wenn der Trenner bei `x` steht; beide Seiten behalten `min` Pixel. */
export function textAnteil(x: number, links: number, breite: number, min = SPALTE_MIN): number {
  if (breite <= 2 * min) return ANTEIL_STANDARD;
  const unten = (min / breite) * 100;
  const a = ((x - links) / breite) * 100;
  return Math.round(Math.max(unten, Math.min(100 - unten, a)) * 10) / 10;
}

/** Gemerkter Anteil, wenn er brauchbar ist (zwischen 10 und 90 %). */
export function anteilLesen(wert: string | null): number | null {
  const n = Number(wert);
  return wert !== null && wert !== "" && n >= 10 && n <= 90 ? n : null;
}

/**
 * Ziehbarer Trenner zwischen Text (`#links`) und Vorschau in `arbeit`: setzt `--text-anteil`.
 * Doppelklick stellt halb und halb wieder her; Pfeiltasten verschieben ihn, wenn er den Fokus hat.
 */
export function trennerZiehbar(arbeit: HTMLElement) {
  const setzen = (a: number | null) => {
    if (a === null) arbeit.style.removeProperty("--text-anteil");
    else arbeit.style.setProperty("--text-anteil", `${a}%`);
    griff.setAttribute("aria-valuenow", String(Math.round(a ?? ANTEIL_STANDARD)));
  };
  const merken = (a: number | null) => {
    try {
      if (a === null) localStorage.removeItem(ANTEIL_MERKER);
      else localStorage.setItem(ANTEIL_MERKER, String(a));
    } catch {
      // ohne localStorage gilt der Anteil bis zum Neuladen
    }
  };
  const jetzt = () => anteilLesen(arbeit.style.getPropertyValue("--text-anteil").replace("%", "")) ?? ANTEIL_STANDARD;

  const griff = document.createElement("div");
  griff.className = "arbeit-griff";
  griff.tabIndex = 0;
  griff.setAttribute("role", "separator");
  griff.setAttribute("aria-orientation", "vertical");
  griff.setAttribute("aria-valuemin", "10");
  griff.setAttribute("aria-valuemax", "90");
  griff.setAttribute("aria-label", t("Breite von Text und Vorschau"));
  griff.title = t("Ziehen verteilt die Breite zwischen Text und Vorschau, Doppelklick stellt halb und halb her");
  arbeit.append(griff);
  let gemerkt: number | null = null;
  try {
    gemerkt = anteilLesen(localStorage.getItem(ANTEIL_MERKER));
  } catch {
    // nichts gemerkt
  }
  setzen(gemerkt);

  griff.addEventListener("pointerdown", (d) => {
    d.preventDefault();
    griff.setPointerCapture(d.pointerId);
    document.body.classList.add("trenner-zieht");
    const ziehen = (m: PointerEvent) => {
      const r = arbeit.getBoundingClientRect();
      setzen(textAnteil(m.clientX, r.left, r.width));
    };
    const ende = () => {
      griff.removeEventListener("pointermove", ziehen);
      griff.removeEventListener("pointerup", ende);
      griff.removeEventListener("pointercancel", ende);
      document.body.classList.remove("trenner-zieht");
      merken(jetzt());
    };
    griff.addEventListener("pointermove", ziehen);
    griff.addEventListener("pointerup", ende);
    griff.addEventListener("pointercancel", ende);
  });
  griff.addEventListener("dblclick", () => {
    setzen(null);
    merken(null);
  });
  griff.addEventListener("keydown", (e) => {
    const schritt = e.key === "ArrowLeft" ? -2 : e.key === "ArrowRight" ? 2 : 0;
    if (!schritt) return;
    e.preventDefault();
    const r = arbeit.getBoundingClientRect();
    const a = textAnteil(r.left + (r.width * (jetzt() + schritt)) / 100, r.left, r.width);
    setzen(a);
    merken(a);
  });
}
