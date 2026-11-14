// TFTN-Editor (PLAN-M1 AP3, AP4): CodeMirror 6 als einziger Schreiber, Rust führt den Spiegel
// und kompiliert die Vorschau.

import { meldungMit } from "./meldung";
import { type ChangeSet, Compartment, EditorState, Prec, StateEffect, StateField, type Transaction } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  drawSelection,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { history, isolateHistory } from "@codemirror/commands";
import { highlightSelectionMatches } from "@codemirror/search";
import { type Diagnostic, lintGutter, setDiagnostics } from "@codemirror/lint";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type Aenderung, PRUEFSUMME_ALLE, aenderungenAus, pruefsumme } from "./protokoll";
import { type Stelle, Vorschau, type VorschauEreignis } from "./vorschau";
import { ZitatPalette, type ZitatSuche } from "./zitat";
import type { DoiJs } from "./doi";
import { type ExportJs, ExportMenue, type ProfilJs, type WordJs } from "./export";
import { autocompletion, startCompletion } from "@codemirror/autocomplete";
import {
  type BausteinTextJs,
  type EintragJs,
  type Modus,
  type VervollstaendigungJs,
  kuerzelText,
  menue,
  slashQuelle,
  typstQuelle, type EingesetztJs,
} from "./bausteine";
import { einsetzenSpec, felder } from "./felder";
import { BausteinPalette } from "./bausteinpalette";
import { type Belegung, PLATTFORM, aktionDerTaste, aktionen, belegen, einstellungenNotfall, leistenSchritt, fehlerHinweis, keymapAus, konflikte } from "./tasten";
import { editorKeymaps } from "./reserviert";
import { EinstellDialog, einstellHinweise, type EinstellungenJs, type SprachenJs, type UpdatesJs } from "./einstellungen";
import { t, texteUebersetzen } from "./sprache";
import { auszeichnungEntfernen } from "./auszeichnung";
import { type Fach, type HierJs, type MarkupAktion, Seitenleiste } from "./seitenleiste";
import { DokumentFach } from "./dokumentfach";
import { FormelFach, type FormelBildJs, type FormelStandJs, type PaletteJs } from "./formelfach";
import { PaketFach, type PaketeStandJs } from "./paketfach";
import { type Treffer, verbindungText } from "./pakete";
import type { FormularJs, ProjektJs, VergleichJs } from "./formular";
import { NeuDialog, PfadDialog } from "./neudialog";
import { einDialogOffen, escAktivieren } from "./dialoge";
import * as schreibansicht from "./schreibansicht";
import { type FassungJs, FassungenDialog } from "./fassungen";
import { type GliederungJs, GliederungLeiste } from "./gliederung";
import { type Zahlen, zahlText } from "./zaehler";
import { type RechtschreibungJs, falschSetzen, ohneIgnorierte, rechtschreibMenue } from "./rechtschreibung";
import { abbildungText, blockEinfuegen, eingefuegterName, istBild } from "./bildeinfuegen";
import { SeiteFach, type SeitenFeld, type SeitenJs } from "./seitefach";
import { type TypenJs, oeffnenArt, wechselNachSpeichern, entwurfHatText } from "./neuprojekt";
import type { Wert } from "./werte";
import { HALTEN_MS, type MarkenEreignis, MarkenSchalter } from "./marken";
import { type MarkeJs, Popover, markenErweiterung, setzeMarken } from "./markenansicht";
import { editor } from "./editoren";
import { type KlickAntwort, VorschauMenue, menueEintraege } from "./vorschaumenue";

type Bereich = [number, number, string];

interface Geoeffnet {
  ordner: string;
  datei: string;
  text: string;
  rev: number;
  entwurf: boolean;
  vorschlag: string | null;
}

interface Antwort {
  rev: number;
  hervorhebung: Bereich[];
  pruefsumme_ok: boolean | null;
  ungespeichert: boolean;
}

// --- Hervorhebung aus typst-syntax als Dekorationen ---------------------------------------

const setzeHervorhebung = StateEffect.define<DecorationSet>();

const hervorhebung = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deko, tr) {
    deko = deko.map(tr.changes);
    for (const e of tr.effects) if (e.is(setzeHervorhebung)) deko = e.value;
    return deko;
  },
  provide: (feld) => EditorView.decorations.from(feld),
});

const marken = new Map<string, Decoration>();
function marke(klasse: string): Decoration {
  let m = marken.get(klasse);
  if (!m) marken.set(klasse, (m = Decoration.mark({ class: klasse })));
  return m;
}

function dekorationen(bereiche: Bereich[], laenge: number): DecorationSet {
  const liste = bereiche
    .filter(([von, bis]) => von < bis && bis <= laenge)
    .map(([von, bis, klasse]) => marke(klasse).range(von, bis));
  return Decoration.set(liste, true);
}

// --- Synchronisation ---------------------------------------------------------------------

/** Revision, die der Spiegel nach allen bisher abgeschickten Transaktionen haben wird. */
let rev = 0;
/** Nach jeder Neusynchronisation erhöht; ältere, noch wartende Nachrichten werden verworfen. */
let epoche = 0;
/** Nachrichten an Rust laufen streng nacheinander. */
let kette: Promise<unknown> = Promise.resolve();
const zahlen = { abgleiche: 0, abweichungen: 0, neusyncs: 0, fehler: 0 };
let ungespeichert = false;
/** Das Dokument hat noch keinen Namen und liegt im Entwurfsordner. */
let entwurf = false;
let namensVorschlag = "";
/** Jedes Dokument wird eine Sekunde nach der letzten Änderung von selbst gesichert. */
let sichernZeitgeber: number | undefined;
let view: EditorView;
let vorschau: Vorschau;

// --- Positionen älterer Revisionen auf den aktuellen Text abbilden (Klicksprung, KONZEPT 4.3) ----
const verlauf: { rev: number; changes: ChangeSet }[] = [];
/** Ältere Revisionen lassen sich nicht mehr abbilden (Neusynchronisation oder Verlauf gekürzt). */
let verlaufAb = 0;

function abbilden(pos: number, vonRev: number): number {
  if (vonRev >= verlaufAb) {
    for (const e of verlauf) if (e.rev > vonRev) pos = e.changes.mapPos(pos, 1);
  }
  return Math.min(pos, view.state.doc.length);
}

// --- Latenz Taste → Vorschau gezeichnet (PLAN-M1 4.4) ---------------------------------------
const tastenZeit = new Map<number, number>();
const latenzen: number[] = [];

function gezeichnet(rev: number) {
  const zeit = tastenZeit.get(rev);
  if (zeit !== undefined) {
    latenzen.push(performance.now() - zeit);
    if (latenzen.length > 200) latenzen.shift();
  }
  for (const r of tastenZeit.keys()) if (r <= rev) tastenZeit.delete(r);
  const s = [...latenzen].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.round((s.length - 1) * p))];
  document.getElementById("latenz")!.textContent = s.length
    ? t("Taste → Vorschau p50 {p50} ms · p95 {p95} ms (n={n})", { p50: q(0.5).toFixed(0), p95: q(0.95).toFixed(0), n: s.length })
    : "";
}

function sichtbar(): [number, number] {
  return [view.viewport.from, view.viewport.to];
}

function anwenden(a: Antwort) {
  ungespeichert = a.ungespeichert;
  if (ungespeichert) sichernSpaeter();
  if (a.rev === rev) {
    view.dispatch({ effects: setzeHervorhebung.of(dekorationen(a.hervorhebung, view.state.doc.length)) });
  }
  status();
}

function senden(tr: Transaction) {
  const aenderungen: Aenderung[] = aenderungenAus(tr.changes);
  const basis = rev;
  rev += 1;
  tastenZeit.set(rev, performance.now());
  verlauf.push({ rev, changes: tr.changes });
  if (verlauf.length > 2000) verlaufAb = verlauf.shift()!.rev;
  const meineEpoche = epoche;
  const summe = rev % PRUEFSUMME_ALLE === 0 ? pruefsumme(tr.newDoc.toString()) : null;
  kette = kette.then(async () => {
    if (meineEpoche !== epoche) return;
    try {
      const a = await invoke<Antwort>("aendern", {
        anfrage: { basis, aenderungen, sichtbar: sichtbar(), pruefsumme: summe },
      });
      if (a.pruefsumme_ok === true) zahlen.abgleiche += 1;
      if (a.pruefsumme_ok === false) {
        zahlen.abweichungen += 1;
        neusync();
      }
      anwenden(a);
    } catch (fehler) {
      zahlen.fehler += 1;
      console.error("Sync-Fehler", fehler);
      neusync();
    }
  });
}

