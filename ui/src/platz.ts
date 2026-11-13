/** Wohin ein Popover oder Menü kommt, damit es ganz im Fenster bleibt (Abschlussprüfung AP6). */

const RAND = 8;
const ABSTAND = 4;

export interface Anker {
  x: number;
  oben: number;
  unten: number;
}

/** Unter dem Anker, sonst darüber; waagrecht und senkrecht in das Fenster geschoben. */
export function platzieren(
  anker: Anker,
  groesse: { breite: number; hoehe: number },
  fenster: { breite: number; hoehe: number },
): { links: number; oben: number } {
  const links = Math.max(RAND, Math.min(anker.x, fenster.breite - groesse.breite - RAND));
  let oben = anker.unten + ABSTAND;
  if (oben + groesse.hoehe > fenster.hoehe - RAND) {
    const darueber = anker.oben - ABSTAND - groesse.hoehe;
    oben = darueber >= RAND ? darueber : fenster.hoehe - RAND - groesse.hoehe;
  }
  return { links, oben: Math.max(RAND, oben) };
}

/** Setzt `huelle` (schon sichtbar, also messbar) an den Anker. */
export function anAnker(huelle: HTMLElement, anker: Anker) {
  const p = platzieren(
    anker,
    { breite: huelle.offsetWidth, hoehe: huelle.offsetHeight },
    { breite: window.innerWidth, hoehe: window.innerHeight },
  );
  huelle.style.left = `${p.links}px`;
  huelle.style.top = `${p.oben}px`;
}
