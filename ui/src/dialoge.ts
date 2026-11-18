// Gemeinsames aller Dialoge: Schließen-Knopf, Esc (wirkt auch, wenn der Dialog den Fokus verloren hat)
// und ein Klick daneben.

import { ikoneSvg } from "./ikonen";
import { t } from "./sprache";

export interface Dialog {
  readonly offen: boolean;
  schliessen(): void;
  /** Gehört der angeklickte Knoten zum Dialog (oder zu dem Knopf, der ihn öffnet)? Ohne diese Angabe
   *  schließt ein Klick daneben den Dialog nicht. */
  enthaelt?(ziel: Node): boolean;
  /** Ginge beim Schließen etwas verloren (etwa ein eingetippter Name)? Dann bleibt er offen. */
  behalten?(): boolean;
}

/** Der Dialog, den ein Klick auf `ziel` schließt: der oberste offene, wenn der Klick daneben ging. */
export function klickZiel(dialoge: readonly Dialog[], ziel: Node): Dialog | null {
  const d = escZiel(dialoge);
  if (!d?.enthaelt || d.enthaelt(ziel) || d.behalten?.()) return null;
  return d;
}

/** `enthaelt` für einen Dialog aus seinen Elementen (Hülle, Auslöser). */
export function bereich(...elemente: (Element | null | undefined)[]): (ziel: Node) => boolean {
  return (ziel) => elemente.some((e) => e?.contains(ziel));
}

/** Der Dialog, den Esc schließt: der zuletzt angemeldete offene (er liegt oben). */
export function escZiel(dialoge: readonly Dialog[]): Dialog | null {
  for (let i = dialoge.length - 1; i >= 0; i--) if (dialoge[i].offen) return dialoge[i];
  return null;
}

const dialoge: Dialog[] = [];

/** `d` mit `enthaelt`: Ein Klick daneben schließt ihn, außer `behalten` sagt nein. */
export function mitBereich(d: Dialog, enthaelt: (ziel: Node) => boolean): Dialog {
  return {
    get offen() {
      return d.offen;
    },
    schliessen: () => d.schliessen(),
    enthaelt,
    behalten: () => d.behalten?.() ?? false,
  };
}

/**
 * Meldet einen Dialog an (Esc schließt den obersten). Mit `bereich` (Hülle, Auslöser) schließt ihn
 * auch ein Klick daneben; ohne nur Esc und ×.
 */
export function dialogAnmelden(d: Dialog, ...elemente: (Element | null | undefined)[]) {
  dialoge.push(elemente.length ? mitBereich(d, bereich(...elemente)) : d);
}

export function einDialogOffen(): boolean {
  return escZiel(dialoge) !== null;
}

/** Schließt alle offenen Dialoge und Menüs, den obersten zuerst, außer `ausser` (etwa der Rundgang). */
export function alleSchliessen(ausser?: Dialog) {
  for (let i = dialoge.length - 1; i >= 0; i--) {
    const d = dialoge[i];
    if (d.offen && d !== ausser) d.schliessen();
  }
}

/** Einmal beim Start. Was ein Dialog oder der Editor schon selbst behandelt hat, bleibt unberührt. */
export function escAktivieren() {
  // Vor allem anderen (Capture), damit der Klick, der daneben ging, trotzdem seine Wirkung hat.
  window.addEventListener(
    "pointerdown",
    (e) => {
      if (e.button === 0 && e.target instanceof Node) klickZiel(dialoge, e.target)?.schliessen();
    },
    true,
  );
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    const d = escZiel(dialoge);
    if (d) {
      e.preventDefault();
      d.schliessen();
    }
  });
}

/** Schließen-Knopf mit demselben Symbol wie im Hinweisband. */
export function schliessenKnopf(schliessen: () => void): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = "neu-zu nur-ikone";
  b.innerHTML = ikoneSvg("schliessen");
  b.title = t("Schließen (Esc)");
  b.setAttribute("aria-label", t("Schließen"));
  b.addEventListener("click", schliessen);
  return b;
}

/** Kopf eines Dialogs: Titel links, Schließen rechts. */
export function dialogKopf(titel: string, schliessen: () => void, ebene: "h2" | "span" = "h2"): HTMLDivElement {
  const kopf = document.createElement("div");
  kopf.className = "neu-kopf";
  const h = document.createElement(ebene);
  h.textContent = titel;
  kopf.append(h, schliessenKnopf(schliessen));
  return kopf;
}
