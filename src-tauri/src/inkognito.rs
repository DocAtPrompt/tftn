//! Inkognito für KI-Assistenten (MCP): Personendaten gehen nur als Ersatztext hinaus.
//!
//! Der Kern (Regeln, Suchmuster, Platzhalter, Zurücksetzen) steht in `tftn_formular::inkognito`.
//! Hier wird er um jeden MCP-Aufruf gelegt, an einer einzigen Stelle (`ausfuehren`), damit kein
//! Werkzeug vergessen wird: Argumente bekommen die echten Werte zurück, jeder Text der Antwort wird
//! ersetzt, Bilder anderer Werkzeuge fallen weg. Das Seitenbild (`seite_ansehen`) setzt TFTN eigens
//! aus den ersetzten Quellen neu; scheitert das, gibt es kein Bild.
//!
//! Kann TFTN die Werte nicht sicher bestimmen (Datendatei kaputt), gibt es über MCP nichts heraus.
//!
//! Texte an den Assistenten bleiben deutsch und unübersetzt; was der Benutzer sieht, geht über `tr!`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tftn_formular::inkognito::{self as kern, Markierung, Regel, Tabelle};
use tftn_mcp::{Ergebnis, Inhalt};
use tftn_sprachen::tr;
use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, Source, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_layout::PagedDocument;

use crate::Zustand;

/// Eine Tabelle für die ganze Sitzung der App: Werte früher geöffneter Dokumente bleiben bekannt
/// und werden weiter ersetzt (etwa der Name in einem Ordner, den der Assistent nach ihnen benannt
/// hat), und vergebene Platzhalter behalten ihre Bedeutung. Lebt nur im Speicher.
#[derive(Default)]
struct Stand {
    tabelle: Tabelle,
    /// Regeln des offenen Dokuments (für den Satz des Seitenbilds)
    regeln: Vec<Regel>,
    /// Werte aller Dokumente seit dem Start, ohne Doppelte
    werte: Vec<kern::Wert>,
    /// Feldbeschriftungen und Anzahl der Werte des offenen Dokuments
    felder: Vec<String>,
    anzahl: usize,
}

static STAND: std::sync::LazyLock<Mutex<Stand>> =
    std::sync::LazyLock::new(|| Mutex::new(Stand::default()));

fn stand() -> std::sync::MutexGuard<'static, Stand> {
    STAND.lock().unwrap_or_else(|e| e.into_inner())
}

/// Endungen, deren Inhalt als Text ersetzt wird (beim Satz des Seitenbilds).
const TEXT: [&str; 12] = [
    "typ", "toml", "txt", "csv", "tsv", "json", "yaml", "yml", "bib", "md", "xml", "html",
];

// --- Regeln und Werte des Projekts ---------------------------------------------------------------

/// Markierungen aus `tftn.toml` des Projekts.
pub fn markierungen(wurzel: &Path) -> kern::Markierungen {
    std::fs::read_to_string(wurzel.join("tftn.toml"))
        .map(|t| kern::markierungen_lesen(&t))
        .unwrap_or_default()
}

fn regeln(s: &crate::Sitzung) -> Vec<Regel> {
    match s.formular.schema() {
        Some(f) => kern::regeln(f, &markierungen(&s.projekt.wurzel)),
        None => Vec::new(),
    }
}

/// TOML-Dateien im Projekt (ohne `tftn.toml`, versteckte, `_`-Ordner und `packages/`), in denen
/// Datensätze einer Serie liegen können.
fn toml_dateien(wurzel: &Path) -> Vec<PathBuf> {
    fn sammeln(wurzel: &Path, ordner: &Path, tiefe: usize, aus: &mut Vec<PathBuf>) {
        let Ok(eintraege) = std::fs::read_dir(ordner) else {
            return;
        };
        for e in eintraege.filter_map(Result::ok) {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with(['.', '_']) || (ordner == wurzel && name == "packages") {
                continue;
            }
            let p = e.path();
            match e.file_type() {
                Ok(a) if a.is_dir() && tiefe > 1 => sammeln(wurzel, &p, tiefe - 1, aus),
                Ok(a)
                    if a.is_file()
                        && p.extension()
                            .is_some_and(|x| x.eq_ignore_ascii_case("toml"))
                        && !(ordner == wurzel && name == "tftn.toml") =>
                {
                    aus.push(p)
                }
                _ => {}
            }
        }
    }
    let mut aus = Vec::new();
    sammeln(wurzel, wurzel, 8, &mut aus);
    aus.sort();
    aus.truncate(1000);
    aus
}

