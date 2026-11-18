// Symbole der Knöpfe: eigene Strichzeichnungen (24 × 24, Textfarbe), ohne Bibliothek.
// In index.html steht `data-ikone="name"`; `ikonenSetzen` stellt die Zeichnung vor den Text.

const BLATT = "M14 3v4a1 1 0 0 0 1 1h4";

/** Zahnrad: acht Zähne um einen Ring, gerechnet statt abgezeichnet. */
function zahnrad(): string {
  const punkte: string[] = [];
  const zaehne = 8;
  for (let i = 0; i < zaehne * 4; i++) {
    const r = i % 4 < 2 ? 9.5 : 7.2;
    const w = ((i - 0.5) * Math.PI * 2) / (zaehne * 4);
    punkte.push(`${(12 + r * Math.cos(w)).toFixed(2)} ${(12 + r * Math.sin(w)).toFixed(2)}`);
  }
  return `M${punkte.join("L")}Z M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0`;
}

export const IKONEN: Record<string, string> = {
  neu: `${BLATT} M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z M12 11v6 M9 14h6`,
  vorlage: "M4 4h16v6H4z M4 14h7v6H4z M15 14h5v6h-5z",
  importieren: `${BLATT} M5 12V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2h-5 M2 18h7 M6 15l3 3-3 3`,
  oeffnen: "M5 4h4l3 3h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2",
  fassungen: "M12 8v4l2.5 2.5 M3.05 11a9 9 0 1 1 .5 4 M3 20v-5h5",
  export: `${BLATT} M11 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v5 M14 19h7 M18 16l3 3-3 3`,
  seitenleiste: "M4 4h16a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M15 4v16",
  schreibansicht:
    "M4 8V6a2 2 0 0 1 2-2h2 M4 16v2a2 2 0 0 0 2 2h2 M16 4h2a2 2 0 0 1 2 2v2 M16 20h2a2 2 0 0 0 2-2v-2 M9 10h6 M9 14h4",
  zahnrad: zahnrad(),
  baustein: "M4 4h7v7H4z M13 4h7v7h-7z M4 13h7v7H4z M16.5 13v7 M13 16.5h7",
  zitat:
    "M10 11H6a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v6c0 2.7-1.3 4.3-4 5 M19 11h-4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v6c0 2.7-1.3 4.3-4 5",
  gliederung: "M9 6h11 M9 12h11 M9 18h11 M5 6v.01 M5 12v.01 M5 18v.01",
  // Seite mit Satzspiegel (Ränder)
  seite: "M7 3h10a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2 M8.5 6.5h7v11h-7z",
  // Reiter der Seitenleiste
  hier: "M12 3v3 M12 18v3 M3 12h3 M18 12h3 M12 7a5 5 0 1 0 0 10a5 5 0 1 0 0-10 M12 12v.01",
  formel: "M17 5H6l6 7-6 7h11",
  dokument: "M9 3h6v3H9z M9 4.5H6.5a1.5 1.5 0 0 0-1.5 1.5v13.5a1.5 1.5 0 0 0 1.5 1.5h11a1.5 1.5 0 0 0 1.5-1.5V6a1.5 1.5 0 0 0-1.5-1.5H15 M8.5 11h7 M8.5 14.5h7 M8.5 18h4",
  pakete: "M12 3l8 4.5v9L12 21l-8-4.5v-9z M4 7.5l8 4.5 8-4.5 M12 12v9 M8 5.25l8 4.5",
  pfeil: "M6 9l6 6 6-6",
  // Kreis mit Fragezeichen (Über TFTN, Hilfe)
  hilfe: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18 M9.6 9.3a2.5 2.5 0 1 1 3.4 2.3c-.7.3-1 .8-1 1.5v.6 M12 16.8v.01",
  schliessen: "M6 6l12 12 M18 6L6 18",
};

/** SVG als Text; leer bei unbekanntem Namen. */
export function ikoneSvg(name: string): string {
  const d = IKONEN[name];
  if (!d) return "";
  return `<svg class="ikone" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${d}"/></svg>`;
}

/** Setzt vor jeden Knopf mit `data-ikone` seine Zeichnung (einmal beim Start). */
export function ikonenSetzen(wurzel: ParentNode) {
  for (const el of wurzel.querySelectorAll<HTMLElement>("[data-ikone]")) {
    if (el.querySelector(":scope > svg.ikone")) continue;
    el.insertAdjacentHTML("afterbegin", ikoneSvg(el.dataset.ikone ?? ""));
  }
}
