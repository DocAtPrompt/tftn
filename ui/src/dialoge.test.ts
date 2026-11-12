import { describe, expect, it } from "vitest";
import { type Dialog, escZiel } from "./dialoge";

const d = (offen: boolean): Dialog => ({ offen, schliessen: () => {} });

describe("escZiel", () => {
  it("nimmt_den_zuletzt_geoeffneten", () => {
    const [a, b, c] = [d(true), d(true), d(false)];
    expect(escZiel([a, b, c])).toBe(b);
  });
  it("nichts_offen", () => {
    expect(escZiel([d(false)])).toBeNull();
    expect(escZiel([])).toBeNull();
  });
});
