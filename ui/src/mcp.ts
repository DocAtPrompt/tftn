// MCP (KI-Assistenten): Der Benutzer schaltet es selbst ein. Die Statusleiste zeigt, ob es an oder
// pausiert ist; nach einem Wechsel durch MCP fragt TFTN, ob es aktiv bleiben soll.

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

/** Text und Erklärung für die Statusleiste; leer, wenn MCP aus ist. */
export function mcpAnzeige(m: McpJs | null): { text: string; titel: string } {
  if (!m?.an) return { text: "", titel: "" };
  if (m.wartet)
    return {
      text: t("MCP pausiert"),
      titel: t("Ein über MCP geöffnetes Dokument wartet auf deine Bestätigung. Klicken zum Bestätigen."),
    };
  return { text: t("MCP an"), titel: t("KI-Assistenten dürfen im offenen Dokument arbeiten. Klicken schaltet MCP aus.") };
}

/** Nach einem Wechsel durch MCP: Die Frage „aktiv lassen?“ bleibt stehen, bis der Benutzer antwortet;
 *  andere Hinweise dürfen sie nur vorübergehend verdrängen. */
export function mcpFragtNoch(m: McpJs | null): boolean {
  return m !== null && m.an && m.wartet;
}
