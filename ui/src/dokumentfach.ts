// Fach „Dokument“ der Seitenleiste (PLAN-M2 AP7, Spec 6): Titelei und Literatur als Formular aus dem
// Schema des Dokumenttyps, dazu „Projekt“ und „Export“. Geschrieben wird nur dokument.toml, und
// zwar in Rust (geprüft, verlustfrei, atomar); jede Änderung ist ein Schritt in der eigenen
// Historie des Fachs. Die Logik (Eingaben lesen, Gruppen, Tasten) steht in formular.ts.

import type { ExportJs, ProfilJs } from "./export";
import {
  type Art,
  type EintragAktion,
  type Feld,
  type FormularJs,
  type Meldung,
  type Pfad,
  type ProjektJs,
  type VergleichJs,
  Reihe,
  eingabeLesen,
  eingabeZeigen,
  gruppen,
  historieTaste,
  pfadText,
  wertAn,
  zustandUebertragen,
} from "./formular";
import { vergleichTeil } from "./vergleichansicht";
import { t } from "./sprache";

export interface DokumentAktionen {
  laden(): Promise<FormularJs>;
  setzen(stand: string, pfad: Pfad, wert: unknown | null): Promise<FormularJs>;
  eintrag(stand: string, pfad: Pfad, aktion: EintragAktion): Promise<FormularJs>;
  rueckgaengig(stand: string): Promise<FormularJs>;
  wiederholen(stand: string): Promise<FormularJs>;
  projekt(): Promise<ProjektJs>;
  profile(): Promise<ProfilJs[]>;
  exportieren(profil: string, abnehmen: boolean): Promise<ExportJs>;
  /** Speichert und vergleicht mit der neueren Version des Dokumenttyps (PLAN-M2 AP8). */
  vergleichen(): Promise<VergleichJs>;
  /** Übernimmt sie: Patch der Importzeile im Editor, speichern; gibt den Hinweis zurück. */
  uebernehmen(): Promise<string>;
  /** Kurzer Hinweis in der Statuszeile der App. */
  hinweis(text: string): void;
}

type Eingabe = HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

const MAC = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "");

const gleich = (a: Pfad, b: Pfad) => pfadText(a) === pfadText(b);

export class DokumentFach {
  readonly huelle: HTMLElement;
  private js: FormularJs | null = null;
  private projekt: ProjektJs | null = null;
  private profile: ProfilJs[] = [];
  private exportErgebnis = el("div", "export-ergebnis");
  /** Ergebnis von „Vergleichen …“, bleibt beim Neuzeichnen stehen, bis übernommen oder verworfen. */
  private vergleich: VergleichJs | "laeuft" | string | null = null;
  /** Eine Änderung von außen wartete, weil ein Feld ungeschriebene Eingaben hatte. */
  private ausstehend = false;
  private laeuft: Promise<void> = Promise.resolve();
  private datalistNr = 0;
  /** Schreibanfragen nacheinander, jede mit dem Stand der vorigen Antwort. */
  private reihe = new Reihe(() => this.js?.stand ?? "");

  constructor(private a: DokumentAktionen) {
    this.huelle = el("div", "dokument-fach");
    this.huelle.addEventListener("keydown", (e) => {
      const ziel = this.feldGeaendert();
      const was = historieTaste({ key: e.key, mod: MAC ? e.metaKey : e.ctrlKey, shift: e.shiftKey, ctrl: e.ctrlKey }, MAC, ziel);
      if (!was) return;
      e.preventDefault();
      void this.historie(was);
    });
    this.huelle.addEventListener("focusout", (e) => {
      if (this.ausstehend && !this.huelle.contains(e.relatedTarget as Node | null)) void this.zeichnen();
    });
  }

  /** Lädt alles neu und zeichnet. */
  zeichnen(): Promise<void> {
    this.ausstehend = false;
    this.laeuft = (async () => {
      try {
        const [js, projekt, profile] = await Promise.all([this.a.laden(), this.a.projekt(), this.a.profile().catch(() => [])]);
        this.js = js;
        this.projekt = projekt;
        this.profile = profile;
        this.darstellen();
      } catch (fehler) {
        this.huelle.replaceChildren(el("p", "dok-fehler", String(fehler)));
      }
    })();
    return this.laeuft;
  }

  /** Ereignis `dokument-geaendert`: neu laden, außer es ist der eigene Stand oder ein Feld ist in Arbeit. */
  aussen(stand: string) {
    if (this.js?.stand === stand) return;
    if (this.feldGeaendert()) {
      this.ausstehend = true;
      return;
    }
    void this.zeichnen();
  }

