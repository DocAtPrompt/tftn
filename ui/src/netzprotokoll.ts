// Netzprotokoll: jeder Netzzugriff von TFTN seit dem Start – im Normalfall keiner. Ins Netz geht nur
// das Hilfsprogramm tftn-holer, und nur auf Klick; dazu gezählt die Zugriffe von KI-Programmen über
// MCP (lokal). Das Blatt für den Datenschutz setzt Rust als PDF und öffnet es im PDF-Betrachter.

import { dialogAnmelden, dialogKopf } from "./dialoge";
import { groesse, t, zahl } from "./sprache";

/** Eine Verbindung, wie sie der Holer meldet. */
export interface AbrufJs {
  url: string;
  status: number | null;
  bytes: number;
  fehler: string | null;
}

export interface NetzEintragJs {
  beginn_ms: number;
  dauer_ms: number;
  zweck: string;
  ausloeser: string;
  hosts: string[];
  abrufe: AbrufJs[];
  /** Der Holer hat seine Verbindungen gemeldet (sonst sind `hosts` nur die erlaubten). */
  gemeldet: boolean;
  ok: boolean;
  fehler: string | null;
  ergebnis: string;
}

export interface McpZeileJs {
  werkzeug: string;
  schreibt: boolean;
  anzahl: number;
  /** davon mit Inkognito (Personendaten nur als Ersatztext) */
  inkognito: number;
  erstes_ms: number;
  letztes_ms: number;
}

export interface NetzprotokollJs {
  start_ms: number;
  netz: NetzEintragJs[];
  verworfen: number;
  mcp: McpZeileJs[];
  mcp_abgewiesen: number;
  /** Beschriftungen der Felder, die bei Inkognito ersetzt wurden */
  inkognito_felder: string[];
  ganz_offline: boolean;
  mcp_an: boolean;
}

const zwei = (n: number) => String(n).padStart(2, "0");

/** „14:02:11“ in Ortszeit. */
export function uhrzeit(ms: number): string {
  const d = new Date(ms);
  return `${zwei(d.getHours())}:${zwei(d.getMinutes())}:${zwei(d.getSeconds())}`;
}

/** Abstand der Ortszeit zu UTC in Minuten (für das Blatt, das Rust setzt). */
export function versatzMinuten(d = new Date()): number {
  return -d.getTimezoneOffset();
}

/** Wie viele Netzzugriffe es gab (auch die, die nicht mehr in der Liste stehen). */
export function anzahlNetz(p: NetzprotokollJs): number {
  return p.netz.length + p.verworfen;
}

/** Die große Zeile oben im Fenster. `ruhig`: keine Netzzugriffe. */
export function kopfZeile(p: NetzprotokollJs): { text: string; ruhig: boolean } {
  const uhr = uhrzeit(p.start_ms);
  const n = anzahlNetz(p);
  if (n === 0) return { text: t("Keine Netzzugriffe seit dem Start um {uhr}.", { uhr }), ruhig: true };
  if (n === 1) return { text: t("1 Netzzugriff seit dem Start um {uhr}.", { uhr }), ruhig: false };
  return { text: t("{anzahl} Netzzugriffe seit dem Start um {uhr}.", { anzahl: zahl(n), uhr }), ruhig: false };
}

/**
 * Statusleiste (`#netz`): Läuft der Holer, steht dort `laeuft`; sonst nach einem Netzzugriff die
 * Zahl, klickbar. Im Normalfall bleibt sie leer.
 */
export function statusAnzeige(p: NetzprotokollJs | null, laeuft = ""): { text: string; titel: string } {
  if (laeuft) return { text: laeuft, titel: t("Netzprotokoll zeigen") };
  const n = p ? anzahlNetz(p) : 0;
  if (n === 0) return { text: "", titel: "" };
  return {
    text: n === 1 ? t("1 Netzzugriff") : t("{anzahl} Netzzugriffe", { anzahl: zahl(n) }),
    titel: t("Seit dem Start hat TFTN auf Klick ins Netz zugegriffen. Klicken zeigt das Netzprotokoll."),
  };
}

/** Eine Verbindung in einer Zeile: „200 · 12 kB · https://…“ bzw. „Fehler: … · https://…“; ohne Daten ohne Größe. */
export function abrufZeile(a: AbrufJs): string {
  if (a.fehler) return `${t("Fehler: {fehler}", { fehler: a.fehler })} · ${a.url}`;
  const teile = [String(a.status ?? "–"), ...(a.bytes > 0 ? [groesse(a.bytes)] : []), a.url];
  return teile.join(" · ");
}

