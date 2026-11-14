// Einstellungen (PLAN-M2 AP3, Entwurf 7): Ordner im Tresor, „ganz offline“ für diesen Rechner und die
// Tabelle der Tastenkürzel mit Suchfeld. Zeile anklicken, neues Kürzel drücken: Esc bricht ab,
// Entf/Rücktaste heißt „keine Belegung“, „Standard“ nimmt die eigene Belegung zurück. Konflikte beider
// Plattformen stehen sofort in der Zeile; gespeichert wird trotzdem (bewusst gesetzt).

import { kuerzelText } from "./bausteine";
import { type Aktion, type Belegung, type Konflikt, type Plattform, PLATTFORM, kuerzelAusTaste } from "./tasten";
import { dialogAnmelden } from "./dialoge";
import { t } from "./sprache";

export interface EinstellungenJs {
  ordner: string;
  synchronisiert: boolean;
  hinweis: string | null;
  ganz_offline: boolean;
  kuerzel: Record<string, string>;
  /** `tasten.toml` nicht lesbar: Umbelegen ist gesperrt */
  fehler: string | null;
  /** einzelne Einträge übersprungen; Umbelegen geht weiter */
  warnung: string | null;
}

export interface EinstellDaten {
  einstellungen: EinstellungenJs;
  aktionen: Aktion[];
  belegung: Belegung;
  konflikte: Konflikt[];
}

export interface EinstellAktionen {
  kuerzelSetzen(aktion: string, kuerzel: string | null): Promise<void>;
  ordnerSetzen(ordner: string | null, waehlen: boolean): Promise<void>;
  offlineSetzen(an: boolean): Promise<void>;
  updatesPruefen(): Promise<UpdatesJs>;
  sprachenLesen(): Promise<SprachenJs>;
  /** `null`: wie das System. Gilt nach dem Neuladen. */
  spracheSetzen(kennung: string | null): Promise<void>;
  /** Legt die Vorlage für eine neue Sprache an und zeigt sie; gibt den Pfad zurück. */
  sprachvorlage(): Promise<string>;
  geschlossen(): void;
}

export interface SprachenJs {
  gewaehlt: string | null;
  aktuell: string;
  liste: { kennung: string; name: string }[];
  ordner: string;
}

/** Auswahl „Sprache der Oberfläche“: erst „wie das System“, dann alle Sprachen. */
export function sprachOptionen(s: SprachenJs): { wert: string; text: string; gewaehlt: boolean }[] {
  return [
    { wert: "", text: t("Wie das System"), gewaehlt: !s.gewaehlt },
    ...s.liste.map((l) => ({ wert: l.kennung, text: l.name, gewaehlt: s.gewaehlt === l.kennung })),
  ];
}

export interface UpdatesJs {
  tftn_aktuell: string;
  tftn_neu: string | null;
  tftn_link: string;
  typst_aktuell: string;
  typst_neu: string | null;
  hinweise: string[];
}

/** Die Zeilen, die nach „Nach Updates suchen“ erscheinen. */
export function updatesText(u: UpdatesJs): string[] {
  const z: string[] = [];
  const tftnFehlt = u.hinweise.some((h) => /^TFTN:/.test(h));
  if (u.tftn_neu) z.push(t("TFTN {neu} ist erschienen (du hast {aktuell}). Herunterladen: {link}", { neu: u.tftn_neu, aktuell: u.tftn_aktuell, link: u.tftn_link }));
  else if (!tftnFehlt) z.push(t("TFTN {version} ist aktuell.", { version: u.tftn_aktuell }));
  if (u.typst_neu) {
    z.push(t("typst {neu} ist erschienen. TFTN nutzt {aktuell}; die neue Version kommt mit einer neuen TFTN-Version, Dokumente werden dann geführt umgestellt.", { neu: u.typst_neu, aktuell: u.typst_aktuell }));
  } else if (!u.hinweise.some((h) => /^typst:/.test(h))) {
    z.push(t("typst {version} ist aktuell.", { version: u.typst_aktuell }));
  }
  return [...z, ...u.hinweise];
}

const HERKUNFT: Record<string, string> = { eigene: t("eigene"), vorschlag: "", standard: "TFTN", keine: "" };
const PLATTFORM_NAME: Record<Plattform, string> = { mac: "Mac", win: "Windows" };

