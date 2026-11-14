// Tastenkürzel (Entwurf 2026-11-04 Abschnitt 7, PLAN-M2 AP3): welche Aktionen es gibt, welches
// Kürzel gilt (eigene Belegung > Vorschlag von Dokumenttyp/Paket > TFTN-Standard) und welche
// Konflikte es auf Mac und Windows gibt. Reine Funktionen; die Ablage liegt in Rust.

import type { KeyBinding } from "@codemirror/view";
import type { EintragJs } from "./bausteine";
import type { EinstellungenJs } from "./einstellungen";
import type { ProfilJs } from "./export";
import { t } from "./sprache";
import { type Plattform, RESERVIERT, befehlsname, editorKeymaps, musterKonflikt } from "./reserviert";

export type { Plattform } from "./reserviert";

export const PLATTFORM: Plattform = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "") ? "mac" : "win";

export interface Aktion {
  id: string;
  titel: string;
  gruppe: string;
  herkunft: "tftn" | "dokumenttyp" | "paket";
  /** `kuerzel` aus Dokumenttyp oder Paket */
  vorschlag: string | null;
}

/** Helfer und App-Befehle; spätere Helfer (Marken, Seitenleiste, Paketsuche) kommen hier dazu. */
export const HELFER: { id: string; titel: string; gruppe: string }[] = [
  { id: "helfer/baustein", titel: t("Baustein einfügen"), gruppe: t("Helfer") },
  { id: "helfer/zitat", titel: t("Zitat einfügen"), gruppe: t("Helfer") },
  { id: "helfer/vervollstaendigen", titel: t("Vervollständigen"), gruppe: t("Helfer") },
  { id: "helfer/einstellungen", titel: t("Einstellungen"), gruppe: t("Helfer") },
  { id: "helfer/werte", titel: t("Werte an dieser Stelle"), gruppe: t("Helfer") },
  { id: "helfer/seitenleiste", titel: t("Seitenleiste"), gruppe: t("Helfer") },
  { id: "helfer/schreibansicht", titel: t("Schreibansicht ohne Ablenkung"), gruppe: t("Helfer") },
  { id: "helfer/fach-einfuegen", titel: t("Seitenleiste: Einfügen"), gruppe: t("Helfer") },
  { id: "helfer/fach-dokument", titel: t("Seitenleiste: Dokument"), gruppe: t("Helfer") },
  { id: "helfer/fach-pakete", titel: t("Seitenleiste: Pakete"), gruppe: t("Helfer") },
  { id: "helfer/fach-formel", titel: t("Seitenleiste: Formel"), gruppe: t("Helfer") },
  { id: "app/speichern", titel: t("Speichern"), gruppe: t("Helfer") },
  { id: "app/neu", titel: t("Neues Projekt …"), gruppe: t("Helfer") },
  { id: "app/oeffnen", titel: t("Projekt öffnen …"), gruppe: t("Helfer") },
  { id: "auszeichnung/ebene-plus", titel: t("Überschrift eine Ebene tiefer"), gruppe: t("Auszeichnung") },
  { id: "auszeichnung/ebene-minus", titel: t("Überschrift eine Ebene höher"), gruppe: t("Auszeichnung") },
];

const beide = (...k: string[]) => ({ mac: k, win: k });

/** TFTN-Standardbelegung (PLAN-M2 AP3, Global Constraints). */
export const STANDARD: Record<string, Partial<Record<Plattform, string[]>>> = {
  "helfer/baustein": beide("Mod-Shift-b"),
  "helfer/zitat": beide("Mod-Shift-l"),
  "helfer/vervollstaendigen": { mac: ["Ctrl-Space", "Alt-`"], win: ["Ctrl-Space"] },
  "helfer/einstellungen": beide("Mod-,"),
  "helfer/werte": beide("Mod-Shift-m"),
  "helfer/seitenleiste": beide("Mod-Shift-p"),
  "helfer/schreibansicht": beide("Mod-Shift-Enter"),
  "helfer/fach-dokument": beide("Mod-Shift-o"),
  "helfer/fach-pakete": beide("Mod-Shift-y"),
  "helfer/fach-formel": beide("Mod-Shift-x"),
  "app/speichern": beide("Mod-s"),
  "app/neu": beide("Mod-n"),
  "app/oeffnen": beide("Mod-o"),
  "auszeichnung/ebene-plus": beide("Mod-Shift-."),
  "auszeichnung/ebene-minus": beide("Mod-Shift-,"),
  "typst/fett": beide("Mod-b"),
  "typst/kursiv": beide("Mod-i"),
  "typst/link": beide("Mod-k"),
  "typst/fussnote": beide("Mod-Shift-f"),
  "typst/abbildung": beide("Mod-Shift-a"),
  "typst/formel": beide("Mod-Shift-e"),
};

