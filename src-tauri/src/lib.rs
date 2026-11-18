//! TFTN-App: Tauri-Hülle um den Kern (KONZEPT 3, 4.1; PLAN-M1 AP3).
//!
//! Der Editor ist der einzige Schreiber. Jede Transaktion kommt als `aendern` mit der
//! Revision, auf der sie beruht; Rust führt einen Spiegel (`tftn_core::spiegel`) und liefert
//! die Syntaxhervorhebung für den sichtbaren Bereich zurück. Bei jeder 50. Revision schickt der
//! Editor eine Prüfsumme des Volltexts mit; weicht sie ab, wird protokolliert und neu
//! synchronisiert.
//!
//! Vorschau (AP4): Jede neue Revision geht als Auftrag an einen Kompilier-Faden; liegen mehrere an,
//! gewinnt die neueste. Gelingt der Lauf, wird er der angezeigte Stand; sonst bleibt der letzte gute
//! stehen und der Editor bekommt die Meldungen. Seitenbilder holt die Oberfläche über das Schema
//! `tftn://` mit Seitennummer und Seiten-Hash, gerastert wird erst bei der Anfrage.
//!
//! Klicksprung (AP5): `klick` (Vorschau → Quelle) und `stelle_zeigen` (Quelle → Vorschau) greifen
//! auf dieselbe `Vorschau` zu wie der Kompilier-Faden; sie warten höchstens einen Lauf.
//!
//! Widgets (AP6): `widgets` nennt die Bausteine-Parameter am Cursor, `farbe_setzen` liefert einen
//! geprüften Patch gegen eine Revision. Angewendet wird er nur vom Editor (einziger Schreiber).
//! Die Bausteine kommen aus dem Helfer-Katalog (PLAN-M2 AP1): `katalog` listet sie, `baustein_text`
//! setzt ein Schnipsel mit Feldern ein.
//!
//! Literatur (AP7): Der Index der `.bib` wird beim Öffnen gebaut und bei jeder Änderung der Datei
//! (Zotero-Auto-Export) neu gebaut; `zitate_suchen` und `zitat_text` bedienen die Befehlspalette.
//!
//! Export (AP8): `exportieren` ruft die beigelegte typst-CLI (Tauri-Sidecar neben dem Programm) mit
//! einem Export-Profil auf. Exportiert wird die gespeicherte Datei; „Abnehmen“ setzt den Zeitstempel
//! in `tftn.toml` und baut die Vorschau-World mit dem neuen Datum neu.
//!
//! Fach „Dokument“ (PLAN-M2 AP7): `formular`, `formular_setzen`, `formular_eintrag`,
//! `formular_rueckgaengig`, `formular_wiederholen` und `projekt_info`; der Kern steht in
//! `formular.rs`. Geschrieben wird nur die Datendatei (`dokument.toml`), nie `main.typ`. Ändert sie
//! sich (auch von außen), meldet der Wächter `dokument-geaendert` mit dem neuen Stand.
//!
//! Fach „Pakete“ (PLAN-M2 AP9): `pakete_stand`, `pakete_index_holen`, `pakete_suchen`,
//! `paket_readme`, `paket_hinzufuegen`; der Kern steht in `pakete.rs`. Netz gibt es nur im
//! Hilfsprogramm `tftn-holer`, das die App auf Klick startet.

mod ablage;
mod bilder;
mod fassungen;
mod formel;
mod formular;
mod helfer;
mod import;
mod mcp;
mod netzprotokoll;
mod pakete;
mod projektdateien;
mod rechtschreibung;
mod schriftwahl;
mod skins;
mod sprachen;
mod typen;
mod word;
mod zuletzt;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tftn_core::Projekt;
use tftn_core::spiegel::{Aenderung, Spiegel, pruefsumme, zeilenenden_lf};
use tftn_core::vorschau::{Sprung, Vorschau, rastern};
use tftn_sprachen::tr;
use typst::World;
use typst::syntax::Source;
use typst_layout::PagedDocument;

struct Sitzung {
    projekt: Projekt,
    /// Bausteine aus Typst, Dokumenttyp und Paketbeschreibungen.
    katalog: tftn_katalog::Katalog,
    literatur: tftn_literatur::Literatur,
    /// Fach „Dokument“ (PLAN-M2 AP7)
    formular: formular::FormularSitzung,
    datei: PathBuf,
    spiegel: Spiegel,
    /// Prüfsumme des zuletzt gespeicherten bzw. geladenen Stands (für Dateiwächter und „ungespeichert“).
    auf_platte: u64,
    /// Fingerabdruck der Schriften beim letzten Einbinden (0: noch nie in dieser Sitzung).
    schriften_abdruck: u64,
}

#[derive(Default)]
struct Zustand {
    sitzung: Mutex<Option<Sitzung>>,
    waechter: Mutex<Option<notify::RecommendedWatcher>>,
    /// Wächter auf den Ordner mit `tasten.toml` und `einstellungen.toml` (PLAN-M2 AP3).
    einstellungen_waechter: Mutex<Option<notify::RecommendedWatcher>>,
    /// World für die Vorschau im Fach „Formel“, je Projekt einmal gebaut (PLAN-M2 AP10).
    formel_world: Mutex<Option<(PathBuf, tftn_core::ProjektWorld)>>,
    /// Ordner mit Paketbeschreibungen, der erste hat Vorrang (PLAN-M2 AP10).
    beschreibungen: Mutex<Vec<PathBuf>>,
    kompilierer: Mutex<Option<mpsc::Sender<Auftrag>>>,
    vorschau: Mutex<Option<Arc<Mutex<Vorschau>>>>,
    /// Der angezeigte (gute) Stand, aus dem `tftn://` rastert.
    anzeige: Arc<Mutex<Option<Anzeige>>>,
    /// Projektwechsel (PLAN-M2 AP8): das Projekt, das `oeffnen` nach dem Neuladen nimmt.
    naechstes: Mutex<Option<PathBuf>>,
    /// Vorschaubilder der Dokumenttypen als data-URL, je `name@version`.
    vorschaubilder: Mutex<std::collections::HashMap<String, String>>,
    /// Ein Holer läuft gerade (PLAN-M2 AP9); höchstens einer zugleich.
    holer_laeuft: std::sync::atomic::AtomicBool,
    /// Paketindex aus dem Zwischenspeicher, je Änderungszeit der Datei einmal gelesen.
    index: Mutex<Option<(std::time::SystemTime, Arc<tftn_pakete::Index>)>>,
    /// Wörterbuch der Dokumentsprache samt eigener Wörter.
    rechtschreibung: Mutex<Option<rechtschreibung::Stand>>,
    /// MCP: Schalter, Verbindung, offene Aufträge an den Editor.
    mcp: Mutex<mcp::Mcp>,
    /// Wer den gemerkten Wechsel (`naechstes`) ausgelöst hat.
    wechsel: Mutex<mcp::Wechsel>,
    /// Letzter Typst-Lauf, für die Antworten an MCP.
    lauf: Arc<Mutex<Option<mcp::LaufStand>>>,
    /// Nummer der offenen Sitzung; `anzeige` und `lauf` nehmen nur Läufe dieser Sitzung an.
    sitzung_nr: Arc<std::sync::atomic::AtomicU64>,
}

struct Auftrag {
    rev: u64,
    quelle: Source,
}

struct Anzeige {
    hashes: Vec<u128>,
    dokument: PagedDocument,
}

#[derive(Clone, Serialize)]
struct SeiteJs {
    hash: String,
    breite: f64,
    hoehe: f64,
}

#[derive(Clone, Serialize)]
struct MeldungJs {
    fehler: bool,
    text: String,
    ort: Option<String>,
    /// UTF-16-Bereich in der Hauptdatei, falls die Meldung dort liegt.
    von: Option<usize>,
    bis: Option<usize>,
}

#[derive(Clone, Serialize)]
struct VorschauEreignis {
    rev: u64,
    gelungen: bool,
    stand_rev: Option<u64>,
    seiten: Vec<SeiteJs>,
    meldungen: Vec<MeldungJs>,
    dauer_ms: f64,
}

/// Meldungen eines Laufs für den Editor (UTF-16-Bereich in der Hauptdatei) und für MCP
/// („Fehler (Zeile 3, Spalte 5): …“); Meldungen aus anderen Dateien behalten ihren Ort.
fn meldungen_aufbereiten(
    meldungen: &[tftn_core::Meldung],
    haupt: &Source,
) -> (Vec<MeldungJs>, Vec<String>) {
    let zeilen = haupt.lines();
    meldungen
        .iter()
        .map(|m| {
            let bereich = m
                .stelle
                .as_ref()
                .filter(|(id, _)| *id == haupt.id())
                .map(|(_, r)| r);
            let utf16 = bereich
                .and_then(|r| Some((zeilen.byte_to_utf16(r.start)?, zeilen.byte_to_utf16(r.end)?)));
            let ort = match bereich.and_then(|r| zeilen.byte_to_line_column(r.start)) {
                Some((z, sp)) => Some(format!("Zeile {}, Spalte {}", z + 1, sp + 1)),
                None => m.ort.clone(),
            };
            let art = if m.fehler { "Fehler" } else { "Warnung" };
            let text = match ort {
                Some(o) => format!("{art} ({o}): {}", m.text),
                None => format!("{art}: {}", m.text),
            };
            let js = MeldungJs {
                fehler: m.fehler,
                text: m.text.clone(),
                ort: m.ort.clone(),
                von: utf16.map(|b| b.0),
                bis: utf16.map(|b| b.1),
            };
            (js, text)
        })
        .unzip()
}

/// Startet den Kompilier-Faden der Sitzung `meine`; endet, wenn der Sender fallen gelassen wird.
/// Ergebnisse legt er nur ab, solange `meine` die laufende Sitzung ist (siehe `ablage`).
fn kompilierer_starten(
    app: tauri::AppHandle,
    vorschau: Arc<Mutex<Vorschau>>,
    anzeige: Arc<Mutex<Option<Anzeige>>>,
    stand: Arc<Mutex<Option<mcp::LaufStand>>>,
    laufend: Arc<std::sync::atomic::AtomicU64>,
    meine: u64,
) -> mpsc::Sender<Auftrag> {
    let (tx, rx) = mpsc::channel::<Auftrag>();
    std::thread::spawn(move || {
        while let Ok(mut auftrag) = rx.recv() {
            // neueste gewinnt
            while let Ok(neuer) = rx.try_recv() {
                auftrag = neuer;
            }
            let zeilen = auftrag.quelle.clone();
            let lauf = {
                let mut vorschau = vorschau.lock().unwrap();
                let lauf = vorschau.lauf(auftrag.rev, Some(auftrag.quelle));
                if lauf.gelungen
                    && let Some(gut) = vorschau.gut()
                {
                    ablage::ablegen(&anzeige, &laufend, meine, || Anzeige {
                        hashes: gut.hashes.clone(),
                        dokument: gut.dokument.clone(),
                    });
                }
                lauf
            };
            let (meldungen, fuer_mcp) = meldungen_aufbereiten(&lauf.meldungen, &zeilen);
            ablage::ablegen(&stand, &laufend, meine, || mcp::LaufStand {
                rev: lauf.rev,
                gelungen: lauf.gelungen,
                seiten: lauf.seiten.len(),
                meldungen: fuer_mcp,
            });
            let ereignis = VorschauEreignis {
                rev: lauf.rev,
                gelungen: lauf.gelungen,
                stand_rev: lauf.stand_rev,
                seiten: lauf
                    .seiten
                    .iter()
                    .map(|s| SeiteJs {
                        hash: format!("{:032x}", s.hash),
                        breite: s.breite,
                        hoehe: s.hoehe,
                    })
                    .collect(),
                meldungen,
                dauer_ms: lauf.dauer_ms,
            };
            // Nach einem Wechsel gehört der Lauf zum alten Dokument: nicht mehr melden.
            if ablage::aktuell(&laufend, meine) {
                let _ = app.emit("vorschau", ereignis);
            }
        }
    });
    tx
}

fn kompilieren_lassen(zustand: &Zustand, s: &Sitzung) {
    if let Some(tx) = zustand.kompilierer.lock().unwrap().as_ref() {
        let _ = tx.send(Auftrag {
            rev: s.spiegel.rev(),
            quelle: s.spiegel.quelle().clone(),
        });
    }
}

/// `tftn://localhost/seite/<nr>/<hash>?ppi=<ppi>` → PNG der Seite, falls der Hash noch aktuell ist.
fn seite_ausliefern(
    anzeige: &Mutex<Option<Anzeige>>,
    pfad: &str,
    abfrage: Option<&str>,
) -> Option<Vec<u8>> {
    let mut teile = pfad.trim_start_matches('/').split('/');
    if teile.next()? != "seite" {
        return None;
    }
    let nr: usize = teile.next()?.parse().ok()?;
    let hash = u128::from_str_radix(teile.next()?, 16).ok()?;
    let ppi = abfrage
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("ppi=")))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(144.0)
        .clamp(18.0, 600.0);
    let seite = {
        let a = anzeige.lock().unwrap();
        let a = a.as_ref()?;
        if a.hashes.get(nr) != Some(&hash) {
            return None;
        }
        a.dokument.pages().get(nr)?.clone()
    };
    Some(rastern(&seite, ppi))
}

type Hervorhebung = Vec<(usize, usize, &'static str)>;

#[derive(Serialize)]
struct Geoeffnet {
    ordner: String,
    datei: String,
    text: String,
    rev: u64,
    /// Das Dokument hat noch keinen Namen: Beim Speichern wird danach gefragt.
    entwurf: bool,
    /// Vorschlag für den Namen (etwa nach einem Import).
    vorschlag: Option<String>,
}

#[derive(Deserialize)]
struct Anfrage {
    basis: u64,
    aenderungen: Vec<Aenderung>,
    sichtbar: (usize, usize),
    /// FNV-1a 64 des Volltexts nach dieser Transaktion, hexadezimal; nur jede 50. Revision.
    pruefsumme: Option<String>,
}

#[derive(Serialize)]
struct Antwort {
    rev: u64,
    hervorhebung: Hervorhebung,
    /// `Some(false)`: Spiegel weicht ab, der Editor muss neu synchronisieren.
    pruefsumme_ok: Option<bool>,
    ungespeichert: bool,
}

/// Die zuletzt geöffneten Projekte, eines je Zeile, das neueste oben (`zuletzt.rs`).
fn zuletzt_datei(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("zuletzt.txt"))
}

/// Im Entwurf: der Name, den das Speichern vorschlägt.
const NAMENSVORSCHLAG: &str = ".tftn-name";

#[derive(Clone, Copy)]
enum Wahl {
    Datei,
    Ordner,
}

/// Argumente für `osascript`: JavaScript statt AppleScript, weil `activate` dort sofort wirkt
/// (AppleScripts „tell me to activate“ kostete rund zwei Sekunden, bevor der Dialog erschien).
/// Die Aufforderung kommt als Argument, nicht in den Skripttext.
/// `start`: Ordner (oder Datei darin), wo der Dialog öffnet; sonst dort, wo er zuletzt war.
fn wahl_argumente(wahl: Wahl, aufforderung: &str, start: Option<&Path>) -> Vec<String> {
    let funktion = match wahl {
        Wahl::Datei => "chooseFile",
        Wahl::Ordner => "chooseFolder",
    };
    let skript = format!(
        "function run(argv) {{ var a = Application.currentApplication(); \
         a.includeStandardAdditions = true; a.activate(); \
         var o = {{ withPrompt: argv[0] }}; if (argv[1]) o.defaultLocation = Path(argv[1]); \
         return a.{funktion}(o).toString(); }}"
    );
    let mut a: Vec<String> = ["-l", "JavaScript", "-e", &skript, aufforderung]
        .map(String::from)
        .to_vec();
    // Eine Datei als Start: ihr Ordner.
    let start = start.and_then(|s| if s.is_file() { s.parent() } else { Some(s) });
    if let Some(s) = start.filter(|s| s.is_dir()) {
        a.push(s.display().to_string());
    }
    a
}

/// Systemdialog zur Datei- oder Ordnerwahl (macOS); `None` bei Abbruch und auf anderen Systemen.
fn waehlen(wahl: Wahl, aufforderung: &str) -> Option<PathBuf> {
    waehlen_ab(wahl, aufforderung, None)
}

fn waehlen_ab(wahl: Wahl, aufforderung: &str, start: Option<&Path>) -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let aus = std::process::Command::new("osascript")
        .args(wahl_argumente(wahl, aufforderung, start))
        .output()
        .ok()?;
    if !aus.status.success() {
        return None;
    }
    let pfad = String::from_utf8(aus.stdout).ok()?;
    let pfad = pfad.trim_end_matches('\n');
    (!pfad.is_empty()).then(|| PathBuf::from(pfad))
}

#[tauri::command(async)]
fn datei_waehlen_dialog(aufforderung: String, start: Option<String>) -> Option<String> {
    waehlen_ab(Wahl::Datei, &aufforderung, start.as_deref().map(Path::new))
        .map(|p| p.display().to_string())
}