/** Was der Dialog über `tasten.toml` sagt: Nur eine unlesbare Datei sperrt das Umbelegen. */
export function einstellHinweise(
  e: EinstellungenJs,
  meldungen: string[],
): { gesperrt: boolean; fehler: string | null; warnungen: string[] } {
  return {
    gesperrt: e.fehler !== null,
    fehler: e.fehler,
    warnungen: e.fehler ? [] : [...(e.warnung ? [e.warnung] : []), ...meldungen],
  };
}

/** Ein Eingabefeld bekommt seine Tasten, auch während ein Kürzel aufgenommen wird. */
export function tasteDurchlassen(ziel: { tagName: string; type?: string } | null): boolean {
  if (!ziel) return false;
  if (ziel.tagName === "TEXTAREA") return true;
  return ziel.tagName === "INPUT" && !["checkbox", "radio", "button"].includes(ziel.type ?? "text");
}

export class EinstellDialog {
  private huelle: HTMLDivElement;
  private suche: HTMLInputElement;
  private liste: HTMLTableSectionElement;
  private kopf: HTMLDivElement;
  private pfad: HTMLInputElement;
  private offline: HTMLInputElement;
  private sprache: HTMLSelectElement;
  private spracheText: HTMLSpanElement;
  private daten: EinstellDaten | null = null;
  /** Aktion, für die gerade ein Kürzel aufgenommen wird */
  private aufnahme: string | null = null;