/** Schickt den ganzen Text; der Stand wird jetzt festgehalten, nicht erst beim Versand. */
function neusync() {
  epoche += 1;
  rev += 1;
  verlauf.length = 0;
  verlaufAb = rev;
  const text = view.state.doc.toString();
  const ziel = rev;
  zahlen.neusyncs += 1;
  kette = kette.then(async () => {
    const a = await invoke<Antwort>("neusync", { text, rev: ziel, sichtbar: sichtbar() });
    anwenden(a);
  });
}

const synchron = EditorView.updateListener.of((u: ViewUpdate) => {
  for (const tr of u.transactions) if (tr.docChanged) senden(tr);
  if (u.selectionSet || u.docChanged) stelleZeigenSpaeter();
  if (u.docChanged) markenEreignis({ art: "dokument-geaendert" });
  if (u.docChanged) gliederungSpaeter();
  if (u.docChanged) formelFach?.aenderung((p) => u.changes.mapPos(p, 1));
  else if (u.viewportChanged && markenSichtbar) markenLaden();
});

// --- Klicksprung (PLAN-M1 AP5) ------------------------------------------------------------

let stelleZeitgeber: number | undefined;
function stelleZeigenSpaeter() {
  clearTimeout(stelleZeitgeber);
  stelleZeitgeber = window.setTimeout(() => {
    const pos = view.state.selection.main.head;
    seitenleiste?.cursor(pos);
    gliederung?.cursor(pos);
    auswahlZaehlen();
    // Hinter den wartenden Änderungen einreihen, damit der Spiegel dieselbe Revision hat.
    kette = kette.then(async () => {
      const r = rev;
      const stellen = await invoke<Stelle[]>("stelle_zeigen", { rev: r, pos });
      if (r === rev) vorschau.folgen(stellen);
    });
  }, 150);
}

// --- Gliederung ------------------------------------------------------------------------------

let gliederung: GliederungLeiste | undefined;
let gliederungZeitgeber: number | undefined;
/** Merker in localStorage: Gliederung ausgeblendet. */
const GLIEDERUNG_AUS = "tftn-gliederung-aus";

function gliederungOrt(): HTMLElement {
  return document.getElementById("gliederung")!;
}

function gliederungSpaeter(warten = 300) {
  clearTimeout(gliederungZeitgeber);
  gliederungZeitgeber = window.setTimeout(() => {
    if (!view) return;
    kette = kette.then(async () => {
      try {
        const g = await invoke<GliederungJs>("gliederung_lesen");
        if (g.rev !== rev) return;
        zahlen_.gesamt = g.gesamt;
        zaehlerZeigen();
        if (!gliederungOrt().hidden) gliederung?.zeigen(g.eintraege, view.state.selection.main.head);
      } catch (fehler) {
        console.error("Gliederung", fehler);
      }
    });
  }, warten);
}

// --- Rechtschreibung nach Dokumentsprache -------------------------------------------------------

let rechtschreibZeitgeber: number | undefined;
/** In dieser Sitzung ignorierte Wörter. */
const ignoriert = new Set<string>();

function rechtschreibungSpaeter(warten = 400) {
  clearTimeout(rechtschreibZeitgeber);
  rechtschreibZeitgeber = window.setTimeout(() => {
    if (!view) return;
    kette = kette.then(async () => {
      try {
        const r = await invoke<RechtschreibungJs | null>("rechtschreibung_pruefen", { rev });
        if (!r || r.rev !== rev) return;
        view.dispatch({ effects: falschSetzen.of(ohneIgnorierte(view.state, r.falsch, ignoriert)) });
        document.getElementById("sprache")!.textContent = r.sprache;
      } catch (fehler) {
        console.error("Rechtschreibung", fehler);
      }
    });
  }, warten);
}

const rechtschreibung = rechtschreibMenue({
  vorschlaege: (wort) => invoke<string[]>("rechtschreibung_vorschlaege", { wort }),
  aufnehmen: async (wort) => {
    try {
      await invoke("wort_aufnehmen", { wort });
    } catch (fehler) {
      hinweis(t("Nicht aufgenommen: {fehler}", { fehler: String(fehler) }));
    }
    rechtschreibungSpaeter(0);
  },
  ignorieren: (wort) => {
    ignoriert.add(wort);
    rechtschreibungSpaeter(0);
  },
});

// --- Wörter und Zeichen ----------------------------------------------------------------------

const zahlen_: { gesamt: Zahlen | null; auswahl: Zahlen | null } = { gesamt: null, auswahl: null };

function zaehlerZeigen() {
  if (zahlen_.gesamt) document.getElementById("zaehler")!.textContent = zahlText(zahlen_.gesamt, zahlen_.auswahl);
}

/** Nach einer Änderung der Auswahl: Zahlen der Auswahl holen (leer: wieder das ganze Dokument). */
function auswahlZaehlen() {
  const { from, to } = view.state.selection.main;
  if (from === to) {
    if (zahlen_.auswahl) {
      zahlen_.auswahl = null;
      zaehlerZeigen();
    }
    return;
  }
  kette = kette.then(async () => {
    try {
      const r = rev;
      const z = await invoke<Zahlen | null>("zaehlen_bereich", { rev: r, von: from, bis: to });
      if (z && r === rev) {
        zahlen_.auswahl = z;
        zaehlerZeigen();
      }
    } catch (fehler) {
      console.error("Zählen", fehler);
    }
  });
}

function gliederungSchalten(an: boolean) {
  gliederungOrt().hidden = !an;
  try {
    if (an) localStorage.removeItem(GLIEDERUNG_AUS);
    else localStorage.setItem(GLIEDERUNG_AUS, "1");
  } catch {
    // ohne localStorage gilt die Wahl nur bis zum Neuladen
  }
  if (an) gliederungSpaeter(0);
  else view.focus();
}

function gliederungAnfang(): boolean {
  try {
    return localStorage.getItem(GLIEDERUNG_AUS) === null;
  } catch {
    return true;
  }
}

function zuStelle(pos: number) {
  view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "start", yMargin: 24 }) });
}

/** Abschnitt `nr` vor `vor` schieben (ein Undo-Schritt); der Cursor geht mit der Überschrift mit. */
async function abschnittVerschieben(nr: number, vor: number | null) {
  await kette;
  const g = await invoke<GliederungJs>("gliederung_lesen");
  const e = g.eintraege[nr];
  if (g.rev !== rev || !e) return;
  const zeile = view.state.doc.lineAt(e.von);
  const kopf = view.state.sliceDoc(zeile.from, zeile.to);
  try {
    const p = await invoke<PatchJs>("abschnitt_verschieben", { rev: g.rev, nr, vor });
    if (!patchAnwenden(p, "input.tftn-gliederung")) return;
    const runter = vor === null || vor > nr;
    zuStelle(runter ? p.von + p.text.lastIndexOf(kopf) : p.von);
    gliederungSpaeter(0);
  } catch (fehler) {
    fehlerMelden(fehler, t("Verschieben"));
  }
}

// --- Patches der Helfer (PLAN-M1 AP6, PLAN-M2 AP6) -----------------------------------------
// Die automatische Widget-Leiste aus M1 ist entfallen (Helfer werden gerufen, Entwurf 3).

interface PatchJs {
  rev: number;
  von: number;
  bis: number;
  text: string;
}

/** Benutzerereignis für Widget-Patches: die Historie fasst sie zu einem Undo-Schritt zusammen. */
const WIDGET_EREIGNIS = "input.type.tftn-widget";

// --- Bausteine einfügen (PLAN-M2 AP2) -------------------------------------------------------

let katalog: EintragJs[] = [];
let paketFach: PaketFach;
let formelFach: FormelFach | undefined;
let bausteinPalette: BausteinPalette;
const VERALTET = t("Inzwischen getippt – bitte noch einmal wählen.");
/** Strg+Leertaste wurde gedrückt; die IDE-Quelle verbraucht das (sonst kehrt die Liste beim Weitertippen zurück). */
let ideGerufen = false;
const ideRufen = (v: EditorView) => ((ideGerufen = true), startCompletion(v));

/** Reiht eine Anfrage hinter die wartenden Änderungen; `f` bekommt die Revision, auf die sie sich bezieht. */
function abfragen<T>(f: (r: number) => Promise<T>): Promise<T> {
  const p = kette.then(() => f(rev));
  kette = p.catch(() => undefined);
  return p;
}

function modusAn(pos: number): Promise<Modus | null> {
  return abfragen((r) => invoke<Modus | null>("modus", { rev: r, pos }));
}

/** Holt das Schnipsel aus Rust und ersetzt `von..bis` damit, samt Importzeile, in einem Undo-Schritt. */
/** Bausteine neu laden (Paket geholt oder entfernt, Beschreibung geändert); die Kürzel hängen daran. */
async function katalogNeu() {
  try {
    katalog = await invoke<EintragJs[]>("katalog").then(texteUebersetzen);
    if (einstellungen) einstellungenSetzen(einstellungen);
  } catch (fehler) {
    hinweis(t("Bausteine nicht geladen: {fehler}", { fehler: String(fehler) }));
  }
}