/// Import über Pandoc in ein neues Dokument ohne Namen; liefert Hinweise (etwa fehlende Bilder).
#[tauri::command(async)]
fn importieren(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    datei: String,
) -> Result<Vec<String>, String> {
    let pandoc = import::pandoc_finden().ok_or_else(import::fehlt)?;
    let datei = PathBuf::from(datei);
    let (quellen, _) = typquellen(&app)?;
    let entwurf = entwurf_ordner(&app)?;
    // Vorher hat die Oberfläche einen Entwurf mit Text benennen lassen; ein leerer wird ersetzt.
    let _ = std::fs::remove_dir_all(&entwurf);
    let ziel = typen::entwurf_sichern_in(&quellen, &entwurf, Some(&tftn_sprachen::kennung()))?;
    let u = import::umwandeln(&pandoc, &datei, &ziel)?;
    let main = ziel.join("main.typ");
    let vorlage = std::fs::read_to_string(&main).map_err(|e| e.to_string())?;
    std::fs::write(&main, format!("{vorlage}\n{}", u.text)).map_err(|e| e.to_string())?;
    if let Some(stamm) = datei.file_stem() {
        let _ = std::fs::write(
            ziel.join(NAMENSVORSCHLAG),
            stamm.to_string_lossy().as_bytes(),
        );
    }
    *zustand.naechstes.lock().unwrap() = Some(ziel);
    Ok(u.hinweise)
}

#[derive(Serialize)]
struct UmgewandeltJs {
    text: String,
    hinweise: Vec<String>,
}

/// Eingefügten Text (Markdown oder HTML aus der Zwischenablage) über Pandoc nach Typst. Schreibt
/// nichts in `main.typ`; das setzt die Oberfläche ein (ein Rückgängig-Schritt).
#[tauri::command(async)]
fn einfuegen_umwandeln(
    zustand: State<'_, Zustand>,
    text: String,
    html: bool,
) -> Result<UmgewandeltJs, String> {
    let pandoc = import::pandoc_finden().ok_or_else(import::fehlt_fuer_umwandeln)?;
    let wurzel = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .wurzel
            .clone()
    };
    let u = import::umwandeln_text(&pandoc, &text, html, &wurzel)?;
    Ok(UmgewandeltJs {
        text: u.text,
        hinweise: u.hinweise,
    })
}

/// Der Ordner des Dokuments ohne Namen.
fn entwurf_ordner(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("entwurf"))
        .map_err(|e| e.to_string())
}

/// Der Standardordner für alles, was einen Namen bekommt.
fn dokumente_ordner(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().document_dir().map_err(|e| e.to_string())
}

/// Projektordner beim Start: erstes Argument, `TFTN_PROJEKT`, das zuletzt geöffnete Projekt,
/// im Entwicklungsbau der Testartikel im Quellordner, sonst das leere Dokument. `waehlen`: Ordnerwahl.
fn startordner(app: &tauri::AppHandle, waehlen: bool) -> Result<PathBuf, String> {
    let gewaehlt = std::env::args_os()
        .skip(1)
        .find(|a| !a.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TFTN_PROJEKT").map(PathBuf::from))
        .or_else(|| {
            let text = std::fs::read_to_string(zuletzt_datei(app)?).ok()?;
            zuletzt::aus_text(&text)
                .into_iter()
                .find(|p| p.join("tftn.toml").is_file())
        })
        .filter(|_| !waehlen);
    if let Some(p) = gewaehlt {
        return Ok(p);
    }
    if cfg!(debug_assertions) && !waehlen {
        return Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel"));
    }
    if !waehlen {
        let (quellen, _) = typquellen(app)?;
        return typen::entwurf_sichern_in(
            &quellen,
            &entwurf_ordner(app)?,
            Some(&tftn_sprachen::kennung()),
        );
    }
    match ordner_waehlen(&tr!("TFTN-Projektordner wählen (mit tftn.toml)")) {
        Some(p) if p.join("tftn.toml").is_file() => Ok(p),
        Some(p) => Err(tr!(
            "{ordner} ist kein TFTN-Projekt (keine tftn.toml).",
            ordner = p.display()
        )),
        None => Err(tr!(
            "Kein Projekt gewählt. Alternativ im Terminal: open -a TFTN --args <Ordner mit tftn.toml>"
        )),
    }
}

/// Systemdialog zur Ordnerwahl; `None` bei Abbruch. Unter macOS über `osascript` (keine
/// zusätzliche Abhängigkeit), sonst noch keiner.
fn ordner_waehlen(aufforderung: &str) -> Option<PathBuf> {
    waehlen(Wahl::Ordner, aufforderung)
}

/// Hängt eine Zeile an `<projekt>/_lokal/tftn-sync.log` an (nicht synchronisiert, nicht in Git).
fn protokollieren(projekt: &Projekt, zeile: &str) {
    eprintln!("tftn: {zeile}");
    let ordner = projekt.wurzel.join("_lokal");
    if std::fs::create_dir_all(&ordner).is_ok()
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(ordner.join("tftn-sync.log"))
    {
        let sekunden = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "{sekunden} {zeile}");
    }
}

/// Nicht auf dem Hauptfaden: Die Ordnerwahl wartet auf den Benutzer.
#[tauri::command(async)]
fn oeffnen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    waehlen: Option<bool>,
) -> Result<Geoeffnet, String> {
    // Ein gewünschter Projektwechsel (Neu, Öffnen) geht allem anderen vor.
    let naechstes = zustand.naechstes.lock().unwrap().take();
    let wechsel = std::mem::take(&mut *zustand.wechsel.lock().unwrap());
    let gewechselt = naechstes.is_some();
    let ordner = match naechstes {
        Some(p) => p,
        None => startordner(&app, waehlen.unwrap_or(false))?,
    };
    let projekt = Projekt::oeffnen(&ordner).map_err(|e| e.to_string())?;
    if let Some(datei) = zuletzt_datei(&app) {
        let bisher = std::fs::read_to_string(&datei).unwrap_or_default();
        let liste = zuletzt::vorn(&zuletzt::aus_text(&bisher), &projekt.wurzel);
        let _ = std::fs::create_dir_all(datei.parent().unwrap());
        let _ = std::fs::write(&datei, zuletzt::als_text(&liste));
    }
    let datei = projekt.haupt();
    let roh = std::fs::read_to_string(&datei).map_err(|e| format!("{}: {e}", datei.display()))?;
    let vorschau = Vorschau::neu(&projekt).map_err(|e| e.to_string())?;
    let spiegel = Spiegel::neu(vorschau.world().haupt(), &roh);
    let vorschau = Arc::new(Mutex::new(vorschau));
    *zustand.vorschau.lock().unwrap() = Some(vorschau.clone());
    let antwort = Geoeffnet {
        ordner: projekt.wurzel.display().to_string(),
        datei: projekt.konfig.haupt.clone(),
        text: spiegel.text().to_string(),
        rev: spiegel.rev(),
        entwurf: entwurf_ordner(&app)
            .ok()
            .and_then(|o| o.canonicalize().ok())
            .is_some_and(|o| o == projekt.wurzel),
        vorschlag: std::fs::read_to_string(projekt.wurzel.join(NAMENSVORSCHLAG))
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty()),
    };
    let auf_platte = pruefsumme(&zeilenenden_lf(&roh));
    beobachten(&app, &zustand, &datei)?;
    let katalog = katalog_bauen(&zustand, &projekt);
    let literatur = literatur_laden(&projekt);
    let formular = formular::FormularSitzung::neu(&projekt);
    let sitzung = Sitzung {
        projekt,
        katalog,
        literatur,
        formular,
        datei,
        spiegel,
        auf_platte,
        schriften_abdruck: 0,
    };
    // Vorschau und Lauf des vorigen Dokuments gelten nicht mehr; sein Kompilierer legt nichts mehr ab.
    let meine = ablage::neue_sitzung(&zustand.sitzung_nr);
    ablage::leeren(&zustand.anzeige);
    ablage::leeren(&zustand.lauf);
    let tx = kompilierer_starten(
        app.clone(),
        vorschau,
        zustand.anzeige.clone(),
        zustand.lauf.clone(),
        zustand.sitzung_nr.clone(),
        meine,
    );
    *zustand.kompilierer.lock().unwrap() = Some(tx);
    kompilieren_lassen(&zustand, &sitzung);
    *zustand.sitzung.lock().unwrap() = Some(sitzung);
    if gewechselt {
        mcp::nach_wechsel(&app, wechsel);
    }
    Ok(antwort)
}

/// Meldet Änderungen der Hauptdatei von außen (anderes Programm, Tresorit) als Ereignis
/// `extern-geaendert` mit dem neuen Text. Eigene Speicherungen werden über die Prüfsumme erkannt.
fn beobachten(app: &tauri::AppHandle, zustand: &Zustand, datei: &Path) -> Result<(), String> {
    let app = app.clone();
    let ziel = datei.to_path_buf();
    let mut waechter =
        notify::recommended_watcher(move |ereignis: notify::Result<notify::Event>| {
            let Ok(ereignis) = ereignis else { return };
            if !ereignis
                .paths
                .iter()
                .any(|p| p.file_name() == ziel.file_name())
            {
                // Andere Projektdateien (Literatur, Formulardaten, Trennungen): nur neu kompilieren.
                let relevant = ereignis.paths.iter().any(|p| {
                    p.extension().is_some_and(|e| {
                        ["bib", "toml", "txt", "yaml", "yml"].contains(&&*e.to_string_lossy())
                    })
                });
                if relevant {
                    let zustand = app.state::<Zustand>();
                    let mut sitzung = zustand.sitzung.lock().unwrap();
                    if let Some(s) = sitzung.as_mut() {
                        // Datendatei (Fach „Dokument“): Stand melden; hat sich die Liste der
                        // Literaturdateien geändert, die Literatur neu laden.
                        let daten = s.formular.datei().file_name().map(|n| n.to_os_string());
                        let datendatei = ereignis
                            .paths
                            .iter()
                            .any(|p| p.file_name().map(|n| n.to_os_string()) == daten);
                        if datendatei {
                            let text =
                                std::fs::read_to_string(s.formular.datei()).unwrap_or_default();
                            let _ = app.emit("dokument-geaendert", tftn_formular::stand(&text));
                        }
                        let literatur_neu =
                            datendatei && literatur_dateien(&s.projekt) != s.literatur.dateien;
                        if literatur_neu
                            || ereignis.paths.iter().any(|p| {
                                s.literatur
                                    .dateien
                                    .iter()
                                    .any(|d| d.file_name() == p.file_name())
                            })
                        {
                            let dateien = literatur_dateien(&s.projekt);
                            let (neu, fehler) = tftn_literatur::Literatur::laden(&dateien);
                            for f in &fehler {
                                protokollieren(&s.projekt, &format!("Literatur: {f}"));
                            }
                            let _ = app.emit("literatur", (neu.len(), fehler));
                            s.literatur = neu;
                        }
                        kompilieren_lassen(&zustand, s);
                    }
                }
                return;
            }
            let Ok(text) = std::fs::read_to_string(&ziel) else {
                return;
            };
            let text = zeilenenden_lf(&text);
            let zustand = app.state::<Zustand>();
            let mut sitzung = zustand.sitzung.lock().unwrap();
            let Some(s) = sitzung.as_mut() else { return };
            let neu = pruefsumme(&text);
            if neu == s.auf_platte {
                return; // eigene Speicherung oder unverändert
            }
            s.auf_platte = neu;
            drop(sitzung);
            let _ = app.emit("extern-geaendert", text);
        })
        .map_err(|e| e.to_string())?;
    let ordner = datei.parent().ok_or_else(|| tr!("Datei ohne Ordner"))?;
    waechter
        .watch(ordner, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;
    *zustand.waechter.lock().unwrap() = Some(waechter);
    Ok(())
}

use tauri::Manager;

fn antwort(s: &Sitzung, sichtbar: (usize, usize), pruefsumme_ok: Option<bool>) -> Antwort {
    Antwort {
        rev: s.spiegel.rev(),
        hervorhebung: s.spiegel.hervorhebung(sichtbar.0..sichtbar.1),
        pruefsumme_ok,
        ungespeichert: s.spiegel.pruefsumme() != s.auf_platte,
    }
}

#[tauri::command]
fn aendern(anfrage: Anfrage, zustand: State<'_, Zustand>) -> Result<Antwort, String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if let Err(fehler) = s.spiegel.anwenden(anfrage.basis, &anfrage.aenderungen) {
        protokollieren(&s.projekt, &format!("Sync-Fehler: {fehler}"));
        return Err(fehler.to_string());
    }
    let ok = anfrage.pruefsumme.map(|erwartet| {
        let ist = format!("{:016x}", s.spiegel.pruefsumme());
        let gleich = ist == erwartet;
        if !gleich {
            protokollieren(
                &s.projekt,
                &format!(
                    "Prüfsumme weicht ab bei Rev {}: Editor {erwartet}, Spiegel {ist}",
                    s.spiegel.rev()
                ),
            );
        }
        gleich
    });
    kompilieren_lassen(&zustand, s);
    Ok(antwort(s, anfrage.sichtbar, ok))
}

/// Übernimmt den ganzen Editortext mit der Revision, die der Editor vorgibt.
#[tauri::command]
fn neusync(
    text: String,
    rev: u64,
    sichtbar: (usize, usize),
    zustand: State<'_, Zustand>,
) -> Result<Antwort, String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    s.spiegel.neu_aufsetzen(&text, rev);
    protokollieren(&s.projekt, &format!("Neusynchronisation, jetzt Rev {rev}"));
    kompilieren_lassen(&zustand, s);
    Ok(antwort(s, sichtbar, None))
}

#[tauri::command]
fn hervorheben(
    rev: u64,
    sichtbar: (usize, usize),
    zustand: State<'_, Zustand>,
) -> Option<Hervorhebung> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref()?;
    (s.spiegel.rev() == rev).then(|| s.spiegel.hervorhebung(sichtbar.0..sichtbar.1))
}

/// Baut den Helfer-Katalog des Projekts (PLAN-M2 AP1); Warnungen ins Protokoll.
fn katalog_bauen(zustand: &Zustand, projekt: &Projekt) -> tftn_katalog::Katalog {
    let beschreibungen = zustand.beschreibungen.lock().unwrap().clone();
    let (katalog, warnungen) = tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
        projekt: Some(projekt),
        beschreibungen: &beschreibungen,
    });
    for w in warnungen {
        protokollieren(projekt, &format!("Katalog: {w}"));
    }
    katalog
}

/// Liest die Literatur des Projekts (Manifest `literatur.dateien`, sonst `literatur.bib`).
/// Literaturdateien: `[literatur].dateien` aus der Datendatei (Fachartikel ab 0.2.0), sonst die
/// Vorgabe aus dem Manifest (PLAN-M2 AP7).
fn literatur_dateien(projekt: &Projekt) -> Vec<PathBuf> {
    let paket = tftn_manifest::Typpaket::vom_projekt(projekt).ok();
    let manifest = paket.as_ref().and_then(|p| p.manifest().ok());
    let daten = manifest
        .and_then(|m| m.daten.clone())
        .unwrap_or_else(|| "dokument.toml".into());
    let liste = std::fs::read_to_string(projekt.wurzel.join(daten))
        .ok()
        .and_then(|t| tftn_formular::literatur_dateien(&t))
        .or_else(|| {
            manifest
                .and_then(|m| m.literatur.as_ref())
                .map(|l| l.dateien.clone())
        })
        .unwrap_or_default();
    tftn_literatur::dateien_im_projekt(&projekt.wurzel, &liste)
}

/// Regelt der Dokumenttyp die Literatur selbst (Datendatei oder Manifest)? Dann braucht `main.typ`
/// kein eigenes `#bibliography(…)`.
fn typ_regelt_literatur(projekt: &Projekt) -> bool {
    let paket = tftn_manifest::Typpaket::vom_projekt(projekt).ok();
    let manifest = paket.as_ref().and_then(|p| p.manifest().ok());
    if manifest.is_some_and(|m| m.literatur.is_some()) {
        return true;
    }
    let daten = manifest
        .and_then(|m| m.daten.clone())
        .unwrap_or_else(|| "dokument.toml".into());
    std::fs::read_to_string(projekt.wurzel.join(daten))
        .ok()
        .and_then(|t| tftn_formular::literatur_dateien(&t))
        .is_some()
}

fn literatur_laden(projekt: &Projekt) -> tftn_literatur::Literatur {
    let dateien = literatur_dateien(projekt);
    let (lit, fehler) = tftn_literatur::Literatur::laden(&dateien);
    for f in &fehler {
        protokollieren(projekt, &format!("Literatur: {f}"));
    }
    lit
}

#[derive(Serialize)]
struct ProfilJs {
    id: String,
    art: String,
    standards: Vec<String>,
}

#[tauri::command]
fn export_profile(zustand: State<'_, Zustand>) -> Result<Vec<ProfilJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(tftn_export::profil::profile(&s.projekt)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| ProfilJs {
            id: p.id,
            art: p.manifest.art,
            standards: p.manifest.pdf_standards,
        })
        .collect())
}

#[derive(Serialize)]
struct ExportJs {
    pfad: String,
    relativ: String,
    gegenprobe: String,
    sha256: Option<String>,
    warnungen: Vec<String>,
}

