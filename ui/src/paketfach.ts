// Fach „Pakete“ in der Seitenleiste (PLAN-M2 AP9, Festlegung 9): Suche im zuletzt geholten Index,
// „Index holen“, „Hinzufügen“ (auf Wunsch mit Importzeile), Pakete im Projekt mit README und Pakete,
// die beim letzten Übersetzen fehlten. Netz gibt es nur im Holer; dieses Fach startet ihn über Rust
// und nur auf Klick. Die Logik der Knöpfe steht in `pakete.ts` (vitest).

import type { VergleichJs } from "./formular";
import {
  type IndexStand,
  type Treffer,
  entfernenKnopf,
  helferText,
  hinzufuegenKnopf,
  holbarKnopf,
  indexKnopf,
  indexText,
  wechselKnopf,
} from "./pakete";
import { vergleichTeil } from "./vergleichansicht";
import { t } from "./sprache";

export interface ProjektPaketJs {
  spec: string;
  name: string;
  version: string;
  pruefsumme: string | null;
  readme: boolean;
  /** Eine Beschreibung passt zu dieser Version. */
  helfer: boolean;
  /** Es gibt eine Beschreibung, gleich für welche Version. */
  beschreibung: boolean;
  /** Eine `.typ`-Datei des Projekts oder ein anderes Paket importiert es. */
  benutzt: boolean;
  /** Eine `.typ`-Datei des Projekts importiert es direkt. */
  direkt: boolean;
  /** Höhere passende Version laut Index */
  neuer: string | null;
}

export interface HolbarJs {
  paket: string;
  spec: string;
  titel: string;
  bausteine: string[];
  im_zwischenspeicher: boolean;
}

export interface PaketeStandJs {
  index: IndexStand | null;
  offline: boolean;
  laeuft: boolean;
  im_projekt: ProjektPaketJs[];
  fehlend: string[];
  holbar: HolbarJs[];
}