/// Datensätze aus der Datendatei und allen TOML-Dateien im Projekt. Eine Datei, die sich nicht
/// lesen lässt, ist ein Fehler (Name in `Err`): Dann sind die Werte nicht sicher bekannt.
fn datensaetze(wurzel: &Path, datendatei: &Path) -> Result<Vec<Value>, PathBuf> {
    let mut dateien = vec![datendatei.to_path_buf()];
    for p in toml_dateien(wurzel) {
        if !dateien.contains(&p) {
            dateien.push(p);
        }
    }
    let mut aus = Vec::new();
    for p in dateien {
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(p),
        };
        aus.extend(kern::datensaetze(&text).map_err(|_| p.clone())?);
    }
    Ok(aus)
}

/// Liest Regeln und Werte des offenen Projekts neu und nimmt die Werte zu denen früherer Dokumente.
/// `Ok(false)`: nichts zu ersetzen. `Err`: Die Werte sind nicht sicher bekannt (Text für den
/// Benutzer); dann gibt TFTN über MCP nichts heraus.
fn vorbereiten(app: &AppHandle) -> Result<bool, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let (regeln, werte) = match sitzung.as_ref() {
        None => (Vec::new(), Vec::new()),
        Some(s) => {
            if s.formular.unsicher() {
                return Err(tr!(
                    "Inkognito: Das Formular des Dokumenttyps fehlt oder ist fehlerhaft; TFTN kennt die Personendaten nicht sicher und gibt über MCP nichts heraus."
                ));
            }
            let regeln = regeln(s);
            let wurzel = &s.projekt.wurzel;
            let werte = if regeln.is_empty() {
                Vec::new()
            } else {
                let saetze = datensaetze(wurzel, s.formular.datei()).map_err(|p| {
                    tr!(
                        "Inkognito: {datei} lässt sich nicht lesen. Bis das behoben ist, gibt TFTN über MCP nichts heraus.",
                        datei = p.strip_prefix(wurzel).unwrap_or(&p).display()
                    )
                })?;
                kern::werte(&regeln, &saetze)
            };
            (regeln, werte)
        }
    };
    drop(sitzung);
    let mut st = stand();
    let mut felder: Vec<String> = Vec::new();
    for w in &werte {
        if !felder.contains(&w.titel) {
            felder.push(w.titel.clone());
        }
    }
    st.felder = felder;
    st.anzahl = werte.len();
    // Die Werte des offenen Dokuments zuerst, dann die früheren.
    let mut alle = werte;
    for w in std::mem::take(&mut st.werte) {
        if !alle.iter().any(|x| x.echt == w.echt && x.basis == w.basis) {
            alle.push(w);
        }
    }
    st.tabelle.regeln_merken(&regeln);
    st.tabelle.werte_setzen(&alle)?;
    st.werte = alle;
    st.regeln = regeln;
    Ok(st.tabelle.aktiv())
}

// --- Um jeden MCP-Aufruf -------------------------------------------------------------------------

/// Hinweis an den Assistenten (in `status`).
fn hinweis(beispiele: &[String]) -> String {
    format!(
        "Inkognito ist an: Personendaten stehen in allen Antworten als Ersatztexte in eckigen \
         Klammern da (hier etwa {}). Die Ersatztexte genau so stehen lassen oder im neuen Text \
         verwenden; TFTN setzt beim Schreiben die echten Werte ein. Selbst erfundene Platzhalter \
         lehnt TFTN ab. Typst-Meldungen kommen aus einem Lauf mit den Ersatztexten.",
        beispiele.join(", ")
    )
}

/// Antwort, wenn die Werte nicht sicher bekannt sind.
fn gesperrt(werkzeug: &str) -> Ergebnis {
    crate::netzprotokoll::mcp_inkognito(werkzeug, &[]);
    Ergebnis::fehler(
        "Inkognito: TFTN kennt die Personendaten dieses Dokuments gerade nicht sicher (eine \
         Datendatei oder das Formular ist fehlerhaft); bis der Benutzer das in TFTN behoben hat, \
         gibt TFTN über MCP nichts heraus.",
    )
}