  /** Aufklappen, hinscrollen und das erste Eingabefeld von `feld` fokussieren. */
  async feldZeigen(feld: string) {
    await this.laeuft;
    const ziel = [...this.huelle.querySelectorAll<HTMLElement>("[data-pfad]")].find(
      (e) => e.dataset.pfad === feld || e.dataset.pfad!.startsWith(`${feld}.`),
    );
    if (!ziel) return;
    ziel.scrollIntoView({ block: "center" });
    ziel.querySelector<HTMLElement>("input, textarea, select")?.focus();
    ziel.classList.add("dok-gezeigt");
    setTimeout(() => ziel.classList.remove("dok-gezeigt"), 1200);
  }

  /** Hat das fokussierte Feld Eingaben, die noch nicht geschrieben sind? */
  private feldGeaendert(): boolean {
    const a = document.activeElement as Eingabe | null;
    if (!a || !this.huelle.contains(a) || !("value" in a) || a.dataset.gezeigt === undefined) return false;
    return a.value !== a.dataset.gezeigt;
  }

  // --- Schreiben --------------------------------------------------------------------------

  private async antwort(p: Promise<FormularJs>, eingabe?: { pfad: Pfad; roh: string }) {
    try {
      this.js = await p;
      this.darstellen();
      // Abgelehnt: Die Eingabe bleibt im Feld stehen, die Meldung steht darunter.
      if (this.js.abgelehnt && eingabe && gleich(this.js.abgelehnt.pfad, eingabe.pfad)) {
        const f = this.huelle.querySelector<Eingabe>(`[data-pfad="${CSS.escape(pfadText(eingabe.pfad))}"] .dok-eingabe`);
        if (f && !(f instanceof HTMLSelectElement)) f.value = eingabe.roh;
      }
    } catch (fehler) {
      if (String(fehler) === "veraltet") this.a.hinweis(t("dokument.toml wurde inzwischen geändert – neu geladen."));
      else this.a.hinweis(String(fehler));
      await this.zeichnen();
    }
  }

  private schreiben(feld: Feld, pfad: Pfad, roh: string | boolean, meldung: HTMLElement) {
    if (!this.js) return;
    const e = eingabeLesen(feld.art, roh);
    if ("fehler" in e) {
      meldung.textContent = e.fehler;
      meldung.className = "dok-meldung dok-meldung-fehler";
      return;
    }
    const eingabe = typeof roh === "string" ? { pfad, roh } : undefined;
    void this.reihe.einreihen((stand) => this.antwort(this.a.setzen(stand, pfad, e.wert), eingabe));
  }

  private eintrag(pfad: Pfad, aktion: EintragAktion) {
    if (this.js) void this.reihe.einreihen((stand) => this.antwort(this.a.eintrag(stand, pfad, aktion)));
  }

  private historie(was: "rueck" | "vor"): Promise<void> {
    return this.reihe.einreihen(async (stand) => {
      if (!this.js || (was === "rueck" ? !this.js.rueck : !this.js.vor)) return;
      await this.antwort(was === "rueck" ? this.a.rueckgaengig(stand) : this.a.wiederholen(stand));
    });
  }

  // --- Darstellung ------------------------------------------------------------------------

  private darstellen() {
    const js = this.js;
    if (!js) return;
    const aktiv = document.activeElement as HTMLElement | null;
    const fokus = aktiv?.closest<HTMLElement>("[data-pfad]")?.dataset.pfad;
    // Ungeschriebene Eingabe, Cursor und Scrollstand des Felds in Arbeit über das Neuzeichnen retten.
    const altesFeld =
      aktiv instanceof HTMLTextAreaElement || (aktiv instanceof HTMLInputElement && aktiv.type === "text") ? aktiv : null;
    const teile: HTMLElement[] = [this.kopf(js)];
    if (js.formular && !js.dateifehler) {
      for (const g of gruppen(js.formular)) {
        const s = el("section", "dok-gruppe");
        s.append(el("h3", "", g.titel));
        for (const f of g.felder) s.append(this.feld(f, [f.name]));
        teile.push(s);
      }
    } else if (!js.formular && !js.profilfehler.length) {
      teile.push(el("p", "leiste-leer", t("Dieser Dokumenttyp hat kein Formular.")));
    }
    if (this.projekt && !this.projekt.literatur_im_formular && js.formular) {
      const s = el("section", "dok-gruppe");
      s.append(el("h3", "", t("Literatur")));
      s.append(
        el("p", "leiste-hilfe", t("Dieser Dokumenttyp (Version {version}) liest die Literatur aus main.typ.", { version: this.projekt.version })),
      );
      teile.push(s);
    }
    teile.push(this.projektTeil(), this.exportTeil());
    this.huelle.replaceChildren(...teile);
    if (fokus) {
      const ziel = this.huelle.querySelector<HTMLElement>(`[data-pfad="${CSS.escape(fokus)}"]`);
      const neu = ziel?.querySelector<HTMLElement>("input, textarea, select");
      neu?.focus();
      if (altesFeld && (neu instanceof HTMLTextAreaElement || neu instanceof HTMLInputElement) && neu.type === altesFeld.type) {
        zustandUebertragen(altesFeld, neu);
      }
    }
  }

