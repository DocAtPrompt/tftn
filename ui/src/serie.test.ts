import { describe, expect, it } from "vitest";
import {
  bereit,
  type CsvJs,
  csvKopf,
  dateiname,
  ergebnisInhalt,
  feldOptionen,
  fehlerListe,
  fehlerText,
  freieSpalten,
  hauptdateiAnpassung,
  namenListe,
  OFFEN,
  type SerieJs,
  type StueckJs,
  wahlLesen,
} from "./serie";

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

describe("CSV im Dialog", () => {
  const csv: CsvJs = {
    trennzeichen: ";",
    kodierung: "Windows-1252",
    spalten: [
      { name: "Empfänger 1", feld: "empfaenger", eigen: false, beispiel: "Frau" },
      { name: "Telefon", feld: null, eigen: false, beispiel: "0664" },
      { name: "Notiz", feld: null, eigen: true, beispiel: "privat" },
      { name: "Nr", feld: "nummer", eigen: true, beispiel: "1" },
    ],
    felder: [
      { pfad: "nummer", titel: "Rechnungsnummer", liste: false },
      { pfad: "empfaenger", titel: "Empfänger", liste: true },
    ],
    vorschau: [["Empfänger", "Frau\nAnna Muster"]],
    fehler: [],
    zuordnungsfehler: [],
  };

  it("Kopfzeile mit Trennzeichen und Kodierung", () => {
    expect(csvKopf(csv, 3)).toBe("3 Datensätze aus der CSV (Trennzeichen „;“, Windows-1252)");
  });

  it("frei sind nur Spalten ohne Feld, die nicht bewusst abgewählt sind", () => {
    expect(freieSpalten(csv)).toEqual(["Telefon"]);
  });

  it("Auswahl: nicht verwenden zuerst, Listen gekennzeichnet; offene Spalte mit „bitte wählen“", () => {
    expect(feldOptionen(csv.felder)).toEqual([
      { wert: "", text: "– nicht verwenden –" },
      { wert: "nummer", text: "Rechnungsnummer" },
      { wert: "empfaenger", text: "Empfänger (Liste)" },
    ]);
    expect(feldOptionen(csv.felder, true).slice(0, 2)).toEqual([
      { wert: OFFEN, text: "– bitte wählen –" },
      { wert: "", text: "– nicht verwenden –" },
    ]);
  });

  it("Fehler mit Zeile und Spalte, lange Listen gekürzt", () => {
    expect(fehlerText({ zeile: 3, spalte: "Datum", text: "kein Datum" })).toBe("Zeile 3, Spalte „Datum“: kein Datum");
    expect(fehlerText({ zeile: 4, spalte: null, text: "zu viele Zellen" })).toBe("Zeile 4: zu viele Zellen");
    const viele = Array.from({ length: 5 }, (_, i) => ({ zeile: i + 2, spalte: null, text: "x" }));
    expect(fehlerListe(viele, 2)).toEqual(["Zeile 2: x", "Zeile 3: x", "… und 3 weitere"]);
  });

  it("bereit: nicht bei widersprüchlicher Zuordnung", () => {
    const s: SerieJs = { namen: ["a"], zielordner: "/d", hauptdatei: "bereit", daten: "dokument.toml", gesamt_name: "a-alle", csv };
    expect(bereit(s)).toBe(true);
    expect(bereit({ ...s, csv: { ...csv, zuordnungsfehler: ["doppelt"] } })).toBe(false);
    expect(bereit({ ...s, csv: null, hauptdatei: "anpassbar" })).toBe(false);
    expect(bereit({ ...s, namen: [] })).toBe(false);
  });
});
