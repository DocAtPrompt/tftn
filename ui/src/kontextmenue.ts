// Rechtsklick: Das Menü des Systems (in der WebView mit „Neu laden“, im Entwicklungsbau auch
// „Element untersuchen“) gehört nicht in die App. Es bleibt nur dort, wo es etwas taugt: in
// Eingabefeldern und im Editor (Ausschneiden, Kopieren, Einsetzen). Eigene Menüs (Rechtschreibung,
// Vorschau) verhindern es ohnehin selbst.

const BEARBEITBAR = 'input, textarea, [contenteditable=""], [contenteditable="true"], [contenteditable="plaintext-only"]';

export function systemMenueLassen(ziel: { closest?(auswahl: string): unknown } | null): boolean {
  return typeof ziel?.closest === "function" && ziel.closest(BEARBEITBAR) != null;
}

/** Einmal beim Start. */
export function kontextmenueBegrenzen() {
  window.addEventListener("contextmenu", (e) => {
    if (!systemMenueLassen(e.target instanceof Element ? e.target : null)) e.preventDefault();
  });
}
