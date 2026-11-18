// Neues Projekt und Projektwechsel (PLAN-M2 AP8): reine Logik zum Dialog in neudialog.ts.

import { t } from "./sprache";

export interface TypJs {
  name: string;
  version: string;
  titel: string;
  beschreibung: string | null;
  braucht?: string[];
  herkunft: "eigen" | "tftn";
}

export interface TypenJs {
  typen: TypJs[];
  warnungen: string[];
  eigener_ordner: string;
  vorgabe_ordner: string | null;
}

/** Unter Windows reservierte Gerätenamen (auch mit Endung). */
const RESERVIERT = new Set(["con", "prn", "aux", "nul", ...[1, 2, 3, 4, 5, 6, 7, 8, 9].flatMap((i) => [`com${i}`, `lpt${i}`])]);

/** null = gültig, sonst die Meldung (gleiche Regeln wie Rust `name_pruefen`). */
export function namePruefen(name: string): string | null {
  const n = name.trim();
  if (n === "") return t("Name fehlt");
  if (n.includes("/") || n.includes("\\")) return t("Name darf kein / oder \\ enthalten");
  // eslint-disable-next-line no-control-regex
  if (/[<>:"|?*\u0000-\u001f\u007f]/.test(n)) return t("Name darf keines der Zeichen < > : \" | ? * enthalten");
  if (n.startsWith(".")) return t("Name darf nicht mit einem Punkt beginnen");
  if (n.endsWith(".")) return t("Name darf nicht mit einem Punkt enden");
  if (RESERVIERT.has(n.split(".")[0].toLowerCase())) return t("Name ist unter Windows reserviert");
  return null;
}

/** Elternordner + Name mit dem Trenner des Elternpfads (\ unter Windows, sonst /), ohne doppelten Trenner. */
export function zielPfad(eltern: string, name: string): string {
  const trenner = eltern.includes("\\") && !eltern.includes("/") ? "\\" : "/";
  const basis = eltern.endsWith(trenner) ? eltern.slice(0, -1) : eltern;
  return `${basis}${trenner}${name.trim()}`;
}

/** Eigene zuerst, dann nach Titel. */
export function typenOrdnen(typen: TypJs[]): TypJs[] {
  return [...typen].sort((a, b) => (a.herkunft === b.herkunft ? a.titel.localeCompare(b.titel, "de") : a.herkunft === "eigen" ? -1 : 1));
}

/** Speichern, dann wechseln; scheitert das Speichern, wird nicht gewechselt. */
export async function wechselNachSpeichern(speichern: () => Promise<boolean>, wechseln: () => Promise<void>): Promise<boolean> {
  if (!(await speichern())) return false;
  await wechseln();
  return true;
}

/** Eine gemeinsame Sperre: Solange ein Aufruf läuft, werden weitere übergangen (`false`), etwa
 *  mehrfaches Klicken auf „Öffnen“, das sonst mehrere Systemdialoge öffnet. `besetzt` sagt dem
 *  Benutzer, warum nichts geschieht. */
export function sperre(besetzt?: () => void): <T>(f: () => Promise<T>) => Promise<boolean> {
  let laeuft = false;
  return async (f) => {
    if (laeuft) {
      besetzt?.();
      return false;
    }
    laeuft = true;
    try {
      await f();
      return true;
    } finally {
      laeuft = false;
    }
  };
}

/** Steht im Dokument ohne Namen schon etwas außer der Sprachzeile der Vorlage? */
export function entwurfHatText(text: string): boolean {
  return (
    text
      .replace(/^#import "@local\/tftn-leer:[^"]*": \*\s*/, "")
      .replace(/^#set text\(lang: "de", region: "AT"\)\s*/, "")
      .trim() !== ""
  );
}

/** Was der Dialog unter „Das brauchst du“ zeigt. */
export function brauchtListe(braucht: string[] | undefined): string[] {
  return braucht?.length ? braucht : [t("Nichts weiter – einfach losschreiben.")];
}
