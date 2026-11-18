import { describe, expect, it } from "vitest";
import { eigenerErsatz, inkognitoAnzeige, inkognitoSchluessel } from "./inkognito";
import { type McpJs, mcpAnzeige } from "./mcp";
import { type NetzprotokollJs, mcpZeile } from "./netzprotokoll";

describe("inkognito", () => {
  it("Schlüssel ohne Eintragsnummern", () => {
    expect(inkognitoSchluessel(["patient", "name"])).toBe("patient.name");
    expect(inkognitoSchluessel(["autoren", 2, "name"])).toBe("autoren.name");
  });

  it("eigener Ersatztext: ohne Klammern, leer oder gleich der Vorgabe heißt Vorgabe", () => {
    expect(eigenerErsatz("  [Frau  X] ", "Patientin")).toBe("Frau X");
    expect(eigenerErsatz("", "Patientin")).toBeNull();
    expect(eigenerErsatz("[]", "Patientin")).toBeNull();
    expect(eigenerErsatz("Patientin", "Patientin")).toBeNull();
  });

  const stand = { an: true, felder: ["Name", "Geburtsdatum"], werte: 2, fehler: null };

  it("Zusatz für die Statusleiste", () => {
    expect(inkognitoAnzeige(null).zusatz).toBe("");
    expect(inkognitoAnzeige({ ...stand, an: false }).zusatz).toBe("");
    const a = inkognitoAnzeige(stand);
    expect(a.zusatz).toBe("inkognito");
    expect(a.titel).toContain("Name, Geburtsdatum");
    expect(inkognitoAnzeige({ ...stand, fehler: "dokument.toml kaputt" })).toEqual({
      zusatz: "inkognito – gesperrt",
      titel: "dokument.toml kaputt",
    });
  });

  it("MCP-Abzeichen nennt Inkognito, nicht aber beim Pausieren oder wenn MCP aus ist", () => {
    const an: McpJs = { an: true, wartet: false, beim_wechsel_aus: false, programm: "", fehler: null };
    expect(mcpAnzeige(an, null, false, stand).text).toBe("MCP an · inkognito");
    expect(mcpAnzeige(an, null, false, stand).titel).toContain("Geburtsdatum");
    expect(mcpAnzeige(an, null, false, null).text).toBe("MCP an");
    expect(mcpAnzeige({ ...an, wartet: true }, null, false, stand).text).toBe("MCP pausiert");
    expect(mcpAnzeige({ ...an, an: false }, null, false, stand).text).toBe("");
  });

  it("Netzprotokoll zählt pseudonymisierte Zugriffe", () => {
    const p: NetzprotokollJs = {
      start_ms: 0,
      netz: [],
      verworfen: 0,
      mcp: [{ werkzeug: "quelltext_lesen", schreibt: false, anzahl: 3, inkognito: 2, erstes_ms: 0, letztes_ms: 0 }],
      mcp_abgewiesen: 0,
      inkognito_felder: ["Name"],
      ganz_offline: false,
      mcp_an: true,
    };
    expect(mcpZeile(p)).toContain("Davon pseudonymisiert (Inkognito): 2; ersetzt: Name.");
    expect(mcpZeile({ ...p, mcp: [{ ...p.mcp[0], inkognito: 0 }] })).not.toContain("Inkognito");
  });
});