/// Export mit Profil `profil`; `abnehmen` legt nach `export/` ab (KONZEPT 5.7).
#[tauri::command]
async fn exportieren(
    profil: String,
    abnehmen: bool,
    zustand: State<'_, Zustand>,
) -> Result<ExportJs, String> {
    let projekt = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.pruefsumme() != s.auf_platte {
            return Err(tr!(
                "Bitte zuerst speichern: exportiert wird die Datei auf der Platte."
            ));
        }
        s.projekt.clone()
    };
    let cli = tftn_export::TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
    let e = tftn_export::profil::profil_exportieren(&projekt, &cli, &profil, abnehmen)
        .map_err(|e| e.to_string())?;
    if abnehmen {
        // Neues Datum: Projekt neu lesen und die Vorschau-World damit neu aufbauen.
        let neu = Projekt::oeffnen(&projekt.wurzel).map_err(|e| e.to_string())?;
        let vorschau = Vorschau::neu(&neu).map_err(|e| e.to_string())?;
        if let Some(v) = zustand.vorschau.lock().unwrap().as_ref() {
            *v.lock().unwrap() = vorschau;
        }
        let mut sitzung = zustand.sitzung.lock().unwrap();
        if let Some(s) = sitzung.as_mut() {
            protokollieren(
                &neu,
                &format!("abgenommen: {} (SHA-256 {:?})", e.relativ, e.sha256),
            );
            s.projekt = neu;
            kompilieren_lassen(&zustand, s);
        }
    }
    Ok(ExportJs {
        pfad: e.pfad.display().to_string(),
        relativ: e.relativ,
        gegenprobe: e.gegenprobe,
        sha256: e.sha256,
        warnungen: e.warnungen.iter().map(|w| w.to_string()).collect(),
    })
}

#[derive(Serialize)]
struct ArchivJs {
    pfad: String,
    relativ: String,
    pdf: String,
    standard: String,
    sha256: String,
    dateien: usize,
    groesse: u64,
    warnungen: Vec<String>,
}

/// Archivpaket nach `export/`: Projekt, Schriften, Pakete, PDF/A und Liesmich in einem Zip.
#[tauri::command]
async fn archivieren(zustand: State<'_, Zustand>) -> Result<ArchivJs, String> {
    let projekt = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.pruefsumme() != s.auf_platte {
            return Err(tr!(
                "Bitte zuerst speichern: exportiert wird die Datei auf der Platte."
            ));
        }
        s.projekt.clone()
    };
    let cli = tftn_export::TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
    let a = tftn_export::archiv::archiv_erzeugen(&projekt, &cli, env!("CARGO_PKG_VERSION"))
        .map_err(|e| e.to_string())?;
    protokollieren(
        &projekt,
        &format!("archiviert: {} (PDF SHA-256 {})", a.relativ, a.sha256),
    );
    Ok(ArchivJs {
        pfad: a.pfad.display().to_string(),
        relativ: a.relativ,
        pdf: a.pdf,
        standard: a.standard,
        sha256: a.sha256,
        dateien: a.dateien,
        groesse: a.groesse,
        warnungen: a.warnungen,
    })
}

// --- Serie (ein PDF je Datensatz) ------------------------------------------------------------

#[derive(Serialize)]
struct SerieJs {
    /// Namen der PDFs (ohne Endung), in ihrer Reihenfolge
    namen: Vec<String>,
    zielordner: String,
    /// „bereit“, „anpassbar“ oder „nein“
    hauptdatei: &'static str,
    /// Name der Datendatei, etwa `dokument.toml`
    daten: String,
    /// Dateiname des Sammel-PDFs (ohne Endung)
    gesamt_name: String,
}

#[derive(Serialize)]
struct StueckJs {
    name: String,
    pdf: Option<String>,
    fehler: Option<String>,
    gegenprobe: String,
    seiten: usize,
}

impl From<tftn_export::serie::Stueck> for StueckJs {
    fn from(s: tftn_export::serie::Stueck) -> Self {
        StueckJs {
            name: s.name,
            pdf: s.pdf.map(|p| p.display().to_string()),
            fehler: s.fehler,
            gegenprobe: s.gegenprobe,
            seiten: s.seiten,
        }
    }
}

#[derive(Serialize)]
struct SerieErgebnisJs {
    stuecke: Vec<StueckJs>,
    gesamt: Option<StueckJs>,
}

fn serie_projekt(zustand: &Zustand) -> Result<Projekt, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(s.projekt.clone())
}

/// Liest die Datensätze (Datei oder Ordner) und sagt, ob die Hauptdatei bereit ist.
#[tauri::command(async)]
fn serie_pruefen(quelle: String, zustand: State<'_, Zustand>) -> Result<SerieJs, String> {
    use tftn_export::serie::{self, Hauptdatei};
    let projekt = serie_projekt(&zustand)?;
    let q = serie::lesen(Path::new(quelle.trim()))?;
    let daten = serie::datendatei(&projekt);
    let text = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .map(|s| s.spiegel.text().to_string())
            .unwrap_or_default()
    };
    Ok(SerieJs {
        gesamt_name: q.gesamt_name(),
        namen: q.datensaetze.into_iter().map(|d| d.name).collect(),
        zielordner: q.zielordner.display().to_string(),
        hauptdatei: match serie::hauptdatei_pruefen(&text, &daten) {
            Hauptdatei::Bereit => "bereit",
            Hauptdatei::Anpassbar { .. } => "anpassbar",
            Hauptdatei::Nein => "nein",
        },
        daten,
    })
}

/// Erzeugt die Serie über die typst-CLI; meldet `serie-fortschritt` (fertig, gesamt, name).
#[tauri::command(async)]
fn serie_erzeugen(
    app: tauri::AppHandle,
    quelle: String,
    ausgabe: String,
    beidseitig: bool,
    zustand: State<'_, Zustand>,
) -> Result<SerieErgebnisJs, String> {
    use tftn_export::serie::{Ausgabe, Optionen};
    let ausgabe = match ausgabe.as_str() {
        "gesamt" => Ausgabe::Gesamt,
        "beides" => Ausgabe::Beides,
        _ => Ausgabe::Einzeln,
    };
    {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.pruefsumme() != s.auf_platte {
            return Err(tr!(
                "Bitte zuerst speichern: exportiert wird die Datei auf der Platte."
            ));
        }
    }
    let projekt = serie_projekt(&zustand)?;
    let q = tftn_export::serie::lesen(Path::new(quelle.trim()))?;
    let cli = tftn_export::TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
    let optionen = Optionen {
        ausgabe,
        beidseitig,
    };
    let serie =
        tftn_export::serie::erzeugen(&projekt, &cli, &q, optionen, |fertig, gesamt, name| {
            let _ = app.emit("serie-fortschritt", (fertig, gesamt, name.to_string()));
        })
        .map_err(|e| e.to_string())?;
    let gut = serie.stuecke.iter().filter(|s| s.pdf.is_some()).count();
    let sammel = match &serie.gesamt {
        Some(g) if g.pdf.is_some() => format!(", Sammel-PDF {} Seiten", g.seiten),
        Some(_) => ", Sammel-PDF fehlgeschlagen".into(),
        None => String::new(),
    };
    protokollieren(
        &projekt,
        &format!(
            "Serie: {gut} von {} nach {}{sammel}",
            serie.stuecke.len(),
            q.zielordner.display()
        ),
    );
    Ok(SerieErgebnisJs {
        stuecke: serie.stuecke.into_iter().map(StueckJs::from).collect(),
        gesamt: serie.gesamt.map(StueckJs::from),
    })
}

// --- Dokumenttypen, Neues Projekt, Projektwechsel (PLAN-M2 AP8) ---------------------------------

/// Typquellen: eigener Ordner im Tresor (`<Einstellungsordner>/typen`) vor dem App-Bündel.
fn typquellen(app: &tauri::AppHandle) -> Result<(Vec<typen::Typquelle>, PathBuf), String> {
    let konfig = app_konfig(app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    let buendel = typen::buendel_ordner(app.path().resource_dir().ok());
    Ok((
        typen::quellen(&buendel, &ort.ordner),
        ort.ordner.join("typen"),
    ))
}

/// „Eigener Dokumenttyp …“: kopiert `basis` als neuen Typ `name` in den Ordner `typen/` und zeigt ihn.
#[tauri::command(async)]
fn typ_eigen_anlegen(app: tauri::AppHandle, basis: String, name: String) -> Result<String, String> {
    let (mut quellen, eigen) = typquellen(&app)?;
    if !quellen
        .iter()
        .any(|q| q.herkunft == tftn_manifest::typen::Herkunft::Eigen)
    {
        quellen.insert(
            0,
            typen::Typquelle {
                ordner: eigen,
                herkunft: tftn_manifest::typen::Herkunft::Eigen,
            },
        );
    }
    let ordner = tftn_manifest::eigen::eigenen_typ_anlegen(&quellen, &basis, &name)?;
    word::im_ordner_zeigen(&ordner)?;
    Ok(ordner.display().to_string())
}

#[tauri::command]
fn typen_liste(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<typen::TypenJs, String> {
    let (quellen, eigen) = typquellen(&app)?;
    let vorgabe = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .map(|_| dokumente_ordner(&app))
        .transpose()?;
    Ok(typen::typen_js(&quellen, &eigen, vorgabe.as_deref()))
}

/// Seite 1 eines Typs als data-URL; je Sitzung und Version nur einmal gerechnet.
#[tauri::command(async)]
fn typ_vorschaubild(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    name: String,
) -> Result<String, String> {
    let (quellen, _) = typquellen(&app)?;
    let version = tftn_manifest::typen::finden_in(&quellen, &name, None)
        .map(|p| p.toml.package.version)
        .map_err(|e| e.to_string())?;
    let schluessel = format!("{name}@{version}");
    if let Some(url) = zustand.vorschaubilder.lock().unwrap().get(&schluessel) {
        return Ok(url.clone());
    }
    let url = typen::png_url(&typen::vorschaubild(
        &quellen,
        &name,
        Some(&tftn_sprachen::kennung()),
    )?);
    zustand
        .vorschaubilder
        .lock()
        .unwrap()
        .insert(schluessel, url.clone());
    Ok(url)
}

/// Systemdialog zur Ordnerwahl (nur macOS; sonst `None`).
#[tauri::command(async)]
fn ordner_waehlen_dialog(aufforderung: String, start: Option<String>) -> Option<String> {
    waehlen_ab(Wahl::Ordner, &aufforderung, start.as_deref().map(Path::new))
        .map(|p| p.display().to_string())
}

/// Legt `<eltern>/<name>` an und merkt es für den Wechsel (die Oberfläche lädt danach neu).
#[tauri::command(async)]
fn projekt_anlegen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    typ: String,
    eltern: String,
    name: String,
    schriften: bool,
) -> Result<String, String> {
    let (quellen, _) = typquellen(&app)?;
    let ziel = typen::anlegen(
        &quellen,
        &typ,
        &typen::pfad_aus_eingabe(&eltern).ok_or_else(|| tr!("Kein Ordner angegeben."))?,
        &name,
        schriften,
        Some(&tftn_sprachen::kennung()),
    )?;
    *zustand.naechstes.lock().unwrap() = Some(ziel.clone());
    Ok(ziel.display().to_string())
}

/// Neues leeres Dokument (Cmd/Strg+N): der Entwurf, ohne Dialog.
#[tauri::command(async)]
fn neues_dokument(app: tauri::AppHandle, zustand: State<'_, Zustand>) -> Result<(), String> {
    let (quellen, _) = typquellen(&app)?;
    let ziel = typen::entwurf_sichern_in(
        &quellen,
        &entwurf_ordner(&app)?,
        Some(&tftn_sprachen::kennung()),
    )?;
    *zustand.naechstes.lock().unwrap() = Some(ziel);
    Ok(())
}

/// Erstes Speichern: Der Entwurf zieht unter seinem Namen nach Dokumente.
#[tauri::command(async)]
fn entwurf_benennen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    name: String,
    schriften: bool,
) -> Result<String, String> {
    let ziel = typen::entwurf_benennen(
        &entwurf_ordner(&app)?,
        &dokumente_ordner(&app)?,
        &name,
        schriften,
    )?;
    *zustand.naechstes.lock().unwrap() = Some(ziel.clone());
    *zustand.wechsel.lock().unwrap() = mcp::Wechsel::Umbenannt;
    Ok(ziel.display().to_string())
}

/// Menü „Öffnen“: die zuletzt geöffneten Projekte, die es noch gibt, ohne das offene und den Entwurf.
#[tauri::command(async)]
fn zuletzt_liste(app: tauri::AppHandle, zustand: State<'_, Zustand>) -> Vec<zuletzt::ZuletztJs> {
    let Some(datei) = zuletzt_datei(&app) else {
        return Vec::new();
    };
    let liste = zuletzt::aus_text(&std::fs::read_to_string(datei).unwrap_or_default());
    let offen = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.projekt.wurzel.clone());
    let entwurf = entwurf_ordner(&app)
        .ok()
        .and_then(|o| o.canonicalize().ok());
    zuletzt::fuer_menue(&liste, offen.as_deref(), entwurf.as_deref(), |p| {
        p.join("tftn.toml").is_file()
    })
}

/// „Liste leeren“: Nur das offene Projekt bleibt gemerkt, damit TFTN wieder mit ihm startet.
#[tauri::command]
fn zuletzt_leeren(app: tauri::AppHandle, zustand: State<'_, Zustand>) -> Result<(), String> {
    let Some(datei) = zuletzt_datei(&app) else {
        return Ok(());
    };
    let offen = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.projekt.wurzel.clone());
    match offen {
        Some(o) => std::fs::write(&datei, zuletzt::als_text(&[o])),
        None => std::fs::remove_file(&datei).or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        }),
    }
    .map_err(|e| e.to_string())
}

/// Merkt ein anderes Projekt für den Wechsel; ohne `ordner` über die Ordnerwahl.
#[tauri::command(async)]
fn projekt_wechseln(zustand: State<'_, Zustand>, ordner: Option<String>) -> Result<(), String> {
    let ordner = match ordner {
        Some(o) => typen::pfad_aus_eingabe(&o).ok_or_else(|| tr!("Kein Projekt gewählt."))?,
        None => ordner_waehlen(&tr!("TFTN-Projektordner wählen (mit tftn.toml)"))
            .ok_or_else(|| tr!("Kein Projekt gewählt."))?,
    };
    *zustand.naechstes.lock().unwrap() = Some(typen::wechsel_pruefen(&ordner)?);
    Ok(())
}

/// Gespeichertes Projekt der Sitzung; verglichen und übernommen wird die Datei auf der Platte.
fn gespeichertes_projekt(zustand: &Zustand) -> Result<Projekt, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.spiegel.pruefsumme() != s.auf_platte {
        return Err(tr!(
            "Bitte zuerst speichern: verglichen wird die Datei auf der Platte."
        ));
    }
    Ok(s.projekt.clone())
}

/// Baut nach „Übernehmen“ Projekt, Vorschau, Katalog, Formular und Literatur neu auf – aber nur,
/// wenn die Sitzung noch dasselbe Projekt hat (inzwischen gewechselt: `Ok(false)`, nichts getauscht).
fn uebernahme_einspielen(zustand: &Zustand, wurzel: &Path) -> Result<bool, String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let Some(s) = sitzung.as_mut().filter(|s| s.projekt.wurzel == wurzel) else {
        return Ok(false);
    };
    let neu = Projekt::oeffnen(wurzel).map_err(|e| e.to_string())?;
    let vorschau = Vorschau::neu(&neu).map_err(|e| e.to_string())?;
    if let Some(v) = zustand.vorschau.lock().unwrap().as_ref() {
        *v.lock().unwrap() = vorschau;
    }
    s.katalog = katalog_bauen(zustand, &neu);
    s.formular = formular::FormularSitzung::neu(&neu);
    s.literatur = literatur_laden(&neu);
    s.projekt = neu;
    kompilieren_lassen(zustand, s);
    Ok(true)
}

/// Probe mit der neueren Version des Dokumenttyps und Seitenvergleich (PLAN-M2 AP8).
#[tauri::command(async)]
fn typ_vergleichen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<typen::VergleichJs, String> {
    let projekt = gespeichertes_projekt(&zustand)?;
    let (quellen, _) = typquellen(&app)?;
    typen::vergleichen(&projekt, &quellen)
}

#[derive(Serialize)]
struct UebernahmeJs {
    /// Importstelle in der Hauptdatei; der Editor wendet ihn als einen Undo-Schritt an.
    patch: Option<PatchJs>,
    hinweis: Option<String>,
}

/// Übernimmt die neuere Version (nur ohne Fehler im Vergleich) und baut Projekt, Vorschau, Katalog,
/// Formular und Literatur neu auf.
#[tauri::command(async)]
fn typ_uebernehmen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    rev: u64,
) -> Result<UebernahmeJs, String> {
    let projekt = gespeichertes_projekt(&zustand)?;
    let (quellen, _) = typquellen(&app)?;
    let quelle = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        helfer::rev_pruefen(s.spiegel.rev(), rev)?;
        s.spiegel.quelle().clone()
    };
    let (a, patch) = typen::uebernehmen(&projekt, &quellen, &projekt.konfig.haupt, &quelle)?;
    if !uebernahme_einspielen(&zustand, &projekt.wurzel)? {
        return Err(tr!(
            "Inzwischen wurde das Projekt gewechselt; die Übernahme gilt nur für die Dateien des alten Projekts."
        ));
    }
    protokollieren(
        &projekt,
        &format!("Dokumenttyp aktualisiert: {} → {}", a.alt, a.neu),
    );
    let mut hinweis = match &patch {
        Some(_) => tr!("Dokumenttyp auf {neu} umgestellt.", neu = a.neu),
        None => tr!(
            "Dokumenttyp auf {neu} umgestellt; {datei} importiert ihn nicht direkt – bitte prüfen.",
            neu = a.neu,
            datei = projekt.konfig.haupt
        ),
    };
    if !a.schriften.is_empty() {
        hinweis.push(' ');
        hinweis.push_str(&tr!(
            "Neue Schriften: {schriften}.",
            schriften = a.schriften.join(", ")
        ));
    }
    Ok(UebernahmeJs {
        patch: patch.map(|p| patch_js(&quelle, rev, p)).transpose()?,
        hinweis: Some(hinweis),
    })
}