/// Werkzeuge, die in der Fassung mit Platzhaltern ersetzen (`ersetzen`): `alt` und `neu` bleiben
/// dafür, wie der Assistent sie geschickt hat.
const ERSETZEN: [&str; 2] = ["text_ersetzen", "datei_ersetzen"];

/// Werkzeuge, deren Antwort die Werte schon ersetzt enthält (`werte_verbergen`).
const SCHON_ERSETZT: [&str; 2] = ["formular_lesen", "formular_setzen"];

/// Führt einen MCP-Aufruf aus; ist Inkognito aktiv, mit Ersatztexten statt Personendaten.
pub fn ausfuehren(
    app: &AppHandle,
    werkzeug: &str,
    argumente: &Value,
    innen: impl FnOnce(&Value) -> Result<Ergebnis, String>,
) -> Ergebnis {
    let Ok(aktiv) = vorbereiten(app) else {
        return gesperrt(werkzeug);
    };
    let mut args = argumente.clone();
    if aktiv {
        let felder = stand().tabelle.felder().to_vec();
        crate::netzprotokoll::mcp_inkognito(werkzeug, &felder);
        if let Err(e) = stand().tabelle.aufdecken_json(&mut args) {
            return filtern(Ergebnis::fehler(e), false, true);
        }
        if ERSETZEN.contains(&werkzeug) {
            for k in ["alt", "neu"] {
                if let Some(v) = argumente.get(k) {
                    args[k] = v.clone();
                }
            }
        }
    }
    let ergebnis = if aktiv && werkzeug == "seite_ansehen" {
        seite_ansehen(app, &args).unwrap_or_else(Ergebnis::fehler)
    } else {
        innen(&args).unwrap_or_else(Ergebnis::fehler)
    };
    // Nach einem Wechsel kommen die Werte des neuen Dokuments dazu.
    let aktiv = if matches!(werkzeug, "dokument_oeffnen" | "dokument_anlegen") {
        match vorbereiten(app) {
            Ok(neu) => aktiv || neu,
            Err(_) => return gesperrt(werkzeug),
        }
    } else {
        aktiv
    };
    if !aktiv {
        return ergebnis;
    }
    let mut ergebnis = filtern(
        ergebnis,
        werkzeug == "seite_ansehen",
        !SCHON_ERSETZT.contains(&werkzeug),
    );
    if werkzeug == "status" {
        let beispiele = stand().tabelle.beispiele(3);
        ergebnis.inhalt.push(Inhalt::Text {
            text: hinweis(&beispiele),
        });
    }
    ergebnis
}

/// Ist Inkognito gerade aktiv (es gibt Werte zu ersetzen)?
fn aktiv() -> bool {
    stand().tabelle.aktiv()
}

/// Für `formular_lesen`/`formular_setzen`: die Werte ersetzen, bevor sie zu JSON-Text werden
/// (sonst stünden Zeilenumbrüche als `\n` darin und passten zu keinem Muster).
pub fn werte_verbergen(werte: &mut Value) {
    let mut st = stand();
    if st.tabelle.aktiv() {
        st.tabelle.verbergen_json(werte);
    }
}

/// Für `text_ersetzen`/`datei_ersetzen`: ersetzt so, wie der Assistent es in der Fassung mit
/// Platzhaltern meint (bei „alle“ also nur die Stellen, die er als Platzhalter gesehen hat).
pub fn ersetzen(text: &str, alt: &str, neu: &str, alle: bool) -> Result<String, String> {
    let mut st = stand();
    if !st.tabelle.aktiv() {
        return tftn_mcp::ersetzen(text, alt, neu, alle);
    }
    st.tabelle
        .im_platzhaltertext(text, |p| tftn_mcp::ersetzen(p, alt, neu, alle))
}

/// Würde das Wort ersetzt? Dann keine Rechtschreibvorschläge (sie verrieten es).
pub fn betrifft(wort: &str) -> bool {
    let mut st = stand();
    st.tabelle.aktiv() && st.tabelle.betrifft(wort)
}

