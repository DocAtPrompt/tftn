// Tabellenraster (PLAN-M2 AP6, Patch-Klasse Tabellenstruktur aus AP5): Auswahl und Knöpfe als reine
// Logik. Die Aktionen haben die JSON-Form von `tftn_edit::TabellenAktion`.

export interface Zelle {
  zeile: number;
  spalte: number;
  zeilen: number;
  spalten: number;
  inhalt: { art: string; text: string };
  weitere: string[];
}

export interface Tabelle {
  spalten: number;
  zeilen: number;
  spuren: string[] | null;
  weitere: string[];
  zellen: Zelle[];
}

export interface Auswahl {
  zeile: number;
  spalte: number;
  zeilen: number;
  spalten: number;
}

export type Knopf =
  | "zeile-oben"
  | "zeile-unten"
  | "spalte-links"
  | "spalte-rechts"
  | "zeile-loeschen"
  | "spalte-loeschen"
  | "zeile-hoch"
  | "zeile-runter"
  | "spalte-vor"
  | "spalte-zurueck"
  | "verbinden"
  | "trennen";

const schneidet = (z: Zelle, a: Auswahl) =>
  z.zeile < a.zeile + a.zeilen && z.zeile + z.zeilen > a.zeile && z.spalte < a.spalte + a.spalten && z.spalte + z.spalten > a.spalte;

/** Rechteck von a nach b ([zeile, spalte]), gewachsen, bis es nur ganze Zellen enthält. */
export function auswahlRechteck(t: Tabelle, a: [number, number], b: [number, number]): Auswahl {
  let oben = Math.min(a[0], b[0]);
  let unten = Math.max(a[0], b[0]) + 1;
  let links = Math.min(a[1], b[1]);
  let rechts = Math.max(a[1], b[1]) + 1;
  for (let geaendert = true; geaendert; ) {
    geaendert = false;
    const r = { zeile: oben, spalte: links, zeilen: unten - oben, spalten: rechts - links };
    for (const z of t.zellen.filter((z) => schneidet(z, r))) {
      const [o, u, l, re] = [Math.min(oben, z.zeile), Math.max(unten, z.zeile + z.zeilen), Math.min(links, z.spalte), Math.max(rechts, z.spalte + z.spalten)];
      if (o !== oben || u !== unten || l !== links || re !== rechts) {
        [oben, unten, links, rechts] = [o, u, l, re];
        geaendert = true;
      }
    }
  }
  return { zeile: oben, spalte: links, zeilen: unten - oben, spalten: rechts - links };
}

/** Die eine Zelle, die genau die Auswahl ist, sonst null. */
function genauEineZelle(t: Tabelle, a: Auswahl): Zelle | null {
  const drin = t.zellen.filter((z) => schneidet(z, a));
  const z = drin.length === 1 ? drin[0] : null;
  return z && z.zeile === a.zeile && z.spalte === a.spalte && z.zeilen === a.zeilen && z.spalten === a.spalten ? z : null;
}

/** Die Tabellen-Aktion für Knopf und Auswahl, oder null, wenn der Knopf hier nicht geht. */
export function aktionFuer(k: Knopf, a: Auswahl, t: Tabelle): object | null {
  switch (k) {
    case "zeile-oben":
      return { art: "zeile-einfuegen", vor: a.zeile };
    case "zeile-unten":
      return { art: "zeile-einfuegen", vor: a.zeile + a.zeilen };
    case "spalte-links":
      return { art: "spalte-einfuegen", vor: a.spalte };
    case "spalte-rechts":
      return { art: "spalte-einfuegen", vor: a.spalte + a.spalten };
    case "zeile-loeschen":
      return a.zeilen === 1 && t.zeilen > 1 ? { art: "zeile-loeschen", zeile: a.zeile } : null;
    case "spalte-loeschen":
      return a.spalten === 1 && t.spalten > 1 ? { art: "spalte-loeschen", spalte: a.spalte } : null;
    case "zeile-hoch":
      return a.zeilen === 1 && a.zeile > 0 ? { art: "zeile-verschieben", von: a.zeile, nach: a.zeile - 1 } : null;
    case "zeile-runter":
      return a.zeilen === 1 && a.zeile < t.zeilen - 1 ? { art: "zeile-verschieben", von: a.zeile, nach: a.zeile + 1 } : null;
    case "spalte-vor":
      return a.spalten === 1 && a.spalte > 0 ? { art: "spalte-verschieben", von: a.spalte, nach: a.spalte - 1 } : null;
    case "spalte-zurueck":
      return a.spalten === 1 && a.spalte < t.spalten - 1 ? { art: "spalte-verschieben", von: a.spalte, nach: a.spalte + 1 } : null;
    case "verbinden":
      return a.zeilen * a.spalten >= 2 && !genauEineZelle(t, a)
        ? { art: "verbinden", zeile: a.zeile, spalte: a.spalte, zeilen: a.zeilen, spalten: a.spalten }
        : null;
    case "trennen": {
      const z = genauEineZelle(t, a);
      return z && z.zeilen * z.spalten > 1 ? { art: "trennen", zeile: z.zeile, spalte: z.spalte } : null;
    }
  }
}

/** Ausdruckszellen (`image("a.png")`, `"Text"`) werden nicht als Text überschrieben. */
export function bearbeitbar(z: Zelle): boolean {
  return z.inhalt.art === "markup";
}

/** Die Auswahl nach einer Aktion: sie folgt verschobenen und eingefügten Zeilen bzw. Spalten. */
export function auswahlNach(aktion: any, a: Auswahl): Auswahl | null {
  switch (aktion.art) {
    case "zeile-verschieben":
      return { ...a, zeile: aktion.nach };
    case "spalte-verschieben":
      return { ...a, spalte: aktion.nach };
    case "zeile-einfuegen":
      return aktion.vor <= a.zeile ? { ...a, zeile: a.zeile + 1 } : a;
    case "spalte-einfuegen":
      return aktion.vor <= a.spalte ? { ...a, spalte: a.spalte + 1 } : a;
    case "zeile-loeschen":
    case "spalte-loeschen":
      return null;
    default:
      return a;
  }
}
