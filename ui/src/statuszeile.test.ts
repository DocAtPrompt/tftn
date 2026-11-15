import { describe, expect, it } from "vitest";
import { dauerText, syncText } from "./statuszeile";

const z = { abgleiche: 3, abweichungen: 0, neusyncs: 0, fehler: 0 };

describe("syncText", () => {
  it("in der fertigen App nur gespeichert/ungespeichert", () => {
    expect(syncText(z, 7, false, false)).toEqual({ text: "gespeichert", warnung: false });
    expect(syncText(z, 7, true, false)).toEqual({ text: "ungespeichert", warnung: false });
  });
  it("ein gestörter Abgleich bleibt auch dort sichtbar", () => {
    const s = syncText({ ...z, abweichungen: 1, fehler: 1 }, 7, false, false);
    expect(s.warnung).toBe(true);
    expect(s.text).toBe("Abgleich gestört (2) · gespeichert");
  });
  it("im Entwicklungsbau alle Messwerte", () => {
    expect(syncText(z, 7, false, true).text).toBe("Rev 7 · Prüfsummen gleich 3 · abweichend 0 · Neusync 0 · Fehler 0 · gespeichert");
  });
});

describe("dauerText", () => {
  it("unter einer Sekunde in ms, darüber in s mit einer Stelle", () => {
    expect(dauerText(87.4)).toBe("87 ms");
    expect(dauerText(0.4)).toBe("< 1 ms");
    expect(dauerText(999.4)).toBe("999 ms");
    expect(dauerText(1234)).toBe("1.2 s");
    expect(dauerText(12345)).toBe("12.3 s");
    expect(dauerText(1234, ",")).toBe("1,2 s");
  });
});