async function bausteinEinsetzen(id: string, von: number, bis: number, auswahl: string) {
  if (id.startsWith("holen/")) {
    // Kein Netz auf diesen Klick: Das Fach zeigt das Paket, geholt wird mit seinem Knopf.
    if (auswahl === "" && von < bis) view.dispatch({ changes: { from: von, to: bis } });
    paketFach.zeigen(id.slice("holen/".length));
    seitenleisteOeffnen("pakete");
    return;
  }
  let r = -1;
  try {
    const e = await abfragen((rr) => {
      r = rr;
      return invoke<BausteinTextJs>("baustein_text", { id, auswahl, rev: rr, pos: von });
    });
    if (r !== rev) {
      hinweis(VERALTET);
    } else {
      view.dispatch(einsetzenSpec(view.state, von, bis, e, e.import));
    }
  } catch (fehler) {
    hinweis(String(fehler) === "veraltet" ? VERALTET : t("Baustein nicht eingefügt: {fehler}", { fehler: String(fehler) }));
  }
  view.focus();
}

/** Kürzel „Baustein einfügen“ bzw. Knopf: Palette für den Modus an der Auswahl. */
async function bausteinPaletteOeffnen() {
  const m = await modusAn(view.state.selection.main.from);
  if (m === null) {
    hinweis(t("Hier lässt sich kein Baustein einfügen (Kommentar oder Raw-Text)."));
    return;
  }
  bausteinPalette.oeffnen(m);
}

// --- Tastenkürzel und Einstellungen (PLAN-M2 AP3) -------------------------------------------

const tastenFach = new Compartment();
let profile: ProfilJs[] = [];
let einstellungen: EinstellungenJs | null = null;
let belegung: Belegung | null = null;
let einstellDialog: EinstellDialog;

/** Rechnet die Belegung neu und tauscht das Keymap aus; wirkt sofort, ohne Neustart. */
function tastenNeu() {
  const a = aktionen(katalog, profile);
  const eigene = einstellungen?.kuerzel ?? {};
  belegung = belegen(a, eigene, PLATTFORM);
  if (view) view.dispatch({ effects: tastenFach.reconfigure(keymap.of(keymapAus(belegung, ausfuehren))) });
  einstellDialog?.setzen({ einstellungen: einstellungen!, aktionen: a, belegung, konflikte: konflikte(a, eigene) });
  const titel = (id: string, text: string) => {
    const k = belegung!.zeilen.find((z) => z.aktion.id === id)?.kuerzel[0];
    return k ? `${text} (${kuerzelText(k)})` : text;
  };
  document.getElementById("baustein-knopf")!.title = titel("helfer/baustein", t("Baustein einfügen; in einer Leerzeile auch „/“"));
  document.getElementById("zitat-knopf")!.title = titel("helfer/zitat", t("Zitat einfügen"));
  document.getElementById("einstellungen-knopf")!.title = titel("helfer/einstellungen", t("Einstellungen und Tastenkürzel"));
  document.getElementById("seitenleiste-knopf")!.title = titel("helfer/seitenleiste", t("Seitenleiste: was an der Schreibmarke steht"));
  document.getElementById("schreibansicht-knopf")!.title = titel("helfer/schreibansicht", t("Nur der Text, ohne Ablenkung"));
}

/** Bausteine mit dem Kürzel, das gerade gilt (für Menü und Palette). */
function katalogAnzeige(): EintragJs[] {
  if (!belegung) return katalog;
  const geltend = new Map(belegung.zeilen.map((z) => [z.aktion.id, z.kuerzel[0] ?? null]));
  return katalog.map((e) => ({ ...e, kuerzel: geltend.get(e.id) ?? null }));
}

function einstellungenSetzen(e: EinstellungenJs) {
  const h = fehlerHinweis(einstellungen?.fehler ?? null, e.fehler) ?? (e.warnung && e.warnung !== einstellungen?.warnung ? e.warnung : null);
  einstellungen = e;
  tastenNeu();
  if (h) hinweis(h);
}

/** Führt die Aktion eines Kürzels aus; `false` lässt die Taste an den Editor weiter. */
function ausfuehren(id: string): boolean {
  switch (id) {
    case "helfer/baustein":
      void bausteinPaletteOeffnen();
      return true;
    case "helfer/zitat":
      palette.oeffnen();
      return true;
    case "helfer/vervollstaendigen":
      return ideRufen(view);
    case "helfer/einstellungen":
      einstellDialog.oeffnen();
      return true;
    case "app/speichern":
      void (entwurf ? entwurfBenennen(true) : speichern());
      return true;
    case "app/neu":
      void neuesDokument();
      return true;
    case "app/oeffnen":
      void projektWechseln();
      return true;
    case "auszeichnung/ebene-plus":
      markupAktion("ebene-plus");
      return true;
    case "auszeichnung/ebene-minus":
      markupAktion("ebene-minus");
      return true;
    case "helfer/seitenleiste":
      seitenleisteKuerzel();
      return true;
    case "helfer/schreibansicht":
      schreibansichtSchalten(!schreibansicht.istAn());
      return true;
    case "helfer/werte":
      markenEreignis({ art: "kuerzel" });
      return true;
    case "helfer/fach-einfuegen":
      seitenleisteOeffnen("einfuegen");
      return true;
    case "helfer/fach-dokument":
      seitenleisteOeffnen("dokument");
      return true;
    case "helfer/fach-pakete":
      seitenleisteOeffnen("pakete");
      return true;
    case "helfer/fach-formel":
      seitenleisteOeffnen("formel");
      return true;
  }
  if (id.startsWith("export/")) {
    void kuerzelExport(id.slice("export/".length));
    return true;
  }
  const e = katalog.find((k) => k.id === id);
  if (!e) return false;
  void bausteinPerKuerzel(e);
  return true;
}

/** Baustein über sein Kürzel: nur, wenn er an der Stelle passt; die Markierung wandert mit. */
async function bausteinPerKuerzel(e: EintragJs) {
  const { from, to } = view.state.selection.main;
  // ⌘B/⌘I auf schon ausgezeichnetem Text: Auszeichnung entfernen (umschalten).
  const art = e.id === "typst/fett" ? "fett" : e.id === "typst/kursiv" ? "kursiv" : null;
  const weg = art && auszeichnungEntfernen(view.state.doc.toString(), from, to, art);
  if (weg) {
    view.dispatch({
      changes: { from: weg.von, to: weg.bis, insert: weg.einsetzen },
      selection: { anchor: weg.auswahlVon, head: weg.auswahlBis },
      userEvent: "input.tftn-auszeichnung",
    });
    return;
  }
  const m = await modusAn(from);
  if (m === null || !e.modus.includes(m)) {
    const wo = m === null ? t("Kommentar oder Raw-Text") : m === "code" ? t("Code") : m === "mathe" ? t("Formel") : t("Text");
    hinweis(t("„{titel}“ passt hier nicht ({wo}).", { titel: e.titel, wo }));
    return;
  }
  await bausteinEinsetzen(e.id, from, to, view.state.sliceDoc(from, to));
}

async function kuerzelExport(profil: string) {
  await speichern();
  try {
    const r = await invoke<ExportJs>("exportieren", { profil, abnehmen: false });
    const w = r.warnungen.length;
    hinweis(
      w === 0
        ? t("Exportiert: {datei}", { datei: r.relativ })
        : w === 1
          ? t("Exportiert: {datei} (1 Warnung)", { datei: r.relativ })
          : t("Exportiert: {datei} ({n} Warnungen)", { datei: r.relativ, n: w }),
    );
  } catch (fehler) {
    hinweis(t("Export {profil}: {fehler}", { profil, fehler: String(fehler) }));
  }
}

const KEINE_STELLE: Record<MarkupAktion, string> = {
  "ebene-plus": t("Hier steht keine Überschrift."),
  "ebene-minus": t("Hier steht keine Überschrift."),
  "liste-umschalten": t("Hier steht keine Liste."),
  "formel-umschalten": t("Hier steht keine Formel."),
};

/** Markup-Helfer (AP4) an der Schreibmarke, als ein Undo-Schritt. */
function markupAktion(aktion: MarkupAktion) {
  const pos = view.state.selection.main.head;
  void abfragen(async (r) => {
    try {
      const p = await invoke<PatchJs | null>("markup_aktion", { rev: r, pos, aktion });
      if (p === null) hinweis(KEINE_STELLE[aktion]);
      else patchAnwenden(p, "input.tftn-markup");
    } catch (fehler) {
      fehlerMelden(fehler, t("Nicht geändert"));
    }
  });
}

