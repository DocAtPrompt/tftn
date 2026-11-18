import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type Anzeige,
  type Befehl,
  type Buehne,
  FRIST,
  HANDGRIFFE,
  type Handgriffe,
  Lauf,
  MIN_DAUER,
  type Rundgang,
  VORLAUF,
  bandAbstandUnten,
  dauerVon,
  laeuftIn,
  naechsterSchritt,
} from "./rundgang";

/** Bühne zum Mitschreiben; beim Ansehen antwortet sie der Reihe nach mit `befehle`. */
class Probe implements Buehne {
  gezeigt: Anzeige[] = [];
  constructor(private befehle: Befehl[] = []) {}
  zeigen(a: Anzeige) {
    this.gezeigt.push(a);
  }
  befehl(): Promise<Befehl> {
    const b = this.befehle.shift();
    return b ? Promise.resolve(b) : new Promise(() => {});
  }
}

/** Handgriffe, die nur mitschreiben. */
function mitschreiben(log: string[]) {
  return () => {
    const api: Record<string, (...a: unknown[]) => Promise<void>> = {};
    for (const n of HANDGRIFFE) if (n !== "warten") api[n] = async (...a) => void log.push(`${n}(${a.join(",")})`);
    return api as unknown as Omit<Handgriffe, "warten">;
  };
}

const ziele = new Map<string, Element>();
const suchen = (z: string | (() => Element | null)) => (typeof z === "string" ? (ziele.get(z) ?? null) : z());

function rundgang(...texte: string[]): Rundgang {
  return { id: "probe", titel: "Probe", beschreibung: "", schritte: texte.map((text) => ({ text })) };
}

describe("Rundgang: reine Regeln", () => {
  it("Standzeit aus der Textlänge, mindestens drei Sekunden", () => {
    expect(dauerVon({ text: "kurz" })).toBe(MIN_DAUER);
    expect(dauerVon({ text: "x".repeat(100) })).toBe(6000);
    expect(dauerVon({ text: "x".repeat(100), dauer: 1000 })).toBe(1000);
  });

  it("Handgriffe laufen beim Vorführen, beim Ansehen nur mit „beide“", () => {
    const vorher = async () => {};
    expect(laeuftIn({ vorher }, "vorfuehren")).toBe(true);
    expect(laeuftIn({ vorher }, "ansehen")).toBe(false);
    expect(laeuftIn({ vorher, vorherIn: "beide" }, "ansehen")).toBe(true);
    expect(laeuftIn({}, "vorfuehren")).toBe(false);
  });

  it("Weiter, Zurück und Ende beim Ansehen", () => {
    expect(naechsterSchritt(0, "weiter", 3)).toBe(1);
    expect(naechsterSchritt(2, "weiter", 3)).toBeNull();
    expect(naechsterSchritt(0, "zurueck", 3)).toBe(0);
    expect(naechsterSchritt(2, "zurueck", 3)).toBe(1);
    expect(naechsterSchritt(1, "ende", 3)).toBeNull();
  });

  it("Band über einem Ziel im unteren Teil des Fensters", () => {
    expect(bandAbstandUnten(null, 800)).toBe(48);
    expect(bandAbstandUnten({ oben: 100, unten: 700 }, 800)).toBe(48);
    expect(bandAbstandUnten({ oben: 770, unten: 800 }, 800)).toBe(48);
    expect(bandAbstandUnten({ oben: 600, unten: 640 }, 800)).toBe(216);
  });
});

describe("Rundgang: Ansehen", () => {
  it("blättert vor und zurück bis zum Ende", async () => {
    const b = new Probe(["weiter", "zurueck", "weiter", "weiter", "weiter"]);
    const lauf = new Lauf(rundgang("a", "b", "c"), "ansehen", b, mitschreiben([]), suchen);
    expect(await lauf.starten()).toBe("fertig");
    expect(b.gezeigt.filter((_, i) => i % 2 === 1).map((a) => a.text)).toEqual(["a", "b", "a", "b", "c"]);
    expect(lauf.laeuft).toBe(false);
  });

  it("vor dem Zurückblättern wird aufgeräumt", async () => {
    let geraeumt = 0;
    const b = new Probe(["weiter", "zurueck", "weiter", "weiter", "weiter"]);
    await new Lauf(rundgang("a", "b", "c"), "ansehen", b, mitschreiben([]), suchen, () => geraeumt++).starten();
    expect(geraeumt).toBe(1);
  });

  it("Beenden gilt als Abbruch", async () => {
    const lauf = new Lauf(rundgang("a", "b"), "ansehen", new Probe(["ende"]), mitschreiben([]), suchen);
    expect(await lauf.starten()).toBe("abgebrochen");
  });

  it("Esc, während auf den Benutzer gewartet wird", async () => {
    const b = new Probe();
    const lauf = new Lauf(rundgang("a", "b"), "ansehen", b, mitschreiben([]), suchen);
    const ende = lauf.starten();
    await Promise.resolve();
    lauf.abbrechen();
    expect(await ende).toBe("abgebrochen");
    expect(b.gezeigt.map((a) => a.text)).toEqual(["a", "a"]);
  });

  it("Handgriff nur mit „beide“, Ziel hervorgehoben, fehlendes Ziel ohne Hervorhebung", async () => {
    const log: string[] = [];
    const da = { id: "da" } as unknown as Element;
    ziele.set("#da", da);
    const r: Rundgang = {
      id: "x",
      titel: "",
      beschreibung: "",
      schritte: [
        { text: "eins", ziel: "#da", vorher: (api) => api.fachOeffnen("einfuegen"), vorherIn: "beide" },
        { text: "zwei", ziel: "#fehlt", vorher: (api) => api.tippen("nein") },
        { text: "drei", ziel: () => { throw new Error("kaputt"); } },
      ],
    };
    const b = new Probe(["weiter", "weiter", "weiter"]);
    expect(await new Lauf(r, "ansehen", b, mitschreiben(log), suchen).starten()).toBe("fertig");
    expect(log).toEqual(["fachOeffnen(einfuegen)"]);
    expect(b.gezeigt.map((a) => [a.text, a.ziel])).toEqual([
      ["eins", null],
      ["eins", da],
      ["zwei", null],
      ["zwei", null],
      ["drei", null],
      ["drei", null],
    ]);
  });
});

