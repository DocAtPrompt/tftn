// Bilder in den Text ziehen oder aus der Zwischenablage einfügen: Datei nach bilder/, Abbildung in den Text.

export const BILD_ENDUNGEN = ["png", "jpg", "jpeg", "gif", "svg", "webp", "pdf"];

const TYP_ENDUNG: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/svg+xml": "svg",
  "image/webp": "webp",
  "application/pdf": "pdf",
};

function endung(name: string): string {
  const i = name.lastIndexOf(".");
  return i < 0 ? "" : name.slice(i + 1).toLowerCase();
}

export function istBild(name: string, typ: string): boolean {
  return BILD_ENDUNGEN.includes(endung(name)) || typ in TYP_ENDUNG;
}

const zwei = (n: number) => String(n).padStart(2, "0");

/** Name zum Ablegen; die Zwischenablage liefert „image.png“ oder nichts, das wird ein Bildschirmfoto mit Zeit. */
export function eingefuegterName(name: string, typ: string, jetzt = new Date()): string {
  const e = endung(name);
  if (name && BILD_ENDUNGEN.includes(e) && !/^image\.\w+$/.test(name)) return name;
  const zeit =
    `${jetzt.getFullYear()}-${zwei(jetzt.getMonth() + 1)}-${zwei(jetzt.getDate())}-` +
    `${zwei(jetzt.getHours())}${zwei(jetzt.getMinutes())}${zwei(jetzt.getSeconds())}`;
  return `bildschirmfoto-${zeit}.${TYP_ENDUNG[typ] ?? (BILD_ENDUNGEN.includes(e) ? e : "png")}`;
}

/** Abbildung mit Unterschrift; `cursor` steht in den Klammern der Unterschrift. */
export function abbildungText(pfad: string): { text: string; cursor: number } {
  const kopf = `#figure(\n  image("${pfad}", width: 80%),\n  caption: [`;
  return { text: `${kopf}],\n)`, cursor: kopf.length };
}

/** Wohin ein Block kommt: in eine leere Zeile direkt, sonst hinter die Zeile mit Leerzeile davor und danach. */
export function blockEinfuegen(
  zeile: { from: number; to: number; text: string },
  block: string,
): { von: number; text: string; versatz: number } {
  if (zeile.text.trim() === "") return { von: zeile.from, text: block, versatz: 0 };
  return { von: zeile.to, text: `\n\n${block}\n`, versatz: 2 };
}