export interface PaketAktionen {
  stand(): Promise<PaketeStandJs>;
  indexHolen(): Promise<{ pakete: number; uebersprungen: number }>;
  suchen(anfrage: string): Promise<Treffer[]>;
  readme(spec: string): Promise<string | null>;
  /** Gibt den Hinweis für die Statuszeile zurück. */
  hinzufuegen(spec: string, importieren: boolean): Promise<string>;
  /** Gibt den Hinweis für die Statuszeile zurück. */
  entfernen(spec: string): Promise<string>;
  /** Speichert, holt die Version und vergleicht die Seiten. */
  vergleichen(spec: string, version: string): Promise<VergleichJs>;
  /** Übernimmt die Version; gibt den Hinweis für die Statuszeile zurück. */
  wechseln(spec: string, version: string): Promise<string>;
  hinweis(text: string): void;
  /** Der Holer läuft (`netz`) bzw. ist fertig: Anzeige in der Statuszeile. */
  verbindung(netz: boolean): void;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class PaketFach {
  readonly huelle = el("div", "paket-fach");
  private stand: PaketeStandJs | null = null;
  private treffer: Treffer[] = [];
  private anfrage = "";
  private laeuft = false;
  private readme: { spec: string; text: string } | null = null;
  private zeitgeber: number | undefined;
  private suchNr = 0;
  private suche = el("input", "paket-suche");
  private importieren = el("input");
  private wechsel: { spec: string; version: string; stand: "laeuft" | VergleichJs | string } | null = null;
  /** Nach dem nächsten Zeichnen zu diesem Paket scrollen (Menü „Paket holen“). */
  private gesucht: string | null = null;

  constructor(private a: PaketAktionen) {
    this.suche.type = "search";
    this.suche.placeholder = t("Paket suchen (Name, Beschreibung, Schlagwort) …");
    this.suche.addEventListener("input", () => {
      this.anfrage = this.suche.value;
      clearTimeout(this.zeitgeber);
      this.zeitgeber = window.setTimeout(() => void this.suchen(), 200);
    });
    this.importieren.type = "checkbox";
    this.importieren.checked = true;
  }

  /** Zeigt ein Paket aus „Mit Helfern“ (Klick im Menü; geholt wird erst mit dem Knopf). */
  zeigen(spec: string) {
    this.gesucht = spec;
  }

  /** Lädt den Stand neu und zeichnet; die Suche bleibt stehen. */
  async zeichnen(): Promise<void> {
    try {
      this.stand = await this.a.stand();
      this.laeuft ||= this.stand.laeuft;
      if (this.anfrage.trim()) this.treffer = await this.a.suchen(this.anfrage);
      this.darstellen();
    } catch (fehler) {
      this.huelle.replaceChildren(el("p", "dok-fehler", String(fehler)));
    }
  }

  private async suchen() {
    const nr = ++this.suchNr;
    try {
      const gefunden = this.anfrage.trim() ? await this.a.suchen(this.anfrage) : [];
      if (nr !== this.suchNr) return;
      this.treffer = gefunden;
      this.darstellen();
    } catch (fehler) {
      this.a.hinweis(String(fehler));
    }
  }

  /** Führt `f` aus, sperrt dabei alle Knöpfe und zeigt mit `netz` die Verbindung an. */
  private async mitLauf(netz: boolean, f: () => Promise<string | void>) {
    if (this.laeuft) return;
    this.laeuft = true;
    if (netz) this.a.verbindung(true);
    this.darstellen();
    try {
      const text = await f();
      if (text) this.a.hinweis(text);
    } catch (fehler) {
      this.a.hinweis(String(fehler));
    } finally {
      this.laeuft = false;
      if (netz) this.a.verbindung(false);
      await this.zeichnen();
    }
  }

  private knopf(text: string, k: { an: boolean; titel: string }, aktion: () => void): HTMLButtonElement {
    const b = el("button", "", text);
    b.disabled = !k.an;
    b.title = k.titel;
    b.addEventListener("click", aktion);
    return b;
  }

  private darstellen() {
    const s = this.stand;
    if (!s) return;
    const fokus = document.activeElement === this.suche;
    const kopf = el("div", "paket-kopf");
    const ik = indexKnopf(s.index, s.offline, this.laeuft);
    kopf.append(
      this.suche,
      this.knopf(ik.text, ik, () =>
        void this.mitLauf(true, async () => {
          const z = await this.a.indexHolen();
          return z.uebersprungen
            ? t("Index geholt: {pakete} Pakete, {uebersprungen} unlesbare übersprungen.", { pakete: z.pakete, uebersprungen: z.uebersprungen })
            : t("Index geholt: {pakete} Pakete.", { pakete: z.pakete });
        }),
      ),
      el("span", "paket-index", indexText(s.index)),
    );
    const imp = el("label", "paket-import");
    imp.append(this.importieren, " ", t("Import einfügen"));
    kopf.append(imp);
    if (s.offline) kopf.append(el("p", "paket-hinweis", t("Ganz offline: nur Pakete aus dem Zwischenspeicher.")));

    const teile: HTMLElement[] = [kopf];
    if (this.anfrage.trim()) {
      const liste = el("ul", "paket-liste");
      if (this.treffer.length === 0) {
        liste.append(el("li", "paket-leer", s.index ? t("Kein Paket gefunden.") : t("Erst den Index holen.")));
      }
      for (const tr of this.treffer) liste.append(this.trefferZeile(tr, s));
      teile.push(liste);
    }
    if (s.fehlend.length) {
      teile.push(el("h4", "", t("Fehlt im Projekt")));
      const liste = el("ul", "paket-liste");
      for (const spec of s.fehlend) {
        const li = el("li");
        li.append(
          el("span", "paket-name", spec),
          this.knopf(t("Holen"), { an: !this.laeuft, titel: t("Paket samt Abhängigkeiten ins Projekt holen (ohne Importzeile)") }, () =>
            void this.mitLauf(!s.offline, () => this.a.hinzufuegen(spec, false)),
          ),
        );
        liste.append(li);
      }
      teile.push(liste);
    }
    if (s.holbar.length) {
      teile.push(el("h4", "", t("Mit Helfern")));
      const liste = el("ul", "paket-liste");
      for (const h of s.holbar) {
        const li = el("li");
        li.dataset.spec = h.spec;
        li.append(
          el("span", "paket-name", h.titel),
          el("span", "paket-marke", h.spec.replace("@preview/", "")),
          this.knopf(t("Holen"), holbarKnopf(h, s.offline, this.laeuft), () =>
            void this.mitLauf(!h.im_zwischenspeicher, () => this.a.hinzufuegen(h.spec, false)),
          ),
          el("div", "paket-beschreibung", h.bausteine.join(", ")),
        );
        liste.append(li);
      }
      teile.push(liste);
    }
    teile.push(el("h4", "", t("Im Projekt")));
    if (s.im_projekt.length === 0) teile.push(el("p", "paket-leer", t("Noch keine Universe-Pakete.")));
    const liste = el("ul", "paket-liste");
    for (const p of s.im_projekt) {
      const li = el("li");
      li.append(el("span", "paket-name", `${p.name} ${p.version}`));
      li.append(el("span", "paket-marke", p.pruefsumme ? t("Prüfsumme vermerkt") : t("ohne Prüfsumme")));
      if (p.helfer) li.append(el("span", "paket-marke", t("Helfer")));
      const ohne = p.direkt ? helferText(p, p.beschreibung) : null;
      if (ohne) li.append(el("span", "paket-marke", ohne));
      if (p.readme) {
        li.append(
          this.knopf("README", { an: true, titel: t("Beschreibung des Pakets anzeigen") }, async () => {
            if (this.readme?.spec === p.spec) this.readme = null;
            else this.readme = { spec: p.spec, text: (await this.a.readme(p.spec)) ?? t("(keine README)") };
            this.darstellen();
          }),
        );
      }
      li.append(
        this.knopf(t("Entfernen"), entfernenKnopf(p, this.laeuft), () =>
          void this.mitLauf(false, () => this.a.entfernen(p.spec)),
        ),
      );
      const neuer = p.neuer;
      if (neuer) {
        li.append(
          this.knopf(t("Neuere Version {version} …", { version: neuer }), wechselKnopf(s.offline, this.laeuft), () => void this.vergleichen(p.spec, neuer)),
        );
      }
      liste.append(li);
      if (this.readme?.spec === p.spec) liste.append(el("pre", "paket-readme", this.readme.text));
      if (this.wechsel?.spec === p.spec) liste.append(this.wechselZeile());
    }
    teile.push(liste);
    this.huelle.replaceChildren(...teile);
    if (fokus) this.suche.focus();
    if (this.gesucht) {
      const ziel = this.huelle.querySelector<HTMLElement>(`[data-spec="${CSS.escape(this.gesucht)}"]`);
      this.gesucht = null;
      ziel?.scrollIntoView({ block: "nearest" });
      ziel?.classList.add("paket-gesucht");
    }
  }

  private wechselZeile(): HTMLLIElement {
    const li = el("li", "paket-wechsel");
    const w = this.wechsel;
    if (!w) return li;
    if (w.stand === "laeuft") li.append(el("p", "leiste-hilfe", t("Vergleiche … (Probe wird übersetzt)")));
    else if (typeof w.stand === "string") li.append(el("p", "dok-fehler", w.stand));
    else {
      li.append(
        vergleichTeil(w.stand, {
          uebernehmen: (knopf) => {
            knopf.disabled = true;
            void this.uebernehmen(w.spec, w.version);
          },
          verwerfen: () => {
            this.wechsel = null;
            this.darstellen();
          },
          ohneImport: t("Kein direkter Import des Pakets gefunden; bitte danach prüfen."),
        }),
      );
    }
    return li;
  }

  private async vergleichen(spec: string, version: string) {
    this.wechsel = { spec, version, stand: "laeuft" };
    await this.mitLauf(true, async () => {
      try {
        const v = await this.a.vergleichen(spec, version);
        this.wechsel = { spec, version, stand: v };
      } catch (fehler) {
        this.wechsel = { spec, version, stand: String(fehler) };
      }
    });
  }

  private async uebernehmen(spec: string, version: string) {
    await this.mitLauf(false, async () => {
      try {
        const text = await this.a.wechseln(spec, version);
        this.wechsel = null;
        return text;
      } catch (fehler) {
        this.wechsel = { spec, version, stand: String(fehler) };
      }
    });
  }

  private trefferZeile(tr: Treffer, s: PaketeStandJs): HTMLLIElement {
    const li = el("li");
    const kopf = el("div", "paket-zeile");
    kopf.append(el("span", "paket-name", `${tr.name} ${tr.version}`));
    if (tr.lizenz) kopf.append(el("span", "paket-marke", tr.lizenz));
    if (tr.im_projekt) kopf.append(el("span", "paket-marke", t("im Projekt")));
    else if (tr.im_zwischenspeicher) kopf.append(el("span", "paket-marke", t("im Zwischenspeicher")));
    if (tr.helfer) kopf.append(el("span", "paket-marke", t("Helfer")));
    const spec = `@preview/${tr.name}:${tr.version}`;
    kopf.append(
      this.knopf(t("Hinzufügen"), hinzufuegenKnopf(tr, s.offline, this.laeuft), () =>
        void this.mitLauf(!tr.im_zwischenspeicher, () => this.a.hinzufuegen(spec, this.importieren.checked)),
      ),
    );
    li.append(kopf);
    if (tr.beschreibung) li.append(el("div", "paket-beschreibung", tr.beschreibung));
    return li;
  }
}