// --- Seitenleiste, Werte und Tabellenraster (PLAN-M2 AP6) -----------------------------------

let seitenleiste: Seitenleiste | undefined;
/** Ein Zug im Farbwähler läuft: seine Zwischenwerte bilden mit dem Endwert einen Undo-Schritt. */
let zugLaeuft = false;

/**
 * Wendet einen geprüften Patch an: ein Undo-Schritt (`isolateHistory.of("full")`). Mit `zusammen`
 * (Ziehen im Farbwähler) bilden die Schritte eines Zugs einen (Muster aus M1). Bei veralteter
 * Revision geschieht nichts außer dem Hinweis.
 */
function patchAnwenden(p: PatchJs, ereignis: string, zusammen = false): boolean {
  if (p.rev !== rev) {
    hinweis(VERALTET);
    return false;
  }
  if (view.state.sliceDoc(p.von, p.bis) === p.text) return true;
  view.dispatch({
    changes: { from: p.von, to: p.bis, insert: p.text },
    userEvent: ereignis,
    annotations: zusammen ? (zugLaeuft ? [] : isolateHistory.of("before")) : isolateHistory.of("full"),
  });
  return true;
}

/** Nach einem Patch aus der Leiste: neue Revision und Werte holen, der Fokus bleibt (Abschlussprüfung AP11). */
function leisteNachPatch() {
  seitenleiste?.nachladen(view.state.selection.main.head);
}

function fehlerMelden(fehler: unknown, was: string) {
  if (String(fehler) === "veraltet") {
    hinweis(VERALTET);
    // Die Positionen der Leiste gehören zu einem älteren Text: neu holen.
    seitenleiste?.nachladen();
  } else {
    hinweis(meldungMit(was, String(fehler)));
  }
}

/**
 * Neuer Wert eines Widgets; `ende=false` nur beim Ziehen im Farbwähler. `hrev` ist die Revision, zu
 * der `aufruf` gehört (aus `HierJs`); ist der Text inzwischen anders, lehnt Rust ab („veraltet“).
 */
function wertSetzen(hrev: number, aufruf: number, parameter: string, wert: Wert, ende: boolean) {
  void abfragen(async () => {
    try {
      const p = await invoke<PatchJs>("wert_setzen", { rev: hrev, aufruf, parameter, wert });
      const zusammen = !ende || zugLaeuft;
      patchAnwenden(p, zusammen ? WIDGET_EREIGNIS : "input.tftn-wert", zusammen);
      zugLaeuft = !ende;
      if (ende) leisteNachPatch();
    } catch (fehler) {
      zugLaeuft = false;
      fehlerMelden(fehler, t("Nicht geändert"));
    }
  });
}

/** Aktion im Tabellenraster (JSON wie `TabellenAktion`) an der Tabelle, die bei `aufruf` (Revision `hrev`) beginnt. */
/** Ändert die Tabelle; `false`, wenn die Änderung abgelehnt wurde (die Meldung kommt hier). */
function tabelleAktion(hrev: number, aufruf: number, aktion: object): Promise<boolean> {
  return abfragen(async () => {
    try {
      const p = await invoke<PatchJs>("tabelle_aktion", { rev: hrev, aufruf, aktion });
      patchAnwenden(p, "input.tftn-tabelle");
      return true;
    } catch (fehler) {
      fehlerMelden(fehler, t("Tabelle nicht geändert"));
      return false;
    }
  });
}

function hierAbfragen(pos: number): Promise<HierJs | null> {
  return abfragen((r) => invoke<HierJs | null>("hier", { rev: r, pos }).then(texteUebersetzen));
}

// --- Kontextmenü D in der Vorschau (PLAN-M2 AP6) ----------------------------------------------

let vorschauMenue: VorschauMenue | undefined;
/** Quellstelle des zuletzt geöffneten Vorschau-Menüs und die Revision seiner Werte. */
let menuePos = 0;
let menueRev = -1;

function zurStelle(pos: number) {
  view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "center" }) });
}

/** Rechtsklick in der Vorschau: Menü öffnen; ohne Menü wie ein Linksklick. */
async function kontext(seite: number, x: number, y: number, maus: { x: number; y: number }) {
  const a = await invoke<KlickAntwort>("klick", { seite, x, y });
  let hier: HierJs | null = null;
  if (a.art === "haupt") {
    menuePos = abbilden(a.pos, a.rev);
    hier = await hierAbfragen(menuePos);
    menueRev = hier?.rev ?? -1;
    const pos = menuePos;
    void abfragen(async (r) => {
      const stellen = await invoke<Stelle[]>("stelle_zeigen", { rev: r, pos });
      if (r === rev) vorschau.folgen(stellen);
    });
  }
  const eintraege = menueEintraege(a, hier);
  if (!eintraege.length) return void klick(seite, x, y);
  vorschauMenue?.zeigen(eintraege, maus);
}

// --- Marken B (PLAN-M2 AP6) -----------------------------------------------------------------

const markenSchalter = new MarkenSchalter();
let markenSichtbar = false;
let popover: Popover | undefined;

function markenEreignis(e: MarkenEreignis) {
  const { sichtbar: jetzt } = markenSchalter.ereignis(e);
  if (jetzt === markenSichtbar) return;
  markenSichtbar = jetzt;
  if (jetzt) markenLaden();
  // Auch aus einem laufenden Update heraus (Textänderung): erst danach dispatchen.
  else queueMicrotask(() => !markenSichtbar && view.dispatch({ effects: setzeMarken.of([]) }));
}

function markenLaden() {
  const [von, bis] = sichtbar();
  void abfragen(async (r) => {
    const liste = await invoke<MarkeJs[]>("marken", { rev: r, von, bis }).then(texteUebersetzen);
    if (markenSichtbar && r === rev) view.dispatch({ effects: setzeMarken.of(liste) });
  });
}

async function markeGeklickt(m: MarkeJs, dom: HTMLElement) {
  let h: HierJs | null;
  try {
    h = await hierAbfragen(m.aufruf);
  } catch (fehler) {
    fehlerMelden(fehler, t("Werte nicht geladen"));
    return;
  }
  const feld = h?.ebenen.find((e) => e.von === m.aufruf)?.felder.find((f) => f.parameter === m.parameter);
  if (!feld) {
    hinweis(VERALTET);
    return;
  }
  const hrev = h!.rev;
  popover?.zeigen(dom, `${m.baustein} · ${m.titel}`, editor(feld, (w, ende) => wertSetzen(hrev, m.aufruf, m.parameter, w, ende)));
}

/** Tasten am Editor für den MarkenSchalter (Capture-Phase, nur beobachten). */
function markenTasten() {
  const jetzt = () => performance.now();
  view.dom.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape") return markenEreignis({ art: "escape" });
      markenEreignis({ art: "taste-ab", key: e.key, alt: e.altKey, ctrl: e.ctrlKey, meta: e.metaKey, shift: e.shiftKey, zeit: jetzt() });
      if (e.key === "Alt" && !e.repeat) setTimeout(() => markenEreignis({ art: "zeit", zeit: jetzt() }), HALTEN_MS + 10);
    },
    true,
  );
  view.dom.addEventListener("keyup", (e) => markenEreignis({ art: "taste-auf", key: e.key, zeit: jetzt() }), true);
  view.contentDOM.addEventListener("mousedown", () => markenEreignis({ art: "maus" }));
  // Cmd/Alt+Tab: das Loslassen kommt nicht mehr an.
  window.addEventListener("blur", () => markenEreignis({ art: "taste-auf", key: "Alt", zeit: jetzt() }));
}

function seitenleisteOeffnen(fach: Fach) {
  seitenleiste?.cursor(view.state.selection.main.head);
  seitenleiste?.oeffnen(fach);
}

/** Kürzel: öffnen, dann in die Leiste springen, dann schließen; so bleibt sie per Tastatur bedienbar. */
function seitenleisteKuerzel() {
  const schritt = leistenSchritt(seitenleiste?.offen ?? false, seitenleiste?.fokusInnen ?? false);
  if (schritt === "oeffnen") seitenleisteOeffnen("hier");
  else if (schritt === "fokus") seitenleiste?.fokussieren();
  else seitenleiste?.schliessen();
}

/** Schreibansicht an oder aus; die Schreibmarke bleibt im Blick. Eine offene Seitenleiste geht zu. */
function schreibansichtSchalten(an: boolean) {
  if (an) seitenleiste?.schliessen();
  schreibansicht.setzen(an);
  view.focus();
  view.dispatch({ effects: EditorView.scrollIntoView(view.state.selection.main.head, { y: "center" }) });
}

function seitenleisteUmschalten() {
  if (seitenleiste?.offen) seitenleiste.schliessen();
  else seitenleisteOeffnen("hier");
}

