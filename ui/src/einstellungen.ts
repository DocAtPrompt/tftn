// Einstellungen (PLAN-M2 AP3, Entwurf 7): Ordner für eigene Anpassungen (unten), „ganz offline“ und MCP für diesen Rechner und die
// Tabelle der Tastenkürzel mit Suchfeld. Zeile anklicken, neues Kürzel drücken: Esc bricht ab,
// Entf/Rücktaste heißt „keine Belegung“, „Standard“ nimmt die eigene Belegung zurück. Konflikte beider
// Plattformen stehen sofort in der Zeile; gespeichert wird trotzdem (bewusst gesetzt).

import { kuerzelText } from "./bausteine";
import { type Aktion, type Belegung, type Konflikt, type Plattform, PLATTFORM, kuerzelAusTaste } from "./tasten";
import { dialogAnmelden, schliessenKnopf } from "./dialoge";
import { type McpJs, mcpBefehl } from "./mcp";
import { type SkinsJs, skinOptionen } from "./skin";
import type { TypenJs } from "./neuprojekt";
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
  /** Zeigt den Ordner für eigene Anpassungen im Finder bzw. Explorer. */
  ordnerZeigen(): Promise<void>;
  offlineSetzen(an: boolean): Promise<void>;
  updatesPruefen(): Promise<UpdatesJs>;
  /** Öffnet das Netzprotokoll (jeder Netzzugriff seit dem Start). */
  netzprotokoll(): void;
  sprachenLesen(): Promise<SprachenJs>;
  /** `null`: wie das System. Gilt nach dem Neuladen. */
  spracheSetzen(kennung: string | null): Promise<void>;
  /** Legt die Vorlage für eine neue Sprache an und zeigt sie; gibt den Pfad zurück. */
  sprachvorlage(): Promise<string>;
  skinsLesen(): Promise<SkinsJs>;
  /** `null`: wie das System. Gilt sofort. */
  skinSetzen(kennung: string | null): Promise<SkinsJs>;
  /** Legt die Vorlage für einen eigenen Skin an und zeigt sie; gibt den Pfad zurück. */
  skinvorlage(): Promise<string>;
  oberflaecheLesen(): Promise<number>;
  /** Prozent, eine der Stufen; gibt die geltende zurück. Gilt sofort. */
  oberflaecheSetzen(prozent: number): Promise<number>;
  typenLesen(): Promise<TypenJs>;
  /** Kopiert `basis` als neuen Typ `name`, zeigt den Ordner; gibt den Pfad zurück. */
  typAnlegen(basis: string, name: string): Promise<string>;
  mcpLesen(): Promise<McpJs>;
  mcpSetzen(an: boolean): Promise<McpJs>;
  mcpBeimWechsel(aus: boolean): Promise<McpJs>;
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
/** Klick daneben: Ein eingetippter, noch nicht übernommener Pfad oder Typname hält den Dialog offen. */
export function einstellungenBehalten(e: { pfad: string; pfadGezeigt: string; typName: string }): boolean {
  return e.pfad.trim() !== e.pfadGezeigt.trim() || e.typName.trim() !== "";
}

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
  /** Fehler und Warnungen zu tasten.toml, direkt über der Tabelle */
  private befundOrt: HTMLDivElement;
  private pfad: HTMLInputElement;
  /** Was im Pfadfeld zuletzt vom Stand gezeigt wurde. */
  private pfadGezeigt = "";
  private typName: HTMLInputElement;
  private offline: HTMLInputElement;
  private sprache: HTMLSelectElement;
  private spracheText: HTMLSpanElement;
  private skin: HTMLSelectElement;
  private groesse: HTMLSelectElement;
  private typBasis: HTMLSelectElement;
  private skinText: HTMLSpanElement;
  private mcpAn: HTMLInputElement;
  private mcpWechsel: HTMLInputElement;
  private mcpBefehl: HTMLInputElement;
  private mcpText: HTMLSpanElement;
  private daten: EinstellDaten | null = null;
  /** Aktion, für die gerade ein Kürzel aufgenommen wird */
  private aufnahme: string | null = null;

  constructor(private a: EinstellAktionen) {
    this.huelle = document.createElement("div");
    this.huelle.id = "einstellungen";
    this.huelle.hidden = true;
    this.huelle.innerHTML = `
      <div class="neu-kopf einst-kopf"><div class="palette-kopf">${t("Einstellungen")}</div></div>
      <div class="einst-ordner">
        <div class="einst-zeile einst-updates"><button class="einst-updates-knopf" title="${t("Fragt GitHub nach den neuesten Versionen von TFTN und typst – nur auf Klick")}">${t("Nach Updates suchen")}</button><button class="einst-netz-knopf" title="${t("Jeder Netzzugriff von TFTN seit dem Start, dazu ein Blatt für den Datenschutz")}">${t("Netzprotokoll …")}</button><span class="einst-updates-text leiste-hilfe"></span></div>
        <div class="einst-zeile einst-sprache"><label>${t("Sprache der Oberfläche")} <select class="einst-sprache-wahl"></select></label><button class="einst-sprachvorlage" title="${t("Legt eine Vorlage zum Übersetzen an und zeigt sie; übersetzt als <kennung>.toml speichern, etwa fr.toml")}">${t("Weitere Sprache …")}</button><span class="einst-sprache-text leiste-hilfe"></span></div>
        <div class="einst-zeile einst-skin"><label>${t("Aussehen")} <select class="einst-skin-wahl"></select></label><button class="einst-skinvorlage" title="${t("Legt eine Vorlage für einen eigenen Skin an und zeigt sie; Farben und Editorschrift lassen sich darin ändern")}">${t("Eigener Skin …")}</button><span class="einst-skin-text leiste-hilfe"></span></div>
        <div class="einst-zeile einst-groesse"><label>${t("Größe der Oberfläche")} <select class="einst-groesse-wahl" title="${t("Leisten, Knöpfe und Dialoge; Editor und Vorschau bleiben, wie sie sind")}">
          <option value="90">${t("Klein")}</option><option value="100">${t("Normal")}</option><option value="115">${t("Groß")}</option><option value="130">${t("Sehr groß")}</option>
        </select></label></div>
        <label class="einst-offline"><input type="checkbox" /> ${t("Ganz offline (nur dieser Rechner): TFTN holt keine Pakete aus dem Netz")}</label>
        <div class="einst-mcp">
          <label><input type="checkbox" class="einst-mcp-an" /> ${t("MCP: KI-Assistenten (lokal oder in der Cloud) dürfen im offenen Dokument lesen und schreiben")}</label>
          <label><input type="checkbox" class="einst-mcp-wechsel" /> ${t("MCP ausschalten, wenn ein anderes Dokument geöffnet wird (öffnet MCP es selbst, fragt TFTN nach)")}</label>
          <div class="einst-zeile"><input class="einst-mcp-befehl" readonly spellcheck="false" title="${t("Im Assistenten als MCP-Server eintragen; in Claude Code diese Zeile im Terminal ausführen")}" /><button class="einst-mcp-kopieren">${t("Kopieren")}</button></div>
          <span class="einst-mcp-text leiste-hilfe">${t("Im Assistenten als MCP-Server eintragen; in Claude Code diese Zeile im Terminal ausführen.")}</span>
        </div>
      </div>
      <div class="einst-befund"></div>
      <div class="palette-felder palette-einzeln">
        <input class="palette-suche" placeholder="${t("Aktion oder Kürzel suchen …")}" spellcheck="false" />
      </div>
      <div class="einst-tabelle">
        <table>
          <thead><tr><th>${t("Aktion")}</th><th>${t("Herkunft")}</th><th>${t("Kürzel")}</th><th>${t("Hinweis")}</th><th></th></tr></thead>
          <tbody></tbody>
        </table>
      </div>
      <div class="einst-ordner einst-anpassungen">
        <div class="einst-titel">${t("Ordner für eigene Anpassungen (Kürzel, Typen, Skins, Sprachen)")}</div>
        <div class="einst-zustand"></div>
        <div class="einst-zeile">
          <input class="einst-pfad" placeholder="${t("Anderer Ordner (optional), etwa ein Sync-Ordner für mehrere Rechner")}" spellcheck="false" />
          <button class="einst-uebernehmen">${t("Übernehmen")}</button>
          <button class="einst-waehlen">${t("Ordner wählen …")}</button>
          <button class="einst-zuruecksetzen" title="${t("Wieder den Standardordner dieses Rechners nehmen")}">${t("Zurücksetzen")}</button>
          <button class="einst-zeigen">${t("Im Ordner zeigen")}</button>
        </div>
        <div class="einst-zeile einst-typ">
          <label>${t("Eigener Dokumenttyp aus")} <select class="einst-typ-basis"></select></label>
          <input class="einst-typ-name" placeholder="${t("Name, z. B. Mein Brief")}" spellcheck="false" />
          <button class="einst-typ-anlegen" title="${t("Kopiert den gewählten Typ unter diesem Namen in den Ordner typen/; dort Vorgaben (vorlage/dokument.toml) und Layout (lib.typ) ändern")}">${t("Anlegen")}</button>
        </div>
        <span class="einst-typ-text leiste-hilfe"></span>
      </div>
      <div class="palette-fuss">${t("Zeile anklicken und das neue Kürzel drücken (Esc bricht ab, Entf = keine Belegung) · Esc schließt die Einstellungen")}</div>`;
    document.body.append(this.huelle);
    const q = <T extends Element>(s: string) => this.huelle.querySelector<T>(s)!;
    q(".einst-kopf").append(schliessenKnopf(() => this.schliessen()));
    dialogAnmelden(this, this.huelle, document.getElementById("einstellungen-knopf"));
    q(".einst-netz-knopf").addEventListener("click", () => this.a.netzprotokoll());
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
    const typBasis = q<HTMLSelectElement>(".einst-typ-basis");
    const typName = q<HTMLInputElement>(".einst-typ-name");
    this.typName = typName;
    const typText = q<HTMLSpanElement>(".einst-typ-text");
    const typAnlegen = async () => {
      try {
        const pfad = await this.a.typAnlegen(typBasis.value, typName.value);
        typName.value = "";
        typText.textContent = t("Angelegt: {pfad} – Vorgaben in vorlage/dokument.toml, Layout in lib.typ. „Neu“ → „Aus Vorlage …“ bietet ihn an.", { pfad });
        this.typenZeigen();
      } catch (fehler) {
        typText.textContent = String(fehler);
      }
    };
    q(".einst-typ-anlegen").addEventListener("click", () => void typAnlegen());
    typName.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void typAnlegen();
    });
    this.typBasis = typBasis;
    const groesse = q<HTMLSelectElement>(".einst-groesse-wahl");
    groesse.addEventListener("change", async () => {
      groesse.value = String(await this.a.oberflaecheSetzen(Number(groesse.value)).catch(() => 100));
    });
    this.groesse = groesse;
    this.skin = q(".einst-skin-wahl");
    this.skinText = q(".einst-skin-text");
    this.skin.addEventListener("change", async () => {
      try {
        this.skinZeigen(await this.a.skinSetzen(this.skin.value || null));
      } catch (fehler) {
        this.skinText.textContent = String(fehler);
      }
    });
    q(".einst-skinvorlage").addEventListener("click", async () => {
      try {
        const pfad = await this.a.skinvorlage();
        this.skinText.textContent = t("Vorlage: {pfad} – nach dem Ändern hier wählen.", { pfad });
        this.skinZeigen(await this.a.skinsLesen(), false);
      } catch (fehler) {
        this.skinText.textContent = String(fehler);
      }
    });
    this.mcpAn = q(".einst-mcp-an");
    this.mcpWechsel = q(".einst-mcp-wechsel");
    this.mcpBefehl = q(".einst-mcp-befehl");
    this.mcpText = q(".einst-mcp-text");
    const mcp = async (f: () => Promise<McpJs>) => {
      try {
        this.mcpZeigen(await f());
      } catch (fehler) {
        this.mcpText.textContent = String(fehler);
      }
    };
    this.mcpAn.addEventListener("change", () => void mcp(() => this.a.mcpSetzen(this.mcpAn.checked)));
    this.mcpWechsel.addEventListener("change", () => void mcp(() => this.a.mcpBeimWechsel(this.mcpWechsel.checked)));
    q(".einst-mcp-kopieren").addEventListener("click", async () => {
      this.mcpBefehl.select();
      try {
        await navigator.clipboard.writeText(this.mcpBefehl.value);
        this.mcpText.textContent = t("Kopiert.");
      } catch {
        this.mcpText.textContent = t("Markiert – mit Cmd/Strg+C kopieren.");
      }
    });
    this.kopf = q(".einst-zustand");
    this.befundOrt = q(".einst-befund");
    this.pfad = q(".einst-pfad");
    this.offline = q(".einst-offline input");
    this.suche = q(".palette-suche");
    this.liste = q("tbody");
    this.suche.addEventListener("input", () => this.zeichnen());
    q(".einst-uebernehmen").addEventListener("click", () => void this.a.ordnerSetzen(this.pfad.value, false));
    q(".einst-waehlen").addEventListener("click", () => void this.a.ordnerSetzen(null, true));
    q(".einst-zuruecksetzen").addEventListener("click", () => void this.a.ordnerSetzen(null, false));
    q(".einst-zeigen").addEventListener("click", () => void this.a.ordnerZeigen());
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


  behalten(): boolean {
    return einstellungenBehalten({ pfad: this.pfad.value, pfadGezeigt: this.pfadGezeigt, typName: this.typName.value });
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
    this.typenZeigen();
    void this.a.oberflaecheLesen().then(
      (p) => (this.groesse.value = String(p)),
      () => {},
    );
    void this.a.skinsLesen().then(
      (s) => this.skinZeigen(s),
      (fehler) => (this.skinText.textContent = String(fehler)),
    );
    void this.a.mcpLesen().then(
      (m) => this.mcpZeigen(m),
      (fehler) => (this.mcpText.textContent = String(fehler)),
    );
  }

  /** Schalter und Befehlszeile für MCP; auch, wenn es sich von außen ändert. */
  mcpZeigen(m: McpJs) {
    this.mcpAn.checked = m.an;
    this.mcpWechsel.checked = m.beim_wechsel_aus;
    this.mcpBefehl.value = mcpBefehl(m.programm);
    if (m.fehler) this.mcpText.textContent = m.fehler;
  }

  /** Auswahl der Vorbilder für „Eigener Dokumenttyp“; der Brief ist vorgewählt. */
  private typenZeigen() {
    void this.a.typenLesen().then(
      (j) => {
        const vorher = this.typBasis.value || "tftn-brief";
        this.typBasis.replaceChildren(
          ...j.typen.map((typ) => {
            const o = document.createElement("option");
            o.value = typ.name;
            o.textContent = typ.herkunft === "eigen" ? `${typ.titel} (${t("eigen")})` : typ.titel;
            return o;
          }),
        );
        if (j.typen.some((typ) => typ.name === vorher)) this.typBasis.value = vorher;
      },
      () => {},
    );
  }

  /** Auswahl „Aussehen“ und darunter Warnungen des Skins bzw. der Ordner für eigene. */
  private skinZeigen(s: SkinsJs, text = true) {
    this.skin.replaceChildren(
      ...skinOptionen(s.liste, s.gewaehlt).map((o) => {
        const el = document.createElement("option");
        el.value = o.wert;
        el.textContent = o.text;
        el.selected = o.gewaehlt;
        return el;
      }),
    );
    if (text)
      this.skinText.textContent = s.aktiv?.warnungen.length
        ? s.aktiv.warnungen.join(" · ")
        : t("Eigene Skins: {ordner}", { ordner: s.ordner });
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
    ort.textContent =
      e.hinweis ??
      (e.synchronisiert ? t("Gewählter Ordner: {ordner}", { ordner: e.ordner }) : t("Standardordner: {ordner}", { ordner: e.ordner }));
    ort.className = e.hinweis ? "einst-warnung" : "leiste-hilfe";
    this.kopf.append(ort);
    const typen = document.createElement("div");
    typen.className = "leiste-hilfe";
    typen.textContent = t("Eigene Dokumenttypen: {ordner} (haben Vorrang vor den mitgelieferten)", { ordner: `${e.ordner}${e.ordner.includes("\\") && !e.ordner.includes("/") ? "\\" : "/"}typen` });
    this.kopf.append(typen);
    this.befundOrt.replaceChildren();
    const befund = einstellHinweise(e, d.belegung.meldungen);
    if (befund.fehler) {
      const f = document.createElement("div");
      f.className = "einst-fehler";
      f.textContent = t("{fehler} – Umbelegen ist gesperrt, bis die Datei repariert ist.", { fehler: befund.fehler });
      this.befundOrt.append(f);
    }
    for (const w of befund.warnungen) {
      const f = document.createElement("div");
      f.className = "einst-warnung";
      f.textContent = w;
      this.befundOrt.append(f);
    }
    this.pfadGezeigt = e.synchronisiert ? e.ordner : "";
    if (document.activeElement !== this.pfad) this.pfad.value = this.pfadGezeigt;
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