// --- Fach „Pakete“ (PLAN-M2 AP9) -------------------------------------------------------------

/// Zwischenspeicher am Cache-Ort des Systems (nicht im Tresor).
fn zwischenspeicher(app: &tauri::AppHandle) -> Result<tftn_pakete::Zwischenspeicher, String> {
    let d = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    Ok(tftn_pakete::Zwischenspeicher {
        wurzel: d.join("universe"),
    })
}

/// Index aus dem Zwischenspeicher; neu gelesen nur, wenn sich die Datei geändert hat.
fn index_laden(
    zustand: &Zustand,
    zs: &tftn_pakete::Zwischenspeicher,
) -> Result<Option<(std::time::SystemTime, Arc<tftn_pakete::Index>)>, String> {
    let pfad = zs.index_pfad();
    let Ok(zeit) = std::fs::metadata(&pfad).and_then(|m| m.modified()) else {
        return Ok(None);
    };
    let mut merker = zustand.index.lock().unwrap();
    if let Some((z, i)) = merker.as_ref()
        && *z == zeit
    {
        return Ok(Some((zeit, i.clone())));
    }
    let daten = std::fs::read(&pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    let index = Arc::new(tftn_pakete::Index::lesen(&daten).map_err(|e| e.to_string())?);
    *merker = Some((zeit, index.clone()));
    Ok(Some((zeit, index)))
}

#[derive(Serialize)]
struct IndexStandJs {
    datum: String,
    pakete: usize,
}

#[derive(Serialize)]
struct PaketeStandJs {
    index: Option<IndexStandJs>,
    offline: bool,
    laeuft: bool,
    im_projekt: Vec<pakete::ProjektPaketJs>,
    /// Pakete, die beim letzten Übersetzen fehlten
    fehlend: Vec<String>,
    /// Pakete mit Beschreibung, die im Projekt fehlen
    holbar: Vec<pakete::HolbarJs>,
}

#[tauri::command(async)]
fn pakete_stand(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<PaketeStandJs, String> {
    let zs = zwischenspeicher(&app)?;
    let (wurzel, katalog, typst) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        (
            s.projekt.wurzel.clone(),
            s.katalog.clone(),
            semver::Version::parse(&s.projekt.konfig.typst)
                .unwrap_or_else(|_| semver::Version::new(0, 15, 1)),
        )
    };
    let geladen = index_laden(&zustand, &zs)?;
    let im_projekt = pakete::im_projekt_js(
        &wurzel,
        &katalog,
        geladen.as_ref().map(|(_, i)| i.as_ref()),
        &typst,
    );
    let index = geladen.map(|(zeit, i)| IndexStandJs {
        datum: pakete::datum(
            zeit.duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        ),
        pakete: i.eintraege.len(),
    });
    let fehlend = vorschau_von(&zustand)?
        .lock()
        .unwrap()
        .world()
        .fehlende_pakete()
        .iter()
        .map(ToString::to_string)
        .filter(|s| s.starts_with("@preview/"))
        .collect();
    Ok(PaketeStandJs {
        index,
        offline: tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).ganz_offline,
        laeuft: zustand
            .holer_laeuft
            .load(std::sync::atomic::Ordering::SeqCst),
        im_projekt,
        fehlend,
        holbar: pakete::holbar_js(&katalog, &zs),
    })
}

/// Startet den Holer für `spec` (nur auf Klick).
fn holer_paket(
    zustand: &Zustand,
    zs: &tftn_pakete::Zwischenspeicher,
    projekt: &Projekt,
    spec: &tftn_pakete::PaketSpec,
    ausloeser: netzprotokoll::Ausloeser,
) -> Result<pakete::HolerAntwort, String> {
    let mut argumente = vec![
        "paket".to_string(),
        "--zwischenspeicher".into(),
        zs.wurzel.display().to_string(),
    ];
    argumente.extend(pakete::erwartet_argumente(projekt));
    argumente.push(spec.to_string());
    pakete::holer_aufrufen(
        &pakete::holer_finden()?,
        &argumente,
        pakete::HOLER_FRIST,
        &zustand.holer_laeuft,
        ausloeser,
    )
}

/// Entfernt ein Paket samt verwaister Abhängigkeiten (Entwurf 6, PLAN-M2 AP10).
#[tauri::command(async)]
fn paket_entfernen(zustand: State<'_, Zustand>, spec: String) -> Result<String, String> {
    let spec = tftn_pakete::PaketSpec::lesen(&spec).map_err(|e| e.to_string())?;
    let projekt = gespeichertes_projekt(&zustand)?;
    let weg = tftn_pakete::entfernen(&projekt.wurzel, &spec).map_err(|e| e.to_string())?;
    uebernahme_einspielen(&zustand, &projekt.wurzel)?;
    let namen: Vec<String> = weg.iter().map(ToString::to_string).collect();
    protokollieren(&projekt, &format!("Paket entfernt: {}", namen.join(", ")));
    Ok(tr!("Entfernt: {namen}.", namen = namen.join(", ")))
}

/// Neue Version bereitstellen (Klick) und im Zwischenspeicher vollständig machen.
fn wechsel_holen(
    app: &tauri::AppHandle,
    zustand: &Zustand,
    projekt: &Projekt,
    spec: &str,
    version: &str,
) -> Result<
    (
        tftn_pakete::Zwischenspeicher,
        tftn_pakete::PaketSpec,
        pakete::Geholte,
    ),
    String,
> {
    let alt = tftn_pakete::PaketSpec::lesen(spec).map_err(|e| e.to_string())?;
    let neu = tftn_pakete::PaketSpec::neu(&alt.name, version).map_err(|e| e.to_string())?;
    let zs = zwischenspeicher(app)?;
    let rechner = tftn_einstellungen::Rechner::laden(&app_konfig(app)?);
    let (pakete, _) = pakete::holen_kern(&zs, &rechner, &neu, || {
        holer_paket(
            zustand,
            &zs,
            projekt,
            &neu,
            netzprotokoll::Ausloeser::VersionWechseln,
        )
    })?;
    Ok((zs, alt, pakete))
}

/// Versionswechsel: holen, Probe in einer Temp-Kopie, Seitenvergleich (PLAN-M2 AP10).
#[tauri::command(async)]
fn paket_vergleichen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    spec: String,
    version: String,
) -> Result<typen::VergleichJs, String> {
    let projekt = gespeichertes_projekt(&zustand)?;
    let (zs, alt, pakete) = wechsel_holen(&app, &zustand, &projekt, &spec, &version)?;
    pakete::wechsel_vergleichen_in(&projekt, &zs, &pakete, &alt, &std::env::temp_dir())
}

/// Versionswechsel übernehmen; die Importe der Hauptdatei ändert ein Patch (ein Undo-Schritt).
#[tauri::command(async)]
fn paket_wechseln(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    spec: String,
    version: String,
    rev: u64,
) -> Result<UebernahmeJs, String> {
    let projekt = gespeichertes_projekt(&zustand)?;
    let (zs, alt, pakete) = wechsel_holen(&app, &zustand, &projekt, &spec, &version)?;
    let quelle = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .spiegel
            .quelle()
            .clone()
    };
    let aktuell = || {
        zustand
            .sitzung
            .lock()
            .unwrap()
            .as_ref()
            .map_or(u64::MAX, |s| s.spiegel.rev())
    };
    let patch = pakete::wechsel_uebernehmen(
        &projekt,
        &zs,
        &pakete,
        &alt,
        &projekt.konfig.haupt,
        &quelle,
        &aktuell,
        rev,
    )?;
    if !uebernahme_einspielen(&zustand, &projekt.wurzel)? {
        return Err(tr!(
            "Inzwischen wurde das Projekt gewechselt; die Übernahme gilt nur für die Dateien des alten Projekts."
        ));
    }
    protokollieren(&projekt, &format!("Paket gewechselt: {alt} → {}", version));
    Ok(UebernahmeJs {
        patch: patch.map(|p| patch_js(&quelle, rev, p)).transpose()?,
        hinweis: Some(tr!(
            "{paket} auf Version {version} umgestellt.",
            paket = alt.name,
            version
        )),
    })
}

/// Holt den Index (nur auf Klick, nicht „ganz offline“).
#[tauri::command(async)]
fn pakete_index_holen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<pakete::IndexZahl, String> {
    pakete::darf_holen(&tftn_einstellungen::Rechner::laden(&app_konfig(&app)?))?;
    let zs = zwischenspeicher(&app)?;
    let argumente = vec![
        "index".to_string(),
        "--zwischenspeicher".into(),
        zs.wurzel.display().to_string(),
    ];
    let a = pakete::holer_aufrufen(
        &pakete::holer_finden()?,
        &argumente,
        pakete::HOLER_FRIST,
        &zustand.holer_laeuft,
        netzprotokoll::Ausloeser::IndexHolen,
    )?;
    a.index
        .ok_or_else(|| tr!("Der Holer hat keinen Index gemeldet."))
}

#[derive(Serialize)]
struct DoiJs {
    schluessel: String,
    /// Eintrag neu angelegt (sonst war die DOI schon da)
    neu: bool,
    /// Datei, in der der Eintrag steht, relativ zum Dokument
    datei: String,
    /// Fehlt `#bibliography(…)`: der Text, der ans Ende von `main.typ` gehört
    verzeichnis: Option<String>,
}

/// Literatur aus DOI: über den Holer bei Crossref bzw. DataCite holen, in die Literaturdatei des
/// Dokuments schreiben und den Schlüssel zum Zitieren liefern. Nur auf Klick.
#[tauri::command(async)]
fn literatur_aus_doi(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    eingabe: String,
) -> Result<DoiJs, String> {
    if tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).ganz_offline {
        return Err(tr!(
            "„Ganz offline“ ist eingeschaltet: TFTN schlägt keine DOI im Netz nach."
        ));
    }
    let projekt = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .clone()
    };
    let a = pakete::holer_aufrufen(
        &pakete::holer_finden()?,
        &["doi".to_string(), eingabe],
        pakete::HOLER_FRIST,
        &zustand.holer_laeuft,
        netzprotokoll::Ausloeser::Doi,
    )?;
    let (Some(doi), Some(bibtex)) = (a.doi, a.bibtex) else {
        return Err(tr!("Der Holer hat keinen Eintrag gemeldet."));
    };
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.projekt.wurzel != projekt.wurzel {
        return Err(tr!("Inzwischen ist ein anderes Dokument offen."));
    }
    let datei = literatur_dateien(&s.projekt)
        .into_iter()
        .next()
        .ok_or_else(|| tr!("Keine Literaturdatei."))?;
    let e = tftn_literatur::doi::doi_eintrag(&bibtex, &s.literatur, &doi)?;
    if let Some(text) = &e.text {
        tftn_literatur::doi::anhaengen(&datei, text)?;
        protokollieren(
            &s.projekt,
            &format!("Literatur aus DOI {doi}: {}", e.schluessel),
        );
    }
    s.literatur = literatur_laden(&s.projekt);
    let relativ = datei
        .strip_prefix(&s.projekt.wurzel)
        .unwrap_or(&datei)
        .to_string_lossy()
        .replace('\\', "/");
    let verzeichnis = (!typ_regelt_literatur(&s.projekt)
        && !s.spiegel.quelle().text().contains("bibliography("))
    .then(|| format!("\n#bibliography(\"{relativ}\")\n"));
    kompilieren_lassen(&zustand, s);
    Ok(DoiJs {
        schluessel: e.schluessel,
        neu: e.text.is_some(),
        datei: relativ,
        verzeichnis,
    })
}

/// „Nach Updates suchen“: neueste Versionen von TFTN und typst über den Holer, nur auf Klick.
#[tauri::command(async)]
fn updates_pruefen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<pakete::UpdatesJs, String> {
    pakete::darf_holen(&tftn_einstellungen::Rechner::laden(&app_konfig(&app)?))?;
    let a = pakete::holer_aufrufen(
        &pakete::holer_finden()?,
        &["versionen".to_string()],
        pakete::HOLER_FRIST,
        &zustand.holer_laeuft,
        netzprotokoll::Ausloeser::Updates,
    )?;
    let v = a.versionen.unwrap_or_default();
    Ok(pakete::updates_js(
        &v,
        env!("CARGO_PKG_VERSION"),
        tftn_manifest::anlegen::TYPST_VERSION,
    ))
}

#[tauri::command(async)]
fn pakete_suchen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    anfrage: String,
) -> Result<Vec<pakete::TrefferJs>, String> {
    let zs = zwischenspeicher(&app)?;
    let Some((_, index)) = index_laden(&zustand, &zs)? else {
        return Ok(Vec::new());
    };
    let (projekt, katalog) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        (s.projekt.clone(), s.katalog.clone())
    };
    Ok(pakete::treffer_js(
        &index,
        &anfrage,
        &projekt,
        &zs,
        &|n, v| katalog.hat_helfer(n, v),
    ))
}

#[tauri::command(async)]
fn paket_readme(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    spec: String,
) -> Result<Option<String>, String> {
    let spec = tftn_pakete::PaketSpec::lesen(&spec).map_err(|e| e.to_string())?;
    let wurzel = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .wurzel
            .clone()
    };
    Ok(tftn_pakete::readme(
        &wurzel,
        &zwischenspeicher(&app)?,
        &spec,
    ))
}

#[derive(Serialize)]
struct HinzufuegenJs {
    /// Importzeile in der Hauptdatei; der Editor wendet sie als einen Undo-Schritt an.
    patch: Option<PatchJs>,
    hinweis: String,
    warnungen: Vec<String>,
}

/// Holt (falls nötig) und übernimmt `spec` samt Importen ins Projekt; mit `import` auch die
/// Importzeile als Patch gegen `rev`.
#[tauri::command(async)]
fn paket_hinzufuegen(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
    spec: String,
    import: bool,
    rev: u64,
) -> Result<HinzufuegenJs, String> {
    let spec = tftn_pakete::PaketSpec::lesen(&spec).map_err(|e| e.to_string())?;
    let projekt = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .clone()
    };
    let zs = zwischenspeicher(&app)?;
    let rechner = tftn_einstellungen::Rechner::laden(&app_konfig(&app)?);
    let holer = || {
        holer_paket(
            &zustand,
            &zs,
            &projekt,
            &spec,
            netzprotokoll::Ausloeser::PaketHinzufuegen,
        )
    };
    let (u, warnungen) = pakete::hinzufuegen_kern(&projekt, &zs, &rechner, &spec, holer)?;
    if !uebernahme_einspielen(&zustand, &projekt.wurzel)? {
        return Err(tr!(
            "Inzwischen wurde das Projekt gewechselt; die Pakete liegen im alten Projekt."
        ));
    }
    let neu: Vec<String> = u.kopiert.iter().map(ToString::to_string).collect();
    protokollieren(
        &projekt,
        &format!(
            "Paket hinzugefügt: {spec} (neu im Projekt: {})",
            neu.join(", ")
        ),
    );
    let mut hinweis = if neu.is_empty() {
        tr!("{spec} lag schon im Projekt.", spec)
    } else {
        tr!("Hinzugefügt: {namen}.", namen = neu.join(", "))
    };
    let mut patch = None;
    if import {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        let quelle = s.spiegel.quelle();
        let zeile = format!("#import \"{spec}\"");
        if s.spiegel.rev() != rev {
            hinweis.push(' ');
            hinweis.push_str(&tr!(
                "Inzwischen getippt – Importzeile bitte selbst einfügen: {zeile}",
                zeile
            ));
        } else {
            match tftn_katalog::importzeile(quelle, &spec.name, &zeile) {
                Some(e) => {
                    patch = Some(patch_js(
                        quelle,
                        rev,
                        tftn_edit::Patch {
                            bereich: e.byte..e.byte,
                            text: e.text,
                        },
                    )?);
                }
                None => {
                    hinweis.push(' ');
                    hinweis.push_str(&tr!(
                        "{datei} importiert {paket} schon.",
                        datei = projekt.konfig.haupt,
                        paket = spec.name
                    ));
                }
            }
        }
    }
    Ok(HinzufuegenJs {
        patch,
        hinweis,
        warnungen,
    })
}

// --- Fach „Dokument“ (PLAN-M2 AP7) -----------------------------------------------------------

/// Eingebaute Zitierstile von Typst, einmal ermittelt.
fn stile() -> &'static [&'static str] {
    static STILE: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    STILE.get_or_init(tftn_literatur::eingebaute_stile)
}