function escVerlaesstJetzt(e: KeyboardEvent): boolean {
  return schreibansicht.escVerlaesst(schreibansicht.istAn(), e.defaultPrevented, einDialogOffen());
}

// --- Zitat einfügen (PLAN-M1 AP7) ----------------------------------------------------------

let palette: ZitatPalette;

/** `anhang` (etwa `#bibliography(…)` nach „Literatur aus DOI“) kommt im selben Undo-Schritt ans Ende. */
function zitatEinfuegen(schluessel: string, seite: string | null, anhang?: string | null) {
  kette = kette.then(async () => {
    const r = rev;
    const pos = view.state.selection.main.head;
    try {
      const text = await invoke<string>("zitat_text", { rev: r, pos, schluessel, seite });
      if (r !== rev) return;
      const ende = view.state.doc.length;
      view.dispatch({
        changes: anhang && ende > pos ? [{ from: pos, insert: text }, { from: ende, insert: anhang }] : { from: pos, insert: text + (anhang ?? "") },
        selection: { anchor: pos + text.length },
        userEvent: "input.tftn-zitat",
        scrollIntoView: true,
      });
    } catch (fehler) {
      hinweis(String(fehler));
    }
    view.focus();
  });
}

/** Zitat am Cursor ändern (PLAN-M2 AP11), als ein Undo-Schritt. */
function zitatAendern(r: number, seite: string | null, schluessel: string | null) {
  const pos = view.state.selection.main.head;
  kette = kette.then(async () => {
    try {
      const p = await invoke<PatchJs>("zitat_aendern", { rev: r, pos, schluessel, seite });
      patchAnwenden(p, "input.tftn-zitat");
      leisteNachPatch();
    } catch (fehler) {
      fehlerMelden(fehler, t("Nicht geändert"));
    }
  });
}

/** Fach „Dokument“ öffnen, am Feld `feld` (Klick auf die Titelei, PLAN-M2 AP7). */
function dokumentOeffnen(feld: string | null) {
  hinweis("");
  seitenleiste?.feldZeigen(feld);
}

async function klick(seite: number, x: number, y: number) {
  const a = await invoke<KlickAntwort>("klick", { seite, x, y });
  switch (a.art) {
    case "haupt": {
      const pos = abbilden(a.pos, a.rev);
      view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: "center" }) });
      view.focus();
      hinweis("");
      break;
    }
    case "feld":
      dokumentOeffnen(a.feld);
      break;
    case "vorlage":
      dokumentOeffnen(null);
      break;
    case "datei":
      hinweis(t("Diese Stelle steht in {datei}; TFTN öffnet derzeit nur die Hauptdatei.", { datei: a.datei }));
      break;
    case "url":
      hinweis(t("Verweis auf {url}", { url: a.url }));
      break;
    case "position":
      vorschau.scrollen(a.seite, a.y);
      break;
  }
}

// --- Bilder ziehen oder einfügen --------------------------------------------------------------

function bilderAus(liste: FileList | undefined | null): File[] {
  return [...(liste ?? [])].filter((f) => istBild(f.name, f.type));
}

/** Legt die Bilder unter bilder/ ab und setzt je eine Abbildung bei `pos` ein (je ein Undo-Schritt). */
async function bilderEinfuegen(dateien: File[], pos: number) {
  for (const f of dateien) {
    try {
      const name = eingefuegterName(f.name, f.type);
      const daten = new Uint8Array(await f.arrayBuffer());
      const pfad = await invoke<string>("bild_ablegen", daten, {
        headers: { "tftn-name": encodeURIComponent(name) },
      });
      const a = abbildungText(pfad);
      const b = blockEinfuegen(view.state.doc.lineAt(Math.min(pos, view.state.doc.length)), a.text);
      view.dispatch({
        changes: { from: b.von, insert: b.text },
        selection: { anchor: b.von + b.versatz + a.cursor },
        userEvent: "input.tftn-bild",
        annotations: isolateHistory.of("full"),
        scrollIntoView: true,
      });
      pos = b.von + b.text.length;
      hinweis(t("{pfad} übernommen. Die Unterschrift kommt in die eckigen Klammern.", { pfad }));
    } catch (fehler) {
      hinweis(t("Bild nicht übernommen: {fehler}", { fehler: String(fehler) }));
    }
  }
  view.focus();
}

const bildAblage = EditorView.domEventHandlers({
  dragover(e) {
    if (!e.dataTransfer?.types.includes("Files")) return false;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    return true;
  },
  drop(e, v) {
    const alle = [...(e.dataTransfer?.files ?? [])];
    if (alle.length === 0) return false;
    e.preventDefault();
    const bilder = bilderAus(e.dataTransfer?.files);
    if (bilder.length < alle.length) {
      hinweis(t("Nur Bilder lassen sich in den Text ziehen (png, jpg, gif, svg, webp, pdf). Andere Dokumente über „Importieren …“."));
    }
    const pos = v.posAtCoords({ x: e.clientX, y: e.clientY }) ?? v.state.selection.main.head;
    if (bilder.length) void bilderEinfuegen(bilder, pos);
    return true;
  },
  paste(e, v) {
    const bilder = bilderAus(e.clipboardData?.files);
    if (bilder.length === 0) return false;
    e.preventDefault();
    void bilderEinfuegen(bilder, v.state.selection.main.head);
    return true;
  },
});

// Eine Datei, die neben den Editor fällt, darf das Fenster nicht wegnavigieren.
for (const art of ["dragover", "drop"] as const) {
  window.addEventListener(art, (e) => {
    if (e.dataTransfer?.types.includes("Files")) e.preventDefault();
  });
}

const sichtbereich = ViewPlugin.fromClass(
  class {
    update(u: ViewUpdate) {
      if (u.viewportChanged && !u.docChanged) {
        const r = rev;
        kette = kette.then(async () => {
          const h = await invoke<Bereich[] | null>("hervorheben", { rev: r, sichtbar: sichtbar() });
          if (h && r === rev) view.dispatch({ effects: setzeHervorhebung.of(dekorationen(h, view.state.doc.length)) });
        });
      }
    }
  },
);

async function speichern(): Promise<boolean> {
  await kette;
  try {
    await invoke("speichern");
    ungespeichert = false;
    hinweis("");
  } catch (fehler) {
    hinweis(String(fehler));
    status();
    return false;
  }
  status();
  return true;
}

// --- Automatisch sichern, Dokument ohne Namen ------------------------------------------------

function sichernSpaeter() {
  clearTimeout(sichernZeitgeber);
  sichernZeitgeber = window.setTimeout(() => {
    kette = kette.then(async () => {
      try {
        await invoke("speichern");
        ungespeichert = false;
        status();
      } catch (fehler) {
        hinweis(String(fehler));
      }
    });
  }, 1000);
}

/** Fragt nach dem Namen und legt das Dokument in Dokumente ab; `false` bei Abbruch oder Fehler. */
async function entwurfBenennen(neuLaden: boolean): Promise<boolean> {
  pfadDialog ??= new PfadDialog();
  const antwort = await pfadDialog.namenFragen(t("Speichern in Dokumente als"), namensVorschlag);
  if (antwort === null) {
    view.focus();
    return false;
  }
  clearTimeout(sichernZeitgeber);
  await kette;
  try {
    await invoke("speichern");
    await invoke("entwurf_benennen", { name: antwort.name, schriften: antwort.schriften });
  } catch (fehler) {
    hinweis(String(fehler));
    return false;
  }
  ungespeichert = false;
  if (neuLaden) location.reload();
  return true;
}

/** Vor einem Wechsel: Ein Dokument ohne Namen, in dem etwas steht, bekommt erst einen. */
function vorWechsel(): Promise<boolean> {
  return entwurf && entwurfHatText(view.state.doc.toString()) ? entwurfBenennen(false) : speichern();
}

async function neuesDokument() {
  if (entwurf && !entwurfHatText(view.state.doc.toString())) {
    view.focus();
    return;
  }
  try {
    await wechselNachSpeichern(vorWechsel, async () => {
      await invoke("neues_dokument");
      location.reload();
    });
  } catch (fehler) {
    hinweis(String(fehler));
  }
}

// --- Neues Projekt, Öffnen, Dokumenttyp aktualisieren (PLAN-M2 AP8) -------------------------

/** Merker in sessionStorage: nach dem Neuladen das Fach „Dokument“ öffnen. */
const NACH_START = "tftn-nach-start";
/** Merker in sessionStorage: Meldung des Imports nach dem Neuladen. */
const NACH_IMPORT = "tftn-nach-import";

