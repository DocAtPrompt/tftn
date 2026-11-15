// Kopfzeile: was links steht (Projekt, Datei) und wie lange ein Hinweis darunter bleibt.

/** Projektname fett, dahinter die Datei; der volle Pfad nur als Titel beim Darüberfahren. */
export function kopfName(
  ordner: string,
  datei: string,
  entwurf: boolean,
  ohneNamen: string,
): { name: string; rest: string; titel: string } {
  if (entwurf) return { name: ohneNamen, rest: "", titel: "" };
  const ohneEnde = ordner.replace(/[\\/]+$/, "");
  const name = ohneEnde.split(/[\\/]/).pop() || ohneEnde;
  const trenner = ohneEnde.includes("\\") && !ohneEnde.includes("/") ? "\\" : "/";
  return { name, rest: datei, titel: `${ohneEnde}${trenner}${datei}` };
}

/** `kurz`: eine Meldung („Gespeichert“, „MCP ausgeschaltet.“); `bleibt`: Fehler, Warnungen. */
export type HinweisArt = "kurz" | "bleibt";

/** Millisekunden bis zum Ausblenden; `null`: bleibt, bis man ihn schließt. Mit Knöpfen bleibt er immer. */
export function ausblendenNach(art: HinweisArt, knoepfe: number): number | null {
  return art === "kurz" && knoepfe === 0 ? 5000 : null;
}