  constructor(private a: EinstellAktionen) {
    this.huelle = document.createElement("div");
    this.huelle.id = "einstellungen";
    this.huelle.hidden = true;
    this.huelle.innerHTML = `
      <div class="neu-kopf einst-kopf"><div class="palette-kopf">${t("Einstellungen")}</div><button class="neu-zu einst-zu" title="${t("Schließen (Esc)")}">×</button></div>
      <div class="einst-ordner">
        <div class="einst-zustand"></div>
        <div class="einst-zeile">
          <input class="einst-pfad" placeholder="${t("Ordner im Tresor für tasten.toml und einstellungen.toml")}" spellcheck="false" />
          <button class="einst-uebernehmen">${t("Übernehmen")}</button>
          <button class="einst-waehlen">${t("Ordner wählen …")}</button>
          <button class="einst-zuruecksetzen" title="${t("Wieder nur auf diesem Rechner speichern")}">${t("Zurücksetzen")}</button>
        </div>
        <div class="einst-zeile einst-updates"><button class="einst-updates-knopf" title="${t("Fragt GitHub nach den neuesten Versionen von TFTN und typst – nur auf Klick")}">${t("Nach Updates suchen")}</button><span class="einst-updates-text leiste-hilfe"></span></div>
        <div class="einst-zeile einst-sprache"><label>${t("Sprache der Oberfläche")} <select class="einst-sprache-wahl"></select></label><button class="einst-sprachvorlage" title="${t("Legt eine Vorlage zum Übersetzen an und zeigt sie; übersetzt als <kennung>.toml speichern, etwa fr.toml")}">${t("Weitere Sprache …")}</button><span class="einst-sprache-text leiste-hilfe"></span></div>
        <label class="einst-offline"><input type="checkbox" /> ${t("Ganz offline (nur dieser Rechner): TFTN holt keine Pakete aus dem Netz")}</label>
      </div>
      <div class="palette-felder palette-einzeln">
        <input class="palette-suche" placeholder="${t("Aktion oder Kürzel suchen …")}" spellcheck="false" />
      </div>
      <div class="einst-tabelle">
        <table>
          <thead><tr><th>${t("Aktion")}</th><th>${t("Herkunft")}</th><th>${t("Kürzel")}</th><th>${t("Hinweis")}</th><th></th></tr></thead>
          <tbody></tbody>
        </table>
      </div>
      <div class="palette-fuss">${t("Zeile anklicken und das neue Kürzel drücken (Esc bricht ab, Entf = keine Belegung) · Esc schließt die Einstellungen")}</div>`;
    document.body.append(this.huelle);
    const q = <T extends Element>(s: string) => this.huelle.querySelector<T>(s)!;
    q<HTMLButtonElement>(".einst-zu").addEventListener("click", () => this.schliessen());
    dialogAnmelden(this);
    const knopf = q<HTMLButtonElement>(".einst-updates-knopf");
    const text = q<HTMLSpanElement>(".einst-updates-text");
    knopf.addEventListener("click", async () => {
      knopf.disabled = true;
      text.textContent = t("Frage nach …");
      try {
        text.textContent = updatesText(await this.a.updatesPruefen()).join(" · ");
      } catch (fehler) {
        text.textContent = String(fehler);
      }
      knopf.disabled = false;
    });
    this.sprache = q(".einst-sprache-wahl");
    this.spracheText = q(".einst-sprache-text");
    this.sprache.addEventListener("change", () => {
      this.spracheText.textContent = t("Wird umgestellt …");
      void this.a.spracheSetzen(this.sprache.value || null);
    });
    q(".einst-sprachvorlage").addEventListener("click", async () => {
      try {
        const pfad = await this.a.sprachvorlage();
        this.spracheText.textContent = t("Vorlage: {pfad}", { pfad });
      } catch (fehler) {
        this.spracheText.textContent = String(fehler);
      }
    });
    this.kopf = q(".einst-zustand");
    this.pfad = q(".einst-pfad");
    this.offline = q(".einst-offline input");
    this.suche = q(".palette-suche");
    this.liste = q("tbody");
    this.suche.addEventListener("input", () => this.zeichnen());
    q(".einst-uebernehmen").addEventListener("click", () => void this.a.ordnerSetzen(this.pfad.value, false));
    q(".einst-waehlen").addEventListener("click", () => void this.a.ordnerSetzen(null, true));
    q(".einst-zuruecksetzen").addEventListener("click", () => void this.a.ordnerSetzen(null, false));
    this.offline.addEventListener("change", () => void this.a.offlineSetzen(this.offline.checked));
    this.liste.addEventListener("click", (e) => {
      const ziel = e.target as HTMLElement;
      const tr = ziel.closest("tr[data-id]") as HTMLElement | null;
      if (!tr || this.daten?.einstellungen.fehler) return;
      const id = tr.dataset.id!;
      if (ziel.closest(".einst-standard")) {
        this.aufnahme = null;
        void this.a.kuerzelSetzen(id, null);
        return;
      }
      this.aufnahme = this.aufnahme === id ? null : id;
      // Fokus aus Such- oder Pfadfeld nehmen, sonst landet die Taste dort statt in der Aufnahme.
      if (this.aufnahme !== null) (document.activeElement as HTMLElement | null)?.blur();
      this.zeichnen();
    });
    // In der Aufnahme fängt der Dialog jede Taste ab, bevor Editor oder System sie sehen.
    window.addEventListener("keydown", (e) => this.taste(e), true);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  oeffnen() {
    this.huelle.hidden = false;
    this.aufnahme = null;
    this.suche.value = "";
    this.zeichnen();
    this.suche.focus();
    void this.sprachenZeichnen();
  }

  private async sprachenZeichnen() {
    try {
      const s = await this.a.sprachenLesen();
      this.sprache.replaceChildren(
        ...sprachOptionen(s).map((o) => {
          const el = document.createElement("option");
          el.value = o.wert;
          el.textContent = o.text;
          el.selected = o.gewaehlt;
          return el;
        }),
      );
      this.spracheText.textContent = t("Eigene Sprachen: {ordner}", { ordner: s.ordner });
    } catch (fehler) {
      this.spracheText.textContent = String(fehler);
    }
  }

  schliessen() {
    this.huelle.hidden = true;
    this.aufnahme = null;
    this.a.geschlossen();
  }

  /** Neuer Stand (nach dem Laden, einer Änderung oder einem Sync). */
  setzen(d: EinstellDaten) {
    this.daten = d;
    if (this.offen) this.zeichnen();
  }

  private taste(e: KeyboardEvent) {
    if (!this.offen) return;
    if (this.aufnahme === null) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        this.schliessen();
      }
      return;
    }
    if (tasteDurchlassen(e.target as HTMLInputElement | null)) {
      // Ins Suchfeld oder Pfadfeld getippt: Aufnahme beenden, die Taste gehört dem Feld.
      this.aufnahme = null;
      this.zeichnen();
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const id = this.aufnahme;
    if (e.key === "Escape") {
      this.aufnahme = null;
      this.zeichnen();
      return;
    }
    if ((e.key === "Delete" || e.key === "Backspace") && !e.altKey && !e.ctrlKey && !e.metaKey) {
      this.aufnahme = null;
      void this.a.kuerzelSetzen(id, "");
      return;
    }
    const k = kuerzelAusTaste(e, PLATTFORM);
    if (k === null) return; // Modifikator allein oder ungültig: weiter warten
    this.aufnahme = null;
    void this.a.kuerzelSetzen(id, k);
  }