/** Eine Datei über Pandoc in ein neues Dokument ohne Namen holen. */
async function importieren() {
  const was = t("Datei zum Importieren (.docx, .odt, .rtf, .md, .html, .epub, .tex)");
  let datei: string | null;
  try {
    if (oeffnenArt(PLATTFORM) === "pfad") {
      pfadDialog ??= new PfadDialog();
      datei = await pfadDialog.fragen(was, "", { platzhalter: t("z. B. C:\\Users\\…\\Artikel.docx"), knopf: t("Importieren") });
    } else {
      datei = await invoke<string | null>("datei_waehlen_dialog", { aufforderung: was });
    }
    if (datei === null) return;
    hinweis(t("Importiere …"));
    const ziel = datei;
    await wechselNachSpeichern(vorWechsel, async () => {
      const h = await invoke<string[]>("importieren", { datei: ziel });
      try {
        sessionStorage.setItem(NACH_IMPORT, h.length ? t("Importiert. {hinweise}", { hinweise: h.join(" · ") }) : t("Importiert. Mit Cmd/Strg+S bekommt es einen Namen."));
      } catch {
        // ohne sessionStorage keine Meldung
      }
      location.reload();
    });
  } catch (fehler) {
    hinweis(String(fehler));
  }
}
let neuDialog: NeuDialog | undefined;

let pfadDialog: PfadDialog | undefined;

/** Speichern, ein anderes Projekt wählen (Systemdialog am Mac, Pfad unter Windows), dann neu laden. */
async function projektWechseln() {
  try {
    let ordner: string | null = null;
    if (oeffnenArt(PLATTFORM) === "pfad") {
      pfadDialog ??= new PfadDialog();
      ordner = await pfadDialog.fragen(t("Projekt öffnen (Ordner mit tftn.toml)"));
      if (ordner === null) return;
    }
    const ok = await wechselNachSpeichern(vorWechsel, async () => {
      await invoke("projekt_wechseln", { ordner });
      location.reload();
    });
    if (!ok) hinweis(t("Nicht gespeichert – Projekt nicht gewechselt."));
  } catch (fehler) {
    hinweis(String(fehler));
  }
}

interface UebernahmeJs {
  patch: PatchJs | null;
  hinweis: string | null;
}

/** Neuere Version des Dokumenttyps übernehmen: Importzeile als ein Undo-Schritt, dann speichern. */
async function typUebernehmen(): Promise<string> {
  if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht übernommen."));
  const r = await invoke<UebernahmeJs>("typ_uebernehmen", { rev });
  let text = r.hinweis ?? "";
  if (r.patch) {
    if (patchAnwenden(r.patch, "input.tftn-typ")) await speichern();
    else text += " " + t("Die Importzeile in der Hauptdatei bitte von Hand auf die neue Version ändern.");
  }
  return text;
}

interface HinzufuegenJs {
  patch: PatchJs | null;
  hinweis: string;
  warnungen: string[];
}

/** „Hinzufügen“ im Fach „Pakete“ (PLAN-M2 AP9): Rust holt und übernimmt; die Importzeile kommt als
 *  Patch zurück und ist ein Undo-Schritt. */
async function paketHinzufuegen(spec: string, importieren: boolean): Promise<string> {
  await kette;
  const r = await invoke<HinzufuegenJs>("paket_hinzufuegen", { spec, import: importieren, rev });
  let text = r.hinweis;
  if (r.patch) text += " " + (patchAnwenden(r.patch, "input.tftn-paket") ? t("Importzeile eingefügt.") : t("Importzeile bitte selbst einfügen."));
  if (r.warnungen.length) text += " " + t("Hinweise: {liste}", { liste: r.warnungen.join("; ") });
  await katalogNeu();
  return text;
}

/** „Entfernen“ im Fach „Pakete“ (PLAN-M2 AP10); benutzt wird die Datei auf der Platte. */
async function paketEntfernen(spec: string): Promise<string> {
  if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht entfernt."));
  const text = await invoke<string>("paket_entfernen", { spec });
  await katalogNeu();
  return text;
}

async function paketVergleichen(spec: string, version: string): Promise<VergleichJs> {
  if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht verglichen."));
  return invoke<VergleichJs>("paket_vergleichen", { spec, version });
}

/** Versionswechsel übernehmen; die Importe der Hauptdatei sind ein Undo-Schritt. */
async function paketWechseln(spec: string, version: string): Promise<string> {
  if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht übernommen."));
  await kette;
  let r: UebernahmeJs;
  try {
    r = await invoke<UebernahmeJs>("paket_wechseln", { spec, version, rev });
  } catch (fehler) {
    throw String(fehler) === "veraltet" ? VERALTET : fehler;
  }
  let text = r.hinweis ?? "";
  if (r.patch) {
    if (patchAnwenden(r.patch, "input.tftn-paket")) await speichern();
    else text += " " + t("Die Importzeile in der Hauptdatei bitte von Hand auf die neue Version ändern.");
  }
  await katalogNeu();
  return text;
}

// --- Oberfläche ---------------------------------------------------------------------------

function status() {
  const s = document.getElementById("sync")!;
  s.textContent = t("Rev {rev} · Prüfsummen gleich {gleich} · abweichend {abweichend} · Neusync {neusync} · Fehler {fehler} · {zustand}", {
    rev,
    gleich: zahlen.abgleiche,
    abweichend: zahlen.abweichungen,
    neusync: zahlen.neusyncs,
    fehler: zahlen.fehler,
    zustand: ungespeichert ? t("ungespeichert") : t("gespeichert"),
  });
  s.classList.toggle("warnung", zahlen.abweichungen + zahlen.fehler > 0);
}

function hinweis(text: string, knopf?: { titel: string; aktion: () => void }) {
  const h = document.getElementById("hinweis")!;
  h.hidden = text === "";
  h.textContent = text;
  if (knopf) {
    const b = document.createElement("button");
    b.textContent = knopf.titel;
    b.onclick = knopf.aktion;
    h.append(" ", b);
  }
}

function textErsetzen(text: string) {
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
}

/** Öffnet das Startprojekt; schlägt das fehl (z. B. Ordnerwahl abgebrochen), auf Knopfdruck neu wählen. */
async function projektOeffnen(): Promise<Geoeffnet> {
  let waehlen = false;
  for (;;) {
    try {
      return await invoke<Geoeffnet>("oeffnen", { waehlen });
    } catch (fehler) {
      await new Promise<void>((weiter) =>
        hinweis(`${fehler}`, {
          titel: t("Projekt wählen …"),
          aktion: () => {
            hinweis("");
            weiter();
          },
        }),
      );
      waehlen = true;
    }
  }
}

