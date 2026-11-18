import { describe, expect, it } from "vitest";
import html from "../index.html?raw";
import eingebautToml from "../../crates/tftn-katalog/eingebaut.toml?raw";
import { HANDGRIFFE, type Handgriffe, SCHREIBEND } from "./rundgang";
import { HELFER } from "./tasten";
import { DYNAMISCHE_ZIELE, rundgaenge } from "./rundgaenge";

const quellen = import.meta.glob("./*.ts", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const typen = import.meta.glob("../../types/*/typst.toml", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

const IDS_HTML = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]));
const BAUSTEINE = new Set([...eingebautToml.matchAll(/^id = "([^"]+)"/gm)].map((m) => `typst/${m[1]}`));

/** Handgriffe zum Mitschreiben; ein Name außerhalb der API wirft. */
function aufnahme(): { api: Handgriffe; aufrufe: { name: string; args: unknown[] }[] } {
  const aufrufe: { name: string; args: unknown[] }[] = [];
  const api = new Proxy({} as Handgriffe, {
    get(_, name) {
      if (typeof name !== "string" || !(HANDGRIFFE as readonly string[]).includes(name)) throw new Error(`kein Handgriff: ${String(name)}`);
      return async (...args: unknown[]) => void aufrufe.push({ name, args });
    },
  });
  return { api, aufrufe };
}

describe("Rundgänge", () => {
  const alle = rundgaenge();

  it("gibt es, mit eindeutiger ID, Titel und Beschreibung", () => {
    expect(alle.map((r) => r.id)).toContain("ueberblick");
    expect(new Set(alle.map((r) => r.id)).size).toBe(alle.length);
    for (const r of alle) {
      expect(r.titel.trim(), r.id).not.toBe("");
      expect(r.beschreibung.trim(), r.id).not.toBe("");
      expect(r.schritte.length, r.id).toBeGreaterThan(1);
    }
  });

  it("jeder Untertitel hat Text", () => {
    for (const r of alle) for (const [i, s] of r.schritte.entries()) expect(s.text.trim(), `${r.id} ${i + 1}`).not.toBe("");
  });

  it("Ziele stehen in index.html oder entstehen dokumentiert zur Laufzeit", () => {
    for (const r of alle) {
      for (const [i, s] of r.schritte.entries()) {
        if (s.ziel === undefined) continue;
        const wo = `${r.id} ${i + 1}: ${String(s.ziel)}`;
        expect(typeof s.ziel, wo).toBe("string");
        const auswahl = s.ziel as string;
        const ids = [...auswahl.matchAll(/#([\w-]+)/g)].map((m) => m[1]);
        const klassen = [...auswahl.matchAll(/\.([\w-]+)/g)].map((m) => m[1]);
        expect(ids.length + klassen.length, wo).toBeGreaterThan(0);
        for (const id of ids) {
          if (IDS_HTML.has(id)) continue;
          const modul = DYNAMISCHE_ZIELE[id];
          expect(modul, `${wo}: #${id} weder in index.html noch in DYNAMISCHE_ZIELE`).toBeDefined();
          expect(quellen[`./${modul}`], `${wo}: ${modul}`).toMatch(new RegExp(`\\.id = "${id}"`));
        }
        // Klassen nur von CodeMirror (deren Namen sind fest).
        for (const k of klassen) expect(k, wo).toMatch(/^cm-/);
      }
    }
  });

  it("dynamische Ziele werden wirklich vergeben", () => {
    for (const [id, modul] of Object.entries(DYNAMISCHE_ZIELE)) {
      expect(IDS_HTML.has(id), id).toBe(false);
      expect(quellen[`./${modul}`], modul).toMatch(new RegExp(`\\.id = "${id}"`));
    }
  });

  it("Handgriffe nur aus der API; wer schreibt, läuft in einer Kopie eines vorhandenen Typs", async () => {
    for (const r of alle) {
      if (r.kopie !== undefined) expect(Object.keys(typen), r.id).toContain(`../../types/${r.kopie}/typst.toml`);
      for (const [i, s] of r.schritte.entries()) {
        if (!s.vorher) continue;
        const { api, aufrufe } = aufnahme();
        await s.vorher(api);
        const wo = `${r.id} ${i + 1}`;
        expect(aufrufe.length, wo).toBeGreaterThan(0);
        for (const a of aufrufe) {
          if ((SCHREIBEND as readonly string[]).includes(a.name)) expect(r.kopie, `${wo}: ${a.name} ohne Kopie`).toBeDefined();
          if (a.name === "bausteinWaehlen") expect(BAUSTEINE, wo).toContain(a.args[0]);
          if (a.name === "tippen") expect(String(a.args[0]).length, wo).toBeGreaterThan(0);
          // Eine Aktions-ID muss es geben: ein Baustein oder ein Helfer aus tasten.ts.
          const aktion = String(a.args[0]);
          if (a.name === "taste" && aktion.includes("/")) expect([...BAUSTEINE, ...HELFER.map((h) => h.id)], wo).toContain(aktion);
        }
        // Ein Schritt, der beim Ansehen läuft, darf nicht schreiben (beim Ansehen tippt der Benutzer).
        if (s.vorherIn === "beide") {
          for (const a of aufrufe) expect(SCHREIBEND, `${wo}: ${a.name} auch beim Ansehen`).not.toContain(a.name);
        }
      }
    }
  });
});
