//! Netzprotokoll: jeder Netzzugriff seit dem Start der App, dazu die Zugriffe von KI-Programmen über
//! MCP. Macht das Offline-Versprechen sichtbar: Im Normalfall ist die Liste leer.
//!
//! Ins Netz kommt TFTN nur über das Hilfsprogramm `tftn-holer` (die App selbst hat keinen
//! HTTP-Client, das prüft `cargo xtask netzfreiheit`). Jeder Start des Holers geht durch
//! [`crate::pakete::holer_aufrufen`] und landet hier, samt der Verbindungen, die der Holer selbst
//! meldet. MCP ist kein Netzzugriff (nur `127.0.0.1`), gibt aber Text an ein anderes Programm weiter;
//! deshalb steht es gezählt daneben. Das Protokoll lebt nur im Speicher und endet mit der App.
//!
//! Dazu das Blatt für Verarbeitungsverzeichnis und Datenschutzbeauftragte: eine Seite PDF, gesetzt
//! von der mitgelieferten typst-CLI aus [`BLATT_TYP`] und den Daten aus [`blatt_daten`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tftn_sprachen::tr;

/// Mehr Einträge hebt das Protokoll nicht auf; ältere fallen weg (gezählt in `verworfen`).
pub const EINTRAEGE_MAX: usize = 500;

/// Zeilen der Sitzung auf dem Blatt; mehr passen nicht auf eine Seite.
const BLATT_ZEILEN_MAX: usize = 10;

/// Wozu der Holer gestartet wurde; aus seinen Argumenten gelesen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Zweck {
    Index,
    Paket(String),
    Versionen,
    Doi(String),
    /// Eine feste Adresse aus „Über TFTN“ im Standardbrowser; ins Netz geht der Browser, nicht TFTN.
    Browser(String),
    Anderes(String),
}

impl Zweck {
    /// `index …`, `paket … <spec>`, `versionen`, `doi <eingabe>`.
    pub fn aus_argumenten(a: &[String]) -> Zweck {
        match a.first().map(String::as_str) {
            Some("index") => Zweck::Index,
            Some("paket") => Zweck::Paket(a.last().cloned().unwrap_or_default()),
            Some("versionen") => Zweck::Versionen,
            Some("doi") => Zweck::Doi(a.get(1).cloned().unwrap_or_default()),
            andere => Zweck::Anderes(andere.unwrap_or_default().to_string()),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Zweck::Index => tr!("Paketverzeichnis des Typst Universe holen"),
            Zweck::Paket(spec) => tr!("Paket {paket} samt Abhängigkeiten holen", paket = spec),
            Zweck::Versionen => tr!("Neueste Versionen von TFTN und typst abfragen"),
            Zweck::Doi(doi) => tr!("Literaturangabe zur DOI {doi} holen", doi),
            Zweck::Browser(url) => tr!("{adresse} im Browser öffnen", adresse = url),
            Zweck::Anderes(b) => b.clone(),
        }
    }

    /// Die Gegenstellen, die der Holer für diesen Zweck ansprechen darf.
    pub fn gegenstellen(&self) -> &'static [&'static str] {
        match self {
            Zweck::Index | Zweck::Paket(_) => &["packages.typst.org"],
            Zweck::Versionen => &["api.github.com"],
            Zweck::Doi(_) => &["api.crossref.org", "api.datacite.org"],
            Zweck::Browser(_) | Zweck::Anderes(_) => &[],
        }
    }
}

/// Welcher Klick den Holer gestartet hat. Von selbst startet TFTN ihn nie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ausloeser {
    PaketHinzufuegen,
    VersionWechseln,
    IndexHolen,
    Updates,
    Doi,
    Link,
}

impl Ausloeser {
    pub fn text(self) -> String {
        match self {
            Ausloeser::PaketHinzufuegen => tr!("Klick: Paket hinzufügen"),
            Ausloeser::VersionWechseln => tr!("Klick: Paketversion wechseln"),
            Ausloeser::IndexHolen => tr!("Klick: Paketverzeichnis holen"),
            Ausloeser::Updates => tr!("Klick: Nach Updates suchen"),
            Ausloeser::Doi => tr!("Klick: Literatur aus DOI"),
            Ausloeser::Link => tr!("Klick: Link in „Über TFTN“"),
        }
    }
}

/// Eine Verbindung, wie sie der Holer meldet (Feld `abrufe` seiner Antwort).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Abruf {
    pub url: String,
    pub status: Option<u16>,
    #[serde(default)]
    pub bytes: u64,
    pub fehler: Option<String>,
}

/// Rechnername aus einer Adresse: `https://packages.typst.org/preview/x` → `packages.typst.org`.
pub fn host(url: &str) -> String {
    let ohne = url.split_once("://").map_or(url, |(_, r)| r);
    let ende = ohne.find(['/', '?', '#']).unwrap_or(ohne.len());
    ohne[..ende].to_string()
}

