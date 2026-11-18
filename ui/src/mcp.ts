// MCP (KI-Assistenten): Der Benutzer schaltet es selbst ein. Die Statusleiste zeigt, ob es an oder
// pausiert ist; nach einem Wechsel durch MCP fragt TFTN, ob es aktiv bleiben soll.

import { type InkognitoJs, inkognitoAnzeige } from "./inkognito";
import { t } from "./sprache";

export interface McpJs {
  an: boolean;
  /** Pausiert nach einem Wechsel durch MCP, bis der Benutzer bestätigt. */
  wartet: boolean;
  beim_wechsel_aus: boolean;
  /** Pfad des TFTN-Programms; der Assistent startet es mit `--mcp`. */
  programm: string;
  fehler: string | null;
}

/** Aufträge der App an den Editor (Ereignis `mcp-auftrag`). */
export type McpAuftrag =
  | { id: number; art: "patch"; patch: { rev: number; von: number; bis: number; text: string } }
  | { id: number; art: "wechseln" };

/**
 * Zeile zum Eintragen in Claude Code. Windows-Pfade (`C:\…`) in doppelten Anführungszeichen;
 * sonst in einfachen, damit `$`, Backtick und `\` in einer Mac- oder Linux-Shell nichts bewirken.
 */
export function mcpBefehl(programm: string): string {
  const windows = /^[A-Za-z]:[\\/]|^\\\\/.test(programm);
  const pfad = windows ? `"${programm.replaceAll('"', '\\"')}"` : `'${programm.replaceAll("'", "'\\''")}'`;
  return `claude mcp add tftn -- ${pfad} --mcp`;
}

/**
 * Letzter Schritt eines Wechsels durch MCP, nachdem gespeichert (und ein Dokument ohne Namen benannt)
 * ist: vormerken, antworten, neu laden. Neu geladen wird auch, wenn das Vormerken scheitert (etwa weil
 * der Aufruf abgelaufen ist, während der Benutzer den Namen eintippte): Sonst bliebe der Editor auf
 * dem schon umbenannten Dokument stehen.
 */
export async function mcpWechselAbschliessen(
  vormerken: () => Promise<unknown>,
  antwort: (ok: boolean, fehler?: string) => Promise<unknown>,
  neuLaden: () => void,
): Promise<void> {
  try {
    await vormerken();
    await antwort(true);
  } catch (fehler) {
    await antwort(false, String(fehler));
  }
  neuLaden();
}

/** Ein Werkzeugaufruf des Assistenten (Ereignis `mcp-zugriff`). */
export interface McpZugriff {
  werkzeug: string;
  zeit: Date;
}

/** Was ein Werkzeug tut, in Worten für die Statusleiste; Unbekanntes unter seinem Namen. */
function zugriffWort(werkzeug: string): string {
  const w: Record<string, string> = {
    status: t("Zustand gelesen"),
    quelltext_lesen: t("Text gelesen"),
    text_ersetzen: t("Text geändert"),
    quelltext_setzen: t("Text geändert"),
    formular_lesen: t("Formular gelesen"),
    formular_setzen: t("Formular geändert"),
    seite_ansehen: t("Seite angesehen"),
    typen_auflisten: t("Dokumenttypen gelesen"),
    dokument_anlegen: t("Dokument angelegt"),
    dokument_oeffnen: t("Dokument geöffnet"),
  };
  return w[werkzeug] ?? werkzeug;
}

/**
 * Text und Erklärung für die Statusleiste; leer, wenn MCP aus ist. `aktiv`: Der Assistent hat eben
 * zugegriffen – dann steht da, was er tut; danach nennt der Tooltip den letzten Zugriff. Sind Felder
 * inkognito, steht das dabei.
 */
export function mcpAnzeige(
  m: McpJs | null,
  zuletzt: McpZugriff | null = null,
  aktiv = false,
  inkognito: InkognitoJs | null = null,
): { text: string; titel: string } {
  const a = mcpAnzeigeOhne(m, zuletzt, aktiv);
  const i = inkognitoAnzeige(inkognito);
  if (!a.text || !i.zusatz || m?.wartet) return a;
  return { text: `${a.text} · ${i.zusatz}`, titel: `${a.titel}\n${i.titel}` };
}

function mcpAnzeigeOhne(m: McpJs | null, zuletzt: McpZugriff | null, aktiv: boolean): { text: string; titel: string } {
  if (!m?.an) return { text: "", titel: "" };
  if (m.wartet)
    return {
      text: t("MCP pausiert"),
      titel: t("Ein über MCP geöffnetes Dokument wartet auf deine Bestätigung. Klicken zum Bestätigen."),
    };
  let titel = t("KI-Assistenten dürfen im offenen Dokument arbeiten. Klicken schaltet MCP aus.");
  if (zuletzt) {
    const zeit = `${String(zuletzt.zeit.getHours()).padStart(2, "0")}:${String(zuletzt.zeit.getMinutes()).padStart(2, "0")}`;
    titel += `\n${t("Zuletzt: {was} ({zeit})", { was: zugriffWort(zuletzt.werkzeug), zeit })}`;
  }
  if (aktiv && zuletzt) return { text: t("MCP: {was}", { was: zugriffWort(zuletzt.werkzeug) }), titel };
  return { text: t("MCP an"), titel };
}

/** Nach einem Wechsel durch MCP: Die Frage „aktiv lassen?“ bleibt stehen, bis der Benutzer antwortet;
 *  andere Hinweise dürfen sie nur vorübergehend verdrängen. */
export function mcpFragtNoch(m: McpJs | null): boolean {
  return m !== null && m.an && m.wartet;
}