fn mit_formular<T>(
    zustand: &Zustand,
    f: impl FnOnce(&mut formular::FormularSitzung) -> Result<T, String>,
) -> Result<T, String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    f(&mut s.formular)
}

#[tauri::command]
fn formular(zustand: State<'_, Zustand>) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| Ok(f.laden(stile())))
}

/// Setzt den Wert unter `pfad`; `wert: null` entfernt ihn. `Err("veraltet")`, wenn `stand` nicht
/// mehr der Datei entspricht.
#[tauri::command]
fn formular_setzen(
    stand: String,
    pfad: Vec<tftn_formular::Schritt>,
    wert: Option<serde_json::Value>,
    zustand: State<'_, Zustand>,
) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| f.setzen(&stand, pfad, wert, stile()))
}

#[tauri::command]
fn formular_eintrag(
    stand: String,
    pfad: Vec<tftn_formular::Schritt>,
    aktion: tftn_formular::EintragAktion,
    zustand: State<'_, Zustand>,
) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| f.eintrag(&stand, pfad, aktion, stile()))
}

/// Stand und Text der Datendatei (Fach „Dokument“ → „Als Text bearbeiten“).
#[tauri::command]
fn formular_text(zustand: State<'_, Zustand>) -> Result<(String, String), String> {
    mit_formular(&zustand, |f| f.text_lesen())
}

/// Schreibt die Datendatei als Ganzes; `Err("veraltet")` wie beim Formular, Fehler mit Zeile.
#[tauri::command]
fn formular_text_setzen(
    stand: String,
    text: String,
    zustand: State<'_, Zustand>,
) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| f.text_setzen(&stand, &text, stile()))
}

#[tauri::command]
fn formular_rueckgaengig(
    stand: String,
    zustand: State<'_, Zustand>,
) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| f.rueckgaengig(&stand, stile()))
}

#[tauri::command]
fn formular_wiederholen(
    stand: String,
    zustand: State<'_, Zustand>,
) -> Result<formular::FormularJs, String> {
    mit_formular(&zustand, |f| f.wiederholen(&stand, stile()))
}

#[tauri::command]
fn projekt_info(
    app: tauri::AppHandle,
    zustand: State<'_, Zustand>,
) -> Result<formular::ProjektJs, String> {
    let (quellen, _) = typquellen(&app)?;
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(formular::projekt_js(&s.projekt, &s.formular, &quellen))
}

#[derive(Serialize)]
struct ZitatSuche {
    treffer: Vec<tftn_literatur::Treffer>,
    anzahl: usize,
    dauer_ms: f64,
}

// --- Literaturverwaltung --------------------------------------------------------------------

#[derive(Serialize)]
struct LitEintragJs {
    #[serde(flatten)]
    eintrag: tftn_literatur::verwaltung::BibEintrag,
    /// Im Text zitiert (`@schluessel` oder `<schluessel>`)
    zitiert: bool,
}

#[derive(Serialize)]
struct LitDateiJs {
    /// Relativ zum Projekt, mit `/`
    pfad: String,
    schreibbar: bool,
    fehler: Option<String>,
    eintraege: Vec<LitEintragJs>,
}

fn relativ(projekt: &Projekt, datei: &Path) -> String {
    datei
        .strip_prefix(&projekt.wurzel)
        .unwrap_or(datei)
        .to_string_lossy()
        .replace('\\', "/")
}

fn ist_bib(datei: &Path) -> bool {
    datei
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("bib"))
}

/// Inhalt einer Literaturdatei; fehlt sie, ist sie leer (beim ersten Speichern entsteht sie).
fn bib_lesen(datei: &Path) -> Result<String, String> {
    match std::fs::read_to_string(datei) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("{}: {e}", datei.display())),
    }
}

/// Die Literaturdateien des Dokuments mit ihren Einträgen (Literaturverwaltung).
#[tauri::command]
fn literatur_liste(zustand: State<'_, Zustand>) -> Result<Vec<LitDateiJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(literatur_dateien_js(s))
}

/// Die Literaturdateien des Dokuments samt Einträgen (Fenster „Literatur verwalten“ und MCP).
fn literatur_dateien_js(s: &Sitzung) -> Vec<LitDateiJs> {
    let quelltext = s.spiegel.quelle().text();
    literatur_dateien(&s.projekt)
        .into_iter()
        .map(|d| {
            let pfad = relativ(&s.projekt, &d);
            if !ist_bib(&d) {
                return LitDateiJs {
                    pfad,
                    schreibbar: false,
                    fehler: Some(tr!(
                        "Nur .bib-Dateien lassen sich hier bearbeiten; diese bitte in einem Texteditor."
                    )),
                    eintraege: Vec::new(),
                };
            }
            match bib_lesen(&d).and_then(|t| tftn_literatur::verwaltung::lesen(&t)) {
                Ok(liste) => LitDateiJs {
                    pfad,
                    schreibbar: true,
                    fehler: None,
                    eintraege: liste
                        .into_iter()
                        .map(|g| LitEintragJs {
                            zitiert: tftn_literatur::verwaltung::zitiert(
                                quelltext,
                                &g.eintrag.schluessel,
                            ),
                            eintrag: g.eintrag,
                        })
                        .collect(),
                },
                Err(fehler) => LitDateiJs {
                    pfad,
                    schreibbar: false,
                    fehler: Some(fehler),
                    eintraege: Vec::new(),
                },
            }
        })
        .collect()
}

/// Die Literaturdatei `pfad` (relativ) des Dokuments; nur `.bib`.
fn bib_datei(projekt: &Projekt, pfad: &str) -> Result<PathBuf, String> {
    literatur_dateien(projekt)
        .into_iter()
        .find(|d| relativ(projekt, d) == pfad)
        .filter(|d| ist_bib(d))
        .ok_or_else(|| tr!("{pfad} ist keine Literaturdatei dieses Dokuments.", pfad))
}

/// Schreibt die geänderte Literaturdatei, lädt die Literatur neu und übersetzt.
fn bib_schreiben(
    zustand: &Zustand,
    s: &mut Sitzung,
    datei: &Path,
    text: &str,
    was: &str,
) -> Result<(), String> {
    std::fs::write(datei, text).map_err(|e| format!("{}: {e}", datei.display()))?;
    protokollieren(&s.projekt, was);
    s.literatur = literatur_laden(&s.projekt);
    kompilieren_lassen(zustand, s);
    Ok(())
}

/// Literaturverwaltung: Eintrag ändern (`alt`: sein bisheriger Schlüssel) oder neu anlegen.
#[tauri::command]
fn literatur_eintrag_speichern(
    zustand: State<'_, Zustand>,
    datei: String,
    alt: Option<String>,
    eintrag: tftn_literatur::verwaltung::BibEintrag,
) -> Result<(), String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    literatur_speichern(&zustand, s, &datei, alt, &eintrag)
}

/// Eintrag ändern oder anlegen und schreiben (Fenster und MCP).
fn literatur_speichern(
    zustand: &Zustand,
    s: &mut Sitzung,
    datei: &str,
    alt: Option<String>,
    eintrag: &tftn_literatur::verwaltung::BibEintrag,
) -> Result<(), String> {
    let pfad = bib_datei(&s.projekt, datei)?;
    // Schlüssel über alle Literaturdateien eindeutig (in derselben Datei prüft `verwaltung`).
    if alt.as_deref() != Some(eintrag.schluessel.as_str())
        && s.literatur.enthaelt(&eintrag.schluessel)
    {
        return Err(tr!(
            "Den Schlüssel „{schluessel}“ gibt es schon.",
            schluessel = eintrag.schluessel
        ));
    }
    let text = bib_lesen(&pfad)?;
    let neu = match &alt {
        Some(a) => tftn_literatur::verwaltung::aendern(&text, a, eintrag)?,
        None => tftn_literatur::verwaltung::anlegen(&text, eintrag)?,
    };
    let was = match &alt {
        Some(a) => format!("Literatur: {a} geändert ({})", eintrag.schluessel),
        None => format!("Literatur: {} angelegt", eintrag.schluessel),
    };
    bib_schreiben(zustand, s, &pfad, &neu, &was)
}

/// Literaturverwaltung: Eintrag löschen.
#[tauri::command]
fn literatur_eintrag_loeschen(
    zustand: State<'_, Zustand>,
    datei: String,
    schluessel: String,
) -> Result<(), String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let pfad = bib_datei(&s.projekt, &datei)?;
    let neu = tftn_literatur::verwaltung::loeschen(&bib_lesen(&pfad)?, &schluessel)?;
    bib_schreiben(
        &zustand,
        s,
        &pfad,
        &neu,
        &format!("Literatur: {schluessel} gelöscht"),
    )
}

#[tauri::command]
fn zitate_suchen(anfrage: String, zustand: State<'_, Zustand>) -> Result<ZitatSuche, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let start = std::time::Instant::now();
    let treffer = s.literatur.suchen(&anfrage, 50);
    Ok(ZitatSuche {
        treffer,
        anzahl: s.literatur.len(),
        dauer_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

/// Text zum Einfügen an der UTF-16-Position `pos` (Revision `rev`): `@key[…]` oder `cite(…)`.
#[tauri::command]
fn zitat_text(
    rev: u64,
    pos: usize,
    schluessel: String,
    seite: Option<String>,
    zustand: State<'_, Zustand>,
) -> Result<String, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.spiegel.rev() != rev {
        return Err("veraltet".into());
    }
    let byte = s
        .spiegel
        .quelle()
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    tftn_literatur::einfuegetext(s.spiegel.quelle(), byte, &schluessel, seite.as_deref())
        .map_err(|e| e.to_string())
}

/// Quelle und Seitenangabe des Zitats an der UTF-16-Position `pos` ändern (PLAN-M2 AP11).
#[tauri::command]
fn zitat_aendern(
    rev: u64,
    pos: usize,
    schluessel: Option<String>,
    seite: Option<String>,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let byte = z
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let patch = helfer::zitat_patch_js(
        quelle,
        s.spiegel.rev(),
        rev,
        byte,
        schluessel.as_deref(),
        seite.as_deref(),
    )?;
    Ok(PatchJs {
        rev,
        von: z
            .byte_to_utf16(patch.bereich.start)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        bis: z
            .byte_to_utf16(patch.bereich.end)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        text: patch.text,
    })
}

#[derive(Serialize)]
struct SeitenJs {
    #[serde(flatten)]
    stand: tftn_edit::SeitenStand,
    schriften: Vec<String>,
    /// Das Dokument hat eigene Schriften (`fonts/`).
    eingebunden: bool,
}

/// Fach „Seite“: aktuelle Einstellungen und die Schriften des Dokuments (ohne Formelschriften).
#[tauri::command]
fn seite_stand(zustand: State<'_, Zustand>) -> Result<SeitenJs, String> {
    use typst::World as _;
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let vorschau = zustand.vorschau.lock().unwrap().clone();
    let schriften = vorschau
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?
        .lock()
        .unwrap()
        .world()
        .book()
        .families()
        .map(|(name, _)| name.to_string())
        .filter(|n| !n.contains("Math"))
        .collect();
    Ok(SeitenJs {
        stand: tftn_edit::seite_lesen(s.spiegel.quelle()),
        schriften,
        eingebunden: s.projekt.schriften().is_dir(),
    })
}

/// Schriften nachträglich ins Dokument einbinden.
#[tauri::command(async)]
fn schriften_einbinden(zustand: State<'_, Zustand>) -> Result<(), String> {
    let wurzel = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?
        .projekt
        .wurzel
        .clone();
    typen::schriften_nachreichen(&wurzel)
}

/// Eine Seiteneinstellung ändern.
#[tauri::command]
fn seite_setzen(
    rev: u64,
    feld: tftn_edit::SeitenFeld,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let patch = tftn_edit::seite_patch(quelle, &feld).map_err(|e| e.to_string())?;
    Ok(PatchJs {
        rev,
        von: z
            .byte_to_utf16(patch.bereich.start)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        bis: z
            .byte_to_utf16(patch.bereich.end)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        text: patch.text,
    })
}

// --- Gliederung ------------------------------------------------------------------------------

#[derive(Serialize)]
struct UeberschriftJs {
    ebene: usize,
    titel: String,
    /// UTF-16
    von: usize,
    /// UTF-16: Ende des Abschnitts
    bis: usize,
    /// Wörter im Abschnitt samt Unterabschnitten
    woerter: usize,
}

#[derive(Serialize)]
struct KommentarJs {
    text: String,
    /// UTF-16
    von: usize,
    /// UTF-16
    bis: usize,
}

#[derive(Serialize)]
struct GliederungJs {
    rev: u64,
    eintraege: Vec<UeberschriftJs>,
    kommentare: Vec<KommentarJs>,
    /// Das ganze Dokument
    gesamt: tftn_edit::Zahlen,
}

/// Überschriften der obersten Ebene und Kommentare im jetzigen Text.
#[tauri::command]
fn gliederung_lesen(zustand: State<'_, Zustand>) -> Result<GliederungJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let eintraege = tftn_edit::gliederung(quelle)
        .into_iter()
        .filter_map(|u| {
            Some(UeberschriftJs {
                ebene: u.ebene,
                titel: u.titel,
                von: z.byte_to_utf16(u.von)?,
                bis: z.byte_to_utf16(u.abschnitt.end)?,
                woerter: tftn_edit::zaehlen(quelle, u.abschnitt).woerter,
            })
        })
        .collect();
    Ok(GliederungJs {
        rev: s.spiegel.rev(),
        eintraege,
        kommentare: tftn_edit::kommentare(quelle)
            .into_iter()
            .filter_map(|k| {
                Some(KommentarJs {
                    von: z.byte_to_utf16(k.von)?,
                    bis: z.byte_to_utf16(k.bis)?,
                    text: k.text,
                })
            })
            .collect(),
        gesamt: tftn_edit::zaehlen(quelle, 0..quelle.text().len()),
    })
}

/// Wörter und Zeichen der Auswahl (UTF-16); `None` bei veralteter Revision.
#[tauri::command]
fn zaehlen_bereich(
    rev: u64,
    von: usize,
    bis: usize,
    zustand: State<'_, Zustand>,
) -> Result<Option<tftn_edit::Zahlen>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if helfer::rev_pruefen(s.spiegel.rev(), rev).is_err() {
        return Ok(None);
    }
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let von = z
        .utf16_to_byte(von)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let bis = z
        .utf16_to_byte(bis)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    Ok(Some(tftn_edit::zaehlen(quelle, von..bis)))
}

#[derive(Serialize)]
struct RechtschreibungJs {
    rev: u64,
    /// z. B. „Deutsch (Österreich)“
    sprache: String,
    /// UTF-16-Bereiche unbekannter Wörter in der Hauptdatei
    falsch: Vec<(usize, usize)>,
}

/// Sprache des zuletzt gesetzten Dokuments, sonst `#set text(lang, region)` oben in der Quelle,
/// sonst Deutsch (Österreich).
fn dokumentsprache(zustand: &Zustand, quelle: &Source) -> (String, Option<String>) {
    if let Some(a) = zustand.anzeige.lock().unwrap().as_ref()
        && let Some(s) = tftn_core::sprache::hauptsprache(&a.dokument)
    {
        return s;
    }
    let seite = tftn_edit::seite_lesen(quelle);
    match seite.sprache {
        Some(l) => (l, seite.region),
        None => ("de".into(), Some("AT".into())),
    }
}

/// Unbekannte Wörter im Fließtext nach Dokumentsprache; `None`, wenn `rev` überholt ist.
#[tauri::command(async)]
fn rechtschreibung_pruefen(
    app: tauri::AppHandle,
    rev: u64,
    zustand: State<'_, Zustand>,
) -> Result<Option<RechtschreibungJs>, String> {
    let quelle = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if helfer::rev_pruefen(s.spiegel.rev(), rev).is_err() {
            return Ok(None);
        }
        s.spiegel.quelle().clone()
    };
    let (sprache, falsch) = falsche_woerter(&app, &zustand, &quelle, &quelle)?;
    let z = quelle.lines();
    let falsch = falsch
        .into_iter()
        .filter_map(|r| Some((z.byte_to_utf16(r.start)?, z.byte_to_utf16(r.end)?)))
        .collect();
    Ok(Some(RechtschreibungJs {
        rev,
        sprache,
        falsch,
    }))
}

/// Unbekannte Wörter in `quelle` (Bytebereiche) nach der Sprache des Dokuments mit der Hauptquelle
/// `haupt`, dazu die Bezeichnung der Sprache. Ohne Wörterbuch für die Sprache: keine.
fn falsche_woerter(
    app: &tauri::AppHandle,
    zustand: &Zustand,
    haupt: &Source,
    quelle: &Source,
) -> Result<(String, Vec<std::ops::Range<usize>>), String> {
    let (lang, region) = dokumentsprache(zustand, haupt);
    let sprache = rechtschreibung::bezeichnung(&lang, region.as_deref());
    let Some(kennung) = tftn_edit::rechtschreibung::woerterbuch_fuer(&lang, region.as_deref())
    else {
        return Ok((sprache, Vec::new()));
    };
    let konfig = app_konfig(app)?;
    let einstellungen =
        tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig)).ordner;
    let mut stand = zustand.rechtschreibung.lock().unwrap();
    rechtschreibung::Stand::holen(
        &mut stand,
        &rechtschreibung::ordner(app.path().resource_dir().ok()),
        &einstellungen,
        kennung,
    )?;
    let pruefer = &stand.as_ref().expect("eben geholt").pruefer;
    Ok((
        sprache,
        tftn_edit::rechtschreibung::falsche_woerter(quelle, pruefer),
    ))
}

