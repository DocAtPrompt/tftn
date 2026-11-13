// Bausteine einfügen (PLAN-M2 AP2, Entwurf 4 „A“): Menü nach „/“ am Zeilenanfang, Liste für die
// Palette und Vervollständigung mit Strg+Leertaste. Reine Logik und Quellen für @codemirror/autocomplete;
// alles mit Typst-Wissen (Modus, Schnipsel, Vorschläge) kommt aus Rust.

import { closeCompletion } from "@codemirror/autocomplete";
import type { Completion, CompletionContext, CompletionResult, CompletionSection, CompletionSource } from "@codemirror/autocomplete";
import { t } from "./sprache";

export type Modus = "markup" | "code" | "mathe";
export type Herkunft = "typst" | "dokumenttyp" | "paket" | "holen";

export interface EintragJs {
  id: string;
  titel: string;
  gruppe: string | null;
  herkunft: Herkunft;
  modus: Modus[];
  kuerzel: string | null;
}

export interface FeldJs {
  name: string;
  von: number;
  bis: number;
}

/** Eingesetztes Schnipsel; Positionen in UTF-16 relativ zum Anfang von `text`. */
export interface EingesetztJs {
  text: string;
  felder: FeldJs[];
  cursor: number | null;
}

/** Antwort von `baustein_text`: dazu die Importzeile (Position im Dokument). */
export interface BausteinTextJs extends EingesetztJs {
  import: { pos: number; text: string } | null;
}

export interface VorschlagJs extends EingesetztJs {
  titel: string;
  detail: string | null;
  art: string;
}

export interface VervollstaendigungJs {
  von: number;
  vorschlaege: VorschlagJs[];
}

const MAC = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "");

/** Index des „/“, wenn davor nur Leerraum und danach nur Buchstaben, Ziffern oder `-` stehen. */
export function slashAnfang(zeileVorCursor: string): number | null {
  const m = /^([ \t]*)\/[\p{L}\p{N}-]*$/u.exec(zeileVorCursor);
  return m ? m[1].length : null;
}

const REIHENFOLGE: Record<Herkunft, number> = { typst: 0, dokumenttyp: 1, paket: 2, holen: 3 };

/** Einträge für `modus`, die alle Wörter der Anfrage enthalten; Typst, dann Dokumenttyp, dann Pakete. */
export function menue(eintraege: EintragJs[], modus: Modus, anfrage: string): EintragJs[] {
  const woerter = anfrage.toLowerCase().split(/\s+/).filter(Boolean);
  return eintraege
    .map((e, i) => ({ e, i }))
    .filter(({ e }) => e.modus.includes(modus))
    .filter(({ e }) => {
      const text = `${e.titel} ${e.gruppe ?? ""} ${e.id}`.toLowerCase();
      return woerter.every((w) => text.includes(w));
    })
    .sort((a, b) => REIHENFOLGE[a.e.herkunft] - REIHENFOLGE[b.e.herkunft] || a.i - b.i)
    .map(({ e }) => e);
}

const ABSCHNITTE: Record<Herkunft, CompletionSection & { name: string; rank: number }> = {
  typst: { name: "Typst", rank: 0 },
  dokumenttyp: { name: t("Dokumenttyp"), rank: 1 },
  paket: { name: t("Pakete"), rank: 2 },
  holen: { name: t("Pakete holen"), rank: 3 },
};

export function abschnitt(herkunft: Herkunft): { name: string; rank: number } {
  return ABSCHNITTE[herkunft];
}

/** `Mod-Shift-t` → `⇧⌘T` (Mac) bzw. `Strg+Umschalt+T`. */
export function kuerzelText(k: string, mac: boolean = MAC): string {
  const teile = k.split("-");
  let taste = teile.pop() ?? "";
  if (taste === "" && k.endsWith("-")) taste = "-";
  if (taste === "Space") taste = mac ? "␣" : t("Leertaste");
  else if (taste === "Enter") taste = mac ? "↩" : t("Eingabe");
  else if (taste.length === 1) taste = taste.toUpperCase();
  const hat = (...namen: string[]) => teile.some((t) => namen.includes(t));
  const strg = hat("Ctrl") || (!mac && hat("Mod"));
  const befehl = hat("Cmd", "Meta") || (mac && hat("Mod"));
  if (mac) {
    return [strg && "⌃", hat("Alt") && "⌥", hat("Shift") && "⇧", befehl && "⌘", taste].filter(Boolean).join("");
  }
  return [strg && t("Strg"), hat("Alt") && "Alt", hat("Shift") && t("Umschalt"), befehl && "Win", taste]
    .filter(Boolean)
    .join("+");
}

