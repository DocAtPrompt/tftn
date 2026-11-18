import { describe, expect, it } from "vitest";
import { type ZuletztJs, zuletztPunkte } from "./zuletzt";

const liste: ZuletztJs[] = [
  { pfad: "/doku/Brief", name: "Brief", ort: "doku" },
  { pfad: "/doku/Bericht", name: "Bericht", ort: "doku" },
  { pfad: "/tresor/Brief", name: "Brief", ort: "tresor" },
];

describe("zuletztPunkte", () => {
  it("leere Liste: kein Abschnitt", () => {
    expect(zuletztPunkte([], () => {}, () => {})).toEqual([]);
  });

  it("Name, Pfad im Titel, Ordner nur bei gleichnamigen", () => {
    const p = zuletztPunkte(liste, () => {}, () => {});
    expect(p.map((x) => [x.text, x.zusatz, x.titel, x.trennerDavor])).toEqual([
      ["Brief", "doku", "/doku/Brief", true],
      ["Bericht", undefined, "/doku/Bericht", false],
      ["Brief", "tresor", "/tresor/Brief", false],
      ["Liste leeren", undefined, expect.any(String), true],
    ]);
  });

  it("Klick öffnet den Pfad bzw. leert die Liste", () => {
    const geoeffnet: string[] = [];
    let geleert = 0;
    const p = zuletztPunkte(liste, (pfad) => geoeffnet.push(pfad), () => geleert++);
    p[2].aktion();
    p[3].aktion();
    expect(geoeffnet).toEqual(["/tresor/Brief"]);
    expect(geleert).toBe(1);
  });
});