/// Stand des letzten Typst-Laufs für den Assistenten. Bei Inkognito stammen die Meldungen aus einem
/// eigenen Lauf mit den Ersatztexten: Meldungen des echten Laufs können Werte enthalten, auch
/// umgeformt (`repr`, Typst-Code des Assistenten), die kein Suchmuster trifft.
pub fn lauf(app: &AppHandle, lauf: Option<crate::mcp::LaufStand>) -> Option<crate::mcp::LaufStand> {
    let mut l = lauf?;
    if !aktiv() {
        return Some(l);
    }
    let (projekt, quelle) = {
        let zustand = app.state::<Zustand>();
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung.as_ref()?;
        (s.projekt.clone(), s.spiegel.quelle().clone())
    };
    let mut st = stand();
    let st = &mut *st;
    l.meldungen = match setzen(&projekt, &quelle, &mut st.tabelle, &st.regeln) {
        Ok(satz) => {
            let mut m = satz.meldungen;
            if !l.gelungen && satz.dokument.is_some() {
                m.push(
                    "Inkognito: Mit den Ersatztexten übersetzt das Dokument ohne Fehler; die \
                     Meldungen des echten Laufs zeigt TFTN nicht, weil sie Personendaten enthalten \
                     können."
                        .into(),
                );
            }
            m
        }
        Err(e) => vec![e],
    };
    Some(l)
}

/// Ersetzt jeden Text der Antwort; Bilder bleiben nur im eigens gesetzten Seitenbild.
/// `schuetzen`: siehe `Tabelle::verbergen_mit` (aus, wenn die Antwort schon ersetzt ist).
fn filtern(ergebnis: Ergebnis, bild_erlaubt: bool, schuetzen: bool) -> Ergebnis {
    let mut st = stand();
    let inhalt = ergebnis
        .inhalt
        .into_iter()
        .map(|i| match i {
            Inhalt::Text { text } => Inhalt::Text {
                text: st.tabelle.verbergen_mit(&text, schuetzen),
            },
            bild @ Inhalt::Image { .. } if bild_erlaubt => bild,
            Inhalt::Image { .. } => Inhalt::Text {
                text: "(Bild weggelassen: Inkognito ist an)".into(),
            },
        })
        .collect();
    Ergebnis {
        inhalt,
        fehler: ergebnis.fehler,
    }
}

// --- Seitenbild aus den ersetzten Quellen ---------------------------------------------------------

/// World für das Seitenbild: wie die der Vorschau, aber jede Textdatei des Projekts kommt ersetzt
/// an. Eingebundene Bilder und PDFs werden vermerkt.
struct Welt<'a> {
    innen: tftn_core::ProjektWorld,
    tabelle: Mutex<&'a mut Tabelle>,
    regeln: &'a [Regel],
    datum_als_text: bool,
    bilder: Mutex<BTreeSet<String>>,
    /// Eine TOML-Datei ließ sich nicht ersetzen: dann kein Bild.
    fehler: Mutex<Option<String>>,
}

impl Welt<'_> {
    /// Ersetzter Text einer Projektdatei; `None`, wenn sie kein Text ist.
    fn ersetzt(&self, id: FileId, roh: &[u8]) -> Option<String> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return None;
        }
        let pfad = id.vpath();
        let endung = pfad.extension().unwrap_or("").to_lowercase();
        if !TEXT.contains(&endung.as_str()) {
            let name = pfad.get_without_slash().to_string();
            self.bilder.lock().unwrap().insert(name);
            return None;
        }
        let text = String::from_utf8_lossy(roh);
        let mut tabelle = self.tabelle.lock().unwrap();
        Some(if endung == "toml" {
            match tabelle.toml_verbergen(&text, self.regeln, self.datum_als_text) {
                Ok(t) => t,
                Err(e) => {
                    *self.fehler.lock().unwrap() =
                        Some(format!("{}: {e}", pfad.get_without_slash()));
                    String::new()
                }
            }
        } else {
            tabelle.verbergen(&text)
        })
    }
}