  private kopf(js: FormularJs): HTMLElement {
    const k = el("div", "dok-kopf");
    const rueck = el("button", "", t("Rückgängig"));
    rueck.disabled = !js.rueck;
    rueck.title = t("Letzte Änderung in {datei} zurücknehmen ({taste}, solange das Fach den Fokus hat)", {
      datei: js.datei,
      taste: MAC ? "⌘Z" : t("Strg+Z"),
    });
    rueck.addEventListener("click", () => void this.historie("rueck"));
    const vor = el("button", "", t("Wiederholen"));
    vor.disabled = !js.vor;
    vor.addEventListener("click", () => void this.historie("vor"));
    k.append(rueck, vor);
    if (js.sensibel) k.append(el("p", "dok-warnung", js.sensibel));
    if (js.dateifehler) k.append(el("p", "dok-warnung", js.dateifehler));
    if (js.profilfehler.length) {
      const p = el("div", "dok-warnung", t("Das Formular-Schema des Dokumenttyps verletzt das Profil TFTN-Schema 1:"));
      const ul = el("ul");
      for (const f of js.profilfehler) ul.append(el("li", "", `${f.ort || "/"}: ${f.text}`));
      p.append(ul);
      k.append(p);
    }
    return k;
  }

  private meldungen(pfad: Pfad): Meldung[] {
    const js = this.js!;
    const m = js.meldungen.filter((x) => gleich(x.pfad, pfad));
    if (js.abgelehnt && gleich(js.abgelehnt.pfad, pfad)) m.unshift(js.abgelehnt);
    return m;
  }

  private meldungZeile(pfad: Pfad): HTMLElement {
    const m = this.meldungen(pfad)[0];
    const z = el("div", m ? `dok-meldung ${m.fehler ? "dok-meldung-fehler" : "dok-meldung-hinweis"}` : "dok-meldung");
    if (m) z.textContent = m.text;
    return z;
  }

  private feld(f: Feld, pfad: Pfad): HTMLElement {
    const werte = this.js!.werte;
    const art = f.art;
    if (art.art === "gruppe") {
      const fs = el("fieldset", "dok-gruppe-feld");
      fs.dataset.pfad = pfadText(pfad);
      if (f.gruppe !== f.titel) fs.append(el("legend", "", f.titel));
      for (const u of art.felder) fs.append(this.feld(u, [...pfad, u.name]));
      return fs;
    }
    if (art.art === "eintraege") {
      const fs = el("fieldset", "dok-eintraege");
      fs.dataset.pfad = pfadText(pfad);
      fs.append(el("legend", "", f.titel + (f.pflicht ? " *" : "")));
      const liste = wertAn(werte, pfad);
      const eintraege = Array.isArray(liste) ? liste : [];
      eintraege.forEach((_, i) => {
        const d = el("div", "dok-eintrag");
        const kopf = el("div", "dok-eintrag-kopf", `${i + 1}.`);
        const knopf = (text: string, titel: string, aktion: EintragAktion, aus = false) => {
          const b = el("button", "", text);
          b.title = titel;
          b.disabled = aus;
          b.addEventListener("click", () => this.eintrag(pfad, aktion));
          kopf.append(b);
        };
        knopf("↑", t("nach oben"), { art: "verschieben", index: i, nach: i - 1 }, i === 0);
        knopf("↓", t("nach unten"), { art: "verschieben", index: i, nach: i + 1 }, i === eintraege.length - 1);
        knopf(t("Entfernen"), t("Eintrag entfernen"), { art: "entfernen", index: i });
        d.append(kopf);
        for (const u of art.felder) d.append(this.feld(u, [...pfad, i, u.name]));
        fs.append(d);
      });
      const neu = el("button", "dok-neu", t("+ Eintrag"));
      neu.addEventListener("click", () => this.eintrag(pfad, { art: "hinzufuegen" }));
      fs.append(neu, this.meldungZeile(pfad));
      return fs;
    }
    const zeile = el("div", "dok-feld");
    zeile.dataset.pfad = pfadText(pfad);
    const id = `dok-${pfadText(pfad).replace(/[^\w-]/g, "_")}`;
    const titel = el("label", "leiste-titel", f.titel + (f.pflicht ? " *" : ""));
    titel.htmlFor = id;
    const meldung = this.meldungZeile(pfad);
    const eingabe = this.eingabe(f, art, pfad, wertAn(werte, pfad), meldung);
    eingabe.id = id;
    eingabe.classList.add("dok-eingabe");
    if (this.js!.dateifehler) eingabe.disabled = true;
    zeile.append(titel, eingabe, meldung);
    if (f.hilfe) zeile.append(el("div", "leiste-hilfe", f.hilfe));
    return zeile;
  }