  private zeichnen() {
    const d = this.daten;
    if (!d) return;
    const e = d.einstellungen;
    this.kopf.replaceChildren();
    const ort = document.createElement("div");
    ort.textContent = e.synchronisiert ? t("Synchronisiert über {ordner}", { ordner: e.ordner }) : (e.hinweis ?? "");
    ort.className = e.synchronisiert ? "" : "einst-warnung";
    this.kopf.append(ort);
    const typen = document.createElement("div");
    typen.className = "leiste-hilfe";
    typen.textContent = t("Eigene Dokumenttypen: {ordner} (haben Vorrang vor den mitgelieferten)", { ordner: `${e.ordner}${e.ordner.includes("\\") && !e.ordner.includes("/") ? "\\" : "/"}typen` });
    this.kopf.append(typen);
    const befund = einstellHinweise(e, d.belegung.meldungen);
    if (befund.fehler) {
      const f = document.createElement("div");
      f.className = "einst-fehler";
      f.textContent = t("{fehler} – Umbelegen ist gesperrt, bis die Datei repariert ist.", { fehler: befund.fehler });
      this.kopf.append(f);
    }
    for (const w of befund.warnungen) {
      const f = document.createElement("div");
      f.className = "einst-warnung";
      f.textContent = w;
      this.kopf.append(f);
    }
    if (document.activeElement !== this.pfad) this.pfad.value = e.synchronisiert ? e.ordner : "";
    this.offline.checked = e.ganz_offline;

    const woerter = this.suche.value.toLowerCase().split(/\s+/).filter(Boolean);
    const zeilen = d.belegung.zeilen.filter((z) => {
      const text = `${z.aktion.titel} ${z.aktion.gruppe} ${z.aktion.id} ${z.kuerzel.map((k) => kuerzelText(k)).join(" ")}`.toLowerCase();
      return woerter.every((w) => text.includes(w));
    });
    const trs = zeilen.map((z) => {
      const tr = document.createElement("tr");
      tr.dataset.id = z.aktion.id;
      if (this.aufnahme === z.aktion.id) tr.className = "einst-aufnahme";
      const zelle = (text: string, klasse = "") => {
        const td = document.createElement("td");
        td.textContent = text;
        if (klasse) td.className = klasse;
        tr.append(td);
        return td;
      };
      zelle(z.aktion.titel);
      const herkunft =
        z.quelle === "vorschlag" ? (z.aktion.herkunft === "paket" ? t("Paket") : t("Dokumenttyp")) : HERKUNFT[z.quelle];
      zelle(herkunft, "einst-leise");
      zelle(
        this.aufnahme === z.aktion.id
          ? t("Taste drücken …")
          : z.kuerzel.length
            ? z.kuerzel.map((k) => kuerzelText(k)).join(" · ")
            : "—",
        "einst-kuerzel",
      );
      const hinweise = d.konflikte
        .filter((k) => k.aktion === z.aktion.id)
        .map((k) => `${PLATTFORM_NAME[k.plattform]}: ${k.mit}`);
      if (z.ueberdecktVon) {
        const von = d.aktionen.find((a) => a.id === z.ueberdecktVon)?.titel ?? z.ueberdecktVon;
        hinweise.unshift(t("überdeckt von „{aktion}“", { aktion: von }));
      }
      zelle([...new Set(hinweise)].join(" · "), hinweise.length ? "einst-warnung" : "");
      const td = document.createElement("td");
      if (z.quelle === "eigene" || (z.quelle === "keine" && Object.hasOwn(e.kuerzel, z.aktion.id))) {
        const b = document.createElement("button");
        b.className = "einst-standard";
        b.textContent = t("Standard");
        b.title = t("Eigene Belegung entfernen");
        td.append(b);
      }
      tr.append(td);
      return tr;
    });
    this.liste.replaceChildren(...trs);
  }
}
