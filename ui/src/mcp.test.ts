import { describe, expect, it } from "vitest";
import { type McpJs, mcpAnzeige, mcpBefehl, mcpFragtNoch } from "./mcp";

const an: McpJs = { an: true, wartet: false, beim_wechsel_aus: false, programm: "/x/tftn-app", fehler: null };

describe("mcpBefehl", () => {
  it("setzt den Pfad in Anführungszeichen, auch mit Leerzeichen", () => {
    expect(mcpBefehl("/Applications/TFTN.app/Contents/MacOS/tftn-app")).toBe(
      'claude mcp add tftn -- "/Applications/TFTN.app/Contents/MacOS/tftn-app" --mcp',
    );
    expect(mcpBefehl("C:\\Program Files\\TFTN\\tftn-app.exe")).toBe(
      'claude mcp add tftn -- "C:\\Program Files\\TFTN\\tftn-app.exe" --mcp',
    );
  });
});

describe("mcpAnzeige", () => {
  it("aus: nichts in der Statusleiste", () => {
    expect(mcpAnzeige(null).text).toBe("");
    expect(mcpAnzeige({ ...an, an: false, wartet: true }).text).toBe("");
  });
  it("an und pausiert unterscheidbar", () => {
    expect(mcpAnzeige(an).text).toBe("MCP an");
    expect(mcpAnzeige({ ...an, wartet: true }).text).toBe("MCP pausiert");
  });
});

describe("mcpFragtNoch", () => {
  it("die Frage bleibt stehen, solange MCP an und pausiert ist", () => {
    expect(mcpFragtNoch({ ...an, wartet: true })).toBe(true);
    expect(mcpFragtNoch(an)).toBe(false);
    expect(mcpFragtNoch({ ...an, an: false, wartet: true })).toBe(false);
    expect(mcpFragtNoch(null)).toBe(false);
  });
});