/// Vorschläge aus dem zuletzt gebrauchten Wörterbuch.
#[tauri::command(async)]
fn rechtschreibung_vorschlaege(wort: String, zustand: State<'_, Zustand>) -> Vec<String> {
    match zustand.rechtschreibung.lock().unwrap().as_ref() {
        Some(s) => s.pruefer.vorschlaege(&wort),
        None => Vec::new(),
    }
}

/// Wort ins eigene Wörterbuch (`woerter.txt` im Einstellungsordner).
#[tauri::command(async)]
fn wort_aufnehmen(wort: String, zustand: State<'_, Zustand>) -> Result<(), String> {
    match zustand.rechtschreibung.lock().unwrap().as_mut() {
        Some(s) => s.aufnehmen(&wort),
        None => Err(tr!("Noch kein Wörterbuch geladen.")),
    }
}

/// Abschnitt `nr` samt Unterabschnitten vor Abschnitt `vor` (`None`: ans Ende) verschieben.
#[tauri::command]
fn abschnitt_verschieben(
    rev: u64,
    nr: usize,
    vor: Option<usize>,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let patch = tftn_edit::abschnitt_verschieben(quelle, nr, vor).map_err(|e| e.to_string())?;
    patch_js(quelle, rev, patch)
}

/// Fach „Hier“ (PLAN-M2 AP6): Bausteine, Markup und Tabelle an der UTF-16-Position `pos`;
/// `None` bei veralteter Revision.
#[tauri::command]
fn hier(
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<Option<helfer::HierJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if helfer::rev_pruefen(s.spiegel.rev(), rev).is_err() {
        return Ok(None);
    }
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    Ok(Some(helfer::hier_js(quelle, &s.katalog, byte, rev, &|k| {
        s.literatur.enthaelt(k)
    })))
}

/// Marken B (PLAN-M2 AP6): alle Widgets im UTF-16-Bereich `von..bis`; leer bei veralteter Revision.
#[tauri::command]
fn marken(
    rev: u64,
    von: usize,
    bis: usize,
    zustand: State<'_, Zustand>,
) -> Result<Vec<helfer::MarkeJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if helfer::rev_pruefen(s.spiegel.rev(), rev).is_err() {
        return Ok(Vec::new());
    }
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let von = z
        .utf16_to_byte(von)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let bis = z
        .utf16_to_byte(bis)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    Ok(helfer::marken_js(quelle, &s.katalog, von..bis))
}

/// Modus an der UTF-16-Position `pos` (Revision `rev`) für Menü „/“ und Kürzel (PLAN-M2 AP2);
/// `None` bei veralteter Revision, in Kommentaren und im Text eines Raw-Blocks.
#[tauri::command]
fn modus(
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<Option<tftn_manifest::Modus>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.spiegel.rev() != rev {
        return Ok(None);
    }
    let byte = s
        .spiegel
        .quelle()
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    Ok(tftn_katalog::modus_an(s.spiegel.quelle(), byte))
}

#[derive(Serialize)]
struct EintragJs {
    id: String,
    titel: String,
    gruppe: Option<String>,
    /// `typst`, `dokumenttyp`, `paket` oder `holen` (Paket mit Beschreibung fehlt im Projekt)
    herkunft: String,
    modus: Vec<tftn_manifest::Modus>,
    kuerzel: Option<String>,
}

/// Alle Bausteine des Projekts (für Menü und Kürzel, PLAN-M2 AP2/AP3).
#[tauri::command]
fn katalog(zustand: State<'_, Zustand>) -> Result<Vec<EintragJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(s.katalog
        .eintraege()
        .iter()
        .map(|e| EintragJs {
            id: e.id.clone(),
            titel: e.titel.clone(),
            gruppe: e.gruppe.clone(),
            herkunft: e.herkunft.art().into(),
            modus: e.modus.clone(),
            kuerzel: e.kuerzel.clone(),
        })
        .chain(
            pakete::holbar_eintraege(&s.katalog)
                .into_iter()
                .map(|(id, titel)| EintragJs {
                    id,
                    titel,
                    gruppe: None,
                    herkunft: "holen".into(),
                    modus: vec![tftn_manifest::Modus::Markup],
                    kuerzel: None,
                }),
        )
        .collect())
}

#[derive(Serialize)]
struct FeldJs {
    name: String,
    von: usize,
    bis: usize,
}

#[derive(Serialize)]
struct ImportJs {
    /// UTF-16-Position im Dokument
    pos: usize,
    text: String,
}

#[derive(Serialize)]
struct EingesetztJs {
    text: String,
    felder: Vec<FeldJs>,
    cursor: Option<usize>,
    /// Importzeile für Paketbausteine, im selben Undo-Schritt einzufügen.
    import: Option<ImportJs>,
}

/// Schnipsel des Bausteins `id` mit `auswahl` zum Einfügen an der UTF-16-Position `pos` (Revision
/// `rev`); Feldpositionen in UTF-16 relativ zum Anfang von `text`, dazu die nötige Importzeile.
#[tauri::command]
fn baustein_text(
    id: String,
    auswahl: String,
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<EingesetztJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.spiegel.rev() != rev {
        return Err("veraltet".into());
    }
    let quelle = s.spiegel.quelle();
    let stelle = quelle
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let eintrag = s
        .katalog
        .finden(&id)
        .ok_or_else(|| tr!("unbekannter Baustein: {id}", id))?;
    let import = match tftn_katalog::import_fuer_an(quelle, eintrag, stelle) {
        Some(i) => Some(ImportJs {
            pos: quelle
                .lines()
                .byte_to_utf16(i.byte)
                .ok_or_else(|| tr!("Position außerhalb"))?,
            text: i.text,
        }),
        None => None,
    };
    // Mehrzeilige Bausteine bekommen den Einzug der Zeile (im Listenpunkt bis hinter die Marke).
    let text = quelle.text();
    let vor = &text[text[..stelle].rfind('\n').map_or(0, |i| i + 1)..stelle];
    let e = eintrag
        .schnipsel
        .einsetzen_eingerueckt(&auswahl, &tftn_katalog::einzug_fuer(vor))
        .utf16();
    Ok(EingesetztJs {
        text: e.text,
        felder: e
            .felder
            .into_iter()
            .map(|f| FeldJs {
                name: f.name,
                von: f.bereich.start,
                bis: f.bereich.end,
            })
            .collect(),
        cursor: e.cursor,
        import,
    })
}

#[derive(Serialize)]
struct PatchJs {
    rev: u64,
    von: usize,
    bis: usize,
    text: String,
}

/// Patch → UTF-16-Bereich der Revision `rev`.
fn patch_js(quelle: &Source, rev: u64, patch: tftn_edit::Patch) -> Result<PatchJs, String> {
    let z = quelle.lines();
    Ok(PatchJs {
        rev,
        von: z
            .byte_to_utf16(patch.bereich.start)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        bis: z
            .byte_to_utf16(patch.bereich.end)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        text: patch.text,
    })
}

// --- Fach „Formel“ (PLAN-M2 AP10) --------------------------------------------------------------

/// Stand am Cursor: Text der Formel dort bzw. ob eine neue hierher darf.
#[tauri::command]
fn formel_stand(
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<formel::FormelStandJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    Ok(formel::stand(quelle, byte))
}

#[tauri::command(async)]
fn formel_vorschau(
    text: String,
    block: bool,
    zustand: State<'_, Zustand>,
) -> Result<formel::FormelBildJs, String> {
    let (projekt, haupt) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        (s.projekt.clone(), s.spiegel.text().to_string())
    };
    let mut merker = zustand.formel_world.lock().unwrap();
    if merker.as_ref().is_none_or(|(w, _)| *w != projekt.wurzel) {
        let world = tftn_core::ProjektWorld::neu(&projekt).map_err(|e| e.to_string())?;
        *merker = Some((projekt.wurzel.clone(), world));
    }
    let (_, world) = merker.as_mut().expect("eben gesetzt");
    Ok(formel::vorschau(world, &haupt, &text, block))
}

/// Neuer Text der Formel bei `pos`; ein Patch, ein Undo-Schritt. `erwartet` ist der Text, den das
/// Feld geladen hat: Steht dort inzwischen etwas anderes, wird nichts geschrieben.
#[tauri::command]
fn formel_setzen(
    rev: u64,
    pos: usize,
    erwartet: String,
    text: String,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let patch = formel::setzen_patch(quelle, byte, &erwartet, &text)?;
    patch_js(quelle, rev, patch)
}

/// Neue Formel am Cursor.
#[tauri::command]
fn formel_einfuegen(
    rev: u64,
    pos: usize,
    text: String,
    block: bool,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let patch =
        tftn_edit::formel_einfuegen(quelle, byte, &text, block).map_err(|e| e.to_string())?;
    patch_js(quelle, rev, patch)
}

#[tauri::command]
fn formel_palette() -> formel::PaletteJs {
    formel::palette()
}

/// Text eines Bausteins der Palette, für das Formelfeld; Positionen in UTF-16.
#[tauri::command]
fn formel_baustein(id: String, auswahl: String) -> Result<EingesetztJs, String> {
    let e = tftn_katalog::eingebaut()
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| tr!("unbekannter Baustein: {id}", id))?;
    let e = e.schnipsel.einsetzen(&auswahl).utf16();
    Ok(EingesetztJs {
        text: e.text,
        felder: e
            .felder
            .into_iter()
            .map(|f| FeldJs {
                name: f.name,
                von: f.bereich.start,
                bis: f.bereich.end,
            })
            .collect(),
        cursor: e.cursor,
        import: None,
    })
}

/// Neuer Wert für `parameter` am Aufruf, der bei der UTF-16-Position `aufruf` beginnt (PLAN-M2
/// AP6); geprüft nach KONZEPT 4.5. `Err("veraltet")`, wenn sich der Text inzwischen geändert hat.
#[tauri::command]
fn wert_setzen(
    rev: u64,
    aufruf: usize,
    parameter: String,
    wert: tftn_edit::Wert,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(aufruf)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let patch =
        helfer::wert_patch_js(quelle, &s.katalog, byte, &parameter, &wert).map_err(|e| {
            protokollieren(&s.projekt, &format!("Widget-Patch verworfen: {e}"));
            e
        })?;
    patch_js(quelle, rev, patch)
}

/// Tabellenraster (PLAN-M2 AP5/AP6): `aktion` an der Tabelle, deren Aufruf bei der
/// UTF-16-Position `aufruf` beginnt. `Err("veraltet")`, wenn sich der Text inzwischen geändert hat.
#[tauri::command]
fn tabelle_aktion(
    rev: u64,
    aufruf: usize,
    aktion: tftn_edit::TabellenAktion,
    zustand: State<'_, Zustand>,
) -> Result<PatchJs, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    helfer::rev_pruefen(s.spiegel.rev(), rev)?;
    let quelle = s.spiegel.quelle();
    let byte = quelle
        .lines()
        .utf16_to_byte(aufruf)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    let patch = helfer::tabelle_patch_js(quelle, byte, &aktion).map_err(|e| {
        protokollieren(&s.projekt, &format!("Tabellen-Patch verworfen: {e}"));
        e
    })?;
    patch_js(quelle, rev, patch)
}

#[derive(Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
enum KlickAntwort {
    /// UTF-16-Position in der Hauptdatei, bezogen auf Revision `rev`.
    Haupt {
        rev: u64,
        pos: usize,
    },
    Datei {
        datei: String,
    },
    /// Titelei: das Formularfeld im Fach „Dokument“ (PLAN-M2 AP7).
    Feld {
        feld: String,
    },
    Vorlage,
    Url {
        url: String,
    },
    Position {
        seite: usize,
        x: f64,
        y: f64,
    },
    Nichts,
}

fn vorschau_von(zustand: &Zustand) -> Result<Arc<Mutex<Vorschau>>, String> {
    zustand
        .vorschau
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))
}

/// Vorschau → Quelle (KONZEPT 4.4).
#[tauri::command]
async fn klick(
    seite: usize,
    x: f64,
    y: f64,
    zustand: State<'_, Zustand>,
) -> Result<KlickAntwort, String> {
    let vorschau = vorschau_von(&zustand)?;
    let v = vorschau.lock().unwrap();
    Ok(match v.klick(seite, x, y) {
        Sprung::Haupt(byte) => {
            let quelle = v
                .world()
                .source(v.world().haupt())
                .map_err(|e| e.to_string())?;
            let pos = quelle
                .lines()
                .byte_to_utf16(byte)
                .ok_or_else(|| tr!("Position außerhalb"))?;
            KlickAntwort::Haupt {
                rev: v.quelle_rev(),
                pos,
            }
        }
        Sprung::Datei(datei, _) => KlickAntwort::Datei { datei },
        Sprung::Feld(feld) => KlickAntwort::Feld { feld },
        Sprung::Vorlage => KlickAntwort::Vorlage,
        Sprung::Url(url) => KlickAntwort::Url { url },
        Sprung::Position(seite, x, y) => KlickAntwort::Position { seite, x, y },
        Sprung::Nichts => KlickAntwort::Nichts,
    })
}

/// Quelle → Vorschau: Punkte (Seite 0-basiert, x, y in pt) zur UTF-16-Position `pos` in Revision `rev`.
#[tauri::command]
async fn stelle_zeigen(
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<Vec<(usize, f64, f64)>, String> {
    let (quelle, byte) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.rev() != rev {
            return Ok(Vec::new());
        }
        let byte = s
            .spiegel
            .quelle()
            .lines()
            .utf16_to_byte(pos)
            .ok_or_else(|| tr!("Position außerhalb"))?;
        (s.spiegel.quelle().clone(), byte)
    };
    let vorschau = vorschau_von(&zustand)?;
    let v = vorschau.lock().unwrap();
    Ok(v.stellen(&quelle, byte))
}

#[derive(Serialize)]
struct VorschlagJs {
    titel: String,
    detail: Option<String>,
    art: &'static str,
    text: String,
    felder: Vec<FeldJs>,
    cursor: Option<usize>,
}

#[derive(Serialize)]
struct VervollstaendigungJs {
    /// UTF-16-Anfang des zu ersetzenden Bereichs
    von: usize,
    vorschlaege: Vec<VorschlagJs>,
}

/// Strg+Leertaste (PLAN-M2 AP2): Vorschläge von `typst_ide` an der UTF-16-Position `pos` (Revision
/// `rev`); `None` bei veralteter Revision oder ohne Vorschläge.
#[tauri::command]
async fn vervollstaendigen(
    rev: u64,
    pos: usize,
    zustand: State<'_, Zustand>,
) -> Result<Option<VervollstaendigungJs>, String> {
    let (quelle, byte) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.rev() != rev {
            return Ok(None);
        }
        let byte = s
            .spiegel
            .quelle()
            .lines()
            .utf16_to_byte(pos)
            .ok_or_else(|| tr!("Position außerhalb"))?;
        (s.spiegel.quelle().clone(), byte)
    };
    let vorschau = vorschau_von(&zustand)?;
    let v = vorschau.lock().unwrap();
    let dokument = v.gut().map(|g| &g.dokument);
    let Some((von, liste)) =
        tftn_katalog::ide::vervollstaendigen(v.world(), dokument, &quelle, byte)
    else {
        return Ok(None);
    };
    let utf16 = |b: usize| {
        quelle
            .lines()
            .byte_to_utf16(b)
            .ok_or_else(|| tr!("Position außerhalb"))
    };
    Ok(Some(VervollstaendigungJs {
        von: utf16(von)?,
        vorschlaege: liste
            .into_iter()
            .map(|v| {
                let e = v.eingesetzt.utf16();
                VorschlagJs {
                    titel: v.titel,
                    detail: v.detail,
                    art: v.art,
                    text: e.text,
                    felder: e
                        .felder
                        .into_iter()
                        .map(|f| FeldJs {
                            name: f.name,
                            von: f.bereich.start,
                            bis: f.bereich.end,
                        })
                        .collect(),
                    cursor: e.cursor,
                }
            })
            .collect(),
    }))
}

/// Speichert atomar: in eine Nachbardatei schreiben, auf die Platte zwingen, umbenennen.
#[tauri::command]
fn speichern(zustand: State<'_, Zustand>) -> Result<(), String> {
    sitzung_speichern(&zustand)
}

