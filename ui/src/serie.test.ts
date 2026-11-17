import { describe, expect, it } from "vitest";
import { dateiname, hauptdateiAnpassung, namenListe } from "./serie";

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
