import { describe, expect, it } from "vitest";
import { dateiname, ergebnisInhalt, hauptdateiAnpassung, namenListe, type StueckJs, wahlLesen } from "./serie";

describe("dateiname", () => {
  it("nimmt den letzten Teil, auch mit Backslash", () => {
    expect(dateiname("/a/b/Rechnung Huber.pdf")).toBe("Rechnung Huber.pdf");
    expect(dateiname("C:\\x\\y.pdf")).toBe("y.pdf");
  });
});

describe("hauptdateiAnpassung", () => {
  it("ersetzt toml(\"dokument.toml\") durch sys.inputs mit Vorgabe", () => {
    const text = '#show: brief.with(daten: toml("dokument.toml"))\nText';
    const a = hauptdateiAnpassung(text, "dokument.toml");
    expect(a).not.toBeNull();
    const neu = text.slice(0, a!.von) + a!.neu + text.slice(a!.bis);
    expect(neu).toBe('#show: brief.with(daten: toml(sys.inputs.at("daten", default: "dokument.toml")))\nText');
  });
  it("nicht eindeutig oder nicht vorhanden: nichts", () => {
    expect(hauptdateiAnpassung("= Text", "dokument.toml")).toBeNull();
    expect(hauptdateiAnpassung('toml("dokument.toml") toml("dokument.toml")', "dokument.toml")).toBeNull();
  });
});

describe("namenListe", () => {
  it("kürzt lange Listen", () => {
    expect(namenListe(["a", "b"])).toBe("a, b");
    expect(namenListe(["a", "b", "c", "d"], 2)).toBe("a, b und 2 weitere");
  });
});

describe("wahlLesen", () => {
  it("liest die gemerkte Wahl, sonst einzeln und einseitig", () => {
    expect(wahlLesen('{"ausgabe":"beides","beidseitig":true}')).toEqual({ ausgabe: "beides", beidseitig: true });
    expect(wahlLesen('{"ausgabe":"quer"}')).toEqual({ ausgabe: "einzeln", beidseitig: false });
    expect(wahlLesen(null)).toEqual({ ausgabe: "einzeln", beidseitig: false });
    expect(wahlLesen("kaputt")).toEqual({ ausgabe: "einzeln", beidseitig: false });
  });
});

describe("ergebnisInhalt", () => {
  const stueck = (name: string, pdf: string | null, seiten = 1): StueckJs => ({
    name,
    pdf,
    fehler: pdf ? null : "falsches Datum",
    gegenprobe: `typst compile ${name}`,
    seiten,
  });

  it("nur gesamt: Sammel-PDF mit Seitenzahl, keine Liste der Einzelnen, Fehler genannt", () => {
    const i = ergebnisInhalt(
      {
        stuecke: [stueck("a", "/x/_lokal/serie/pdf/a.pdf"), stueck("b", null)],
        gesamt: stueck("november-alle", "/d/november-alle.pdf", 2),
      },
      { ausgabe: "gesamt", beidseitig: true },
      "/d",
    );
    expect(i.kopf).toBe("1 von 2 Dokumenten erzeugt.");
    expect(i.gesamt).toBe("Sammel-PDF november-alle.pdf: 2 Seiten");
    expect(i.dateien).toBeNull();
    expect(i.zeigen).toBe("/d/november-alle.pdf");
    expect(i.fehler).toEqual([{ name: "b", text: "falsches Datum" }]);
    expect(i.zeilen).toEqual(["typst compile a", "typst compile november-alle"]);
  });

  it("einzeln: Ordner und Dateinamen, kein Sammel-PDF", () => {
    const i = ergebnisInhalt({ stuecke: [stueck("a", "/d/a.pdf")], gesamt: null }, { ausgabe: "einzeln", beidseitig: false }, "/d");
    expect(i.kopf).toBe("1 von 1 PDFs erzeugt.");
    expect(i.ordner).toBe("in /d:");
    expect(i.dateien).toBe("a.pdf");
    expect(i.gesamt).toBeNull();
    expect(i.zeigen).toBe("/d/a.pdf");
  });

  it("Sammel-PDF fehlgeschlagen: Fehler statt Seitenzahl", () => {
    const g = { ...stueck("x-alle", null), fehler: "kaputt" };
    const i = ergebnisInhalt({ stuecke: [stueck("a", "/d/a.pdf")], gesamt: g }, { ausgabe: "beides", beidseitig: false }, "/d");
    expect(i.gesamtFehler).toBe("kaputt");
    expect(i.zeigen).toBe("/d/a.pdf");
  });
});