const MODIFIKATOREN = ["Ctrl", "Alt", "Shift", "Meta"] as const;

/** Benannte Tasten, die ein Kürzel tragen dürfen (Namen wie `KeyboardEvent.key`), dazu F1–F24. */
const BENANNT = new Set([
  "Space", "Enter", "Escape", "Tab", "Backspace", "Delete", "Insert", "Home", "End", "PageUp", "PageDown",
  "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`),
]);

/**
 * Kürzel für Plattform `p` in fester Form: `Mod` aufgelöst, Modifikatoren als `Ctrl-Alt-Shift-Meta-`,
 * einzelne Zeichen klein (ein Großbuchstabe heißt Shift). `null`, wenn ungültig: ohne Strg, Alt,
 * Meta bzw. Mod und keine F-Taste, oder ein unbekannter Tastenname.
 */
export function normalisieren(k: string, p: Plattform): string | null {
  const teile = k.split(/-(?!$)/);
  let taste = teile.pop() ?? "";
  if (taste === "" || taste.endsWith("-") && taste.length > 1) return null;
  const mods = new Set<string>();
  for (const teil of teile) {
    const n = teil === "Mod" ? (p === "mac" ? "Meta" : "Ctrl") : teil === "Cmd" ? "Meta" : teil === "Control" ? "Ctrl" : teil;
    if (!(MODIFIKATOREN as readonly string[]).includes(n)) return null;
    mods.add(n);
  }
  if ([...taste].length === 1) {
    if (taste !== taste.toLowerCase()) mods.add("Shift");
    taste = taste.toLowerCase();
  }
  if (taste === " ") taste = "Space";
  if ([...taste].length > 1 && !BENANNT.has(taste)) return null;
  const fTaste = /^F([1-9]|1[0-2])$/.test(taste);
  if (!fTaste && !mods.has("Ctrl") && !mods.has("Alt") && !mods.has("Meta")) return null;
  return [...MODIFIKATOREN.filter((m) => mods.has(m)), taste].join("-");
}

/** Grundtaste je keyCode wie die US-Belegung in CodeMirror (w3c-keyname `base`), für Shift-Kombinationen. */
function grundtaste(keyCode: number): string | undefined {
  if (keyCode >= 65 && keyCode <= 90) return String.fromCharCode(keyCode + 32);
  if (keyCode >= 48 && keyCode <= 57) return String.fromCharCode(keyCode);
  return (
    {
      186: ";",
      187: "=",
      188: ",",
      189: "-",
      190: ".",
      191: "/",
      192: "`",
      219: "[",
      220: "\\",
      221: "]",
      222: "'",
    } as Record<number, string>
  )[keyCode];
}

/**
 * Tastendruck → Kürzel mit `Mod` (⌘ am Mac, Strg unter Windows); `null` bei Modifikator allein oder
 * ungültig. Gespeichert wird, was CodeMirror beim selben Druck sucht: ohne Shift das erzeugte Zeichen
 * (`key`, also auf deutscher Tastatur „z“, „ö“, „-“), mit Shift die Grundtaste nach `keyCode` und
 * Shift dazu (wie CodeMirrors Rückfall über `base[keyCode]`), benannte Tasten mit ihrem Namen.
 */
export function kuerzelAusTaste(
  e: {
    key: string;
    code: string;
    keyCode: number;
    altKey: boolean;
    ctrlKey: boolean;
    metaKey: boolean;
    shiftKey: boolean;
  },
  p: Plattform,
): string | null {
  if (["Shift", "Control", "Alt", "Meta", "AltGraph", "CapsLock", "OS", "Dead", "Unidentified"].includes(e.key))
    return null;
  let taste: string;
  if (e.key === " " || e.code === "Space") taste = "Space";
  else if ([...e.key].length === 1) taste = e.shiftKey ? (grundtaste(e.keyCode) ?? e.key) : e.key.toLowerCase();
  else taste = e.key;
  const mods: string[] = [];
  if (p === "mac" && e.ctrlKey) mods.push("Ctrl");
  if (p === "mac" ? e.metaKey : e.ctrlKey) mods.push("Mod");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (p === "win" && e.metaKey) mods.push("Meta");
  const k = [...mods, taste].join("-");
  return normalisieren(k, p) === null ? null : k;
}

