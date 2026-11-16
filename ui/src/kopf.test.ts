import { describe, expect, it } from "vitest";
import { ausblendenNach, kopfName } from "./kopf";

describe("kopfName", () => {
  it("Projektname vorne, Datei dahinter, voller Pfad als Titel", () => {
    expect(kopfName("/Users/a/Documents/Brief an Bank", "main.typ", false, "Ohne Namen")).toEqual({
      name: "Brief an Bank",
      rest: "main.typ",
      titel: "/Users/a/Documents/Brief an Bank/main.typ",
    });
  });
  it("Windows-Pfade und Schrägstrich am Ende", () => {
    expect(kopfName("C:\\Users\\a\\Artikel\\", "main.typ", false, "-").name).toBe("Artikel");
  });
  it("Dokument ohne Namen", () => {
    expect(kopfName("/x/entwurf", "main.typ", true, "Ohne Namen")).toEqual({ name: "Ohne Namen", rest: "", titel: "" });
  });
});

describe("ausblendenNach", () => {
  it("kurze Meldungen verschwinden von selbst", () => {
    expect(ausblendenNach("kurz", 0)).toBe(5000);
  });
  it("Fehler und Fragen mit Knöpfen bleiben stehen", () => {
    expect(ausblendenNach("bleibt", 0)).toBeNull();
    expect(ausblendenNach("kurz", 2)).toBeNull();
    expect(ausblendenNach("angebot", 0)).toBeNull();
    expect(ausblendenNach("angebot", 1)).toBeNull();
  });
});
