import { describe, expect, it } from "vitest";
import { fassungZeit } from "./fassungen";

const jetzt = new Date(2026, 10, 11, 15, 30); // Mi 11.11.2026
const s = (d: Date) => Math.floor(d.getTime() / 1000);

describe("fassungZeit", () => {
  it("heute und gestern mit Uhrzeit", () => {
    expect(fassungZeit(s(new Date(2026, 10, 11, 9, 5)), jetzt)).toBe("heute 09:05");
    expect(fassungZeit(s(new Date(2026, 10, 10, 23, 59)), jetzt)).toBe("gestern 23:59");
  });
  it("ältere mit Wochentag und Datum", () => {
    expect(fassungZeit(s(new Date(2026, 10, 3, 8, 0)), jetzt)).toBe("Di 3.11. 08:00");
  });
  it("aus einem anderen Jahr mit Jahreszahl", () => {
    expect(fassungZeit(s(new Date(2025, 11, 30, 18, 15)), jetzt)).toBe("Di 30.12.2025 18:15");
  });
});