/// Ein Lauf des Holers.
#[derive(Clone, Debug)]
pub struct NetzEintrag {
    pub beginn_ms: u64,
    pub dauer_ms: u64,
    pub zweck: Zweck,
    pub ausloeser: Ausloeser,
    /// Was der Holer gemeldet hat; `None`, wenn keine lesbare Antwort kam (Frist, Absturz).
    pub abrufe: Option<Vec<Abruf>>,
    /// Fehlermeldung des Laufs, sonst `None`.
    pub fehler: Option<String>,
}

impl NetzEintrag {
    /// Angesprochene Rechner: gemeldete, sonst die für den Zweck erlaubten.
    pub fn hosts(&self) -> Vec<String> {
        let mut h: Vec<String> = match (&self.abrufe, &self.zweck) {
            (_, Zweck::Browser(url)) => vec![host(url)],
            (Some(a), _) => a.iter().map(|a| host(&a.url)).collect(),
            (None, _) => self
                .zweck
                .gegenstellen()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        let mut gesehen = std::collections::BTreeSet::new();
        h.retain(|x| gesehen.insert(x.clone()));
        h
    }

    fn ergebnis(&self) -> String {
        let n = self.abrufe.as_ref().map_or(0, Vec::len);
        if let Zweck::Browser(_) = self.zweck {
            return match &self.fehler {
                None => tr!("an den Standardbrowser übergeben"),
                Some(f) => f.clone(),
            };
        }
        match (&self.fehler, &self.abrufe) {
            (_, Some(a)) if a.is_empty() => tr!("keine Verbindung aufgebaut"),
            (None, _) => tr!("erfolgreich, {anzahl} Verbindung(en)", anzahl = n),
            (Some(_), None) => tr!("abgebrochen, ohne Bericht des Holers"),
            (Some(_), Some(_)) => tr!("Fehler, {anzahl} Verbindung(en)", anzahl = n),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct McpZaehler {
    anzahl: u64,
    /// davon mit Inkognito (Personendaten nur als Ersatztext)
    inkognito: u64,
    erstes_ms: u64,
    letztes_ms: u64,
}

/// Das Protokoll dieser Sitzung.
#[derive(Clone, Debug)]
pub struct Protokoll {
    pub start_ms: u64,
    pub netz: Vec<NetzEintrag>,
    pub verworfen: usize,
    mcp: BTreeMap<String, McpZaehler>,
    mcp_abgewiesen: u64,
    /// Beschriftungen der Felder, die bei Inkognito ersetzt wurden (nie Werte)
    inkognito_felder: Vec<String>,
}

impl Protokoll {
    pub fn neu(start_ms: u64) -> Protokoll {
        Protokoll {
            start_ms,
            netz: Vec::new(),
            verworfen: 0,
            mcp: BTreeMap::new(),
            mcp_abgewiesen: 0,
            inkognito_felder: Vec::new(),
        }
    }

    pub fn netz_eintragen(&mut self, e: NetzEintrag) {
        if self.netz.len() >= EINTRAEGE_MAX {
            self.netz.remove(0);
            self.verworfen += 1;
        }
        self.netz.push(e);
    }

    /// Ein Werkzeugaufruf eines KI-Programms; `werkzeug == None`: abgewiesen (Schlüssel, MCP aus).
    pub fn mcp_eintragen(&mut self, werkzeug: Option<&str>, jetzt_ms: u64) {
        let Some(w) = werkzeug else {
            self.mcp_abgewiesen += 1;
            return;
        };
        let z = self.mcp.entry(w.to_string()).or_default();
        if z.anzahl == 0 {
            z.erstes_ms = jetzt_ms;
        }
        z.anzahl += 1;
        z.letztes_ms = jetzt_ms;
    }

    /// Der Aufruf lief mit Inkognito; `felder`: Beschriftungen der ersetzten Felder.
    pub fn mcp_inkognito(&mut self, werkzeug: &str, felder: &[String]) {
        self.mcp.entry(werkzeug.to_string()).or_default().inkognito += 1;
        for f in felder {
            if !self.inkognito_felder.contains(f) {
                self.inkognito_felder.push(f.clone());
            }
        }
    }

    pub fn mcp_inkognito_anzahl(&self) -> u64 {
        self.mcp.values().map(|z| z.inkognito).sum()
    }

    pub fn mcp_anzahl(&self) -> u64 {
        self.mcp.values().map(|z| z.anzahl).sum()
    }

    pub fn js(&self, ganz_offline: bool, mcp_an: bool) -> ProtokollJs {
        ProtokollJs {
            start_ms: self.start_ms,
            netz: self
                .netz
                .iter()
                .map(|e| NetzEintragJs {
                    beginn_ms: e.beginn_ms,
                    dauer_ms: e.dauer_ms,
                    zweck: e.zweck.text(),
                    ausloeser: e.ausloeser.text(),
                    hosts: e.hosts(),
                    abrufe: e.abrufe.clone().unwrap_or_default(),
                    gemeldet: e.abrufe.is_some(),
                    ok: e.fehler.is_none(),
                    fehler: e.fehler.clone(),
                    ergebnis: e.ergebnis(),
                })
                .collect(),
            verworfen: self.verworfen,
            mcp: self
                .mcp
                .iter()
                .map(|(w, z)| McpZeileJs {
                    werkzeug: w.clone(),
                    schreibt: mcp_schreibt(w),
                    anzahl: z.anzahl,
                    inkognito: z.inkognito,
                    erstes_ms: z.erstes_ms,
                    letztes_ms: z.letztes_ms,
                })
                .collect(),
            mcp_abgewiesen: self.mcp_abgewiesen,
            inkognito_felder: self.inkognito_felder.clone(),
            ganz_offline,
            mcp_an,
        }
    }
}

/// Werkzeuge, die etwas ändern (alle anderen lesen nur).
fn mcp_schreibt(werkzeug: &str) -> bool {
    [
        "setzen",
        "ersetzen",
        "schreiben",
        "speichern",
        "anlegen",
        "oeffnen",
        "sichern",
    ]
    .iter()
    .any(|w| werkzeug.ends_with(w))
}

#[derive(Serialize, Debug, Clone)]
pub struct NetzEintragJs {
    pub beginn_ms: u64,
    pub dauer_ms: u64,
    pub zweck: String,
    pub ausloeser: String,
    pub hosts: Vec<String>,
    pub abrufe: Vec<Abruf>,
    /// Der Holer hat seine Verbindungen gemeldet (sonst sind `hosts` die erlaubten).
    pub gemeldet: bool,
    pub ok: bool,
    pub fehler: Option<String>,
    pub ergebnis: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct McpZeileJs {
    pub werkzeug: String,
    pub schreibt: bool,
    pub anzahl: u64,
    pub inkognito: u64,
    pub erstes_ms: u64,
    pub letztes_ms: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ProtokollJs {
    pub start_ms: u64,
    pub netz: Vec<NetzEintragJs>,
    pub verworfen: usize,
    pub mcp: Vec<McpZeileJs>,
    pub mcp_abgewiesen: u64,
    pub inkognito_felder: Vec<String>,
    pub ganz_offline: bool,
    pub mcp_an: bool,
}

pub fn jetzt_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

static PROTOKOLL: LazyLock<Mutex<Protokoll>> =
    LazyLock::new(|| Mutex::new(Protokoll::neu(jetzt_ms())));

/// Wird nach jeder Änderung gerufen (die App meldet es der Oberfläche).
static MELDER: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Beim Start der App: Startzeit festhalten und den Melder setzen.
pub fn beginnen(melder: impl Fn() + Send + Sync + 'static) {
    LazyLock::force(&PROTOKOLL);
    let _ = MELDER.set(Box::new(melder));
}

fn aendern(f: impl FnOnce(&mut Protokoll)) {
    f(&mut PROTOKOLL.lock().unwrap_or_else(|e| e.into_inner()));
    if let Some(m) = MELDER.get() {
        m();
    }
}

/// Ein Lauf des Holers ist zu Ende (gelungen oder nicht).
pub fn holer_eintragen(
    beginn: SystemTime,
    dauer: Duration,
    argumente: &[String],
    ausloeser: Ausloeser,
    abrufe: Option<Vec<Abruf>>,
    fehler: Option<String>,
) {
    let e = NetzEintrag {
        beginn_ms: beginn
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64),
        dauer_ms: dauer.as_millis() as u64,
        zweck: Zweck::aus_argumenten(argumente),
        ausloeser,
        abrufe,
        fehler,
    };
    aendern(|p| p.netz_eintragen(e));
}

/// Eine feste Adresse wurde dem Standardbrowser übergeben (oder das scheiterte).
pub fn browser_eintragen(url: &str, fehler: Option<String>) {
    let e = NetzEintrag {
        beginn_ms: jetzt_ms(),
        dauer_ms: 0,
        zweck: Zweck::Browser(url.to_string()),
        ausloeser: Ausloeser::Link,
        abrufe: None,
        fehler,
    };
    aendern(|p| p.netz_eintragen(e));
}

/// Ein Werkzeugaufruf über MCP; `None`: abgewiesen.
pub fn mcp_eintragen(werkzeug: Option<&str>) {
    let jetzt = jetzt_ms();
    aendern(|p| p.mcp_eintragen(werkzeug, jetzt));
}

/// Ein Aufruf lief mit Inkognito (Personendaten nur als Ersatztext).
pub fn mcp_inkognito(werkzeug: &str, felder: &[String]) {
    aendern(|p| p.mcp_inkognito(werkzeug, felder));
}

pub fn stand() -> Protokoll {
    PROTOKOLL.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// --- Zeit in Ortszeit -------------------------------------------------------------------------

/// Unix-Millisekunden und Abstand zu UTC in Minuten (wie `-Date.getTimezoneOffset()`) →
/// Datum nach Oberflächensprache und `HH:MM:SS`.
pub fn ortszeit(ms: u64, versatz_min: i32) -> (String, String) {
    let s = (ms / 1000) as i64 + i64::from(versatz_min) * 60;
    let iso = crate::pakete::datum(s);
    let (jahr, rest) = iso.split_once('-').unwrap_or((&iso, ""));
    let (monat, tag) = rest.split_once('-').unwrap_or((rest, ""));
    let datum = tr!("{tag}.{monat}.{jahr}", tag, monat, jahr);
    let t = s.rem_euclid(86_400);
    let uhr = format!("{:02}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60);
    (datum, uhr)
}

// --- Das Blatt --------------------------------------------------------------------------------

/// Was das Blatt außer dem Protokoll braucht.
pub struct BlattAngaben {
    pub tftn_version: String,
    pub typst_version: String,
    pub system: String,
    pub ganz_offline: bool,
    pub mcp_an: bool,
    pub jetzt_ms: u64,
    pub versatz_min: i32,
    pub sprache: String,
}

/// Die Daten für [`BLATT_TYP`], alle Texte schon übersetzt.
pub fn blatt_daten(p: &Protokoll, a: &BlattAngaben) -> Value {
    let zeit = |ms| ortszeit(ms, a.versatz_min);
    let (start_datum, start_uhr) = zeit(p.start_ms);
    let (jetzt_datum, jetzt_uhr) = zeit(a.jetzt_ms);
    let an_aus = |b: bool| {
        if b {
            tr!("eingeschaltet")
        } else {
            tr!("ausgeschaltet")
        }
    };
    let angaben = vec![
        json!([
            tr!("Programm"),
            format!("TFTN {} (typst {})", a.tftn_version, a.typst_version)
        ]),
        json!([tr!("System"), a.system]),
        json!([
            tr!("Sitzung"),
            tr!(
                "seit {datum}, {uhr} – Blatt erstellt {datum2}, {uhr2}",
                datum = start_datum,
                uhr = start_uhr,
                datum2 = jetzt_datum,
                uhr2 = jetzt_uhr
            )
        ]),
        json!([tr!("„Ganz offline“"), an_aus(a.ganz_offline)]),
        json!([tr!("MCP (KI-Programme)"), an_aus(a.mcp_an)]),
    ];
    let lokal = vec![
        tr!(
            "Texte, Vorschau, Rechtschreibprüfung, Literaturverwaltung und Formulare arbeiten nur mit Dateien auf diesem Rechner. Es gibt kein Konto, keine Cloud-Speicherung und keine Nutzungsstatistik."
        ),
        tr!(
            "Der Export (PDF, PDF/A, Archivpaket, Serie) läuft mit der mitgelieferten typst-CLI; vorher prüft TFTN, dass alle Pakete lokal vorliegen, sonst bricht es ab. Word-Export und -Import laufen mit pandoc auf diesem Rechner, beim Export im abgeschotteten Modus (--sandbox)."
        ),
        tr!(
            "Die Oberfläche lädt nichts von außen (Content-Security-Policy nur für eigene Inhalte). Schriften und Wörterbücher liegen auf dem Rechner. Webseiten öffnet TFTN nur auf Klick auf einen Link in „Über TFTN“, und zwar im Standardbrowser."
        ),
        tr!(
            "Die App selbst enthält keinen Netzwerk-Client; das prüft jeder Build automatisch. Ins Netz geht nur das Hilfsprogramm tftn-holer, und nur auf Klick."
        ),
    ];
    let wege = vec![
        json!([
            tr!("Pakete und Paketverzeichnis holen"),
            "packages.typst.org",
            tr!("Klick auf „Hinzufügen“ oder „Verzeichnis holen“"),
            tr!("Name und Version des Pakets; keine Dokumentinhalte")
        ]),
        json!([
            tr!("Nach Updates suchen"),
            "api.github.com",
            tr!("Klick in den Einstellungen"),
            tr!("nur die Anfrage nach der neuesten Version")
        ]),
        json!([
            tr!("Literatur aus DOI"),
            "api.crossref.org, api.datacite.org",
            tr!("Klick nach Eingabe einer DOI"),
            tr!("die eingegebene DOI")
        ]),
        json!([
            tr!("Link in „Über TFTN“"),
            crate::ueber::GEGENSTELLEN.join(", "),
            tr!("Klick auf den Link"),
            tr!("nur der Seitenaufruf durch den Standardbrowser")
        ]),
        json!([
            tr!("KI-Assistent über MCP"),
            tr!("127.0.0.1 (nur dieser Rechner)"),
            tr!("Benutzer schaltet MCP ein; das KI-Programm ruft Werkzeuge auf"),
            tr!(
                "Text des offenen Dokuments an das KI-Programm; ob es ihn ins Netz sendet, hängt von diesem Programm ab"
            )
        ]),
    ];
    let wege_hinweis = tr!(
        "Jede Anfrage des Holers geht verschlüsselt (HTTPS) an genau diese festen Adressen, ohne Weiterleitung und ohne Proxy. Mit jeder Anfrage sieht die Gegenstelle die IP-Adresse und die Programmversion. „Ganz offline“ sperrt den Holer und die Links vollständig."
    );
    let mut sitzung: Vec<Value> = p
        .netz
        .iter()
        .take(BLATT_ZEILEN_MAX)
        .map(|e| {
            // Das Datum nur, wenn es nicht der Tag des Starts ist (der steht oben).
            let (datum, uhr) = zeit(e.beginn_ms);
            json!([
                if datum == start_datum {
                    uhr
                } else {
                    format!("{datum} {uhr}")
                },
                e.zweck.text(),
                e.hosts().join(", "),
                e.ausloeser.text(),
                e.ergebnis()
            ])
        })
        .collect();
    let rest = p.netz.len().saturating_sub(BLATT_ZEILEN_MAX) + p.verworfen;
    if rest > 0 {
        sitzung.push(json!([
            "…",
            tr!(
                "und {anzahl} weitere (siehe Netzprotokoll in der App)",
                anzahl = rest
            ),
            "",
            "",
            ""
        ]));
    }
    let mcp = if p.mcp_anzahl() == 0 && p.mcp_abgewiesen == 0 {
        tr!("Kein Zugriff eines KI-Programms über MCP in dieser Sitzung.")
    } else {
        let mut t = tr!(
            "Zugriffe von KI-Programmen über MCP in dieser Sitzung: {anzahl}",
            anzahl = p.mcp_anzahl()
        );
        let namen: Vec<String> = p
            .mcp
            .iter()
            .map(|(w, z)| format!("{w} ×{}", z.anzahl))
            .collect();
        if !namen.is_empty() {
            t.push_str(&format!(" ({})", namen.join(", ")));
        }
        if p.mcp_abgewiesen > 0 {
            t.push_str(&format!(
                ". {}",
                tr!("Abgewiesen: {anzahl}", anzahl = p.mcp_abgewiesen)
            ));
        }
        t.push('.');
        let inkognito = p.mcp_inkognito_anzahl();
        if inkognito > 0 {
            t.push(' ');
            t.push_str(&tr!(
                "Pseudonymisiert (Inkognito): {anzahl} von {gesamt} Zugriffen; ersetzt wurden die Felder {felder}. Seitenbilder setzt TFTN dafür eigens aus den Ersatztexten.",
                anzahl = inkognito,
                gesamt = p.mcp_anzahl(),
                felder = p.inkognito_felder.join(", ")
            ));
        }
        t
    };
    json!({
        "sprache": a.sprache,
        "papier": if a.sprache == "en" { "us-letter" } else { "a4" },
        "titel": tr!("TFTN – Datenverarbeitung und Netzzugriffe"),
        "untertitel": tr!("Angaben für das Verarbeitungsverzeichnis und die Datenschutzbeauftragte bzw. den Datenschutzbeauftragten"),
        "angaben": angaben,
        "lokal_titel": tr!("1. Was TFTN auf diesem Rechner macht"),
        "lokal": lokal,
        "wege_titel": tr!("2. Welche Netzzugriffe es überhaupt geben kann"),
        "wege_kopf": [tr!("Zweck"), tr!("Gegenstelle"), tr!("Auslöser"), tr!("Was übertragen wird")],
        "wege": wege,
        "wege_hinweis": wege_hinweis,
        "sitzung_titel": tr!("3. Netzzugriffe in dieser Sitzung"),
        "sitzung_kopf": [tr!("Zeit"), tr!("Zweck"), tr!("Gegenstelle"), tr!("Auslöser"), tr!("Ergebnis")],
        "sitzung": sitzung,
        "sitzung_leer": tr!("Keine Netzzugriffe seit dem Start von TFTN am {datum} um {uhr}.", datum = start_datum, uhr = start_uhr),
        "mcp": mcp,
        "fuss": tr!("Erstellt von TFTN {version} am {datum} um {uhr}. Das Protokoll umfasst nur die laufende Sitzung und wird nicht gespeichert.", version = a.tftn_version, datum = jetzt_datum, uhr = jetzt_uhr),
    })
}

/// Vorlage des Blatts; liest `daten.json` aus demselben Ordner. Texte kommen nur als Zeichenketten
/// hinein, Typst wertet in ihnen nichts aus.
pub const BLATT_TYP: &str = r#"#let d = json("daten.json")
#set document(title: d.titel, author: "TFTN")
#set page(
  paper: d.papier,
  margin: (x: 1.9cm, top: 1.7cm, bottom: 1.7cm),
  footer: text(7.5pt, fill: luma(100), d.fuss),
)
#set text(font: ("Source Sans 3", "Libertinus Serif"), size: 9.2pt, lang: d.sprache)
#set par(leading: 0.55em, spacing: 0.75em)
#show heading: set text(size: 10.5pt, weight: "bold")
#show heading: set block(above: 1.3em, below: 0.6em)
#set table(
  stroke: (x, y) => (bottom: 0.4pt + luma(190), top: if y == 0 { 0.8pt + luma(60) } else { none }),
  inset: (x: 4pt, y: 4pt),
)
#show table.cell.where(y: 0): set text(weight: "bold")

#text(15pt, weight: "bold", d.titel) \
#text(fill: luma(90), d.untertitel)
#v(0.6em)
#grid(
  columns: (auto, 1fr),
  column-gutter: 1.2em,
  row-gutter: 0.5em,
  ..d.angaben.map(((k, v)) => (text(fill: luma(90), k), v)).flatten(),
)

= #d.lokal_titel
#list(spacing: 0.6em, ..d.lokal)

= #d.wege_titel
#table(
  columns: (1.15fr, 1.05fr, 1.15fr, 1.65fr),
  table.header(..d.wege_kopf),
  ..d.wege.flatten(),
)
#text(8.5pt, d.wege_hinweis)

= #d.sitzung_titel
#if d.sitzung.len() == 0 {
  block(
    width: 100%,
    inset: 8pt,
    radius: 3pt,
    stroke: 0.6pt + luma(160),
    text(weight: "bold", d.sitzung_leer),
  )
} else {
  set text(8.5pt)
  table(
    columns: (auto, 1.7fr, 1fr, 1fr, 1fr),
    table.header(..d.sitzung_kopf),
    ..d.sitzung.flatten(),
  )
}
#d.mcp
"#;

/// Schreibt Vorlage und Daten nach `ordner` und setzt `<ordner>/<name>.pdf` mit der CLI. Schriften
/// nur aus `schriften` und den eingebauten der CLI.
pub fn blatt_setzen(
    ordner: &Path,
    name: &str,
    daten: &Value,
    cli: &tftn_export::TypstCli,
    schriften: &[PathBuf],
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(ordner).map_err(|e| e.to_string())?;
    std::fs::write(ordner.join("blatt.typ"), BLATT_TYP).map_err(|e| e.to_string())?;
    std::fs::write(
        ordner.join("daten.json"),
        serde_json::to_vec_pretty(daten).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let pdf = ordner.join(format!("{name}.pdf"));
    let mut befehl = std::process::Command::new(&cli.pfad);
    befehl
        .arg("compile")
        .arg("--root")
        .arg(ordner)
        .arg("--ignore-system-fonts");
    for s in schriften {
        befehl.arg("--font-path").arg(s);
    }
    befehl
        .arg(ordner.join("blatt.typ"))
        .arg(&pdf)
        .stdin(std::process::Stdio::null());
    for (n, _) in std::env::vars_os() {
        if n.to_string_lossy().starts_with("TYPST_") {
            befehl.env_remove(n);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        befehl.creation_flags(0x0800_0000);
    }
    let aus = befehl.output().map_err(|e| e.to_string())?;
    if !aus.status.success() {
        return Err(tr!(
            "typst-CLI meldet {status}:\n{ausgabe}",
            status = aus.status,
            ausgabe = String::from_utf8_lossy(&aus.stderr)
        ));
    }
    Ok(pdf)
}

/// Öffnet eine Datei mit dem Standardprogramm des Systems (für das Blatt: PDF-Betrachter).
pub fn mit_system_oeffnen(pfad: &Path) -> Result<(), String> {
    system_oeffnen(pfad.as_os_str())
}

/// Wie [`mit_system_oeffnen`], aber für eine Adresse; nur [`crate::ueber`] ruft das, mit festen
/// Adressen.
pub fn adresse_oeffnen(url: &str) -> Result<(), String> {
    system_oeffnen(std::ffi::OsStr::new(url))
}

fn system_oeffnen(ziel: &std::ffi::OsStr) -> Result<(), String> {
    let mut befehl = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        std::process::Command::new("explorer")
    } else {
        std::process::Command::new("xdg-open")
    };
    befehl
        .arg(ziel)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eintrag(
        argumente: &[&str],
        abrufe: Option<Vec<Abruf>>,
        fehler: Option<&str>,
    ) -> NetzEintrag {
        let a: Vec<String> = argumente.iter().map(|s| s.to_string()).collect();
        NetzEintrag {
            beginn_ms: 1_763_389_800_000,
            dauer_ms: 420,
            zweck: Zweck::aus_argumenten(&a),
            ausloeser: Ausloeser::PaketHinzufuegen,
            abrufe,
            fehler: fehler.map(str::to_string),
        }
    }

    fn abruf(url: &str, status: u16) -> Abruf {
        Abruf {
            url: url.into(),
            status: Some(status),
            bytes: 10,
            fehler: None,
        }
    }

    #[test]
    fn zweck_aus_den_argumenten_des_holers() {
        let z = |a: &[&str]| {
            Zweck::aus_argumenten(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(z(&["index", "--zwischenspeicher", "/x"]), Zweck::Index);
        assert_eq!(
            z(&[
                "paket",
                "--zwischenspeicher",
                "/x",
                "--erwartet",
                "a=b",
                "@preview/cetz:0.3.0"
            ]),
            Zweck::Paket("@preview/cetz:0.3.0".into())
        );
        assert_eq!(z(&["versionen"]), Zweck::Versionen);
        assert_eq!(z(&["doi", "10.1/x"]), Zweck::Doi("10.1/x".into()));
        assert_eq!(z(&[]), Zweck::Anderes(String::new()));
        assert_eq!(
            Zweck::Doi("x".into()).gegenstellen(),
            ["api.crossref.org", "api.datacite.org"]
        );
    }

    #[test]
    fn host_aus_adresse() {
        assert_eq!(
            host("https://packages.typst.org/preview/index.json"),
            "packages.typst.org"
        );
        assert_eq!(host("http://127.0.0.1:4711/x?y"), "127.0.0.1:4711");
        assert_eq!(host("https://api.github.com"), "api.github.com");
    }

    #[test]
    fn gegenstellen_gemeldet_oder_erlaubt() {
        let gemeldet = eintrag(
            &["paket", "@preview/a:1.0.0"],
            Some(vec![
                abruf("https://packages.typst.org/preview/a-1.0.0.tar.gz", 200),
                abruf("https://packages.typst.org/preview/b-1.0.0.tar.gz", 200),
            ]),
            None,
        );
        assert_eq!(gemeldet.hosts(), ["packages.typst.org"]);
        assert!(gemeldet.ergebnis().contains('2'));
        let still = eintrag(&["doi", "10.1/x"], None, Some("Frist"));
        assert_eq!(still.hosts(), ["api.crossref.org", "api.datacite.org"]);
        let ohne = eintrag(&["versionen"], Some(vec![]), Some("kein Netz"));
        assert!(ohne.hosts().is_empty());
    }

    #[test]
    fn link_im_browser_mit_rechner_und_ergebnis() {
        let mut p = Protokoll::neu(0);
        let mut e = eintrag(&["versionen"], None, None);
        e.zweck = Zweck::Browser("https://github.com/DocAtPrompt/tftn/issues".into());
        e.ausloeser = Ausloeser::Link;
        assert_eq!(e.hosts(), ["github.com"]);
        assert!(e.ergebnis().contains("rowser"), "{}", e.ergebnis());
        assert!(e.zweck.text().contains("/issues"));
        p.netz_eintragen(e.clone());
        e.fehler = Some("kein Browser".into());
        assert_eq!(e.ergebnis(), "kein Browser");
        p.netz_eintragen(e);
        let js = p.js(false, false);
        assert_eq!(js.netz.len(), 2);
        assert!(js.netz[0].ok && !js.netz[1].ok);
        let d = blatt_daten(&p, &angaben());
        assert_eq!(d["sitzung"][0][2], "github.com");
    }

    #[test]
    fn begrenzt_und_mcp_gezaehlt() {
        let mut p = Protokoll::neu(0);
        for _ in 0..EINTRAEGE_MAX + 3 {
            p.netz_eintragen(eintrag(&["versionen"], Some(vec![]), None));
        }
        assert_eq!(p.netz.len(), EINTRAEGE_MAX);
        assert_eq!(p.verworfen, 3);
        p.mcp_eintragen(Some("quelltext_lesen"), 5);
        p.mcp_eintragen(Some("quelltext_lesen"), 9);
        p.mcp_eintragen(Some("text_ersetzen"), 7);
        p.mcp_eintragen(None, 8);
        let js = p.js(true, false);
        assert_eq!(js.mcp.len(), 2);
        assert_eq!(js.mcp[0].werkzeug, "quelltext_lesen");
        assert_eq!(
            (js.mcp[0].anzahl, js.mcp[0].erstes_ms, js.mcp[0].letztes_ms),
            (2, 5, 9)
        );
        assert!(!js.mcp[0].schreibt);
        assert!(js.mcp[1].schreibt);
        assert_eq!(js.mcp_abgewiesen, 1);
        assert!(js.ganz_offline && !js.mcp_an);
    }

    #[test]
    fn ortszeit_mit_versatz() {
        // 2025-11-17 14:30:00 UTC
        let ms = 1_763_389_800_000;
        let (d, u) = ortszeit(ms, 60);
        assert_eq!(u, "15:30:00");
        assert!(
            d.contains("17") && d.contains("11") && d.contains("2025"),
            "{d}"
        );
        assert_eq!(ortszeit(ms, -15 * 60).1, "23:30:00");
        assert_eq!(ortszeit(0, -60).1, "23:00:00");
    }

    fn angaben() -> BlattAngaben {
        BlattAngaben {
            tftn_version: "0.6.0".into(),
            typst_version: "0.15.1".into(),
            system: "macOS (aarch64)".into(),
            ganz_offline: false,
            mcp_an: false,
            jetzt_ms: 1_763_393_400_000,
            versatz_min: 60,
            sprache: "de".into(),
        }
    }

    #[test]
    fn blatt_ohne_zugriffe_sagt_es() {
        let p = Protokoll::neu(1_763_389_800_000);
        let d = blatt_daten(&p, &angaben());
        assert_eq!(d["sitzung"], json!([]));
        assert!(
            d["sitzung_leer"].as_str().unwrap().contains("15:30:00"),
            "{d}"
        );
        let wege = d["wege"].as_array().unwrap();
        assert_eq!(wege.len(), 5);
        assert!(wege.iter().any(|w| w[1] == "github.com, ko-fi.com"), "{d}");
        assert_eq!(d["papier"], "a4");
    }

    #[test]
    fn blatt_mit_zugriffen_und_rest() {
        let mut p = Protokoll::neu(0);
        for _ in 0..BLATT_ZEILEN_MAX + 2 {
            p.netz_eintragen(eintrag(
                &["index"],
                Some(vec![abruf(
                    "https://packages.typst.org/preview/index.json",
                    200,
                )]),
                None,
            ));
        }
        p.mcp_eintragen(Some("quelltext_lesen"), 1);
        let d = blatt_daten(&p, &angaben());
        let zeilen = d["sitzung"].as_array().unwrap();
        assert_eq!(zeilen.len(), BLATT_ZEILEN_MAX + 1);
        assert_eq!(zeilen[0][2], "packages.typst.org");
        assert!(zeilen.last().unwrap()[1].as_str().unwrap().contains('2'));
        assert!(
            d["mcp"].as_str().unwrap().contains("quelltext_lesen ×1"),
            "{d}"
        );
        assert!(!d["mcp"].as_str().unwrap().contains("Inkognito"), "{d}");
        p.mcp_eintragen(Some("seite_ansehen"), 2);
        p.mcp_inkognito("seite_ansehen", &["Name".into(), "Geburtsdatum".into()]);
        p.mcp_inkognito("seite_ansehen", &["Name".into()]);
        let d = blatt_daten(&p, &angaben());
        let mcp = d["mcp"].as_str().unwrap();
        assert!(
            mcp.contains("Inkognito): 2 von 2") && mcp.contains("Felder Name, Geburtsdatum."),
            "{mcp}"
        );
        assert_eq!(p.js(false, true).mcp[1].inkognito, 2);
    }

    /// Setzt das Blatt wirklich mit der typst-CLI (wie der Export; `cargo xtask ci` stellt sie).
    #[test]
    fn blatt_wird_ein_pdf_mit_einer_seite() {
        let cli = tftn_export::TypstCli::finden(tftn_manifest::anlegen::TYPST_VERSION)
            .expect("typst-CLI in der Projektversion nötig");
        let ordner =
            std::env::temp_dir().join(format!("tftn-netzblatt-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ordner);
        let schriften =
            vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("../types/tftn-leer/schriften")];
        let mut p = Protokoll::neu(1_763_389_800_000);
        let pdf_leer = blatt_setzen(
            &ordner,
            "leer",
            &blatt_daten(&p, &angaben()),
            &cli,
            &schriften,
        )
        .unwrap();
        for i in 0..BLATT_ZEILEN_MAX + 5 {
            p.netz_eintragen(eintrag(
                &["doi", &format!("10.1000/[#\"*{i}")],
                Some(vec![abruf(
                    "https://api.crossref.org/works/x/transform/application/x-bibtex",
                    404,
                )]),
                Some("nicht gefunden"),
            ));
        }
        p.mcp_eintragen(Some("text_ersetzen"), 3);
        let pdf_voll = blatt_setzen(
            &ordner,
            "voll",
            &blatt_daten(&p, &angaben()),
            &cli,
            &schriften,
        )
        .unwrap();
        for pdf in [pdf_leer, pdf_voll] {
            let daten = std::fs::read(&pdf).unwrap();
            assert!(daten.starts_with(b"%PDF"), "{}", pdf.display());
            assert_eq!(seiten_zaehlen(&daten), 1, "{}", pdf.display());
        }
        let _ = std::fs::remove_dir_all(&ordner);
    }

    /// Seitenobjekte im PDF (`/Type /Page`, nicht `/Pages`).
    fn seiten_zaehlen(pdf: &[u8]) -> usize {
        let text = String::from_utf8_lossy(pdf);
        text.match_indices("/Type /Page")
            .chain(text.match_indices("/Type/Page"))
            .filter(|(i, m)| !text[i + m.len()..].starts_with('s'))
            .count()
    }
}