/// Schreibt den Text der Sitzung, legt bei Bedarf eine Fassung ab und ergänzt eingebundene Schriften.
fn sitzung_speichern(zustand: &Zustand) -> Result<(), String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let name = s
        .datei
        .file_name()
        .ok_or_else(|| tr!("Datei ohne Namen"))?
        .to_string_lossy()
        .into_owned();
    let zwischen = s.datei.with_file_name(format!(".{name}.tftn-speichern"));
    let schreiben = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&zwischen)?;
        f.write_all(s.spiegel.text().as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&zwischen, &s.datei)
    };
    // Erst die Prüfsumme setzen, damit der Dateiwächter die eigene Speicherung erkennt.
    let vorher = s.auf_platte;
    s.auf_platte = s.spiegel.pruefsumme();
    if let Err(e) = schreiben() {
        s.auf_platte = vorher;
        let _ = std::fs::remove_file(&zwischen);
        return Err(tr!("Speichern fehlgeschlagen: {fehler}", fehler = e));
    }
    let fassungen = s.projekt.wurzel.join(fassungen::ORDNER);
    if let Err(e) = fassungen::ablegen(&fassungen, s.spiegel.text(), fassungen::jetzt(), false) {
        eprintln!("Keine Fassung abgelegt: {e}");
    }
    // Eingebundene Schriften ergänzen, wenn das Dokument inzwischen andere benutzt. Das liest den
    // ganzen Vorrat, deshalb nur, wenn sich die benutzten Schriften geändert haben.
    if s.projekt.schriften().is_dir()
        && let Some(v) = zustand.vorschau.lock().unwrap().clone()
        && let Some(gut) = v.lock().unwrap().gut()
    {
        let abdruck = tftn_core::schriften::fingerabdruck(&gut.dokument);
        if abdruck != s.schriften_abdruck {
            match tftn_core::schriften::einbinden(&s.projekt, &gut.dokument) {
                Ok(_) => s.schriften_abdruck = abdruck,
                Err(e) => eprintln!("Schriften nicht ergänzt: {e}"),
            }
        }
    }
    Ok(())
}

/// Beim Schließen des Fensters oder Beenden: Ungesichertes noch schreiben.
fn beim_schliessen(zustand: &Zustand) {
    let offen = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|s| s.spiegel.pruefsumme() != s.auf_platte);
    if offen && let Err(e) = sitzung_speichern(zustand) {
        eprintln!("Beim Schließen nicht gespeichert: {e}");
    }
}

// --- Word ---------------------------------------------------------------------------------

/// Export nach Word über typst (HTML) und Pandoc; die Datei liegt im Dokumentordner.
#[tauri::command(async)]
fn word_exportieren(zustand: State<'_, Zustand>) -> Result<word::WordJs, String> {
    let projekt = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        if s.spiegel.pruefsumme() != s.auf_platte {
            return Err(tr!(
                "Bitte zuerst speichern: exportiert wird die Datei auf der Platte."
            ));
        }
        s.projekt.clone()
    };
    let pandoc = import::pandoc_finden().ok_or_else(import::fehlt_fuer_word)?;
    let cli = tftn_export::TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
    word::exportieren(&projekt, &cli, &pandoc)
}

/// Eine Datei des offenen Dokuments im Finder bzw. Explorer zeigen.
#[tauri::command]
fn im_ordner_zeigen(pfad: String, zustand: State<'_, Zustand>) -> Result<(), String> {
    let wurzel = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .wurzel
            .clone()
    };
    let p = word::zeigbar(&wurzel, Path::new(&pfad))
        .ok_or_else(|| tr!("Die Datei gibt es nicht mehr."))?;
    word::im_ordner_zeigen(&p)
}

// --- Bilder ---------------------------------------------------------------------------------

/// Ein hineingezogenes oder eingefügtes Bild unter `bilder/` ablegen. Rumpf: die Bytes,
/// Kopfzeile `tftn-name`: der Dateiname, prozentkodiert. Liefert den Pfad für `image(…)`.
#[tauri::command]
fn bild_ablegen(
    request: tauri::ipc::Request<'_>,
    zustand: State<'_, Zustand>,
) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(daten) = request.body() else {
        return Err(tr!("keine Bilddaten"));
    };
    let name = request
        .headers()
        .get("tftn-name")
        .and_then(|v| v.to_str().ok())
        .and_then(bilder::prozent_dekodieren)
        .ok_or_else(|| tr!("Dateiname fehlt"))?;
    let wurzel = {
        let sitzung = zustand.sitzung.lock().unwrap();
        sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?
            .projekt
            .wurzel
            .clone()
    };
    bilder::ablegen(&wurzel, &name, daten)
}

// --- Frühere Fassungen ----------------------------------------------------------------------

#[tauri::command]
fn fassungen_liste(zustand: State<'_, Zustand>) -> Result<Vec<fassungen::FassungJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    Ok(fassungen::liste(
        &s.projekt.wurzel.join(fassungen::ORDNER),
        s.spiegel.text(),
    ))
}

#[tauri::command]
fn fassung_text(name: String, zustand: State<'_, Zustand>) -> Result<String, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    fassungen::lesen(&s.projekt.wurzel.join(fassungen::ORDNER), &name)
}

/// Hält den Text jetzt als Fassung fest und liefert den der gewählten; ersetzt wird im Editor.
#[tauri::command]
fn fassung_zurueckholen(name: String, zustand: State<'_, Zustand>) -> Result<String, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let ordner = s.projekt.wurzel.join(fassungen::ORDNER);
    let alt = fassungen::lesen(&ordner, &name)?;
    fassungen::ablegen(&ordner, s.spiegel.text(), fassungen::jetzt(), true).map_err(|e| {
        tr!(
            "Der jetzige Stand ließ sich nicht festhalten: {fehler}",
            fehler = e
        )
    })?;
    Ok(alt)
}

// --- Einstellungen und Kürzel (PLAN-M2 AP3) -------------------------------------------------

#[derive(Serialize, Clone)]
struct EinstellungenJs {
    /// Ordner mit `tasten.toml` und `einstellungen.toml`
    ordner: String,
    synchronisiert: bool,
    hinweis: Option<String>,
    ganz_offline: bool,
    /// eigene Belegung: Aktion → Kürzel (`""` = keine)
    kuerzel: std::collections::BTreeMap<String, String>,
    /// `tasten.toml` fehlerhaft: Umbelegen ist gesperrt
    fehler: Option<String>,
    /// einzelne Einträge in `tasten.toml` übersprungen; Umbelegen geht weiter
    warnung: Option<String>,
}

fn einstellungen_js(app_konfig: &Path) -> EinstellungenJs {
    let rechner = tftn_einstellungen::Rechner::laden(app_konfig);
    let ort = tftn_einstellungen::ort(app_konfig, &rechner);
    let tasten = tftn_einstellungen::tasten_lesen(&ort.ordner);
    EinstellungenJs {
        ordner: ort.ordner.display().to_string(),
        synchronisiert: ort.synchronisiert,
        hinweis: ort.hinweis,
        ganz_offline: rechner.ganz_offline,
        kuerzel: tasten.kuerzel,
        fehler: tasten.fehler,
        warnung: tasten.warnung,
    }
}

fn app_konfig(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    Ok(d)
}

/// Eine Paketbeschreibung hat sich geändert: Katalog neu bauen, ohne Neustart (Entwurf 8.3).
fn katalog_erneuern(app: &tauri::AppHandle, einstellungsordner: &Path) {
    let zustand = app.state::<Zustand>();
    *zustand.beschreibungen.lock().unwrap() = pakete::beschreibungs_ordner(
        &pakete::beschreibungen_buendel(app.path().resource_dir().ok()),
        einstellungsordner,
    );
    let mut sitzung = zustand.sitzung.lock().unwrap();
    if let Some(s) = sitzung.as_mut() {
        s.katalog = katalog_bauen(&zustand, &s.projekt);
        let _ = app.emit("katalog-geaendert", ());
    }
}

/// Setzt den Wächter auf den geltenden Ordner; jede Änderung von außen (Sync, Texteditor) und
/// jede eigene Speicherung kommt als Ereignis `einstellungen-geaendert`.
fn einstellungen_beobachten(app: &tauri::AppHandle) -> Result<(), String> {
    let konfig = app_konfig(app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    *app.state::<Zustand>().beschreibungen.lock().unwrap() = pakete::beschreibungs_ordner(
        &pakete::beschreibungen_buendel(app.path().resource_dir().ok()),
        &ort.ordner,
    );
    let handle = app.clone();
    let einstellungsordner = ort.ordner.clone();
    let w = tftn_einstellungen::beobachten(&ort.ordner, move || {
        let _ = handle.emit("einstellungen-geaendert", einstellungen_js(&konfig));
        katalog_erneuern(&handle, &einstellungsordner);
    })
    .map_err(|e| e.to_string())?;
    *app.state::<Zustand>()
        .einstellungen_waechter
        .lock()
        .unwrap() = Some(w);
    Ok(())
}

#[tauri::command]
fn einstellungen(app: tauri::AppHandle) -> Result<EinstellungenJs, String> {
    Ok(einstellungen_js(&app_konfig(&app)?))
}

/// Eigene Belegung einer Aktion setzen (`Some`, `""` = keine) oder entfernen (`None` = Standard).
#[tauri::command]
fn kuerzel_setzen(
    app: tauri::AppHandle,
    aktion: String,
    kuerzel: Option<String>,
) -> Result<EinstellungenJs, String> {
    let konfig = app_konfig(&app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    tftn_einstellungen::kuerzel_setzen(&ort.ordner, &aktion, kuerzel.as_deref())?;
    Ok(einstellungen_js(&konfig))
}

/// Ordner im Tresor setzen (`ordner`), wählen (`waehlen`) oder zurücksetzen (beides leer). Fehlende
/// Dateien ziehen aus dem bisherigen Ordner mit; vorhandene im neuen Ordner gewinnen.
#[tauri::command(async)]
fn einstellungsordner_setzen(
    app: tauri::AppHandle,
    ordner: Option<String>,
    waehlen: bool,
) -> Result<EinstellungenJs, String> {
    let konfig = app_konfig(&app)?;
    let mut rechner = tftn_einstellungen::Rechner::laden(&konfig);
    let neu = if waehlen {
        match ordner_waehlen(&tr!("Ordner für eigene Anpassungen wählen")) {
            Some(p) => Some(p),
            None if cfg!(target_os = "macos") => return Ok(einstellungen_js(&konfig)),
            None => {
                return Err(tr!(
                    "Ordnerwahl gibt es hier noch nicht – bitte den Pfad eintragen."
                ));
            }
        }
    } else {
        ordner.as_deref().and_then(typen::pfad_aus_eingabe)
    };
    if let Some(n) = &neu {
        if !n.is_dir() {
            return Err(tr!(
                "{ordner} ist kein vorhandener Ordner.",
                ordner = n.display()
            ));
        }
        let alt = tftn_einstellungen::ort(&konfig, &rechner).ordner;
        if &alt != n {
            tftn_einstellungen::umziehen(&alt, n).map_err(|e| e.to_string())?;
        }
    }
    rechner.ordner = neu;
    rechner.speichern(&konfig).map_err(|e| e.to_string())?;
    einstellungen_beobachten(&app)?;
    Ok(einstellungen_js(&konfig))
}

/// Zeigt den Ordner für eigene Anpassungen (gewählt oder Standard) im Finder bzw. Explorer.
#[tauri::command]
fn einstellungsordner_zeigen(app: tauri::AppHandle) -> Result<(), String> {
    let konfig = app_konfig(&app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    word::im_ordner_zeigen(&ort.ordner)
}

// --- Sprache der Oberfläche ---------------------------------------------------------------

/// Ordner der mitgelieferten und der eigenen Sprachdateien.
fn sprachordner(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let konfig = app_konfig(app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    Ok((
        sprachen::mitgeliefert(app.path().resource_dir().ok()),
        sprachen::eigene(&ort.ordner),
    ))
}

#[derive(Serialize)]
struct OberflaecheJs {
    kennung: String,
    texte: std::collections::HashMap<String, String>,
}

/// Lädt die Sprache der Oberfläche (aus den Einstellungen, sonst wie `system`) für diesen Prozess
/// und gibt ihre Texte an die Oberfläche.
#[tauri::command]
fn oberflaeche(app: tauri::AppHandle, system: String) -> Result<OberflaecheJs, String> {
    let (mit, eigene) = sprachordner(&app)?;
    let wunsch = tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).sprache;
    let k = sprachen::laden(
        &sprachen::verfuegbar(&mit, &eigene),
        wunsch.as_deref(),
        &system,
    );
    let antwort = OberflaecheJs {
        kennung: k
            .as_ref()
            .map_or_else(|| "de".into(), |k| k.kennung.clone()),
        texte: k.as_ref().map(|k| k.texte.clone()).unwrap_or_default(),
    };
    tftn_sprachen::setzen(k);
    Ok(antwort)
}

#[derive(Serialize)]
struct SpracheJs {
    kennung: String,
    name: String,
}

#[derive(Serialize)]
struct SprachenJs {
    /// `None`: wie das System
    gewaehlt: Option<String>,
    /// Die geltende Sprache
    aktuell: String,
    liste: Vec<SpracheJs>,
    /// Ordner für eigene Sprachdateien
    ordner: String,
}

#[tauri::command]
fn sprachen_lesen(app: tauri::AppHandle) -> Result<SprachenJs, String> {
    let (mit, eigene) = sprachordner(&app)?;
    let mut liste = vec![SpracheJs {
        kennung: "de".into(),
        name: "Deutsch".into(),
    }];
    liste.extend(
        sprachen::verfuegbar(&mit, &eigene)
            .into_iter()
            .map(|s| SpracheJs {
                kennung: s.kennung,
                name: s.name,
            }),
    );
    Ok(SprachenJs {
        gewaehlt: tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).sprache,
        aktuell: tftn_sprachen::kennung(),
        liste,
        ordner: eigene.display().to_string(),
    })
}

/// Sprache der Oberfläche für diesen Rechner; `None` = wie das System. Gilt nach dem Neuladen.
#[tauri::command]
fn sprache_setzen(app: tauri::AppHandle, kennung: Option<String>) -> Result<(), String> {
    let konfig = app_konfig(&app)?;
    rechner_aendern(&konfig, |r| r.sprache = kennung.filter(|k| !k.is_empty()))
}

/// Einstellungen dieses Rechners laden, ändern, speichern.
fn rechner_aendern<T>(
    konfig: &Path,
    f: impl FnOnce(&mut tftn_einstellungen::Rechner) -> T,
) -> Result<T, String> {
    tftn_einstellungen::Rechner::aendern(konfig, f).map_err(|e| e.to_string())
}

// --- Skins ---------------------------------------------------------------------------------

/// Ordner der mitgelieferten und der eigenen Skins.
fn skinordner(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let konfig = app_konfig(app)?;
    let ort = tftn_einstellungen::ort(&konfig, &tftn_einstellungen::Rechner::laden(&konfig));
    Ok((
        skins::mitgeliefert(app.path().resource_dir().ok()),
        skins::eigene(&ort.ordner),
    ))
}

#[derive(Serialize)]
struct SkinsJs {
    /// gewählter Skin; `None` = hell/dunkel wie das System
    gewaehlt: Option<String>,
    /// der geltende Skin (`None` auch, wenn der gewählte fehlt)
    aktiv: Option<skins::Skin>,
    liste: Vec<skins::SkinEintrag>,
    /// Ordner für eigene Skins
    ordner: String,
}

fn skins_js(app: &tauri::AppHandle) -> Result<SkinsJs, String> {
    let (mit, eigene) = skinordner(app)?;
    let gewaehlt = tftn_einstellungen::Rechner::laden(&app_konfig(app)?).skin;
    Ok(SkinsJs {
        aktiv: gewaehlt
            .as_deref()
            .and_then(|k| skins::laden(&mit, &eigene, k)),
        gewaehlt,
        liste: skins::verfuegbar(&mit, &eigene),
        ordner: eigene.display().to_string(),
    })
}

/// Skins und der geltende; beim Start und für die Einstellungen.
#[tauri::command]
fn skins_lesen(app: tauri::AppHandle) -> Result<SkinsJs, String> {
    skins_js(&app)
}

/// Größe der Oberfläche (Prozent) für diesen Rechner.
#[tauri::command]
fn oberflaeche_lesen(app: tauri::AppHandle) -> Result<u16, String> {
    Ok(tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).oberflaeche_prozent())
}

/// Setzt die Größe der Oberfläche (eine der Stufen); gibt die geltende zurück. Gilt sofort.
#[tauri::command]
fn oberflaeche_setzen(app: tauri::AppHandle, prozent: u16) -> Result<u16, String> {
    let konfig = app_konfig(&app)?;
    rechner_aendern(&konfig, |r| r.oberflaeche_setzen(prozent))
}

/// Skin für diesen Rechner; `None` = wie das System. Gilt sofort.
#[tauri::command]
fn skin_setzen(app: tauri::AppHandle, kennung: Option<String>) -> Result<SkinsJs, String> {
    let konfig = app_konfig(&app)?;
    rechner_aendern(&konfig, |r| r.skin = kennung.filter(|k| !k.is_empty()))?;
    skins_js(&app)
}

/// Legt die Vorlage für einen eigenen Skin an und zeigt sie im Finder bzw. Explorer.
#[tauri::command]
fn skinvorlage_anlegen(app: tauri::AppHandle) -> Result<String, String> {
    let (mit, eigene) = skinordner(&app)?;
    let p = skins::vorlage_anlegen(&mit, &eigene)?;
    word::im_ordner_zeigen(&p)?;
    Ok(p.display().to_string())
}

/// Legt die Vorlage für eine neue Sprache an und zeigt sie im Finder bzw. Explorer.
#[tauri::command]
fn sprachvorlage_anlegen(app: tauri::AppHandle) -> Result<String, String> {
    let (mit, eigene) = sprachordner(&app)?;
    let p = sprachen::vorlage_anlegen(&mit, &eigene)?;
    word::im_ordner_zeigen(&p)?;
    Ok(p.display().to_string())
}

/// „Ganz offline“ für diesen Rechner (Entwurf 8.2).
#[tauri::command]
fn offline_setzen(app: tauri::AppHandle, an: bool) -> Result<EinstellungenJs, String> {
    let konfig = app_konfig(&app)?;
    rechner_aendern(&konfig, |r| r.ganz_offline = an)?;
    Ok(einstellungen_js(&konfig))
}

// --- MCP ------------------------------------------------------------------------------------

/// `tftn-app --mcp`: Brücke für KI-Assistenten (startet keine Oberfläche).
pub fn mcp_bruecke() -> std::process::ExitCode {
    mcp::bruecke()
}

#[tauri::command]
fn mcp_stand(app: tauri::AppHandle) -> mcp::McpJs {
    mcp::js(&app, None)
}

/// Das Netzprotokoll dieser Sitzung für das Fenster.
#[tauri::command]
fn netzprotokoll_lesen(app: tauri::AppHandle) -> netzprotokoll::ProtokollJs {
    let offline = app_konfig(&app)
        .map(|k| tftn_einstellungen::Rechner::laden(&k).ganz_offline)
        .unwrap_or(false);
    netzprotokoll::stand().js(offline, mcp::an(&app))
}

/// Das Blatt zum Netzprotokoll als PDF (Temp-Ordner) setzen und im PDF-Betrachter öffnen, von wo
/// es sich drucken oder sichern lässt. `versatz_min`: Abstand der Ortszeit zu UTC in Minuten.
#[tauri::command(async)]
fn netzblatt_erzeugen(app: tauri::AppHandle, versatz_min: i32) -> Result<String, String> {
    let offline = tftn_einstellungen::Rechner::laden(&app_konfig(&app)?).ganz_offline;
    let jetzt = netzprotokoll::jetzt_ms();
    let system = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        andere => andere,
    };
    let angaben = netzprotokoll::BlattAngaben {
        tftn_version: env!("CARGO_PKG_VERSION").into(),
        typst_version: tftn_manifest::anlegen::TYPST_VERSION.into(),
        system: format!("{system} ({})", std::env::consts::ARCH),
        ganz_offline: offline,
        mcp_an: mcp::an(&app),
        jetzt_ms: jetzt,
        versatz_min,
        sprache: tftn_sprachen::aktuell().map_or_else(|| "de".to_string(), |k| k.kennung.clone()),
    };
    let daten = netzprotokoll::blatt_daten(&netzprotokoll::stand(), &angaben);
    let (_, uhr) = netzprotokoll::ortszeit(jetzt, versatz_min);
    let stempel = pakete::datum((jetzt / 1000) as i64 + i64::from(versatz_min) * 60);
    let name = format!(
        "{}-{stempel}-{}",
        tr!("TFTN-Netzprotokoll"),
        uhr.replace(':', "")
    );
    let cli = tftn_export::TypstCli::finden(tftn_manifest::anlegen::TYPST_VERSION)
        .map_err(|e| e.to_string())?;
    let ordner = std::env::temp_dir().join("tftn-netzblatt");
    let pdf = netzprotokoll::blatt_setzen(
        &ordner,
        &name,
        &daten,
        &cli,
        &typquellen(&app).map_or_else(|_| Vec::new(), |(q, _)| typen::schriftvorrat(&q)),
    )?;
    netzprotokoll::mit_system_oeffnen(&pdf)?;
    Ok(pdf.display().to_string())
}