  private eingabe(f: Feld, art: Art, pfad: Pfad, wert: unknown, meldung: HTMLElement): Eingabe {
    const merken = (e: Eingabe, gezeigt: string) => {
      e.value = gezeigt;
      e.dataset.gezeigt = gezeigt;
    };
    const beiAenderung = (e: Eingabe) => e.addEventListener("change", () => this.schreiben(f, pfad, e.value, meldung));
    switch (art.art) {
      case "schalter": {
        const c = el("input");
        c.type = "checkbox";
        c.checked = eingabeZeigen(art, wert) === true;
        c.addEventListener("change", () => this.schreiben(f, pfad, c.checked, meldung));
        return c;
      }
      case "wahl": {
        const s = el("select");
        if (!f.pflicht || wert === undefined) s.append(new Option("—", ""));
        art.werte.forEach((w, i) => s.append(new Option(art.beschriftungen[i] ?? String(w), String(i))));
        const gezeigt = String(eingabeZeigen(art, wert));
        if (gezeigt === "" && wert !== undefined && wert !== null) {
          // Wert aus der Datei, den die Auswahl nicht kennt: zeigen statt still die erste Option.
          const fremd = new Option(t("{wert} (nicht in der Liste)", { wert: JSON.stringify(wert) }), "?", true, true);
          fremd.disabled = true;
          s.append(fremd);
        }
        merken(s, gezeigt === "" && wert !== undefined && wert !== null ? "?" : gezeigt);
        beiAenderung(s);
        return s;
      }
      case "liste": {
        const ta = el("textarea");
        ta.rows = Math.max(2, Array.isArray(wert) ? wert.length + 1 : 2);
        ta.placeholder = t("ein Eintrag je Zeile");
        merken(ta, String(eingabeZeigen(art, wert)));
        beiAenderung(ta);
        const format = art.element.art === "text" ? art.element.format : null;
        const v = format ? this.js!.vorschlaege[format] : undefined;
        if (v?.length) ta.title = t("Im Projekt: {liste}", { liste: v.filter((x) => !x.includes(" ")).slice(0, 12).join(", ") });
        return ta;
      }
      case "text": {
        if (art.widget !== "einzeilig") {
          const ta = el("textarea");
          ta.rows = art.widget === "markup" ? 5 : 3;
          if (art.widget === "markup") ta.placeholder = t("Typst-Markup, z. B. *fett* und _kursiv_");
          merken(ta, String(eingabeZeigen(art, wert)));
          beiAenderung(ta);
          return ta;
        }
        const i = el("input");
        i.type = "text";
        merken(i, String(eingabeZeigen(art, wert)));
        const v = art.format ? this.js!.vorschlaege[art.format] : undefined;
        if (v?.length) {
          const dl = el("datalist");
          dl.id = `dok-liste-${++this.datalistNr}`;
          for (const x of v) dl.append(new Option(x));
          i.setAttribute("list", dl.id);
          // Erst nach dem Einhängen der Zeile hat das Feld einen Nachbarn.
          queueMicrotask(() => i.after(dl));
        }
        beiAenderung(i);
        return i;
      }
      default: {
        // Zahl und Datum als Text, damit „1,5“ und „31.12.2026“ gehen.
        const i = el("input");
        i.type = "text";
        if (art.art === "datum") i.placeholder = t("TT.MM.JJJJ");
        merken(i, String(eingabeZeigen(art, wert)));
        beiAenderung(i);
        return i;
      }
    }
  }