/** Zusammenfassung der MCP-Zugriffe unter der Tabelle. */
export function mcpZeile(p: NetzprotokollJs): string {
  const anzahl = p.mcp.reduce((s, z) => s + z.anzahl, 0);
  const zustand = p.mcp_an ? t("MCP ist eingeschaltet.") : t("MCP ist ausgeschaltet.");
  let text =
    anzahl === 0
      ? t("Kein Zugriff eines KI-Programms über MCP.")
      : t("{anzahl} Zugriffe von KI-Programmen über MCP (nur auf diesem Rechner, 127.0.0.1).", { anzahl: zahl(anzahl) });
  if (p.mcp_abgewiesen > 0) text += ` ${t("Abgewiesen: {anzahl}.", { anzahl: zahl(p.mcp_abgewiesen) })}`;
  const inkognito = p.mcp.reduce((s, z) => s + (z.inkognito ?? 0), 0);
  if (inkognito > 0)
    text += ` ${t("Davon pseudonymisiert (Inkognito): {anzahl}; ersetzt: {felder}.", {
      anzahl: zahl(inkognito),
      felder: (p.inkognito_felder ?? []).join(", "),
    })}`;
  return `${text} ${zustand}`;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface NetzprotokollAktionen {
  lesen(): Promise<NetzprotokollJs>;
  /** Setzt das Blatt als PDF und öffnet es; gibt den Pfad zurück. */
  blatt(versatz: number): Promise<string>;
}

export class NetzprotokollDialog {
  private huelle = el("div", "neu-dialog netz-dialog");
  private kopf = el("p", "netz-kopf");
  private zustand = el("p", "netz-hilfe");
  private liste = el("div", "netz-liste");
  private mcp = el("p", "netz-hilfe");
  private knopf = el("button", "", t("Blatt für den Datenschutz (PDF) …"));
  private meldung = el("span", "netz-meldung leiste-hilfe");

  constructor(
    private a: NetzprotokollAktionen,
    ausloeser: (Element | null)[] = [],
  ) {
    this.huelle.id = "netzprotokoll";
    this.huelle.hidden = true;
    this.huelle.setAttribute("role", "dialog");
    this.huelle.setAttribute("aria-label", t("Netzprotokoll"));
    const hilfe = el(
      "p",
      "netz-hilfe",
      t(
        "TFTN selbst enthält keinen Netzwerk-Client. Ins Netz geht nur das Hilfsprogramm tftn-holer, und nur auf Klick: Pakete holen, Nach Updates suchen, Literatur aus DOI. Ein Link in „Über TFTN“ öffnet auf Klick den Standardbrowser; auch das steht hier. Das Protokoll lebt nur im Speicher und endet mit dem Programm.",
      ),
    );
    this.knopf.title = t("Eine Seite für Verarbeitungsverzeichnis oder Datenschutzbeauftragte: was TFTN lokal macht, welche Netzzugriffe es geben kann und welche es in dieser Sitzung gab. Öffnet sich im PDF-Betrachter zum Drucken oder Sichern.");
    this.knopf.addEventListener("click", () => void this.blatt());
    const unten = el("div", "neu-ordnerzeile netz-unten");
    unten.append(this.knopf, this.meldung);
    this.huelle.append(dialogKopf(t("Netzprotokoll"), () => this.schliessen()), this.kopf, hilfe, this.zustand, this.liste, this.mcp, unten);
    document.body.append(this.huelle);
    dialogAnmelden(this, this.huelle, ...ausloeser);
  }

  get offen(): boolean {
    return !this.huelle.hidden;
  }

  schliessen() {
    this.huelle.hidden = true;
  }

  async zeigen() {
    this.meldung.textContent = "";
    this.huelle.hidden = false;
    await this.neu();
  }

  /** Neu lesen, wenn offen (nach jedem neuen Eintrag). */
  async neu() {
    if (!this.offen) return;
    try {
      this.zeichnen(await this.a.lesen());
    } catch (fehler) {
      this.kopf.textContent = String(fehler);
    }
  }

  private zeichnen(p: NetzprotokollJs) {
    const k = kopfZeile(p);
    this.kopf.textContent = k.text;
    this.kopf.classList.toggle("ruhig", k.ruhig);
    this.zustand.textContent = p.ganz_offline
      ? t("„Ganz offline“ ist eingeschaltet: Der Holer startet nicht, Links öffnen sich nicht.")
      : t("„Ganz offline“ ist ausgeschaltet.");
    if (p.netz.length === 0) {
      this.liste.replaceChildren();
    } else {
      const tabelle = el("table", "netz-tabelle");
      const kopf = el("tr");
      for (const s of [t("Zeit"), t("Zweck"), t("Gegenstelle"), t("Auslöser"), t("Ergebnis")]) kopf.append(el("th", "", s));
      tabelle.append(el("thead"), el("tbody"));
      tabelle.tHead!.append(kopf);
      // Neueste zuerst.
      for (const e of [...p.netz].reverse()) {
        const zeile = el("tr", e.ok ? "" : "netz-fehler");
        zeile.append(
          el("td", "netz-zeit", uhrzeit(e.beginn_ms)),
          el("td", "", e.zweck),
          el("td", "netz-host", e.hosts.join(", ") || "–"),
          el("td", "", e.ausloeser),
          el("td", "", e.ergebnis),
        );
        tabelle.tBodies[0].append(zeile);
        const einzeln = e.abrufe.map(abrufZeile);
        if (e.fehler) einzeln.push(t("Meldung: {fehler}", { fehler: e.fehler }));
        if (!e.gemeldet) einzeln.push(t("Der Holer hat nichts gemeldet; genannt sind die Gegenstellen, die er hier ansprechen darf."));
        if (einzeln.length) {
          const z = el("tr", "netz-abrufe");
          const td = el("td");
          td.colSpan = 5;
          for (const s of einzeln) td.append(el("div", "", s));
          z.append(td);
          tabelle.tBodies[0].append(z);
        }
      }
      this.liste.replaceChildren(tabelle);
      if (p.verworfen > 0)
        this.liste.append(el("p", "netz-hilfe", t("Ältere Einträge nicht mehr in der Liste: {anzahl}.", { anzahl: zahl(p.verworfen) })));
    }
    this.mcp.textContent = mcpZeile(p);
  }

  private async blatt() {
    this.knopf.disabled = true;
    this.meldung.textContent = t("Wird gesetzt …");
    try {
      const pfad = await this.a.blatt(versatzMinuten());
      this.meldung.textContent = t("Geöffnet: {pfad}", { pfad });
    } catch (fehler) {
      this.meldung.textContent = String(fehler);
    }
    this.knopf.disabled = false;
  }
}
