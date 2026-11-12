import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { aenderungenAus, pruefsumme } from "./protokoll";

describe("pruefsumme", () => {
  it("entspricht dem Rust-Wert", () => {
    expect(pruefsumme("a")).toBe("af63dc4c8601ec8c");
    expect(pruefsumme("")).toBe("cbf29ce484222325");
  });
});

describe("aenderungenAus", () => {
  it("liefert UTF-16-Positionen im alten Text, aufsteigend", () => {
    const s = EditorState.create({ doc: "ä😀b\nzweite" });
    const tr = s.update({
      changes: [
        { from: 3, to: 4, insert: "c" },
        { from: 0, to: 1, insert: "Ö" },
        { from: 5, insert: "neu\nzeile" },
      ],
    });
    expect(aenderungenAus(tr.changes)).toEqual([
      { von: 0, bis: 1, text: "Ö" },
      { von: 3, bis: 4, text: "c" },
      { von: 5, bis: 5, text: "neu\nzeile" },
    ]);
  });

  it("Windows-Zeilenenden im eingefügten Text werden zu \\n", () => {
    const s = EditorState.create({ doc: "" });
    const tr = s.update({ changes: { from: 0, insert: "a\r\nb" } });
    expect(aenderungenAus(tr.changes)).toEqual([{ von: 0, bis: 0, text: "a\nb" }]);
  });
});
