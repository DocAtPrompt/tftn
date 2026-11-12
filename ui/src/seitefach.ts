// Fach „Seite“ der Seitenleiste: Format, Schrift, Sprache. Jede Änderung gilt sofort.

import { SCHRIFTEN_HINWEIS } from "./neudialog";
import { FORMATE, SEITENZAHLEN, SPRACHEN, type Zeile, seitenzahlAnwenden, seitenzahlLage, spracheLesen, spracheWert } from "./seite";
import { n, t, tn } from "./sprache";

export interface SeitenJs {
  gesperrt: boolean;
  format: string | null;
  quer: boolean;
  schrift: string | null;
  groesse: number | null;
  sprache: string | null;
  region: string | null;
  raender: { oben: number; unten: number; innen: number; aussen: number } | null;
  doppelseitig: boolean;
  spalten: number;
  blocksatz: boolean;
  zeilenabstand: number | null;
  nummeriert: boolean;
  kopf: Zeile;
  fuss: Zeile;
  zeilen_moeglich: boolean;
  schriften: string[];
  eingebunden: boolean;
}

export type SeitenFeld =
  | { art: "format"; wert: string }
  | { art: "quer"; wert: boolean }
  | { art: "schrift"; wert: string }
  | { art: "groesse"; wert: number }
  | { art: "sprache"; wert: { lang: string; region: string | null } }
  | { art: "raender"; wert: { oben: number; unten: number; innen: number; aussen: number; doppelseitig: boolean } }
  | { art: "spalten"; wert: number }
  | { art: "blocksatz"; wert: boolean }
  | { art: "zeilenabstand"; wert: number }
  | { art: "nummerierung"; wert: boolean }
  | { art: "kopf"; wert: Zeile }
  | { art: "fuss"; wert: Zeile };

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export class SeiteFach {
  readonly huelle = el("div", "seite-fach");

  constructor(
    private a: {
      stand(): Promise<SeitenJs>;
      setzen(feld: SeitenFeld): Promise<void>;
      einbinden(): Promise<void>;
    },
  ) {}

  private auswahl(titel: string, werte: { wert: string; titel: string }[], aktuell: string, aendern: (w: string) => SeitenFeld) {
    const s = el("select");
    const alle = werte.some((w) => w.wert === aktuell) ? werte : [{ wert: aktuell, titel: aktuell }, ...werte];
    for (const w of alle) {
      const o = el("option", "", tn(w.titel));
      o.value = w.wert;
      s.append(o);
    }
    s.value = aktuell;
    s.addEventListener("change", () => void this.aendern(aendern(s.value)));
    return this.zeile(titel, s);
  }

  private zeile(titel: string, feld: HTMLElement) {
    const z = el("label", "leiste-feld");
    z.append(el("span", "leiste-titel", titel), feld);
    return z;
  }

  /** Text und Zahlen bleiben stehen, damit Tab zum nächsten Feld geht; `neu` zeichnet das Fach neu. */
  private async aendern(feld: SeitenFeld, neu = false) {
    await this.a.setzen(feld);
    if (neu) await this.zeichnen();
  }

  private haken(titel: string, an: boolean, aendern: (an: boolean) => void) {
    const b = el("input");
    b.type = "checkbox";
    b.checked = an;
    b.addEventListener("change", () => aendern(b.checked));
    return this.zeile(titel, b);
  }

  private zahl(titel: string, wert: number, min: number, max: number, schritt: number, aendern: (w: number) => void) {
    const f = el("input");
    f.type = "number";
    f.min = String(min);
    f.max = String(max);
    f.step = String(schritt);
    f.value = String(wert);
    f.addEventListener("change", () => {
      const w = Number(f.value);
      if (Number.isFinite(w)) aendern(w);
    });
    return this.zeile(titel, f);
  }

  private textzeilen(titel: string, z: Zeile, doppelseitig: boolean, aendern: (z: Zeile) => void) {
    const stand = { ...z };
    const feld = (name: "innen" | "mitte" | "aussen", beschriftung: string) => {
      const f = el("input");
      f.type = "text";
      f.value = stand[name];
      f.spellcheck = false;
      f.addEventListener("change", () => {
        stand[name] = f.value;
        aendern({ ...stand });
      });
      return this.zeile(beschriftung, f);
    };
    return [
      el("h3", "", titel),
      feld("innen", doppelseitig ? t("Innen") : t("Links")),
      feld("mitte", t("Mitte")),
      feld("aussen", doppelseitig ? t("Außen") : t("Rechts")),
      this.haken(t("Auch auf der ersten Seite"), stand.erste, (an) => {
        stand.erste = an;
        aendern({ ...stand });
      }),
    ];
  }

  async zeichnen() {
    let s: SeitenJs;
    try {
      s = await this.a.stand();
    } catch (fehler) {
      this.huelle.replaceChildren(el("p", "dok-fehler", String(fehler)));
      return;
    }
    if (s.gesperrt) {
      this.huelle.replaceChildren(
        el("p", "leiste-hilfe", t("Diese Vorlage legt Seite und Schrift selbst fest.")),
        ...this.einbindenTeil(s),
      );
      return;
    }
    const rand = { ...(s.raender ?? { oben: 2.5, unten: 2.5, innen: 2.5, aussen: 2.5 }) };
    const randSetzen = (neu = false) =>
      void this.aendern({ art: "raender", wert: { ...rand, doppelseitig } }, neu);
    let doppelseitig = s.doppelseitig;
    const randFeld = (name: keyof typeof rand, titel: string) =>
      this.zahl(titel, rand[name], 0, 15, 0.1, (w) => {
        rand[name] = w;
        randSetzen();
      });
    const teile: HTMLElement[] = [
      el("h3", "", t("Papier")),
      this.auswahl(t("Format"), FORMATE, s.format ?? "a4", (wert) => ({ art: "format", wert })),
      this.haken(t("Querformat"), s.quer, (wert) => void this.aendern({ art: "quer", wert })),
      this.haken(t("Doppelseitig (Innen- und Außenrand)"), s.doppelseitig, (an) => {
        doppelseitig = an;
        randSetzen(true);
      }),
      this.zahl(t("Spalten"), s.spalten, 1, 4, 1, (wert) => void this.aendern({ art: "spalten", wert })),
      el("h3", "", t("Ränder (cm)")),
      randFeld("oben", t("Oben")),
      randFeld("unten", t("Unten")),
      randFeld("innen", s.doppelseitig ? t("Innen") : t("Links")),
      randFeld("aussen", s.doppelseitig ? t("Außen") : t("Rechts")),
      el("h3", "", t("Text")),
      this.auswahl(
        t("Schrift"),
        s.schriften.map((f) => ({ wert: f, titel: f })),
        s.schrift ?? "Libertinus Serif",
        (wert) => ({ art: "schrift", wert }),
      ),
      this.zahl(t("Größe (pt)"), s.groesse ?? 11, 4, 144, 0.5, (wert) => void this.aendern({ art: "groesse", wert })),
      this.zahl(t("Zeilenabstand (em)"), s.zeilenabstand ?? 0.65, 0.1, 3, 0.05, (wert) =>
        void this.aendern({ art: "zeilenabstand", wert }),
      ),
      this.haken(t("Blocksatz"), s.blocksatz, (wert) => void this.aendern({ art: "blocksatz", wert })),
      this.haken(t("Überschriften nummerieren"), s.nummeriert, (wert) => void this.aendern({ art: "nummerierung", wert })),
      this.auswahl(t("Sprache"), SPRACHEN, spracheWert(s.sprache ?? "en", s.region), (wert) => ({
        art: "sprache",
        wert: spracheLesen(wert),
      })),
    ];
    if (s.zeilen_moeglich) {
      const lage = seitenzahlLage(s.kopf, s.fuss);
      const zahlen = lage === "eigene" ? [{ wert: "eigene", titel: n("Eigener Text") }, ...SEITENZAHLEN] : SEITENZAHLEN;
      const wahl = el("select");
      for (const z of zahlen) {
        const o = el("option", "", tn(z.titel));
        o.value = z.wert;
        wahl.append(o);
      }
      wahl.value = lage;
      wahl.addEventListener("change", async () => {
        const neu = seitenzahlAnwenden(wahl.value as (typeof SEITENZAHLEN)[number]["wert"], s.kopf, s.fuss);
        await this.a.setzen({ art: "kopf", wert: neu.kopf });
        await this.aendern({ art: "fuss", wert: neu.fuss }, true);
      });
      teile.push(
        el("h3", "", t("Seitenzahl")),
        this.zeile(t("Position"), wahl),
        ...this.textzeilen(t("Kopfzeile"), s.kopf, s.doppelseitig, (wert) => void this.aendern({ art: "kopf", wert })),
        ...this.textzeilen(t("Fußzeile"), s.fuss, s.doppelseitig, (wert) => void this.aendern({ art: "fuss", wert })),
        el("p", "leiste-hilfe", t("In Kopf und Fuß werden {seite} und {kapitel} ersetzt. Randtext: Baustein „Randtext“ im Menü „/“.", {
          seite: "{seite}",
          kapitel: "{kapitel}",
        })),
      );
    } else {
      teile.push(el("p", "leiste-hilfe", t("Kopf- und Fußzeilen und Randtext gibt es in neu angelegten leeren Dokumenten.")));
    }
    this.huelle.replaceChildren(...teile, ...this.einbindenTeil(s));
  }

  private einbindenTeil(s: SeitenJs): HTMLElement[] {
    const kopf = el("h3", "", t("Schriften"));
    if (s.eingebunden) return [kopf, el("p", "leiste-hilfe", t("Die Schriften sind in diesem Dokument eingebunden."))];
    const knopf = el("button", "", t("Schriften einbinden"));
    const meldung = el("p", "leiste-hilfe", SCHRIFTEN_HINWEIS.replace(/\s+[^.]*\.\s*$/, ""));
    knopf.addEventListener("click", async () => {
      knopf.disabled = true;
      try {
        await this.a.einbinden();
        await this.zeichnen();
      } catch (fehler) {
        meldung.textContent = String(fehler);
        knopf.disabled = false;
      }
    });
    return [kopf, knopf, meldung];
  }
}