  private projektTeil(): HTMLElement {
    const s = el("section", "dok-gruppe");
    s.append(el("h3", "", t("Projekt")));
    const p = this.projekt;
    if (!p) return s;
    const dl = el("dl", "dok-projekt");
    const zeile = (k: string, v: string) => dl.append(el("dt", "", k), el("dd", "", v));
    zeile(t("Dokumenttyp"), p.version ? `${p.typ} ${p.version}` : p.typ || "–");
    zeile("typst", p.typst);
    zeile(t("Pakete"), p.pakete.length ? p.pakete.map(([n, v]) => `${n} ${v}`).join(", ") : t("keine"));
    s.append(dl);
    if (p.aktualisierung) s.append(this.aktualisierungTeil(p.aktualisierung));
    return s;
  }

  /** Neuere Version: „Vergleichen …“, dann Seiten alt/neu und „Übernehmen“ bzw. „Verwerfen“. */
  private aktualisierungTeil(neu: NonNullable<ProjektJs["aktualisierung"]>): HTMLElement {
    const d = el("div", "dok-aktualisierung");
    d.append(el("div", "", t("Neuere Version: {version} ({herkunft})", {
          version: neu.version,
          herkunft: neu.herkunft === "eigen" ? t("eigener Ordner") : "TFTN",
        })));
    const v = this.vergleich;
    if (v === "laeuft") {
      d.append(el("p", "leiste-hilfe", t("Vergleiche … (Probe wird übersetzt)")));
      return d;
    }
    if (v === null || typeof v === "string") {
      const b = el("button", "", t("Vergleichen …"));
      b.title = t("Speichert, baut eine Probe mit der neuen Version und zeigt die geänderten Seiten");
      b.addEventListener("click", () => void this.vergleichen());
      d.append(b);
      if (typeof v === "string") d.append(el("p", "dok-fehler", v));
      return d;
    }
    d.append(
      vergleichTeil(v, {
        uebernehmen: (knopf) => void this.uebernehmen(knopf),
        verwerfen: () => {
          this.vergleich = null;
          this.darstellen();
        },
        ohneImport: t("Kein direkter Import des Dokumenttyps gefunden; bitte danach prüfen."),
      }),
    );
    return d;
  }

  private async vergleichen() {
    this.vergleich = "laeuft";
    this.darstellen();
    try {
      this.vergleich = await this.a.vergleichen();
    } catch (fehler) {
      this.vergleich = String(fehler);
    }
    this.darstellen();
  }

  private async uebernehmen(knopf: HTMLButtonElement) {
    knopf.disabled = true;
    try {
      this.a.hinweis(await this.a.uebernehmen());
      this.vergleich = null;
    } catch (fehler) {
      this.vergleich = String(fehler);
    }
    await this.zeichnen();
  }

  private exportTeil(): HTMLElement {
    const s = el("section", "dok-gruppe");
    s.append(el("h3", "", t("Export")));
    for (const p of this.profile) {
      const zeile = el("div", "export-zeile");
      zeile.append(el("span", "", `${p.id} · ${p.art}${p.standards.length ? ` · ${p.standards.join(", ")}` : ""}`));
      const probe = el("button", "", t("Probe"));
      probe.title = t("nach _renders/ (nicht synchronisiert)");
      probe.addEventListener("click", () => void this.exportieren(p.id, false));
      const abnehmen = el("button", "", t("Abnehmen …"));
      abnehmen.title = t("nach export/, Zeitstempel wird auf jetzt gesetzt");
      abnehmen.addEventListener("click", () => {
        if (abnehmen.dataset.sicher) void this.exportieren(p.id, true);
        else {
          abnehmen.dataset.sicher = "1";
          abnehmen.textContent = t("Wirklich abnehmen?");
        }
      });
      zeile.append(probe, abnehmen);
      s.append(zeile);
    }
    if (!this.profile.length) s.append(el("p", "leiste-leer", t("Keine Export-Profile.")));
    s.append(this.exportErgebnis);
    return s;
  }

  private async exportieren(profil: string, abnehmen: boolean) {
    const erg = this.exportErgebnis;
    erg.textContent = t("Exportiere …");
    try {
      const e = await this.a.exportieren(profil, abnehmen);
      erg.replaceChildren(el("div", "", t("Fertig: {pfad}", { pfad: e.relativ })));
      if (e.sha256) erg.append(el("div", "", `SHA-256 ${e.sha256}`));
      for (const w of e.warnungen) erg.append(el("div", "", w));
    } catch (fehler) {
      erg.textContent = t("Export fehlgeschlagen: {fehler}", { fehler: String(fehler) });
    }
  }
}
