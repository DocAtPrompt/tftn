import { describe, expect, it } from "vitest";
import html from "../index.html?raw";
import eingebautToml from "../../crates/tftn-katalog/eingebaut.toml?raw";
import enToml from "../../languages/en.toml?raw";
import { HANDGRIFFE, type Handgriffe, MIN_DAUER, MS_JE_ZEICHEN, type Rundgang, SCHREIBEND, VORLAUF, dauerVon } from "./rundgang";
import { HELFER } from "./tasten";
import { DYNAMISCHE_ZIELE, PAKET, rundgaenge } from "./rundgaenge";

const quellen = import.meta.glob("./*.ts", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const typen = import.meta.glob("../../types/*/typst.toml", { query: "?raw", import: "default", eager: true }) as Record<string, string>;
const vorlagen = Object.keys(import.meta.glob("../../types/*/vorlage*/**/*.typ"));

const IDS_HTML = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]));
const BAUSTEINE = new Set([...eingebautToml.matchAll(/^id = "([^"]+)"/gm)].map((m) => `typst/${m[1]}`));
/** Alle Module der Oberfläche (ohne Tests): Klassen in Zielen müssen dort vorkommen. */
const MODULE = Object.entries(quellen)
  .filter(([n]) => !n.endsWith(".test.ts") && n !== "./rundgaenge.ts")
  .map(([, q]) => q)
  .join("\n");

/** Englische Übersetzung eines Texts aus languages/en.toml (ohne Escapes, reicht für Dateinamen). */
function englisch(de: string): string | undefined {
  const zeile = enToml.split("\n").find((z) => z.startsWith(`"${de}" = `));
  return zeile?.slice(de.length + 5).replace(/^"|"$/g, "");
}

/** Antworten der Handgriffe mit Ergebnis: alles gelingt bzw. nichts gelingt. */
const ERFOLG: Partial<Record<keyof Handgriffe, unknown>> = {
  paketSuchen: { index: "da", gefunden: true },
  paketHinzufuegen: "geholt",
  netzZugriffe: 1,
};
const MISSERFOLG: Partial<Record<keyof Handgriffe, unknown>> = {
  paketSuchen: { index: "fehlt", gefunden: false },
  paketHinzufuegen: "offline",
  netzZugriffe: 0,
};

/** Handgriffe zum Mitschreiben; ein Name außerhalb der API wirft. */
function aufnahme(antworten = ERFOLG): { api: Handgriffe; aufrufe: { name: string; args: unknown[] }[] } {
  const aufrufe: { name: string; args: unknown[] }[] = [];
  const api = new Proxy({} as Handgriffe, {
    get(_, name) {
      if (typeof name !== "string" || !(HANDGRIFFE as readonly string[]).includes(name)) throw new Error(`kein Handgriff: ${String(name)}`);
      return async (...args: unknown[]) => {
        aufrufe.push({ name, args });
        return antworten[name as keyof Handgriffe];
      };
    },
  });
  return { api, aufrufe };
}

/** Grobe Dauer einer Vorführung in Sekunden: Vorlauf, Standzeiten, Tippen und Eingaben (wie
 *  main.ts: 70 bzw. 80 ms je Zeichen, Zeilenwechsel dreifach), feste Wartezeiten; ohne Netz und Setzen. */
async function geschaetzt(r: Rundgang): Promise<number> {
  let ms = VORLAUF;
  for (const s of r.schritte) {
    if (s.kopie) ms += VORLAUF + 3000;
    let text = s.text;
    if (s.vorher) {
      const { api, aufrufe } = aufnahme();
      await s.vorher(api);
      for (const a of aufrufe) {
        const arg = String(a.args[0] ?? "");
        if (a.name === "tippen") ms += [...arg].reduce((n, z) => n + (z === "\n" ? 3 : 1), 0) * Number(a.args[1] ?? 70);
        if (a.name === "eingeben") ms += [...String(a.args[1])].length * Number(a.args[2] ?? 80);
        if (a.name === "warten") ms += Number(a.args[0]);
        if (a.name === "feldSetzen") ms += 2000;
        if (a.name === "sagen") text = arg;
      }
    }
    ms += dauerVon({ text, dauer: s.dauer });
  }
  return ms / 1000;
}

describe("Rundgänge", () => {
  const alle = rundgaenge();

  it("gibt es, mit eindeutiger ID, Titel und Beschreibung; die große Vorführung zuerst", () => {
    expect(alle.map((r) => r.id)).toEqual([
      "in-minuten",
      "ueberblick",
      "schreiben",
      "bausteine",
      "vorlagen",
      "literatur",
      "pakete",
      "lange-dokumente",
      "export",
      "datenschutz",
      "aussehen",
    ]);
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
        // Attribute (`[data-pfad="posten.0.menge"]`) zählen nicht als Klasse oder ID.
        const auswahl = (s.ziel as string).replace(/\[[^\]]*\]/g, "");
        const ids = [...auswahl.matchAll(/#([\w-]+)/g)].map((m) => m[1]);
        const klassen = [...auswahl.matchAll(/\.([\w-]+)/g)].map((m) => m[1]);
        expect(ids.length + klassen.length, wo).toBeGreaterThan(0);
        for (const id of ids) {
          if (IDS_HTML.has(id)) continue;
          const modul = DYNAMISCHE_ZIELE[id];
          expect(modul, `${wo}: #${id} weder in index.html noch in DYNAMISCHE_ZIELE`).toBeDefined();
          expect(quellen[`./${modul}`], `${wo}: ${modul}`).toMatch(new RegExp(`\\.id = "${id}"`));
        }
        // Klassen von CodeMirror (deren Namen sind fest) oder aus einem Modul der Oberfläche, unter einer ID.
        for (const k of klassen) {
          if (k.startsWith("cm-")) continue;
          expect(ids.length, `${wo}: .${k} ohne ID davor`).toBeGreaterThan(0);
          expect(MODULE, `${wo}: .${k} in keinem Modul`).toMatch(new RegExp(`["' ]${k}["' ]`));
        }
      }
    }
  });

  it("dynamische Ziele werden wirklich vergeben", () => {
    for (const [id, modul] of Object.entries(DYNAMISCHE_ZIELE)) {
      expect(IDS_HTML.has(id), id).toBe(false);
      expect(quellen[`./${modul}`], modul).toMatch(new RegExp(`\\.id = "${id}"`));
    }
  });

  it("Wechsel der Kopie nur mitten in einem Rundgang mit Kopie, zu einem vorhandenen Typ", () => {
    for (const r of alle) {
      for (const [i, s] of r.schritte.entries()) {
        if (s.kopie === undefined) continue;
        const wo = `${r.id} ${i + 1}`;
        expect(r.kopie, wo).toBeDefined();
        expect(i, wo).toBeGreaterThan(0);
        expect(Object.keys(typen), wo).toContain(`../../types/${s.kopie}/typst.toml`);
      }
    }
  });

  it("eine eigene Frist nur mit Handgriff", () => {
    for (const r of alle) for (const [i, s] of r.schritte.entries()) if (s.frist !== undefined) expect(s.vorher, `${r.id} ${i + 1}`).toBeDefined();
  });

  it("Kapiteldateien, die ein Rundgang öffnet, gibt es in der Vorlage (deutsch und englisch) oder er legt sie mit #include an", async () => {
    for (const r of alle) {
      const angelegt = new Set<string>();
      let typ = r.kopie;
      for (const [i, s] of r.schritte.entries()) {
        typ = s.kopie ?? typ;
        if (!s.vorher) continue;
        const { api, aufrufe } = aufnahme();
        await s.vorher(api);
        for (const a of aufrufe) {
          const arg = String(a.args[0]);
          if (a.name === "tippen") for (const m of arg.matchAll(/#include "([^"]+)"/g)) angelegt.add(m[1]);
          if (a.name !== "reiterOeffnen" || arg === "main.typ" || angelegt.has(arg)) continue;
          const wo = `${r.id} ${i + 1}: ${arg}`;
          expect(vorlagen, wo).toContain(`../../types/${typ}/vorlage/${arg}`);
          const en = englisch(arg);
          expect(en, `${wo}: fehlt in en.toml`).toBeDefined();
          if (en) expect(vorlagen, wo).toContain(`../../types/${typ}/vorlage-en/${en}`);
        }
      }
    }
  });

  it("das Paket wird so verwendet, wie es heißt", async () => {
    const r = alle.find((x) => x.id === "pakete")!;
    const getippt: string[] = [];
    for (const s of r.schritte) {
      if (!s.vorher) continue;
      const { api, aufrufe } = aufnahme();
      await s.vorher(api);
      for (const a of aufrufe) if (a.name === "tippen") getippt.push(String(a.args[0]));
      for (const a of aufrufe) if (a.name === "paketSuchen" || a.name === "paketHinzufuegen") expect(a.args[0]).toBe(PAKET);
    }
    expect(getippt.join("")).toContain(`#${PAKET}.${PAKET}(`);
  });

  it("geht das Holen nicht, tippt der Rundgang das Paket nicht ein und sagt es", async () => {
    for (const id of ["pakete", "in-minuten"]) {
      const r = rundgaenge().find((x) => x.id === id)!;
      const namen: string[] = [];
      for (const s of r.schritte) {
        if (!s.vorher) continue;
        const { api, aufrufe } = aufnahme(MISSERFOLG);
        await s.vorher(api);
        namen.push(...aufrufe.map((a) => (a.name === "tippen" ? `tippen:${String(a.args[0])}` : a.name)));
      }
      expect(namen.some((n) => n.startsWith("tippen:") && n.includes(PAKET)), id).toBe(false);
      expect(namen.filter((n) => n === "sagen").length, id).toBeGreaterThan(1);
    }
  });

  it("Länge: Themen-Rundgänge 6 bis 14 Schritte, die Vorführung etwa 4 bis 7 Minuten", async () => {
    for (const r of alle) {
      if (r.id === "in-minuten" || r.id === "ueberblick" || r.id === "schreiben") continue;
      expect(r.schritte.length, r.id).toBeGreaterThanOrEqual(6);
      expect(r.schritte.length, r.id).toBeLessThanOrEqual(14);
    }
    const sekunden = await geschaetzt(alle[0]);
    expect(sekunden).toBeGreaterThan(200);
    expect(sekunden).toBeLessThan(420);
    // Lesbar: kein Untertitel kürzer als die Mindestzeit, keiner länger als zwei Zeilen im Band.
    for (const r of alle) for (const s of r.schritte) expect(Math.max(MIN_DAUER, [...s.text].length * MS_JE_ZEICHEN), s.text).toBeLessThan(12_000);
  });

  it("Handgriffe nur aus der API; wer schreibt, läuft in einer Kopie eines vorhandenen Typs", async () => {
    for (const r of alle) {
      if (r.kopie !== undefined) expect(Object.keys(typen), r.id).toContain(`../../types/${r.kopie}/typst.toml`);
      for (const [i, s] of r.schritte.entries()) {
        if (!s.vorher) continue;
        const { api, aufrufe } = aufnahme();
        await s.vorher(api);
        // Der Misserfolgsweg darf nicht werfen und ebenso nur die API benutzen.
        const anders = aufnahme(MISSERFOLG);
        await s.vorher(anders.api);
        aufrufe.push(...anders.aufrufe);
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
