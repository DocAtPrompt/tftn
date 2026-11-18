// Eigene Textbausteine (`textbausteine.toml` im Ordner für eigene Anpassungen): markierten Text als
// Baustein speichern. Gelesen, angehängt und beobachtet wird in Rust; hier stehen der Dialog und
// die reinen Hilfen dazu.

import type { EintragJs } from "./bausteine";
import type { PfadDialog } from "./neudialog";
import { t } from "./sprache";

/** Titelvorschlag aus dem markierten Text: die erste Zeile mit Inhalt, gekürzt am Wortende. */
export function titelVorschlag(text: string, laenge = 40): string {
  const zeile = text
    .split("\n")
    .map((z) => z.replace(/^[\s=\-+*/#]+/, "").replace(/\s+/g, " ").trim())
    .find((z) => z !== "");
  if (!zeile) return "";
  if (zeile.length <= laenge) return zeile;
  const kurz = zeile.slice(0, laenge);
  const ende = kurz.lastIndexOf(" ");
  return `${(ende > laenge / 2 ? kurz.slice(0, ende) : kurz).replace(/[\s,;:.]+$/, "")} …`;
}

/** Die Gruppen der eigenen Textbausteine, für die Auswahl im Dialog. */
export function eigeneGruppen(katalog: readonly EintragJs[]): string[] {
  const g = new Set<string>();
  for (const e of katalog) if (e.herkunft === "eigene" && e.gruppe) g.add(e.gruppe);
  return [...g].sort((a, b) => a.localeCompare(b, "de"));
}

/** Hinweis, wenn `textbausteine.toml` fehlerhaft wird oder wieder in Ordnung ist; sonst `null`. */
export function textbausteinHinweis(vorher: string | null, jetzt: string | null): string | null {
  if (jetzt && jetzt !== vorher) return t("Eigene Textbausteine: {meldung}", { meldung: jetzt });
  if (!jetzt && vorher) return t("textbausteine.toml ist wieder in Ordnung.");
  return null;
}

/** Die ersten Zeilen des Textes, damit man im Dialog sieht, was gespeichert wird. */
export function textAuszug(text: string, zeilen = 4): string {
  const alle = text.split("\n");
  return alle.length > zeilen ? `${alle.slice(0, zeilen).join("\n")}\n…` : text;
}

/** Fragt nach Titel und Gruppe; `null` bei Abbruch. */
export async function textbausteinFragen(
  dialog: PfadDialog,
  text: string,
  gruppen: string[],
): Promise<{ titel: string; gruppe: string | null } | null> {
  const zusatz = document.createElement("div");
  zusatz.className = "textbaustein-zusatz";
  const gruppe = document.createElement("input");
  gruppe.type = "text";
  gruppe.className = "neu-eltern";
  gruppe.placeholder = t("Gruppe (optional), z. B. Procedere");
  gruppe.spellcheck = false;
  const liste = document.createElement("datalist");
  liste.id = "textbaustein-gruppen";
  for (const g of gruppen) {
    const o = document.createElement("option");
    o.value = g;
    liste.append(o);
  }
  gruppe.setAttribute("list", liste.id);
  const auszug = document.createElement("pre");
  auszug.className = "textbaustein-auszug";
  auszug.textContent = textAuszug(text);
  const hilfe = document.createElement("p");
  hilfe.className = "leiste-hilfe";
  hilfe.textContent = t("Kommt ans Ende von textbausteine.toml; Lücken zum Ausfüllen dort mit ${feld:Name}.");
  zusatz.append(gruppe, liste, auszug, hilfe);
  const titel = await dialog.fragen(t("Als Textbaustein speichern"), titelVorschlag(text), {
    platzhalter: t("Titel, z. B. Procedere Kontrolle"),
    knopf: t("Speichern"),
  }, zusatz);
  return titel === null ? null : { titel, gruppe: gruppe.value.trim() || null };
}