/// MCP ein- oder ausschalten (nur der Benutzer, über Einstellungen oder Statusleiste).
#[tauri::command]
fn mcp_setzen(app: tauri::AppHandle, an: bool) -> mcp::McpJs {
    let fehler = mcp::setzen(&app, an).err();
    mcp::js(&app, fehler)
}

#[tauri::command]
fn mcp_beim_wechsel_aus(app: tauri::AppHandle, aus: bool) -> mcp::McpJs {
    let fehler = mcp::beim_wechsel_aus_setzen(&app, aus).err();
    mcp::js(&app, fehler)
}

/// Nach einem Wechsel durch MCP: aktiv lassen oder ausschalten.
#[tauri::command]
fn mcp_bestaetigen(app: tauri::AppHandle, aktiv: bool) -> mcp::McpJs {
    let fehler = mcp::bestaetigen(&app, aktiv).err();
    mcp::js(&app, fehler)
}

/// Vor einem MCP-Wechsel ist gespeichert (und ein Dokument ohne Namen benannt): Ziel vormerken.
#[tauri::command]
fn mcp_wechsel_vormerken(app: tauri::AppHandle) -> Result<(), String> {
    mcp::wechsel_vormerken(&app)
}

/// Antwort des Editors auf einen MCP-Auftrag (`ok`: ausgeführt; `false`: veraltet).
#[tauri::command]
fn mcp_antwort(app: tauri::AppHandle, id: u64, ok: bool, fehler: Option<String>) {
    mcp::antwort(&app, id, ok, fehler);
}

/// Die Oberfläche meldet die Auswahl im Editor (nur solange MCP an ist), für `auswahl_lesen`.
#[tauri::command]
fn mcp_auswahl(app: tauri::AppHandle, auswahl: mcp::Auswahl) {
    mcp::auswahl_setzen(&app, auswahl);
}

/// Patch eines Markup-Helfers (AP4) an der UTF-16-Position `pos`; `None`, wenn dort keine passende
/// Stelle ist. Ebene ± gilt der innersten Überschrift.
#[tauri::command]
fn markup_aktion(
    rev: u64,
    pos: usize,
    aktion: tftn_edit::MarkupAktion,
    zustand: State<'_, Zustand>,
) -> Result<Option<PatchJs>, String> {
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    if s.spiegel.rev() != rev {
        return Err("veraltet".into());
    }
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let byte = z
        .utf16_to_byte(pos)
        .ok_or_else(|| tr!("Position außerhalb"))?;
    use tftn_edit::{MarkupAktion as A, MarkupStelle as S};
    let passt = |st: &S| match aktion {
        A::EbenePlus | A::EbeneMinus => matches!(st, S::Ueberschrift { .. }),
        A::ListeUmschalten => matches!(st, S::Liste { .. }),
        A::FormelUmschalten => matches!(st, S::Formel { .. }),
    };
    let Some(stelle) = tftn_edit::markup_stellen(quelle, byte)
        .into_iter()
        .find(passt)
    else {
        return Ok(None);
    };
    let patch = tftn_edit::markup_patch(quelle, &stelle, aktion).map_err(|e| e.to_string())?;
    Ok(Some(PatchJs {
        rev,
        von: z
            .byte_to_utf16(patch.bereich.start)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        bis: z
            .byte_to_utf16(patch.bereich.end)
            .ok_or_else(|| tr!("Position außerhalb"))?,
        text: patch.text,
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let zustand = Zustand::default();
    let anzeige = zustand.anzeige.clone();
    tauri::Builder::default()
        .manage(zustand)
        .register_asynchronous_uri_scheme_protocol("tftn", move |_ctx, anfrage, antwort| {
            let anzeige = anzeige.clone();
            std::thread::spawn(move || {
                let uri = anfrage.uri();
                let png = seite_ausliefern(&anzeige, uri.path(), uri.query());
                let r = match png {
                    Some(daten) => tauri::http::Response::builder()
                        .header("Content-Type", "image/png")
                        .header("Cache-Control", "public, max-age=31536000, immutable")
                        .body(daten),
                    None => tauri::http::Response::builder()
                        .status(404)
                        .body(Vec::new()),
                };
                antwort.respond(r.expect("gültige Antwort"));
            });
        })
        .setup(|app| {
            // Ohne Wächter laufen die Kürzel weiter, nur Änderungen von außen kommen erst beim Neustart an.
            if let Err(e) = einstellungen_beobachten(app.handle()) {
                eprintln!("Einstellungen werden nicht beobachtet: {e}");
            }
            // Netzprotokoll ab jetzt; jede Änderung erfährt die Oberfläche.
            let melder = app.handle().clone();
            netzprotokoll::beginnen(move || {
                let _ = melder.emit("netzprotokoll", ());
            });
            mcp::beim_start(app.handle());
            // Schriften für Dokumente, die keine eigenen eingebunden haben.
            if let Ok((quellen, _)) = typquellen(app.handle()) {
                tftn_core::schriftvorrat_setzen(typen::schriftvorrat(&quellen));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            oberflaeche,
            sprachen_lesen,
            sprache_setzen,
            sprachvorlage_anlegen,
            skins_lesen,
            skin_setzen,
            skinvorlage_anlegen,
            oeffnen,
            aendern,
            neusync,
            hervorheben,
            speichern,
            klick,
            stelle_zeigen,
            hier,
            marken,
            wert_setzen,
            tabelle_aktion,
            katalog,
            baustein_text,
            modus,
            vervollstaendigen,
            zitate_suchen,
            zitat_text,
            export_profile,
            exportieren,
            archivieren,
            einstellungen,
            kuerzel_setzen,
            einstellungsordner_setzen,
            einstellungsordner_zeigen,
            offline_setzen,
            markup_aktion,
            zitat_aendern,
            neues_dokument,
            seite_stand,
            seite_setzen,
            schriften_einbinden,
            schriftwahl::schriften_suchen,
            schriftwahl::schriftdatei_lesen,
            schriftwahl::schrift_hinzufuegen,
            datei_waehlen_dialog,
            importieren,
            einfuegen_umwandeln,
            updates_pruefen,
            entwurf_benennen,
            fassungen_liste,
            bild_ablegen,
            word_exportieren,
            im_ordner_zeigen,
            gliederung_lesen,
            literatur_liste,
            literatur_eintrag_speichern,
            literatur_eintrag_loeschen,
            zaehlen_bereich,
            rechtschreibung_pruefen,
            rechtschreibung_vorschlaege,
            wort_aufnehmen,
            literatur_aus_doi,
            abschnitt_verschieben,
            fassung_text,
            fassung_zurueckholen,
            formular,
            formular_setzen,
            formular_eintrag,
            formular_rueckgaengig,
            formular_text,
            formular_text_setzen,
            formular_wiederholen,
            projekt_info,
            typen_liste,
            typ_vorschaubild,
            ordner_waehlen_dialog,
            projekt_anlegen,
            projekt_wechseln,
            zuletzt_liste,
            zuletzt_leeren,
            typ_vergleichen,
            typ_uebernehmen,
            pakete_stand,
            pakete_index_holen,
            pakete_suchen,
            paket_readme,
            paket_hinzufuegen,
            formel_stand,
            formel_vorschau,
            formel_setzen,
            formel_einfuegen,
            formel_palette,
            formel_baustein,
            paket_entfernen,
            paket_vergleichen,
            paket_wechseln,
            mcp_stand,
            netzprotokoll_lesen,
            netzblatt_erzeugen,
            mcp_setzen,
            mcp_beim_wechsel_aus,
            mcp_bestaetigen,
            mcp_antwort,
            mcp_auswahl,
            serie_pruefen,
            serie_erzeugen,
            mcp_wechsel_vormerken,
            oberflaeche_lesen,
            typ_eigen_anlegen,
            oberflaeche_setzen,
        ])
        .on_window_event(|fenster, ereignis| {
            if let tauri::WindowEvent::CloseRequested { .. } = ereignis {
                beim_schliessen(&fenster.state::<Zustand>());
            }
        })
        .build(tauri::generate_context!())
        .expect("TFTN konnte nicht starten")
        .run(|app, ereignis| {
            if let tauri::RunEvent::ExitRequested { .. } = ereignis {
                beim_schliessen(&app.state::<Zustand>());
                mcp::beim_schliessen(app);
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn meldungen_fuer_editor_und_mcp_in_einem_durchgang() {
        let quelle = super::Source::detached("a\nb€ cd");
        let id = quelle.id();
        let meldung = |fehler, stelle: Option<std::ops::Range<usize>>, ort: Option<&str>| {
            tftn_core::Meldung {
                fehler,
                ort: ort.map(String::from),
                text: "kaputt".into(),
                stelle: stelle.map(|r| (id, r)),
            }
        };
        // „c“ beginnt bei Byte 7 (b, €=3 Byte, Leerzeichen): Zeile 2, Spalte 4; UTF-16 5..6.
        let (js, texte) = super::meldungen_aufbereiten(
            &[
                meldung(true, Some(7..8), Some("main.typ:2:4")),
                meldung(false, None, Some("lib.typ:1:1")),
                meldung(false, None, None),
            ],
            &quelle,
        );
        assert_eq!((js[0].von, js[0].bis), (Some(5), Some(6)));
        assert_eq!((js[1].von, js[1].bis), (None, None));
        assert_eq!(
            texte,
            [
                "Fehler (Zeile 2, Spalte 4): kaputt",
                "Warnung (lib.typ:1:1): kaputt",
                "Warnung: kaputt"
            ]
        );
    }

    use super::*;

    /// Abschlussprüfung AP8: Ohne passende Sitzung (gewechselt oder keine) spielt „Übernehmen“
    /// nichts ein und tauscht auch die Vorschau nicht.
    #[test]
    fn uebernahme_nur_in_dieselbe_sitzung() {
        let zustand = Zustand::default();
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        let p = Projekt::oeffnen(&wurzel).unwrap();
        assert_eq!(uebernahme_einspielen(&zustand, &p.wurzel), Ok(false));
        assert!(zustand.vorschau.lock().unwrap().is_none());
    }

    #[test]
    fn literatur_aus_dokument_toml() {
        let o = std::env::temp_dir().join(format!("tftn-app-literatur-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let p = tftn_manifest::anlegen::projekt_anlegen(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../types"),
            "tftn-fachartikel",
            &o,
            Some(1_793_404_800),
        )
        .unwrap()
        .projekt;
        assert!(literatur_dateien(&p)[0].ends_with("literatur.bib"));
        std::fs::write(o.join("b.bib"), "").unwrap();
        let d = o.join("dokument.toml");
        let t = std::fs::read_to_string(&d).unwrap();
        std::fs::write(
            &d,
            t.replace(r#"dateien = ["literatur.bib"]"#, r#"dateien = ["b.bib"]"#),
        )
        .unwrap();
        let neu = literatur_dateien(&p);
        assert_eq!(neu.len(), 1);
        assert!(neu[0].ends_with("b.bib"), "{neu:?}");
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Die Aufforderung geht unverändert als eigenes Argument an das Skript, nie in den Skripttext.
    #[test]
    fn wahldialog_reicht_die_aufforderung_als_argument() {
        let a = wahl_argumente(Wahl::Ordner, r#"Wählen "x" \ ');"#, None);
        assert_eq!(a[0], "-l");
        assert_eq!(a[1], "JavaScript");
        assert!(a[3].contains("chooseFolder"));
        assert!(!a[3].contains("Wählen"));
        assert_eq!(a[4], r#"Wählen "x" \ ');"#);
        assert_eq!(a.len(), 5);
        let start = std::env::temp_dir();
        let b = wahl_argumente(Wahl::Datei, "y", Some(&start));
        assert!(b[3].contains("chooseFile"));
        assert_eq!(
            b[5],
            start.display().to_string(),
            "Startordner als eigenes Argument"
        );
        let fehlt = start.join("gibt-es-nicht-tftn");
        assert_eq!(wahl_argumente(Wahl::Datei, "y", Some(&fehlt)).len(), 5);
    }

    /// Fachartikel regelt die Literatur selbst; ein leeres Dokument braucht `#bibliography(…)`.
    #[test]
    fn wer_das_literaturverzeichnis_setzt() {
        let o = std::env::temp_dir().join(format!("tftn-app-verzeichnis-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../types");
        let artikel = tftn_manifest::anlegen::projekt_anlegen(
            &typen,
            "tftn-fachartikel",
            &o.join("artikel"),
            Some(1_793_404_800),
        )
        .unwrap()
        .projekt;
        assert!(typ_regelt_literatur(&artikel));
        let leer = tftn_manifest::anlegen::projekt_anlegen(
            &typen,
            "tftn-leer",
            &o.join("leer"),
            Some(1_793_404_800),
        )
        .unwrap()
        .projekt;
        assert!(!typ_regelt_literatur(&leer));
        assert!(literatur_dateien(&leer)[0].ends_with("literatur.bib"));
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn einstellungen_ohne_ordner_und_mit_ordner() {
        let t = std::env::temp_dir().join(format!("tftn-app-einstellungen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        let app = t.join("app");
        let tresor = t.join("tresor");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(&tresor).unwrap();
        let e = einstellungen_js(&app);
        assert!(!e.synchronisiert);
        assert!(e.kuerzel.is_empty());
        assert_eq!(e.hinweis, None, "ohne gemeinsamen Ordner keine Warnung");
        tftn_einstellungen::Rechner {
            ordner: Some(tresor.clone()),
            ganz_offline: true,
            ..Default::default()
        }
        .speichern(&app)
        .unwrap();
        tftn_einstellungen::kuerzel_setzen(&tresor, "typst/link", Some("Mod-Shift-k")).unwrap();
        let e = einstellungen_js(&app);
        assert!(e.synchronisiert);
        assert!(e.ganz_offline);
        assert_eq!(e.ordner, tresor.display().to_string());
        assert_eq!(e.kuerzel["typst/link"], "Mod-Shift-k");
        let _ = std::fs::remove_dir_all(&t);
    }
}
