// Reiter über dem Editor: die Hauptdatei (immer da, nicht schließbar) und geöffnete Kapiteldateien
// (`.typ`). Jeder Reiter zeigt den Dateinamen, einen Punkt, solange er ungespeichert ist, und ×.
// „+“ öffnet eine weitere Kapiteldatei des Projekts. Den Zustand je Datei (Text, Rückgängig,
// Revision) führt main.ts; Rust parkt die nicht angezeigten Dateien (`src-tauri/src/reiter.rs`).

import { KnopfMenue, type MenuePunkt } from "./knopfmenue";
import { t } from "./sprache";

export interface ReiterStand {
  /** Pfad relativ zum Projekt, mit `/`. */
  datei: string;
  haupt: boolean;
  ungespeichert: boolean;
}

/** Was auf dem Reiter steht: der Dateiname ohne Ordner. */
export function reiterName(datei: string): string {
  return datei.split("/").pop() || datei;
}

/** Tooltip: ganzer Pfad, Hauptdatei bzw. ungespeichert dazu. */
export function reiterTitel(r: ReiterStand): string {
  const teile = [r.datei];
  if (r.haupt) teile.push(t("Hauptdatei"));
  if (r.ungespeichert) teile.push(t("ungespeichert"));
  return teile.join(" · ");
}

/**
 * Welcher Reiter aktiv wird, wenn `weg` geschlossen wird: bleibt `aktiv` offen, dieser; sonst der
 * rechte Nachbar, am Ende der linke.
 */
export function nachSchliessen(liste: string[], weg: string, aktiv: string): string {
  if (weg !== aktiv) return aktiv;
  const i = liste.indexOf(weg);
  const rest = liste.filter((d) => d !== weg);
  return rest[Math.min(i, rest.length - 1)] ?? aktiv;
}

/** Punkte im Menü „+“: Kapiteldateien, die noch nicht offen sind. */
export function kapitelPunkte(alle: string[], offen: string[], waehlen: (datei: string) => void): MenuePunkt[] {
  const neu = alle.filter((d) => !offen.includes(d));
  if (!neu.length)
    return [{ text: t("Keine weiteren Kapiteldateien"), titel: t("Alle .typ-Dateien des Projekts sind schon offen."), aktion: () => {} }];
  return neu.map((d) => ({ text: d, ikone: "dokument", aktion: () => waehlen(d) }));
}

export interface ReiterAktionen {
  waehlen(datei: string): void;
  schliessen(datei: string): void;
  /** Kapiteldateien des Projekts (relativ, ohne die Hauptdatei). */
  kapitel(): Promise<string[]>;
}

export class ReiterLeiste {
  private leiste = document.createElement("div");
  private plus = document.createElement("button");
  private liste: ReiterStand[] = [];

  constructor(
    ort: HTMLElement,
    private a: ReiterAktionen,
  ) {
    this.leiste.id = "reiter";
    this.leiste.setAttribute("role", "tablist");
    this.leiste.setAttribute("aria-label", t("Offene Dateien"));
    this.plus.className = "reiter-plus";
    this.plus.textContent = "+";
    this.plus.title = t("Kapiteldatei öffnen");
    this.plus.setAttribute("aria-label", t("Kapiteldatei öffnen"));
    new KnopfMenue(this.plus, async () =>
      kapitelPunkte(
        await this.a.kapitel().catch(() => []),
        this.liste.map((r) => r.datei),
        (d) => this.a.waehlen(d),
      ),
    );
    ort.before(this.leiste);
  }

  zeigen(liste: ReiterStand[], aktiv: string) {
    this.liste = liste;
    const knoepfe = liste.map((r) => {
      const reiter = document.createElement("div");
      reiter.className = "reiter";
      reiter.setAttribute("role", "tab");
      reiter.setAttribute("aria-selected", String(r.datei === aktiv));
      reiter.tabIndex = r.datei === aktiv ? 0 : -1;
      reiter.title = reiterTitel(r);
      reiter.classList.toggle("aktiv", r.datei === aktiv);
      reiter.classList.toggle("ungespeichert", r.ungespeichert);
      const name = document.createElement("span");
      name.className = "reiter-name";
      name.textContent = reiterName(r.datei);
      const punkt = document.createElement("span");
      punkt.className = "reiter-punkt";
      punkt.setAttribute("aria-hidden", "true");
      reiter.append(name, punkt);
      reiter.addEventListener("click", () => this.a.waehlen(r.datei));
      reiter.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          this.a.waehlen(r.datei);
        }
      });
      if (!r.haupt) {
        const zu = document.createElement("button");
        zu.className = "reiter-zu";
        zu.textContent = "×";
        zu.title = t("Reiter schließen (die Datei bleibt im Projekt)");
        zu.setAttribute("aria-label", t("{datei} schließen", { datei: reiterName(r.datei) }));
        zu.addEventListener("click", (e) => {
          e.stopPropagation();
          this.a.schliessen(r.datei);
        });
        // Mittlere Maustaste schließt ebenfalls, wie in Browsern.
        reiter.addEventListener("auxclick", (e) => {
          if (e.button === 1) this.a.schliessen(r.datei);
        });
        reiter.append(zu);
      }
      return reiter;
    });
    this.leiste.replaceChildren(...knoepfe, this.plus);
  }
}
