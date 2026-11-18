// Skins (Farben und Editorschrift): Rust liest und prüft die Dateien, hier werden die Variablen am
// :root-Element gesetzt. Ohne Skin gelten die Standardwerte aus stil.css, hell oder dunkel wie das System.

import { t, tn } from "./sprache";

export interface SkinJs {
  kennung: string;
  name: string;
  art: "hell" | "dunkel";
  /** CSS-Variablen, z. B. `--grund` → `#fbfaf7` (in Rust geprüft) */
  variablen: Record<string, string>;
  warnungen: string[];
}

export interface SkinsJs {
  gewaehlt: string | null;
  aktiv: SkinJs | null;
  liste: { kennung: string; name: string }[];
  ordner: string;
}

interface Stil {
  setProperty(name: string, wert: string): void;
  removeProperty(name: string): string;
}

/**
 * Setzt `skin` auf `stil` (das `style` von :root) und nimmt vorher zurück, was der vorige gesetzt hat
 * (`vorher`). Gibt die gesetzten Variablen für den nächsten Wechsel zurück. Nur `--…`-Variablen.
 */
export function skinAnwenden(stil: Stil, vorher: string[], skin: SkinJs | null): string[] {
  for (const n of vorher) stil.removeProperty(n);
  stil.removeProperty("color-scheme");
  if (!skin) return [];
  const namen = Object.keys(skin.variablen).filter((n) => n.startsWith("--"));
  for (const n of namen) stil.setProperty(n, skin.variablen[n]);
  stil.setProperty("color-scheme", skin.art === "dunkel" ? "dark" : "light");
  return namen;
}

/**
 * Gilt gerade hell oder dunkel? Ein Skin sagt es selbst (`art`), ohne Skin entscheidet das System.
 * main.ts setzt das Ergebnis als `data-farbart` an :root; stil.css färbt damit den Editor
 * (CodeMirror kennt den Skin nicht und bliebe sonst hell: Zeilennummern, Cursor, Auswahl, Suche).
 */
export function farbart(skin: SkinJs | null, systemDunkel: boolean): "hell" | "dunkel" {
  if (skin) return skin.art;
  return systemDunkel ? "dunkel" : "hell";
}

/** Einträge der Auswahl „Aussehen“: „Wie das System“, dann die Skins; ein fehlender gewählter bleibt sichtbar. */
export function skinOptionen(
  liste: { kennung: string; name: string }[],
  gewaehlt: string | null,
): { wert: string; text: string; gewaehlt: boolean }[] {
  const aus = [
    { wert: "", text: t("Wie das System (hell/dunkel)"), gewaehlt: gewaehlt === null },
    // Namen der mitgelieferten Skins stehen in languages/*.toml; eigene bleiben, wie sie sind.
    ...liste.map((s) => ({ wert: s.kennung, text: tn(s.name), gewaehlt: s.kennung === gewaehlt })),
  ];
  if (gewaehlt !== null && !liste.some((s) => s.kennung === gewaehlt))
    aus.push({ wert: gewaehlt, text: t("{kennung} (nicht gefunden)", { kennung: gewaehlt }), gewaehlt: true });
  return aus;
}