describe("Rundgang: Vorführen", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("läuft im Takt von selbst ab, Handgriffe vor der Standzeit", async () => {
    const log: string[] = [];
    const r: Rundgang = {
      id: "x",
      titel: "",
      beschreibung: "",
      kopie: "tftn-fachartikel",
      schritte: [
        { text: "eins", vorher: (api) => api.tippen("Hallo") },
        { text: "zwei", dauer: 5000 },
      ],
    };
    const b = new Probe();
    let ergebnis = "";
    void new Lauf(r, "vorfuehren", b, mitschreiben(log), suchen).starten().then((e) => (ergebnis = e));
    await vi.advanceTimersByTimeAsync(VORLAUF - 1);
    expect(b.gezeigt).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(log).toEqual(["tippen(Hallo)"]);
    expect(b.gezeigt.at(-1)!.text).toBe("eins");
    await vi.advanceTimersByTimeAsync(MIN_DAUER);
    expect(b.gezeigt.at(-1)!.text).toBe("zwei");
    await vi.advanceTimersByTimeAsync(4900);
    expect(ergebnis).toBe("");
    await vi.advanceTimersByTimeAsync(100);
    expect(ergebnis).toBe("fertig");
  });

  it("Esc bricht mitten in einer Pause und mitten im Tippen ab", async () => {
    const log: string[] = [];
    const r: Rundgang = {
      id: "x",
      titel: "",
      beschreibung: "",
      schritte: [
        {
          text: "tippt",
          vorher: async (api) => {
            for (const z of "abcdef") {
              log.push(z);
              await api.warten(100);
            }
          },
        },
        { text: "nie" },
      ],
    };
    const b = new Probe();
    const lauf = new Lauf(r, "vorfuehren", b, mitschreiben([]), suchen);
    const ende = lauf.starten();
    await vi.advanceTimersByTimeAsync(VORLAUF + 250);
    lauf.abbrechen();
    expect(await ende).toBe("abgebrochen");
    await vi.advanceTimersByTimeAsync(5000);
    expect(log).toEqual(["a", "b", "c"]);
    expect(b.gezeigt.every((a) => a.text !== "nie")).toBe(true);
  });

  it("Anhalten hält die Zeit an; Fortsetzen läuft weiter", async () => {
    const b = new Probe();
    const lauf = new Lauf(rundgang("eins", "zwei"), "vorfuehren", b, mitschreiben([]), suchen);
    void lauf.starten();
    await vi.advanceTimersByTimeAsync(VORLAUF + 1000);
    lauf.anhalten();
    expect(lauf.istAngehalten).toBe(true);
    expect(b.gezeigt.at(-1)!.angehalten).toBe(true);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(b.gezeigt.at(-1)!.text).toBe("eins");
    lauf.anhalten();
    await vi.advanceTimersByTimeAsync(MIN_DAUER - 1000);
    expect(b.gezeigt.at(-1)!.text).toBe("zwei");
    lauf.abbrechen();
  });

  it("ein scheiternder oder hängender Handgriff hält nicht auf", async () => {
    const spaet: string[] = [];
    const r: Rundgang = {
      id: "x",
      titel: "",
      beschreibung: "",
      schritte: [
        { text: "scheitert", vorher: async () => { throw new Error("weg"); } },
        {
          text: "hängt",
          vorher: async (api) => {
            await new Promise(() => {});
            await api.tippen("nie");
          },
        },
        {
          text: "trödelt",
          vorher: async (api) => {
            await new Promise((w) => setTimeout(w, FRIST + 5000));
            // Nach der Frist verworfen: Dieser Handgriff darf nichts mehr tun.
            await api.tippen("zu spät").catch(() => spaet.push("abgewiesen"));
          },
          dauer: 10_000,
        },
        { text: "weiter" },
      ],
    };
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const log: string[] = [];
    const b = new Probe();
    let ergebnis = "";
    void new Lauf(r, "vorfuehren", b, mitschreiben(log), suchen).starten().then((e) => (ergebnis = e));
    await vi.advanceTimersByTimeAsync(VORLAUF + MIN_DAUER + FRIST + MIN_DAUER + FRIST + 10_000 + MIN_DAUER + 100);
    expect(ergebnis).toBe("fertig");
    expect(b.gezeigt.filter((a, i, l) => l.findIndex((x) => x.text === a.text) === i).map((a) => a.text)).toEqual(["scheitert", "hängt", "trödelt", "weiter"]);
    expect(log).toEqual([]);
    expect(spaet).toEqual(["abgewiesen"]);
  });
});