/** Alle belegbaren Aktionen: Helfer, Bausteine des Katalogs, Export-Profile. */
export function aktionen(katalog: EintragJs[], profile: ProfilJs[]): Aktion[] {
  const helfer: Aktion[] = HELFER.map((h) => ({ ...h, herkunft: "tftn", vorschlag: null }));
  // „Paket holen“ ist kein Baustein und bekommt kein Kürzel.
  const bausteine: Aktion[] = katalog.filter((e) => e.herkunft !== "holen").map((e) => ({
    id: e.id,
    titel: e.titel,
    gruppe: e.gruppe ?? t("Bausteine"),
    herkunft: e.herkunft === "typst" ? "tftn" : (e.herkunft as Aktion["herkunft"]),
    vorschlag: e.herkunft === "typst" ? null : e.kuerzel,
  }));
  const export_: Aktion[] = profile.map((p) => ({
    id: `export/${p.id}`,
    titel: t("Export: {id}", { id: p.id }),
    gruppe: t("Export"),
    herkunft: "tftn",
    vorschlag: null,
  }));
  return [...helfer, ...bausteine, ...export_];
}

export interface Zeile {
  aktion: Aktion;
  /** in der Schreibweise mit `Mod`, für die Anzeige */
  kuerzel: string[];
  quelle: "eigene" | "vorschlag" | "standard" | "keine";
  ueberdecktVon: string | null;
}

export interface Belegung {
  zeilen: Zeile[];
  /** normalisiertes Kürzel → Aktion */
  taste: Map<string, string>;
  meldungen: string[];
}

const STUFE: Record<Zeile["quelle"], number> = { eigene: 0, vorschlag: 1, standard: 2, keine: 3 };

/** Welches Kürzel für welche Aktion gilt; bei gleichem Kürzel gewinnt die höhere Stufe, dann die frühere Aktion. */
export function belegen(aktionen: Aktion[], eigene: Record<string, string>, p: Plattform): Belegung {
  const meldungen: string[] = [];
  const zeilen: Zeile[] = aktionen.map((aktion) => {
    const zeile = (kuerzel: string[], quelle: Zeile["quelle"]): Zeile => ({ aktion, kuerzel, quelle, ueberdecktVon: null });
    if (Object.hasOwn(eigene, aktion.id)) {
      const k = eigene[aktion.id];
      if (k === "") return zeile([], "keine");
      if (normalisieren(k, p) === null) {
        meldungen.push(t("{id}: „{kuerzel}“ ist kein gültiges Kürzel – ignoriert.", { id: aktion.id, kuerzel: k }));
        return zeile([], "keine");
      }
      return zeile([k], "eigene");
    }
    if (aktion.vorschlag && normalisieren(aktion.vorschlag, p) !== null) return zeile([aktion.vorschlag], "vorschlag");
    const s = STANDARD[aktion.id]?.[p] ?? [];
    return s.length ? zeile([...s], "standard") : zeile([], "keine");
  });
  const taste = new Map<string, string>();
  const reihe = zeilen.map((z, i) => ({ z, i })).sort((a, b) => STUFE[a.z.quelle] - STUFE[b.z.quelle] || a.i - b.i);
  for (const { z } of reihe) {
    for (const k of z.kuerzel) {
      const n = normalisieren(k, p)!;
      const halter = taste.get(n);
      if (halter === undefined) taste.set(n, z.aktion.id);
      else if (halter !== z.aktion.id) z.ueberdecktVon ??= halter;
    }
  }
  return { zeilen, taste, meldungen };
}

/** Normalisiertes Kürzel → Editor-Bindung, so wie CodeMirror sie auf Plattform `p` auswertet. */
export function editorTasten(p: Plattform): Map<string, KeyBinding> {
  const aus = new Map<string, KeyBinding>();
  for (const km of editorKeymaps()) {
    for (const b of km) {
      const k = (p === "mac" ? b.mac : b.win) ?? b.key;
      if (!k) continue;
      const namen = [k];
      if (b.shift) namen.push(`Shift-${k}`);
      for (const n of namen) {
        const norm = normalisieren(n, p);
        if (norm && !aus.has(norm)) aus.set(norm, b);
      }
    }
  }
  return aus;
}

