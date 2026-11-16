// Schwebendes Menü am Mauszeiger (Rechtsklick im Editor): Klick daneben und Esc schließen es.

import { anAnker } from "./platz";

export interface Anker {
  x: number;
  oben: number;
  unten: number;
}

export interface SchwebeMenue {
  /** Eine Zeile; mit `aktion` ein Eintrag, der beim Klick das Menü schließt. */
  zeile(text: string, klasse: string, aktion?: () => void): HTMLLIElement;
  zeigen(zeilen: HTMLElement[], anker: Anker): void;
  /** Nach einer Größenänderung wieder ganz ins Fenster. */
  anpassen(anker: Anker): void;
  offen(): boolean;
}

export function schwebeMenue(): SchwebeMenue {
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
  return {
    zeile(text, klasse, aktion) {
      const li = document.createElement("li");
      li.className = klasse;
      li.textContent = text;
      if (aktion)
        li.addEventListener("click", () => {
          schliessen();
          aktion();
        });
      return li;
    },
    zeigen(zeilen, anker) {
      huelle.replaceChildren(...zeilen);
      huelle.hidden = false;
      anAnker(huelle, anker);
      setTimeout(() => document.addEventListener("mousedown", daneben, true));
    },
    anpassen(anker) {
      anAnker(huelle, anker);
    },
    offen: () => !huelle.hidden,
  };
}