async function start() {
  escAktivieren();
  palette = new ZitatPalette(
    (anfrage) => invoke<ZitatSuche>("zitate_suchen", { anfrage }),
    zitatEinfuegen,
    () => view.focus(),
    (eingabe) => invoke<DoiJs>("literatur_aus_doi", { eingabe }),
  );
  document.getElementById("zitat-knopf")!.addEventListener("click", () => palette.oeffnen());
  bausteinPalette = new BausteinPalette(
    katalogAnzeige,
    (id) => {
      const { from, to } = view.state.selection.main;
      void bausteinEinsetzen(id, from, to, view.state.sliceDoc(from, to));
    },
    () => view.focus(),
  );
  document.getElementById("baustein-knopf")!.addEventListener("click", () => void bausteinPaletteOeffnen());
  const dokumentFach = new DokumentFach({
    laden: () => invoke<FormularJs>("formular").then(texteUebersetzen),
    setzen: (stand, pfad, wert) => invoke<FormularJs>("formular_setzen", { stand, pfad, wert }).then(texteUebersetzen),
    eintrag: (stand, pfad, aktion) => invoke<FormularJs>("formular_eintrag", { stand, pfad, aktion }).then(texteUebersetzen),
    rueckgaengig: (stand) => invoke<FormularJs>("formular_rueckgaengig", { stand }).then(texteUebersetzen),
    wiederholen: (stand) => invoke<FormularJs>("formular_wiederholen", { stand }).then(texteUebersetzen),
    projekt: () => invoke<ProjektJs>("projekt_info"),
    profile: () => invoke<ProfilJs[]>("export_profile"),
    exportieren: async (profil, abnehmen) => {
      // Wie im Export-Menü: exportiert wird die Datei auf der Platte, also vorher speichern.
      await speichern();
      return invoke<ExportJs>("exportieren", { profil, abnehmen });
    },
    hinweis: (text) => hinweis(text),
    vergleichen: async () => {
      if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht verglichen."));
      return invoke<VergleichJs>("typ_vergleichen");
    },
    uebernehmen: typUebernehmen,
  });
  neuDialog = new NeuDialog({
    typen: () => invoke<TypenJs>("typen_liste").then(texteUebersetzen),
    vorschaubild: (name) => invoke<string>("typ_vorschaubild", { name }),
    ordnerWaehlen: () => invoke<string | null>("ordner_waehlen_dialog", { aufforderung: t("Ordner für das neue Projekt wählen") }),
    ordnerdialog: PLATTFORM === "mac",
    // Erst speichern, dann anlegen (Rust merkt sich dabei das neue Projekt), dann neu laden: So bleibt
    // bei einem Speicherfehler alles, wie es war (Abschlussprüfung AP8).
    anlegen: async (typ, eltern, name, schriften) => {
      let pfad = "";
      const ok = await wechselNachSpeichern(vorWechsel, async () => {
        pfad = await invoke<string>("projekt_anlegen", { typ, eltern, name, schriften });
      });
      if (!ok) throw new Error(t("Das offene Projekt ließ sich nicht speichern – nichts angelegt."));
      return pfad;
    },
    wechseln: async () => {
      try {
        sessionStorage.setItem(NACH_START, "dokument");
      } catch {
        // ohne sessionStorage öffnet das Fach eben nicht von selbst
      }
      location.reload();
    },
  });
  document.getElementById("neu-knopf")!.addEventListener("click", () => void neuDialog?.oeffnen());
  document.getElementById("oeffnen-knopf")!.addEventListener("click", () => void projektWechseln());
  document.getElementById("import-knopf")!.addEventListener("click", () => void importieren());
  const fassungen = new FassungenDialog({
    liste: async () => {
      await kette;
      return invoke<FassungJs[]>("fassungen_liste");
    },
    text: (name) => invoke<string>("fassung_text", { name }),
    zurueckholen: async (name) => {
      await kette;
      const alt = await invoke<string>("fassung_zurueckholen", { name });
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: alt },
        userEvent: "input.tftn-fassung",
        annotations: isolateHistory.of("full"),
      });
      view.focus();
    },
  });
  document.getElementById("fassungen-knopf")!.addEventListener("click", () => void fassungen.zeigen());
  await listen<string>("dokument-geaendert", (e) => dokumentFach.aussen(e.payload));
  paketFach = new PaketFach({
    stand: () => invoke<PaketeStandJs>("pakete_stand").then(texteUebersetzen),
    indexHolen: () => invoke<{ pakete: number; uebersprungen: number }>("pakete_index_holen"),
    suchen: (anfrage) => invoke<Treffer[]>("pakete_suchen", { anfrage }).then(texteUebersetzen),
    readme: (spec) => invoke<string | null>("paket_readme", { spec }),
    hinzufuegen: paketHinzufuegen,
    entfernen: paketEntfernen,
    vergleichen: paketVergleichen,
    wechseln: paketWechseln,
    hinweis: (text) => hinweis(text),
    verbindung: (an) => {
      document.getElementById("netz")!.textContent = verbindungText(an);
    },
  });
  /** Patch aus dem Fach „Formel“: ein Undo-Schritt; bei „veraltet“ ein Hinweis statt eines Fehlers. */
  const formelPatch = async (f: (r: number) => Promise<PatchJs>): Promise<boolean> => {
    try {
      return patchAnwenden(await abfragen(f), "input.tftn-formel");
    } catch (fehler) {
      fehlerMelden(fehler, t("Nicht geändert"));
      return false;
    }
  };
  formelFach = new FormelFach({
    stand: (pos) => abfragen((r) => invoke<FormelStandJs>("formel_stand", { rev: r, pos })),
    vorschau: (text, block) => invoke<FormelBildJs>("formel_vorschau", { text, block }),
    palette: () => invoke<PaletteJs>("formel_palette"),
    baustein: (id, auswahl) => invoke<EingesetztJs>("formel_baustein", { id, auswahl }),
    setzen: (pos, erwartet, text) => formelPatch((r) => invoke<PatchJs>("formel_setzen", { rev: r, pos, erwartet, text })),
    einfuegen: (pos, text, block) => formelPatch((r) => invoke<PatchJs>("formel_einfuegen", { rev: r, pos, text, block })),
    hinweis: (text) => hinweis(text),
    zumEditor: () => view.focus(),
    cursor: () => view.state.selection.main.head,
  });
  seitenleiste = new Seitenleiste(document.getElementById("links")!, {
    hier: hierAbfragen,
    setzen: wertSetzen,
    tabelle: tabelleAktion,
    markup: markupAktion,
    zitat: zitatAendern,
    zurueck: () => view.focus(),
    zitatQuelle: (r, seite) => palette.oeffnenZumAendern((schluessel) => {
      zitatAendern(r, seite, schluessel);
      view.focus();
    }),
    bausteine: async (anfrage) => {
      const m = await modusAn(view.state.selection.main.from);
      return m === null ? [] : menue(katalogAnzeige(), m, anfrage);
    },
    einfuegen: (id) => {
      const { from, to } = view.state.selection.main;
      void bausteinEinsetzen(id, from, to, view.state.sliceDoc(from, to));
    },
    geschlossen: () => view.focus(),
  }, dokumentFach, paketFach, formelFach, new SeiteFach({
    stand: () => abfragen(() => invoke<SeitenJs>("seite_stand")),
    einbinden: () => invoke("schriften_einbinden"),
    setzen: (feld: SeitenFeld) =>
      abfragen(async (r) => {
        try {
          patchAnwenden(await invoke<PatchJs>("seite_setzen", { rev: r, feld }), "input.tftn-seite");
        } catch (fehler) {
          fehlerMelden(fehler, t("Nicht geändert"));
        }
      }),
  }));
  gliederung = new GliederungLeiste(gliederungOrt(), {
    springen: (von) => {
      zuStelle(von);
      view.focus();
    },
    verschieben: abschnittVerschieben,
    schliessen: () => gliederungSchalten(false),
  });
  gliederungOrt().hidden = !gliederungAnfang();
  document.getElementById("gliederung-knopf")!.addEventListener("click", () => gliederungSchalten(gliederungOrt().hidden !== false));
  document.getElementById("seite-knopf")!.addEventListener("click", () => seitenleisteOeffnen("seite"));
  document.getElementById("seitenleiste-knopf")!.addEventListener("click", seitenleisteUmschalten);
  document.getElementById("schreibansicht-knopf")!.addEventListener("click", () => schreibansichtSchalten(true));
  document.getElementById("schreibansicht-ende")!.addEventListener("click", () => schreibansichtSchalten(false));
  // Esc verlässt die Schreibansicht, wenn nichts anderes die Taste gebraucht hat; das Kürzel wirkt
  // auch außerhalb des Editors (im Editor über die Keymap).
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (escVerlaesstJetzt(e)) {
        e.preventDefault();
        schreibansichtSchalten(false);
      }
      return;
    }
    if (e.defaultPrevented || einDialogOffen() || view.dom.contains(e.target as Node)) return;
    if (belegung && aktionDerTaste(belegung, e, PLATTFORM) === "helfer/schreibansicht") {
      e.preventDefault();
      schreibansichtSchalten(!schreibansicht.istAn());
    }
  });
  // Im Editor gilt das Kürzel über die Keymap; aus der Leiste heraus hier.
  document.getElementById("links")!.addEventListener("keydown", (e) => {
    if (view.dom.contains(e.target as Node)) return; // im Editor wirkt die Keymap
    if (belegung && aktionDerTaste(belegung, e, PLATTFORM) === "helfer/seitenleiste") {
      e.preventDefault();
      seitenleisteKuerzel();
    }
  });
  const mitHinweis = async (f: () => Promise<EinstellungenJs>) => {
    try {
      einstellungenSetzen(await f());
    } catch (fehler) {
      hinweis(String(fehler));
    }
  };
  einstellDialog = new EinstellDialog({
    kuerzelSetzen: (aktion, kuerzel) => mitHinweis(() => invoke("kuerzel_setzen", { aktion, kuerzel })),
    ordnerSetzen: (ordner, waehlen) => mitHinweis(() => invoke("einstellungsordner_setzen", { ordner, waehlen })),
    offlineSetzen: (an) => mitHinweis(() => invoke("offline_setzen", { an })),
    updatesPruefen: () => invoke<UpdatesJs>("updates_pruefen"),
    sprachenLesen: () => invoke<SprachenJs>("sprachen_lesen"),
    spracheSetzen: async (kennung) => {
      try {
        await invoke("sprache_setzen", { kennung });
        // Neu laden wie bei einem Projektwechsel; ein Dokument ohne Namen braucht vorher einen.
        if (!(await wechselNachSpeichern(vorWechsel, async () => location.reload()))) {
          hinweis(t("Die Sprache gilt ab dem nächsten Start."));
        }
      } catch (fehler) {
        hinweis(String(fehler));
      }
    },
    sprachvorlage: () => invoke<string>("sprachvorlage_anlegen"),
    geschlossen: () => view?.focus(),
  });
  document.getElementById("einstellungen-knopf")!.addEventListener("click", () => einstellDialog.oeffnen());
  await listen<EinstellungenJs>("einstellungen-geaendert", (e) => einstellungenSetzen(e.payload));
  await listen("katalog-geaendert", () => {
    void katalogNeu().then(() => seitenleiste?.nachladen());
  });
  new ExportMenue(
    document.getElementById("export-knopf")!,
    () => invoke<ProfilJs[]>("export_profile"),
    async (profil, abnehmen) => {
      // Exportiert wird die Datei auf der Platte: vorher immer speichern (wartet auf alle Änderungen).
      await speichern();
      return invoke<ExportJs>("exportieren", { profil, abnehmen });
    },
    {
      exportieren: async () => {
        if (entwurf) throw new Error(t("Das Dokument braucht zuerst einen Namen: Cmd/Strg+S."));
        if (!(await speichern())) throw new Error(t("Speichern fehlgeschlagen – nicht exportiert."));
        return invoke<WordJs>("word_exportieren");
      },
      zeigen: (pfad) => invoke("im_ordner_zeigen", { pfad }),
    },
  );
  await listen<[number, string[]]>("literatur", (e) => {
    const [anzahl, fehler] = e.payload;
    hinweis(
      fehler.length
        ? t("Literatur: {fehler}", { fehler: fehler[0] })
        : anzahl === 1
          ? t("Literatur neu geladen: 1 Eintrag.")
          : t("Literatur neu geladen: {anzahl} Einträge.", { anzahl }),
    );
  });

  // Erst anmelden, dann öffnen: Der erste Kompilierlauf startet schon beim Öffnen.
  vorschau = new Vorschau(
    document.getElementById("vorschau")!,
    document.getElementById("seiten")!,
    document.getElementById("banner")!,
    gezeichnet,
    (seite, x, y) => void klick(seite, x, y),
    (seite, x, y, maus) => void kontext(seite, x, y, maus),
  );
  vorschauMenue = new VorschauMenue({
    formel: () => seitenleisteOeffnen("formel"),
    aendern: () => {
      zurStelle(menuePos);
      seitenleisteOeffnen("hier");
    },
    code: () => {
      zurStelle(menuePos);
      view.focus();
    },
    setzen: (aufruf, parameter, w) => wertSetzen(menueRev, aufruf, parameter, w, true),
    editor: (aufruf, feld, maus) => {
      const hrev = menueRev;
      popover?.zeigen(maus, feld.titel, editor(feld, (w, ende) => wertSetzen(hrev, aufruf, feld.parameter, w, ende)));
    },
    formular: (feld) => dokumentOeffnen(feld),
  });
  await listen<VorschauEreignis>("vorschau", (e) => {
    vorschau.aktualisieren(e.payload);
    // Markierungen nur, wenn sie zum aktuellen Text passen; sonst kommt bald ein neuer Lauf.
    if (view && e.payload.rev === rev) {
      const laenge = view.state.doc.length;
      const diagnosen: Diagnostic[] = e.payload.meldungen
        .filter((m) => m.von !== null && m.bis !== null && m.bis <= laenge)
        .map((m) => ({
          from: m.von!,
          to: Math.max(m.bis!, Math.min(m.von! + 1, laenge)),
          severity: m.fehler ? "error" : "warning",
          message: m.text,
        }));
      view.dispatch(setDiagnostics(view.state, diagnosen));
    }
    rechtschreibungSpaeter();
  });

  const g = await projektOeffnen();
  entwurf = g.entwurf;
  namensVorschlag = g.vorschlag ?? "";
  document.getElementById("datei")!.textContent = entwurf ? t("Ohne Namen") : `${g.ordner} / ${g.datei}`;
  rev = g.rev;
  try {
    katalog = await invoke<EintragJs[]>("katalog").then(texteUebersetzen);
  } catch (fehler) {
    hinweis(t("Bausteine nicht geladen: {fehler}", { fehler: String(fehler) }));
  }
  try {
    profile = await invoke<ProfilJs[]>("export_profile");
  } catch {
    profile = [];
  }
  // Scheitert das Lesen, startet das Fenster trotzdem (sonst bliebe es leer).
  einstellungenSetzen(await invoke<EinstellungenJs>("einstellungen").catch((f) => einstellungenNotfall(String(f))));
  const warnungen = einstellHinweise(einstellungen!, belegung!.meldungen).warnungen;
  if (warnungen.length) hinweis(warnungen[0]);
  const [editorStandard, verlaufTasten, suchTasten, vervollstaendigenTasten, tabTaste] = editorKeymaps();
  view = new EditorView({
    parent: document.getElementById("editor")!,
    state: EditorState.create({
      doc: g.text,
      extensions: [
        lineNumbers(),
        highlightSpecialChars(),
        // Widget-Patches einer Farbwahl sind ein Undo-Schritt, auch wenn sie nicht nebeneinander liegen.
        history({ joinToEvent: (tr, nebeneinander) => nebeneinander || tr.isUserEvent(WIDGET_EREIGNIS) }),
        drawSelection(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        EditorView.lineWrapping,
        // Rechtschreibung prüft TFTN selbst nach Dokumentsprache (nur Fließtext), nicht der Webview.
        EditorView.contentAttributes.of({
          spellcheck: "false",
          autocorrect: "off",
          autocapitalize: "off",
        }),
        rechtschreibung,
        // Kürzel aus der Belegung (eigene > Dokumenttyp/Paket > Standard) vor den Editor-Kürzeln,
        // damit ein bewusst gesetzter Konflikt wirkt; austauschbar ohne Neustart.
        Prec.high(tastenFach.of(keymap.of(keymapAus(belegung!, ausfuehren)))),
        keymap.of([...editorStandard, ...verlaufTasten, ...suchTasten, ...tabTaste]),
        // Bedienung der offenen Liste (Pfeile, Enter, Esc) vor allem anderen, wie bei autocompletion selbst.
        Prec.highest(keymap.of(vervollstaendigenTasten)),
        lintGutter(),
        // Helfer werden gerufen (Entwurf 3): „/“ am Zeilenanfang bzw. das Kürzel „Vervollständigen“.
        autocompletion({
          defaultKeymap: false,
          override: [
            slashQuelle({
              modus: modusAn,
              eintraege: katalogAnzeige,
              einsetzen: (id, von, bis) => void bausteinEinsetzen(id, von, bis, ""),
            }),
            typstQuelle({
              gerufen: () => {
                const g = ideGerufen;
                ideGerufen = false;
                return g;
              },
              vorschlaege: (pos) =>
                abfragen((r) => invoke<VervollstaendigungJs | null>("vervollstaendigen", { rev: r, pos })),
              einsetzen: (von, bis, e) => view.dispatch(einsetzenSpec(view.state, von, bis, e, null)),
            }),
          ],
          activateOnTyping: true,
        }),
        felder,
        hervorhebung,
        synchron,
        sichtbereich,
        bildAblage,
        markenErweiterung((m, dom) => void markeGeklickt(m, dom)),
      ],
    }),
  });
  popover = new Popover(() => view.focus());
  markenTasten();
  // Erste Hervorhebung: leere Änderung über neusync vermeiden, einfach anfragen.
  const h = await invoke<Bereich[] | null>("hervorheben", { rev, sichtbar: sichtbar() });
  if (h) view.dispatch({ effects: setzeHervorhebung.of(dekorationen(h, view.state.doc.length)) });

  await listen<string>("extern-geaendert", (e) => {
    if (!ungespeichert) {
      textErsetzen(e.payload);
      hinweis(t("Datei wurde außerhalb von TFTN geändert und neu geladen."));
    } else {
      hinweis(t("Datei wurde außerhalb von TFTN geändert. Deine Änderungen sind noch nicht gespeichert."), {
        titel: t("Fremde Fassung laden"),
        aktion: () => {
          textErsetzen(e.payload);
          hinweis("");
        },
      });
    }
  });
  status();
  if (entwurf) view.dispatch({ selection: { anchor: view.state.doc.length } });
  view.focus();
  gliederungSpaeter(0);
  let nachStart: string | null = null;
  try {
    nachStart = sessionStorage.getItem(NACH_START);
    sessionStorage.removeItem(NACH_START);
  } catch {
    // ohne sessionStorage nichts zu tun
  }
  if (nachStart === "dokument") seitenleisteOeffnen("dokument");
  try {
    const m = sessionStorage.getItem(NACH_IMPORT);
    sessionStorage.removeItem(NACH_IMPORT);
    if (m) hinweis(m);
  } catch {
    // ohne sessionStorage keine Meldung
  }
}

start().catch((fehler) => hinweis(t("Start fehlgeschlagen: {fehler}", { fehler: String(fehler) })));