export interface Konflikt {
  aktion: string;
  kuerzel: string;
  plattform: Plattform;
  art: "doppelt" | "editor" | "system";
  mit: string;
}

/** Konflikte der geltenden Belegung auf Mac und Windows. Bedingte Systemkürzel zählen nicht. */
export function konflikte(aktionen: Aktion[], eigene: Record<string, string>): Konflikt[] {
  const titel = new Map(aktionen.map((a) => [a.id, a.titel]));
  const aus: Konflikt[] = [];
  for (const p of ["mac", "win"] as const) {
    const b = belegen(aktionen, eigene, p);
    const editor = editorTasten(p);
    const reserviert = new Map<string, (typeof RESERVIERT)[number]>();
    for (const r of RESERVIERT) {
      if (r.plattform !== "beide" && r.plattform !== p) continue;
      const n = normalisieren(r.kuerzel, p);
      if (n) reserviert.set(n, r);
    }
    for (const z of b.zeilen) {
      for (const k of z.kuerzel) {
        const n = normalisieren(k, p)!;
        const neu = (art: Konflikt["art"], mit: string) => aus.push({ aktion: z.aktion.id, kuerzel: k, plattform: p, art, mit });
        const halter = b.taste.get(n);
        if (halter !== z.aktion.id) neu("doppelt", titel.get(halter!) ?? halter!);
        const e = editor.get(n);
        if (e) neu("editor", t("Editor: {befehl}", { befehl: befehlsname(e) }));
        const r = reserviert.get(n);
        if (r) {
          if (!r.bedingt && r.entspricht !== z.aktion.id) neu("system", r.was);
        } else {
          const muster = musterKonflikt(n, p);
          if (muster) neu("system", muster);
        }
      }
    }
  }
  return aus;
}

/** Ein CodeMirror-Eintrag je geltendem Kürzel; das Kürzel gehört der Aktion, die es in `taste` hält. */
export function keymapAus(b: Belegung, ausfuehren: (id: string) => boolean): KeyBinding[] {
  return [...b.taste].map(([key, id]) => ({ key, preventDefault: true, run: () => ausfuehren(id) }));
}

/** Was das Kürzel der Seitenleiste tut: öffnen, dann in die Leiste springen, dann schließen (Spec 11). */
export function leistenSchritt(offen: boolean, fokusInLeiste: boolean): "oeffnen" | "fokus" | "schliessen" {
  if (!offen) return "oeffnen";
  return fokusInLeiste ? "schliessen" : "fokus";
}

/** Die Aktion, zu der ein Tastendruck außerhalb des Editors gehört; sonst `null`. */
export function aktionDerTaste(
  b: Belegung,
  e: Parameters<typeof kuerzelAusTaste>[0],
  p: Plattform,
): string | null {
  const k = kuerzelAusTaste(e, p);
  const n = k === null ? null : normalisieren(k, p);
  return n === null ? null : (b.taste.get(n) ?? null);
}

/** Hinweis, wenn `tasten.toml` fehlerhaft wird oder wieder in Ordnung ist; sonst `null`. */
/**
 * Ersatz, wenn die Einstellungen beim Start nicht zu lesen sind: Das Fenster startet trotzdem, mit
 * den Standardkürzeln und vorsichtshalber „ganz offline“; `fehler` erklärt, warum.
 */
export function einstellungenNotfall(fehler: string): EinstellungenJs {
  return {
    ordner: "",
    synchronisiert: false,
    hinweis: null,
    ganz_offline: true,
    kuerzel: {},
    warnung: null,
    fehler: t("Einstellungen nicht gelesen ({fehler}); es gelten die Standardkürzel, das Netz bleibt aus.", { fehler }),
  };
}

export function fehlerHinweis(vorher: string | null, jetzt: string | null): string | null {
  if (jetzt && jetzt !== vorher) return t("{fehler} – Umbelegen ist gesperrt, Details in den Einstellungen.", { fehler: jetzt });
  if (!jetzt && vorher) return t("tasten.toml ist wieder in Ordnung; die eigenen Kürzel gelten.");
  return null;
}
