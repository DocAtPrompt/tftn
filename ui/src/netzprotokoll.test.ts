import { describe, expect, it } from "vitest";
import {
  type NetzEintragJs,
  type NetzprotokollJs,
  abrufZeile,
  anzahlNetz,
  kopfZeile,
  mcpZeile,
  statusAnzeige,
  uhrzeit,
  versatzMinuten,
} from "./netzprotokoll";

const start = new Date(2026, 10, 17, 14, 2, 11).getTime();

const leer: NetzprotokollJs = {
  start_ms: start,
  netz: [],
  verworfen: 0,
  mcp: [],
  mcp_abgewiesen: 0,
  ganz_offline: false,
  mcp_an: false,
};

const eintrag: NetzEintragJs = {
  beginn_ms: start + 60_000,
  dauer_ms: 420,
  zweck: "Neueste Versionen von TFTN und typst abfragen",
  ausloeser: "Klick: Nach Updates suchen",
  hosts: ["api.github.com"],
  abrufe: [
    { url: "https://api.github.com/repos/typst/typst/releases/latest", status: 200, bytes: 4200, fehler: null },
    { url: "https://api.github.com/repos/DocAtPrompt/tftn/releases/latest", status: 404, bytes: 0, fehler: "nicht gefunden" },
  ],
  gemeldet: true,
  ok: true,
  fehler: null,
  ergebnis: "erfolgreich, 2 Verbindung(en)",
};

describe("Netzprotokoll", () => {
  it("im Normalfall leer und ruhig", () => {
    expect(anzahlNetz(leer)).toBe(0);
    expect(kopfZeile(leer)).toEqual({ text: "Keine Netzzugriffe seit dem Start um 14:02:11.", ruhig: true });
    expect(statusAnzeige(leer)).toEqual({ text: "", titel: "" });
    expect(statusAnzeige(null)).toEqual({ text: "", titel: "" });
  });

  it("zählt Netzzugriffe, auch verworfene", () => {
    const eins = { ...leer, netz: [eintrag] };
    expect(kopfZeile(eins)).toEqual({ text: "1 Netzzugriff seit dem Start um 14:02:11.", ruhig: false });
    expect(statusAnzeige(eins).text).toBe("1 Netzzugriff");
    const viele = { ...leer, netz: [eintrag, eintrag], verworfen: 1000 };
    expect(anzahlNetz(viele)).toBe(1002);
    expect(kopfZeile(viele).text).toBe("1.002 Netzzugriffe seit dem Start um 14:02:11.");
    expect(statusAnzeige(viele).text).toBe("1.002 Netzzugriffe");
  });

  it("solange der Holer läuft, steht sein Ziel in der Statusleiste", () => {
    const a = statusAnzeige({ ...leer, netz: [eintrag] }, "Verbindung zu packages.typst.org …");
    expect(a.text).toBe("Verbindung zu packages.typst.org …");
  });

  it("Verbindungen einzeln", () => {
    expect(abrufZeile(eintrag.abrufe[0])).toBe("200 · 4 kB · https://api.github.com/repos/typst/typst/releases/latest");
    expect(abrufZeile(eintrag.abrufe[1])).toBe(
      "Fehler: nicht gefunden · https://api.github.com/repos/DocAtPrompt/tftn/releases/latest",
    );
    expect(abrufZeile({ url: "https://x.org/a", status: null, bytes: 0, fehler: null })).toBe("– · https://x.org/a");
  });

  it("MCP gezählt, mit Zustand", () => {
    expect(mcpZeile(leer)).toBe("Kein Zugriff eines KI-Programms über MCP. MCP ist ausgeschaltet.");
    const mit: NetzprotokollJs = {
      ...leer,
      mcp_an: true,
      mcp_abgewiesen: 2,
      mcp: [
        { werkzeug: "quelltext_lesen", schreibt: false, anzahl: 3, erstes_ms: start, letztes_ms: start },
        { werkzeug: "text_ersetzen", schreibt: true, anzahl: 1, erstes_ms: start, letztes_ms: start },
      ],
    };
    expect(mcpZeile(mit)).toBe(
      "4 Zugriffe von KI-Programmen über MCP (nur auf diesem Rechner, 127.0.0.1). Abgewiesen: 2. MCP ist eingeschaltet.",
    );
  });

  it("Uhrzeit und Versatz in Ortszeit", () => {
    expect(uhrzeit(new Date(2026, 0, 2, 3, 4, 5).getTime())).toBe("03:04:05");
    const d = new Date(2026, 6, 1);
    expect(versatzMinuten(d)).toBe(-d.getTimezoneOffset());
  });
});