impl World for Welt<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.innen.library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.innen.book()
    }

    fn main(&self) -> FileId {
        self.innen.main()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let quelle = self.innen.source(id)?;
        Ok(match self.ersetzt(id, quelle.text().as_bytes()) {
            Some(t) => Source::new(id, t),
            None => quelle,
        })
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let roh = self.innen.file(id)?;
        Ok(match self.ersetzt(id, &roh) {
            Some(t) => Bytes::from_string(t),
            None => roh,
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.innen.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.innen.today(offset)
    }
}

/// Ergebnis eines Laufs mit den Ersatztexten.
struct Satz {
    dokument: Option<PagedDocument>,
    /// „Fehler (Zeile 3, Spalte 5): …“, wie beim echten Lauf
    meldungen: Vec<String>,
    /// eingebundene Dateien, deren Inhalt TFTN nicht prüfen kann (Bilder, PDFs)
    bilder: BTreeSet<String>,
}

/// Setzt das Dokument aus den ersetzten Quellen. Zuerst stehen auch Datumswerte als Ersatztext da;
/// kann die Vorlage das nicht setzen, ein Ersatzdatum (`toml_verbergen`). `Err` nur, wenn sich eine
/// Datei nicht ersetzen lässt.
fn setzen(
    projekt: &tftn_core::Projekt,
    quelle: &Source,
    tabelle: &mut Tabelle,
    regeln: &[Regel],
) -> Result<Satz, String> {
    let mut letzter = None;
    for datum_als_text in [true, false] {
        let mut innen = tftn_core::ProjektWorld::neu(projekt).map_err(|e| e.to_string())?;
        innen.quelle_setzen(quelle.clone());
        let welt = Welt {
            innen,
            tabelle: Mutex::new(&mut *tabelle),
            regeln,
            datum_als_text,
            bilder: Mutex::new(BTreeSet::new()),
            fehler: Mutex::new(None),
        };
        let lauf = typst::compile::<PagedDocument>(&welt);
        if let Some(e) = welt.fehler.lock().unwrap().take() {
            return Err(format!(
                "Inkognito: Eine Datei ließ sich nicht mit Ersatztexten lesen ({e}); TFTN gibt dazu \
                 weder Bild noch Meldungen heraus."
            ));
        }
        let mut meldungen: Vec<String> = lauf.warnings.iter().map(|d| meldung(&welt, d)).collect();
        let dokument = match lauf.output {
            Ok(d) => Some(d),
            Err(fehler) => {
                meldungen.extend(fehler.iter().map(|d| meldung(&welt, d)));
                None
            }
        };
        let satz = Satz {
            dokument,
            meldungen,
            bilder: welt.bilder.into_inner().unwrap(),
        };
        if satz.dokument.is_some() {
            return Ok(satz);
        }
        letzter = Some(satz);
    }
    Ok(letzter.expect("zwei Versuche"))
}

/// Eine Meldung des Laufs mit Ersatztexten, wie `meldungen_aufbereiten` sie für MCP schreibt.
fn meldung(welt: &Welt, d: &typst::diag::SourceDiagnostic) -> String {
    use typst::WorldExt;
    let art = if d.severity == typst::diag::Severity::Error {
        "Fehler"
    } else {
        "Warnung"
    };
    let hinweise: Vec<&str> = d.hints.iter().map(|h| h.v.as_str()).collect();
    let text = tftn_core::deutsch::meldungstext(&d.message, &hinweise);
    let ort = d.span.id().and_then(|id| {
        let quelle = welt.source(id).ok()?;
        let start = welt.range(d.span)?.start;
        let (z, sp) = quelle.lines().byte_to_line_column(start)?;
        Some(if id == welt.main() {
            format!("Zeile {}, Spalte {}", z + 1, sp + 1)
        } else {
            format!("{}:{}:{}", id.vpath().get_without_slash(), z + 1, sp + 1)
        })
    });
    match ort {
        Some(o) => format!("{art} ({o}): {text}"),
        None => format!("{art}: {text}"),
    }
}

fn seite_ansehen(app: &AppHandle, a: &Value) -> Result<Ergebnis, String> {
    let seite = a
        .get("seite")
        .and_then(Value::as_u64)
        .ok_or("`seite` fehlt")? as usize;
    let ppi = a
        .get("ppi")
        .and_then(Value::as_f64)
        .unwrap_or(100.0)
        .clamp(30.0, 300.0);
    let (projekt, quelle) = {
        let zustand = app.state::<Zustand>();
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or("In TFTN ist gerade kein Dokument offen.")?;
        (s.projekt.clone(), s.spiegel.quelle().clone())
    };
    let mut st = stand();
    let st = &mut *st;
    let satz = setzen(&projekt, &quelle, &mut st.tabelle, &st.regeln)?;
    let (Some(dokument), bilder) = (satz.dokument, satz.bilder) else {
        return Err(format!(
            "Inkognito: Mit den Ersatztexten statt der echten Werte lässt sich das Dokument nicht \
             übersetzen; ohne Ersatz gibt TFTN kein Bild heraus.\n{}",
            satz.meldungen.join("\n")
        ));
    };
    let seiten = dokument.pages();
    let bild = seite
        .checked_sub(1)
        .and_then(|i| seiten.get(i))
        .ok_or_else(|| {
            format!(
                "Seite {seite} gibt es nicht; das Dokument hat (mit Ersatztexten gesetzt) {} Seite(n).",
                seiten.len()
            )
        })?;
    let png = tftn_core::vorschau::rastern(bild, ppi);
    let mut text = format!(
        "Seite {seite} von {}, für dich eigens aus den Ersatztexten gesetzt (inkognito): Zeilen \
         können anders umbrechen als beim Benutzer. Ersetzte Datumswerte zeigen den Ersatztext \
         oder, wo die Vorlage ein Datum braucht, den 1. Jänner (des Jahres, wenn der Ersatztext es \
         nennt, sonst 1900).",
        seiten.len()
    );
    if !bilder.is_empty() {
        text.push_str(&format!(
            " Eingebundene Dateien, deren Inhalt TFTN nicht prüfen kann: {}.",
            bilder.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(Ergebnis {
        inhalt: vec![
            Inhalt::Text { text },
            Inhalt::Image {
                data: crate::typen::base64(&png),
                mime_type: "image/png".into(),
            },
        ],
        fehler: false,
    })
}

// --- Für die Oberfläche --------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug, Default)]
pub struct InkognitoJs {
    /// Es gibt Felder, die inkognito sind.
    an: bool,
    /// Ihre Beschriftungen
    felder: Vec<String>,
    /// Anzahl der Werte, die ersetzt werden
    werte: usize,
    /// Grund, warum über MCP nichts hinausgeht (Datendatei kaputt)
    fehler: Option<String>,
}

/// Stand für Statusleiste und Fach „Dokument“.
#[tauri::command(async)]
pub fn inkognito_stand(app: AppHandle) -> InkognitoJs {
    let fehler = vorbereiten(&app).err();
    let felder: Vec<String> = {
        let zustand = app.state::<Zustand>();
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .map(|s| regeln(s).into_iter().map(|r| r.titel).collect())
            .unwrap_or_default()
    };
    let mut felder_eindeutig: Vec<String> = Vec::new();
    for f in felder {
        if !felder_eindeutig.contains(&f) {
            felder_eindeutig.push(f);
        }
    }
    InkognitoJs {
        an: !felder_eindeutig.is_empty(),
        felder: felder_eindeutig,
        werte: stand().anzahl,
        fehler,
    }
}

/// „Was die KI sieht“: Ersatztext → echter Wert, für jeden Wert und jede Schreibweise, die TFTN
/// schon vergeben hat (auch aus früher geöffneten Dokumenten). Bleibt auf dem Rechner.
#[tauri::command(async)]
pub fn inkognito_tabelle(app: AppHandle) -> Result<Vec<(String, String)>, String> {
    vorbereiten(&app)?;
    Ok(stand().tabelle.paare().to_vec())
}

/// Kreuz im Fach „Dokument“: `an` mit eigenem Ersatztext (leer: der der Vorlage). Weicht das
/// Ergebnis nicht von der Vorlage ab, verschwindet der Eintrag aus `tftn.toml`.
#[tauri::command(async)]
pub fn inkognito_setzen(
    app: AppHandle,
    pfad: String,
    an: bool,
    ersatz: Option<String>,
) -> Result<(), String> {
    {
        let zustand = app.state::<Zustand>();
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("Kein Dokument offen."))?;
        let sensibel = s
            .formular
            .schema()
            .map(|f| kern::feld_staende(f, &kern::Markierungen::new()))
            .and_then(|st| st.get(&pfad).map(|x| x.sensibel))
            .ok_or_else(|| tr!("Das Feld {pfad} gibt es nicht.", pfad))?;
        let eigen = ersatz.as_deref().map(str::trim).filter(|e| !e.is_empty());
        let m = match (an, eigen) {
            (true, Some(e)) => Some(Markierung::Text(e.to_string())),
            (true, None) if sensibel => None,
            (true, None) => Some(Markierung::An),
            (false, _) if sensibel => Some(Markierung::Aus),
            (false, _) => None,
        };
        let datei = s.projekt.wurzel.join("tftn.toml");
        let alt = std::fs::read_to_string(&datei).map_err(|e| e.to_string())?;
        let neu = kern::markierung_schreiben(&alt, &pfad, m.as_ref())?;
        if neu != alt {
            tftn_formular::atomar_schreiben(&datei, &neu).map_err(|e| e.to_string())?;
        }
    }
    let _ = app.emit("inkognito-geaendert", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_dateien_ohne_tftn_toml_versteckte_und_pakete() {
        let o = std::env::temp_dir().join(format!("tftn-inkognito-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        for d in ["serie", "_lokal", ".git", "packages/local/x"] {
            std::fs::create_dir_all(o.join(d)).unwrap();
        }
        for f in [
            "tftn.toml",
            "dokument.toml",
            "serie/november.toml",
            "serie/liste.csv",
            "_lokal/a.toml",
            ".git/b.toml",
            "packages/local/x/typst.toml",
        ] {
            std::fs::write(o.join(f), "x = 1\n").unwrap();
        }
        let namen: Vec<String> = toml_dateien(&o)
            .iter()
            .map(|p| {
                p.strip_prefix(&o)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(namen, ["dokument.toml", "serie/november.toml"]);

        // Kaputte Datei: Fehler mit ihrem Namen, damit nichts ungefiltert hinausgeht.
        std::fs::write(o.join("serie/kaputt.toml"), "x = \n").unwrap();
        let e = datensaetze(&o, &o.join("dokument.toml")).unwrap_err();
        assert!(e.ends_with("serie/kaputt.toml"), "{e:?}");
        std::fs::remove_file(o.join("serie/kaputt.toml")).unwrap();
        // Fehlende Datendatei ist kein Fehler.
        assert_eq!(datensaetze(&o, &o.join("fehlt.toml")).unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn bilder_fallen_weg_texte_werden_ersetzt() {
        let schema = include_str!("../../types/tftn-befund/tftn/formular.schema.json");
        let f = tftn_formular::formular_lesen(schema).unwrap();
        let r = kern::regeln(&f, &kern::Markierungen::new());
        let d = serde_json::json!({ "patient": { "name": "Erika Musterfrau" } });
        let alt = std::mem::take(&mut *stand());
        stand()
            .tabelle
            .werte_setzen(&kern::werte(&r, &[d]))
            .unwrap();
        let e = Ergebnis {
            inhalt: vec![
                Inhalt::Text {
                    text: "Erika Musterfrau".into(),
                },
                Inhalt::Image {
                    data: "AAAA".into(),
                    mime_type: "image/png".into(),
                },
            ],
            fehler: false,
        };
        let aus = filtern(e.clone(), false, true);
        assert!(matches!(&aus.inhalt[0], Inhalt::Text { text } if !text.contains("Musterfrau")));
        assert!(matches!(&aus.inhalt[1], Inhalt::Text { text } if text.contains("weggelassen")));
        let aus = filtern(e, true, true);
        assert!(matches!(&aus.inhalt[1], Inhalt::Image { .. }));
        // Ersetzen mit „alle“ nur an den Stellen, die der Assistent als Platzhalter sah.
        assert_eq!(
            ersetzen(
                "Amerika, Erika Musterfrau",
                "[Patient/in]",
                "[Patient/in] (w)",
                true
            )
            .unwrap(),
            "Amerika, Erika Musterfrau (w)"
        );
        assert!(betrifft("Musterfrau") && !betrifft("Amerika"));
        let mut w = serde_json::json!({ "anschrift": "Erika Musterfrau\nWien" });
        werte_verbergen(&mut w);
        assert!(!w.to_string().contains("Musterfrau"), "{w}");
        *stand() = alt;
    }

    /// Text aller Seiten (für Prüfungen ohne Bilderkennung).
    fn seitentext(d: &PagedDocument) -> String {
        fn rahmen(f: &typst::layout::Frame, aus: &mut String) {
            for (_, item) in f.items() {
                match item {
                    typst::layout::FrameItem::Group(g) => rahmen(&g.frame, aus),
                    typst::layout::FrameItem::Text(t) => {
                        aus.push_str(&t.text);
                        aus.push(' ');
                    }
                    _ => {}
                }
            }
        }
        let mut aus = String::new();
        for s in d.pages() {
            rahmen(&s.frame, &mut aus);
        }
        aus
    }

    #[test]
    fn seitenbild_eines_befunds_ohne_echte_werte() {
        let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../types");
        let o = std::env::temp_dir().join(format!("tftn-inkognito-bild-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        let q = crate::typen::quellen(&typen, &o.join("einstellungen"));
        let ziel = crate::typen::anlegen(
            &q,
            "tftn-befund",
            &o,
            "Befund Musterfrau",
            false,
            Some("de"),
        )
        .unwrap();
        tftn_core::schriftvorrat_setzen(crate::typen::schriftvorrat(&q));
        let daten = ziel.join("dokument.toml");
        let t = std::fs::read_to_string(&daten).unwrap();
        assert!(
            t.contains("Erika Musterfrau") && t.contains("1970-02-01"),
            "{t}"
        );
        std::fs::write(&daten, format!("{t}svnr = \"1234 010270\"\n")).unwrap();
        let text = std::fs::read_to_string(ziel.join("main.typ")).unwrap()
            + "\nFrau _Musterfrau_ (SVNr 1234010270) kam am 1. Februar 1970 zur Welt.\n";

        let projekt = tftn_core::Projekt::oeffnen(&ziel).unwrap();
        let id = tftn_core::ProjektWorld::neu(&projekt).unwrap().main();
        let quelle = Source::new(id, text);
        let schema = crate::formular::FormularSitzung::neu(&projekt);
        let r = kern::regeln(schema.schema().unwrap(), &kern::Markierungen::new());
        let mut tabelle = Tabelle::default();
        tabelle
            .werte_setzen(&kern::werte(&r, &datensaetze(&ziel, &daten).unwrap()))
            .unwrap();

        let satz = setzen(&projekt, &quelle, &mut tabelle, &r).unwrap();
        let (d, bilder) = (satz.dokument.unwrap(), satz.bilder);
        assert!(
            satz.meldungen.iter().all(|m| !m.starts_with("Fehler")),
            "{:?}",
            satz.meldungen
        );
        let gesehen = seitentext(&d);
        for verboten in ["Musterfrau", "Erika", "01.02.1970", "1234", "Februar"] {
            assert!(!gesehen.contains(verboten), "{verboten} im Bild: {gesehen}");
        }
        assert!(gesehen.contains("[Patientin]"), "{gesehen}");
        assert!(
            gesehen.contains("[geb. 1970]"),
            "Datum als Ersatztext: {gesehen}"
        );
        assert!(gesehen.contains("[SV-Nummer]"), "{gesehen}");
        assert!(bilder.is_empty(), "{bilder:?}");

        // Meldungen kommen aus dem Lauf mit Ersatztexten: `#panic(daten)` zeigt keine echten Werte.
        let mit_panik = Source::new(
            id,
            format!("{}\n#panic(toml(\"dokument.toml\"))\n", quelle.text()),
        );
        let satz = setzen(&projekt, &mit_panik, &mut tabelle, &r).unwrap();
        let m = satz.meldungen.join("\n");
        assert!(
            m.contains("[Patientin]") && !m.contains("Musterfrau") && !m.contains("month: 2"),
            "{m}"
        );

        // Gegenprobe: ohne Inkognito stehen die echten Werte im Bild.
        let mut w = tftn_core::ProjektWorld::neu(&projekt).unwrap();
        w.quelle_setzen(quelle);
        let echt = seitentext(&tftn_core::kompilieren(&w).dokument.unwrap());
        assert!(
            echt.contains("Musterfrau") && echt.contains("01.02.1970"),
            "{echt}"
        );
        let _ = std::fs::remove_dir_all(&o);
    }
}
