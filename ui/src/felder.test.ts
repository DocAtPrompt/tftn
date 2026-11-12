import { describe, expect, it } from "vitest";
import { EditorSelection, EditorState, type TransactionSpec } from "@codemirror/state";
import { history, undo } from "@codemirror/commands";
import type { EingesetztJs } from "./bausteine";
import { aktiveFelder, einsetzenSpec, feldWechsel, felder } from "./felder";

const ABB_TEXT = '#figure(image("bild.png"), caption: [Text])';
const abb: EingesetztJs = {
  text: ABB_TEXT,
  felder: [
    { name: "Datei", von: ABB_TEXT.indexOf("bild.png"), bis: ABB_TEXT.indexOf("bild.png") + 8 },
    { name: "Bildunterschrift", von: ABB_TEXT.indexOf("Text"), bis: ABB_TEXT.indexOf("Text") + 4 },
  ],
  cursor: null,
};
const IMPORT = { pos: 0, text: '#import "@preview/demo:0.1.3": kasten\n' };

const neu = (doc: string) => EditorState.create({ doc, extensions: [history(), felder] });
const schritt = (s: EditorState, spec: TransactionSpec | null) => {
  expect(spec).not.toBeNull();
  return s.update(spec!).state;
};
const auswahl = (s: EditorState) => s.sliceDoc(s.selection.main.from, s.selection.main.to);

describe("Felder", () => {
  it("ersetzt das / und wählt das erste Feld", () => {
    const s = schritt(neu("/abb"), einsetzenSpec(neu("/abb"), 0, 4, abb, null));
    expect(s.doc.toString()).toBe(ABB_TEXT);
    expect(auswahl(s)).toBe("bild.png");
    expect(aktiveFelder(s)?.map((f) => f.name)).toEqual(["Datei", "Bildunterschrift"]);
  });

  it("Tab läuft durch die Felder und endet am Schluss", () => {
    let s = schritt(neu("/abb"), einsetzenSpec(neu("/abb"), 0, 4, abb, null));
    s = schritt(s, feldWechsel(s, -1)); // am ersten bleibt es stehen
    expect(auswahl(s)).toBe("bild.png");
    s = schritt(s, feldWechsel(s, 1));
    expect(auswahl(s)).toBe("Text");
    s = schritt(s, feldWechsel(s, -1));
    expect(auswahl(s)).toBe("bild.png");
    s = schritt(s, feldWechsel(s, 1));
    s = schritt(s, feldWechsel(s, 1));
    expect(s.selection.main.head).toBe(ABB_TEXT.length);
    expect(s.selection.main.empty).toBe(true);
    expect(aktiveFelder(s)).toBeNull();
    expect(feldWechsel(s, 1)).toBeNull();
  });

  it("Import davor verschiebt die Felder, auch nach Umlauten und Emoji", () => {
    const vorher = neu("Ä😀\n/");
    const s = schritt(vorher, einsetzenSpec(vorher, 4, 5, abb, IMPORT));
    const text = s.doc.toString();
    expect(text).toBe(`${IMPORT.text}Ä😀\n${ABB_TEXT}`);
    expect(s.selection.main.from).toBe(text.indexOf("bild.png"));
    expect(auswahl(s)).toBe("bild.png");
    const t = schritt(s, feldWechsel(s, 1));
    expect(auswahl(t)).toBe("Text");
  });

  it("Tippen im Feld vergrößert es, das nächste verschiebt sich", () => {
    let s = schritt(neu("/abb"), einsetzenSpec(neu("/abb"), 0, 4, abb, null));
    s = schritt(s, s.replaceSelection("neu/bild-2.png"));
    expect(aktiveFelder(s)?.[0]).toMatchObject({ von: 15, bis: 15 + "neu/bild-2.png".length });
    s = schritt(s, feldWechsel(s, 1));
    expect(auswahl(s)).toBe("Text");
  });

  it("leeres Feld wird gefüllt, Tab springt weiter", () => {
    const fn: EingesetztJs = { text: "#footnote[]", felder: [{ name: "Inhalt", von: 10, bis: 10 }], cursor: null };
    let s = schritt(neu(""), einsetzenSpec(neu(""), 0, 0, fn, null));
    expect(s.selection.main.head).toBe(10);
    for (const z of "abc") s = schritt(s, s.replaceSelection(z));
    expect(s.doc.toString()).toBe("#footnote[abc]");
    expect(aktiveFelder(s)?.[0]).toMatchObject({ von: 10, bis: 13 });
    s = schritt(s, feldWechsel(s, 1));
    expect(s.selection.main.head).toBe(14);
    expect(aktiveFelder(s)).toBeNull();
  });

  it("ein Undo nimmt Schnipsel und Import zurück", () => {
    const vorher = neu("Text\n/");
    let s = schritt(vorher, einsetzenSpec(vorher, 5, 6, abb, IMPORT));
    undo({ state: s, dispatch: (tr) => (s = tr.state) });
    expect(s.doc.toString()).toBe("Text\n/");
  });

  it("nach Undo sind die Felder aus, auch mit Cursor hinter /abb", () => {
    const vorher = EditorState.create({ doc: "/abb", selection: { anchor: 4 }, extensions: [history(), felder] });
    let s = schritt(vorher, einsetzenSpec(vorher, 0, 4, abb, null));
    undo({ state: s, dispatch: (tr) => (s = tr.state) });
    expect(s.doc.toString()).toBe("/abb");
    expect(s.selection.main.head).toBe(4);
    expect(aktiveFelder(s)).toBeNull();
    expect(feldWechsel(s, 1)).toBeNull();
  });

  it("Überschreiben des ganzen Schnipsels beendet die Felder", () => {
    let s = schritt(neu("/abb"), einsetzenSpec(neu("/abb"), 0, 4, abb, null));
    s = schritt(s, { changes: { from: 0, to: s.doc.length, insert: "x" }, selection: { anchor: 1 } });
    expect(aktiveFelder(s)).toBeNull();
  });

  it("Klick außerhalb beendet die Felder", () => {
    let s = schritt(neu("Satz\n/"), einsetzenSpec(neu("Satz\n/"), 5, 6, abb, null));
    s = schritt(s, { selection: EditorSelection.cursor(1) });
    expect(aktiveFelder(s)).toBeNull();
  });
});
