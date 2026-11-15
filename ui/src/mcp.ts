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

/** Zeile zum Eintragen in Claude Code; Pfade mit Leerzeichen in Anführungszeichen. */
export function mcpBefehl(programm: string): string {
  return `claude mcp add tftn -- "${programm.replaceAll('"', '\\"')}" --mcp`;
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
