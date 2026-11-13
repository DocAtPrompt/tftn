// Kontextmenü D in der Vorschau (PLAN-M2 AP6, Entwurf 4 D): Rechtsklick auf ein Element bietet
// „Ändern …“ (Seitenleiste am Baustein), die Werte direkt und „Im Code zeigen“; Titelei und Text
// der Vorlage bieten „Im Formular bearbeiten“ (PLAN-M2 AP7). Die Zuordnung
// Vorschau → Quelle kommt aus dem Klicksprung (M1). Linksklick springt wie bisher.

import type { HierJs } from "./seitenleiste";
import type { FeldJs, Wert } from "./werte";
import { t } from "./sprache";
import { anAnker } from "./platz";

/** Antwort von `klick` (Rust `KlickAntwort`). */
export type KlickAntwort =
  | { art: "haupt"; rev: number; pos: number }
  | { art: "datei"; datei: string }
  | { art: "vorlage" }
  | { art: "feld"; feld: string }
  | { art: "url"; url: string }
  | { art: "position"; seite: number; x: number; y: number }
  | { art: "nichts" };

export type MenueEintrag =
  | { art: "titel"; text: string }
  | { art: "aendern" }
  | { art: "formel" }
  | { art: "wert"; aufruf: number; feld: FeldJs }
  | { art: "code" }
  | { art: "gesperrt"; text: string }
  | { art: "formular"; feld: string | null };

const MARKUP_TITEL: Record<string, string> = { ueberschrift: t("Überschrift"), liste: t("Liste"), formel: t("Formel"), zitat: t("Zitat") };

/** Die Einträge des Menüs; eine leere Liste heißt: kein Menü. */
export function menueEintraege(antwort: KlickAntwort, hier: HierJs | null): MenueEintrag[] {
  if (antwort.art === "feld") return [{ art: "titel", text: t("Titelei") }, { art: "formular", feld: antwort.feld }];
  if (antwort.art === "vorlage") return [{ art: "formular", feld: null }];
  if (antwort.art !== "haupt") return [];
  const innen = hier?.ebenen[0];
  if (innen) {
    return [
      { art: "titel", text: innen.baustein },
      { art: "aendern" },
      ...innen.felder.filter((f) => !f.gesperrt).map((feld): MenueEintrag => ({ art: "wert", aufruf: innen.von, feld })),
      { art: "code" },
    ];
  }
  const titel = hier?.tabelle ? t("Tabelle") : hier?.markup[0] ? MARKUP_TITEL[hier.markup[0].art] : null;
  if (!hier?.tabelle && hier?.markup[0]?.art === "formel") {
    return [{ art: "titel", text: t("Formel") }, { art: "aendern" }, { art: "formel" }, { art: "code" }];
  }
  return titel ? [{ art: "titel", text: titel }, { art: "aendern" }, { art: "code" }] : [{ art: "code" }];
}

export interface MenueAktionen {
  aendern(): void;
  /** Fach „Formel“ öffnen (PLAN-M2 AP10). */
  formel(): void;
  code(): void;
  setzen(aufruf: number, parameter: string, w: Wert): void;
  /** Editor für ein Feld, das kein direkter Untermenü-Eintrag ist. */
  editor(aufruf: number, feld: FeldJs, maus: { x: number; y: number }): void;
  /** Fach „Dokument“ öffnen, am Feld `feld` (Titelei, PLAN-M2 AP7). */
  formular(feld: string | null): void;
}

/** Das Menü als schwebende Liste; schließt mit Esc, einem Klick daneben oder nach einer Wahl. */
export class VorschauMenue {
  private huelle: HTMLUListElement;
  private schliesser = (e: Event) => {
    if (!this.huelle.contains(e.target as Node)) this.schliessen();
  };

  constructor(private a: MenueAktionen) {
    this.huelle = document.createElement("ul");
    this.huelle.className = "vorschau-menue";
    this.huelle.hidden = true;
    document.body.append(this.huelle);
    document.addEventListener("keydown", (e) => e.key === "Escape" && this.schliessen());
  }

  zeigen(eintraege: MenueEintrag[], maus: { x: number; y: number }) {
    const zeilen: HTMLLIElement[] = [];
    const zeile = (text: string, klasse: string, aktion?: () => void) => {
      const li = document.createElement("li");
      li.className = klasse;
      li.textContent = text;
      if (aktion) {
        li.addEventListener("click", () => {
          this.schliessen();
          aktion();
        });
      }
      zeilen.push(li);
      return li;
    };
    for (const e of eintraege) {
      switch (e.art) {
        case "titel":
          zeile(e.text, "menue-titel");
          break;
        case "formel":
          zeile(t("Im Formelfeld ändern …"), "menue-eintrag", () => this.a.formel());
          break;
        case "aendern":
          zeile(t("Ändern …"), "menue-eintrag", () => this.a.aendern());
          break;
        case "code":
          zeile(t("Im Code zeigen"), "menue-eintrag", () => this.a.code());
          break;
        case "gesperrt":
          zeile(e.text, "menue-eintrag menue-gesperrt");
          break;
        case "formular": {
          const feld = e.feld;
          zeile(t("Im Formular bearbeiten"), "menue-eintrag", () => this.a.formular(feld));
          break;
        }
        case "wert":
          this.wert(e.aufruf, e.feld, maus, zeile);
          break;
      }
    }
    this.huelle.replaceChildren(...zeilen);
    this.huelle.hidden = false;
    anAnker(this.huelle, { x: maus.x, oben: maus.y, unten: maus.y });
    setTimeout(() => document.addEventListener("mousedown", this.schliesser, true));
  }

  /** Wahl und Schalter direkt als Einträge, alles andere über den Editor. */
  private wert(
    aufruf: number,
    f: FeldJs,
    maus: { x: number; y: number },
    zeile: (text: string, klasse: string, aktion?: () => void) => HTMLLIElement,
  ) {
    const werte: string[] = f.art.mit?.werte ?? [];
    if (f.art.art === "wahl" && werte.length) {
      zeile(f.titel, "menue-gruppe");
      for (const w of werte) {
        const aktuell = f.wert?.wert === w;
        zeile(`${aktuell ? "✓ " : "  "}${w}`, "menue-eintrag menue-wahl", () => this.a.setzen(aufruf, f.parameter, { art: "text", wert: w }));
      }
    } else if (f.art.art === "schalter") {
      const an = f.wert?.wert === true;
      zeile(`${an ? "✓ " : "  "}${f.titel}`, "menue-eintrag", () => this.a.setzen(aufruf, f.parameter, { art: "schalter", wert: !an }));
    } else {
      zeile(`${f.titel} …`, "menue-eintrag", () => this.a.editor(aufruf, f, maus));
    }
  }

  schliessen() {
    this.huelle.hidden = true;
    document.removeEventListener("mousedown", this.schliesser, true);
  }
}