/** Menü nach „/“ am Zeilenanfang (nur Markup). `einsetzen` ersetzt `von..bis` einschließlich „/“. */
export function slashQuelle(d: {
  modus(pos: number): Promise<Modus | null>;
  eintraege(): EintragJs[];
  einsetzen(id: string, von: number, bis: number): void;
}): CompletionSource {
  /** „/“ und Anfrage vor dem Cursor, falls das Menü hier gilt. */
  const stelle = (ctx: CompletionContext) => {
    const zeile = ctx.state.doc.lineAt(ctx.pos);
    const vor = ctx.state.sliceDoc(zeile.from, ctx.pos);
    const i = slashAnfang(vor);
    return i === null ? null : { slash: zeile.from + i, anfrage: vor.slice(i + 1) };
  };
  const ergebnis = (slash: number, anfrage: string): CompletionResult | null => {
    const options: Completion[] = menue(d.eintraege(), "markup", anfrage).map((e) => ({
      label: e.titel,
      detail: e.kuerzel ? kuerzelText(e.kuerzel) : undefined,
      section: ABSCHNITTE[e.herkunft],
      // Liste sofort schließen: Das Einsetzen läuft asynchron, ein zweites Enter träfe sonst die
      // noch offene Liste (Abschlussprüfung AP2).
      apply: (view, _c, from, to) => {
        closeCompletion(view);
        d.einsetzen(e.id, from - 1, to);
      },
    }));
    // filter: false – gefiltert wird mit `menue`, damit Menü und Palette gleich suchen. `update`
    // filtert beim Weitertippen sofort neu (der Modus am „/“ ändert sich dadurch nicht); ohne würde
    // CodeMirror neu fragen, und ein schnelles Enter ginge in dieser Zeit als Zeilenumbruch durch.
    return options.length
      ? {
          from: slash + 1,
          options,
          filter: false,
          update: (_alt, _von, _bis, ctx) => {
            const s = stelle(ctx);
            return s && s.slash === slash ? ergebnis(slash, s.anfrage) : null;
          },
        }
      : null;
  };
  return (ctx: CompletionContext) => {
    const s = stelle(ctx);
    if (s === null) return null;
    return d.modus(s.slash).then((modus) => (modus === "markup" ? ergebnis(s.slash, s.anfrage) : null));
  };
}

/**
 * Strg+Leertaste: Vorschläge von typst_ide, nie beim bloßen Tippen. `gerufen` meldet (und verbraucht)
 * den Tastendruck: CodeMirror fragt nach einem Zeichen außerhalb von `validFor` mit `explicit` neu,
 * die Liste soll dann aber nicht von selbst wiederkommen.
 */
export function typstQuelle(d: {
  gerufen?(): boolean;
  vorschlaege(pos: number): Promise<VervollstaendigungJs | null>;
  einsetzen(von: number, bis: number, e: EingesetztJs): void;
}): CompletionSource {
  return (ctx: CompletionContext) => {
    if (!ctx.explicit || !(d.gerufen?.() ?? true)) return null;
    return d.vorschlaege(ctx.pos).then((a): CompletionResult | null =>
      a && a.vorschlaege.length
        ? {
            from: a.von,
            options: a.vorschlaege.map((v) => ({
              label: v.titel,
              detail: v.detail ?? undefined,
              type: v.art,
              apply: (view, _c, from, to) => {
                closeCompletion(view);
                d.einsetzen(from, to, v);
              },
            })),
            validFor: /^[\p{L}\p{N}_-]*$/u,
          }
        : null,
    );
  };
}
