//! MCP in der App: Schalter, lokale Verbindung und die Werkzeuge (Protokoll in `tftn-mcp`).
//!
//! Der Benutzer schaltet MCP selbst ein (Einstellungen oder Statusleiste); dann horcht die App auf
//! `127.0.0.1` und schreibt Port und Schlüssel in die Verbindungsdatei. Änderungen am Text gehen wie
//! alle anderen als Patch an den Editor, den einzigen Schreiber: Sie erscheinen sofort und lassen sich
//! rückgängig machen. Auf Wunsch schaltet sich MCP aus, sobald ein anderes Dokument geöffnet wird;
//! öffnet MCP es selbst, pausiert es stattdessen, bis der Benutzer bestätigt.
//!
//! Texte an den Assistenten bleiben deutsch und unübersetzt; was der Benutzer sieht, geht über `tr!`.

use std::collections::HashMap;
use std::io::Write;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use tftn_mcp::{Auftrag, Ergebnis, Inhalt, Verbindung};
use tftn_sprachen::tr;

use crate::{Zustand, app_konfig, typen};

/// Ergebnis des letzten Typst-Laufs, für die Antworten an den Assistenten.
#[derive(Clone, Debug)]
pub struct LaufStand {
    pub rev: u64,
    pub gelungen: bool,
    pub seiten: usize,
    /// „Fehler (Zeile 3, Spalte 5): …“
    pub meldungen: Vec<String>,
}

/// Wer den nächsten Dokumentwechsel ausgelöst hat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wechsel {
    #[default]
    Benutzer,
    Mcp,
    /// Dokument ohne Namen bekommt einen: dasselbe Dokument, kein Wechsel.
    Umbenannt,
}

struct Horcher {
    verbindung: Verbindung,
    stopp: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Mcp {
    /// Pausiert nach einem Wechsel durch MCP, bis der Benutzer bestätigt.
    wartet: bool,
    horcher: Option<Horcher>,
    /// Aufträge an den Editor, die auf Antwort warten.
    offen: HashMap<u64, mpsc::Sender<Result<bool, String>>>,
    naechste: u64,
    /// Ziel eines Wechsels durch MCP. Wird erst nach dem Speichern vorgemerkt, weil das Benennen
    /// eines Dokuments ohne Namen die Vormerkung sonst überschreibt.
    ziel: Option<PathBuf>,
    /// Auswahl im Editor, wie ihn die Oberfläche zuletzt gemeldet hat (nur solange MCP an ist).
    auswahl: Option<Auswahl>,
}

/// Auswahl im Editor: UTF-16-Stellen im Text der Revision `rev`; `kopf` ist die Schreibmarke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Auswahl {
    pub rev: u64,
    pub von: usize,
    pub bis: usize,
    pub kopf: usize,
}

/// Die Oberfläche meldet die Auswahl (Befehl `mcp_auswahl`).
pub fn auswahl_setzen(app: &AppHandle, a: Auswahl) {
    app.state::<Zustand>().mcp.lock().unwrap().auswahl = Some(a);
}

/// Merkt das Ziel eines MCP-Wechsels für das Neuladen vor (nach dem Speichern im Editor).
pub fn vormerken(
    ziel: Option<PathBuf>,
    naechstes: &mut Option<PathBuf>,
    wechsel: &mut Wechsel,
) -> Result<(), String> {
    let ziel = ziel.ok_or("kein Wechsel durch MCP vorgemerkt")?;
    *naechstes = Some(ziel);
    *wechsel = Wechsel::Mcp;
    Ok(())
}

/// Befehl des Editors, sobald vor einem MCP-Wechsel gespeichert (und benannt) ist.
pub fn wechsel_vormerken(app: &AppHandle) -> Result<(), String> {
    let zustand = app.state::<Zustand>();
    let ziel = zustand.mcp.lock().unwrap().ziel.take();
    let mut naechstes = zustand.naechstes.lock().unwrap();
    let mut wechsel = zustand.wechsel.lock().unwrap();
    vormerken(ziel, &mut naechstes, &mut wechsel)
}

#[derive(Serialize, Clone)]
pub struct McpJs {
    an: bool,
    wartet: bool,
    beim_wechsel_aus: bool,
    /// So ruft ein Assistent die Brücke auf: Programm und `--mcp`.
    programm: String,
    fehler: Option<String>,
}

pub fn js(app: &AppHandle, fehler: Option<String>) -> McpJs {
    let zustand = app.state::<Zustand>();
    let m = zustand.mcp.lock().unwrap();
    let rechner = app_konfig(app)
        .map(|k| tftn_einstellungen::Rechner::laden(&k))
        .unwrap_or_default();
    McpJs {
        an: m.horcher.is_some(),
        wartet: m.wartet,
        beim_wechsel_aus: rechner.mcp_beim_wechsel_aus,
        programm: programm_aus(
            std::env::var_os("APPIMAGE").map(PathBuf::from),
            std::env::current_exe().ok(),
        ),
        fehler,
    }
}

/// Programm für die Brücke. In einem AppImage liegt die App unter einem Pfad, der bei jedem Start
/// wechselt (`/tmp/.mount_…`); dort zählt die AppImage-Datei selbst (`APPIMAGE`), die `--mcp` an
/// die App weiterreicht.
fn programm_aus(appimage: Option<PathBuf>, exe: Option<PathBuf>) -> String {
    appimage
        .filter(|p| p.is_file())
        .or(exe)
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

/// Horcht MCP gerade (für Netzprotokoll und Blatt)?
pub fn an(app: &AppHandle) -> bool {
    app.state::<Zustand>().mcp.lock().unwrap().horcher.is_some()
}

fn melden(app: &AppHandle) {
    let _ = app.emit("mcp-geaendert", js(app, None));
}

/// Schaltet ein oder aus und merkt es für den nächsten Start.
pub fn setzen(app: &AppHandle, an: bool) -> Result<(), String> {
    let konfig = app_konfig(app)?;
    if an {
        starten(app, &konfig)?;
    } else {
        stoppen(app, &konfig);
    }
    app.state::<Zustand>().mcp.lock().unwrap().wartet = false;
    if tftn_einstellungen::Rechner::laden(&konfig).mcp != an {
        crate::rechner_aendern(&konfig, |r| r.mcp = an)?;
    }
    melden(app);
    Ok(())
}

pub fn beim_wechsel_aus_setzen(app: &AppHandle, aus: bool) -> Result<(), String> {
    let konfig = app_konfig(app)?;
    crate::rechner_aendern(&konfig, |r| r.mcp_beim_wechsel_aus = aus)?;
    melden(app);
    Ok(())
}

/// Antwort des Benutzers auf „MCP aktiv lassen?“ nach einem Wechsel durch MCP.
pub fn bestaetigen(app: &AppHandle, aktiv: bool) -> Result<(), String> {
    if aktiv {
        app.state::<Zustand>().mcp.lock().unwrap().wartet = false;
        melden(app);
        Ok(())
    } else {
        setzen(app, false)
    }
}

/// Beim Start: an, wenn es zuletzt an war; eine liegengebliebene Verbindungsdatei weg (die einer
/// zweiten laufenden Instanz bleibt).
pub fn beim_start(app: &AppHandle) {
    let Ok(konfig) = app_konfig(app) else { return };
    Verbindung::verwaiste_entfernen(&konfig);
    if tftn_einstellungen::Rechner::laden(&konfig).mcp
        && let Err(e) = starten(app, &konfig)
    {
        eprintln!("MCP nicht gestartet: {e}");
    }
}

pub fn beim_schliessen(app: &AppHandle) {
    if let Ok(konfig) = app_konfig(app) {
        stoppen(app, &konfig);
    }
}

/// Nach dem Öffnen eines anderen Dokuments (`oeffnen` mit gemerktem Wechsel).
pub fn nach_wechsel(app: &AppHandle, von: Wechsel) {
    let an = app.state::<Zustand>().mcp.lock().unwrap().horcher.is_some();
    let aus = app_konfig(app)
        .map(|k| tftn_einstellungen::Rechner::laden(&k).mcp_beim_wechsel_aus)
        .unwrap_or(false);
    if !an || !aus {
        return;
    }
    match von {
        Wechsel::Benutzer => {
            if let Err(e) = setzen(app, false) {
                eprintln!("MCP nicht ausgeschaltet: {e}");
            }
        }
        Wechsel::Mcp => {
            app.state::<Zustand>().mcp.lock().unwrap().wartet = true;
            melden(app);
        }
        Wechsel::Umbenannt => {}
    }
}

fn starten(app: &AppHandle, konfig: &Path) -> Result<(), String> {
    let zustand = app.state::<Zustand>();
    let mut m = zustand.mcp.lock().unwrap();
    if m.horcher.is_some() {
        return Ok(());
    }
    let horcher = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
    let port = horcher.local_addr().map_err(|e| e.to_string())?.port();
    let schluessel = tftn_mcp::schluessel();
    let verbindung = Verbindung {
        port,
        schluessel: schluessel.clone(),
    };
    verbindung.schreiben(konfig).map_err(|e| e.to_string())?;
    let stopp = Arc::new(AtomicBool::new(false));
    let halt = stopp.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        for strom in horcher.incoming() {
            if halt.load(Ordering::SeqCst) {
                break;
            }
            let Ok(strom) = strom else { continue };
            let app = app.clone();
            let schluessel = schluessel.clone();
            std::thread::spawn(move || bedienen(&app, strom, &schluessel));
        }
    });
    m.horcher = Some(Horcher { verbindung, stopp });
    Ok(())
}

fn stoppen(app: &AppHandle, konfig: &Path) {
    let horcher = app.state::<Zustand>().mcp.lock().unwrap().horcher.take();
    if let Some(h) = horcher {
        h.verbindung.entfernen_wenn_eigen(konfig);
        h.stopp.store(true, Ordering::SeqCst);
        // `accept` aufwecken, damit der Faden endet.
        let _ = TcpStream::connect_timeout(
            &(Ipv4Addr::LOCALHOST, h.verbindung.port).into(),
            Duration::from_millis(500),
        );
    }
}

/// Eine Verbindung der Brücke: ein Auftrag, eine Antwort.
fn bedienen(app: &AppHandle, strom: TcpStream, schluessel: &str) {
    let _ = strom.set_read_timeout(Some(Duration::from_secs(10)));
    let zeile = match tftn_mcp::auftrag_lesen(&strom, tftn_mcp::GRENZE_AUFTRAG) {
        Ok(Some(z)) => z,
        Ok(None) => return,
        Err(e) => {
            crate::netzprotokoll::mcp_eintragen(None);
            let antwort =
                serde_json::to_string(&Ergebnis::fehler(e.to_string())).unwrap_or_default();
            let _ = writeln!(&strom, "{antwort}");
            return;
        }
    };
    // Netzprotokoll: angenommene Aufrufe je Werkzeug, alle anderen als abgewiesen.
    let ergebnis = match serde_json::from_str::<Auftrag>(&zeile) {
        Err(e) => {
            crate::netzprotokoll::mcp_eintragen(None);
            Ergebnis::fehler(format!("unlesbarer Auftrag: {e}"))
        }
        Ok(a) if !tftn_mcp::schluessel_gleich(&a.schluessel, schluessel) => {
            crate::netzprotokoll::mcp_eintragen(None);
            Ergebnis::fehler("falscher Schlüssel")
        }
        Ok(a) => {
            let (an, wartet) = {
                let zustand = app.state::<Zustand>();
                let m = zustand.mcp.lock().unwrap();
                (m.horcher.is_some(), m.wartet)
            };
            crate::netzprotokoll::mcp_eintragen((an && !wartet).then_some(a.werkzeug.as_str()));
            if !an {
                Ergebnis::fehler("MCP ist in TFTN ausgeschaltet.")
            } else if wartet {
                Ergebnis::fehler(
                    "MCP ist pausiert: Der Benutzer muss in TFTN bestätigen, dass MCP für das neu \
                     geöffnete Dokument aktiv bleibt. Bitte den Benutzer darum bitten.",
                )
            } else {
                // Statusleiste: Der Assistent arbeitet gerade (zeigt, was er tut).
                let _ = app.emit("mcp-zugriff", &a.werkzeug);
                ausfuehren(app, &a.werkzeug, &a.argumente).unwrap_or_else(Ergebnis::fehler)
            }
        }
    };
    let text = serde_json::to_string(&ergebnis).unwrap_or_default();
    let _ = writeln!(&strom, "{text}");
}

fn ausfuehren(app: &AppHandle, werkzeug: &str, a: &Value) -> Result<Ergebnis, String> {
    let text = |n: &str| a.get(n).and_then(Value::as_str).map(str::to_string);
    let pflicht = |n: &str| text(n).ok_or_else(|| format!("`{n}` fehlt"));
    match werkzeug {
        "status" => status(app),
        "quelltext_lesen" => quelltext(app),
        "text_ersetzen" => {
            let (alt, neu) = (pflicht("alt")?, pflicht("neu")?);
            let alle = a.get("alle").and_then(Value::as_bool).unwrap_or(false);
            aendern(app, |t| tftn_mcp::ersetzen(t, &alt, &neu, alle))
        }
        "quelltext_setzen" => {
            let neu = pflicht("text")?;
            aendern(app, |_| Ok(neu.clone()))
        }
        "seite_ansehen" => {
            let seite = a
                .get("seite")
                .and_then(Value::as_u64)
                .ok_or("`seite` fehlt")?;
            let ppi = a.get("ppi").and_then(Value::as_f64).unwrap_or(100.0);
            seite_ansehen(app, seite as usize, ppi)
        }
        "auswahl_lesen" => auswahl_lesen(app),
        "gliederung_lesen" => gliederung_lesen(app),
        "dateien_auflisten" => dateien_auflisten(app),
        "datei_lesen" => datei_lesen(app, &pflicht("pfad")?),
        "datei_schreiben" => {
            let (pfad, neu) = (pflicht("pfad")?, pflicht("text")?);
            datei_aendern(app, &pfad, |_| Ok(neu.clone()))
        }
        "datei_ersetzen" => {
            let (pfad, alt, neu) = (pflicht("pfad")?, pflicht("alt")?, pflicht("neu")?);
            let alle = a.get("alle").and_then(Value::as_bool).unwrap_or(false);
            datei_aendern(app, &pfad, |t| {
                tftn_mcp::ersetzen(t.ok_or("Die Datei gibt es nicht.")?, &alt, &neu, alle)
            })
        }
        "literatur_lesen" => literatur_lesen(app),
        "literatur_eintrag_speichern" => literatur_speichern(app, a),
        "fassung_sichern" => fassung_sichern(app),
        "rechtschreibung_pruefen" => rechtschreibung_pruefen(app, text("pfad")),
        "formular_lesen" => formular_lesen(app),
        "formular_setzen" => {
            let werte = a
                .get("werte")
                .and_then(Value::as_object)
                .ok_or("`werte` fehlt (ein Objekt Feldname → Wert)")?;
            formular_setzen(app, werte)
        }
        "typen_auflisten" => typen_auflisten(app),
        "dokument_anlegen" => {
            let (typ, name) = (pflicht("typ")?, pflicht("name")?);
            let eltern = match text("ordner") {
                Some(o) => typen::pfad_aus_eingabe(&o).ok_or("`ordner` ist leer")?,
                None => crate::dokumente_ordner(app)?,
            };
            let (quellen, _) = crate::typquellen(app)?;
            let ziel = typen::anlegen(
                &quellen,
                &typ,
                &eltern,
                &name,
                false,
                Some(&tftn_sprachen::kennung()),
            )?;
            wechseln(app, ziel.clone()).map_err(|e| {
                format!(
                    "{e} Das Projekt ist aber angelegt: {}; zum Öffnen `dokument_oeffnen` mit diesem \
                     Ordner, nicht noch einmal anlegen.",
                    ziel.display()
                )
            })
        }
        "dokument_oeffnen" => {
            let ordner = typen::pfad_aus_eingabe(&pflicht("ordner")?).ok_or("`ordner` ist leer")?;
            wechseln(app, typen::wechsel_pruefen(&ordner)?)
        }
        _ => Err(format!("unbekanntes Werkzeug: {werkzeug}")),
    }
}

const KEIN_DOKUMENT: &str = "In TFTN ist gerade kein Dokument offen.";

fn lauf_text(lauf: Option<&LaufStand>, rev: u64) -> String {
    let Some(l) = lauf else {
        return "Typst ist noch nicht gelaufen.".into();
    };
    let mut t = if l.gelungen {
        format!(
            "Typst-Lauf (Rev {}): gelungen, {} Seite(n).",
            l.rev, l.seiten
        )
    } else {
        format!(
            "Typst-Lauf (Rev {}): fehlgeschlagen; die Vorschau zeigt weiter den letzten guten Stand.",
            l.rev
        )
    };
    if l.rev < rev {
        t.push_str(&format!(" (Rev {rev} wird noch übersetzt.)"));
    }
    for m in &l.meldungen {
        t.push_str("\n- ");
        t.push_str(m);
    }
    t
}

fn status(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let lauf = zustand.lauf.lock().unwrap().clone();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let rev = s.spiegel.rev();
    Ok(Ergebnis::text(format!(
        "Projekt: {}\nHauptdatei: {}\nRevision: {rev}\n{}\n{}",
        s.projekt.wurzel.display(),
        s.projekt.konfig.haupt,
        if s.spiegel.pruefsumme() == s.auf_platte {
            "gespeichert"
        } else {
            "ungespeichert (TFTN speichert nach einer Sekunde selbst)"
        },
        lauf_text(lauf.as_ref(), rev),
    )))
}

fn quelltext(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let mut t = format!(
        "{} (Rev {}), Zeilennummer und Tabulator stehen nicht im Text:\n",
        s.projekt.konfig.haupt,
        s.spiegel.rev()
    );
    for (i, z) in s.spiegel.text().lines().enumerate() {
        t.push_str(&format!("{}\t{z}\n", i + 1));
    }
    Ok(Ergebnis::text(t))
}

/// Die Felder knapp für den Assistenten: Name, Beschriftung, Art, Pflicht, Hilfe, Auswahl, Unterfelder.
fn felder_js(felder: &[tftn_formular::Feld]) -> Value {
    use tftn_formular::Art;
    Value::Array(
        felder
            .iter()
            .map(|f| {
                let mut o = json!({ "name": f.name, "titel": f.titel });
                let (art, extra) = match &f.art {
                    Art::Text { .. } => ("text", None),
                    Art::Zahl { ganzzahl: true, .. } => ("ganzzahl", None),
                    Art::Zahl { .. } => ("zahl", None),
                    Art::Schalter { .. } => ("schalter", None),
                    Art::Datum { .. } => ("datum (JJJJ-MM-TT)", None),
                    Art::Wahl { werte, .. } => ("wahl", Some(("erlaubt", json!(werte)))),
                    Art::Liste { .. } => ("liste", None),
                    Art::Gruppe { felder } => ("gruppe", Some(("felder", felder_js(felder)))),
                    Art::Eintraege { felder, .. } => {
                        ("eintraege", Some(("felder", felder_js(felder))))
                    }
                };
                o["art"] = json!(art);
                if let Some((k, v)) = extra {
                    o[k] = v;
                }
                if f.pflicht {
                    o["pflicht"] = json!(true);
                }
                if let Some(h) = &f.hilfe {
                    o["hilfe"] = json!(h);
                }
                if f.sensibel {
                    o["sensibel"] = json!(true);
                }
                o
            })
            .collect(),
    )
}

/// Werte und Meldungen nach Lesen oder Setzen.
fn formular_antwort(kopf: String, js: &crate::formular::FormularJs, mit_feldern: bool) -> Ergebnis {
    let mut t = kopf;
    if let Some(e) = &js.dateifehler {
        t.push_str(&format!("\nDatei fehlerhaft: {e}"));
    }
    if let Some(f) = js.formular.as_ref().filter(|_| mit_feldern) {
        t.push_str(&format!(
            "\nFelder:\n{}",
            serde_json::to_string_pretty(&felder_js(&f.felder)).unwrap_or_default()
        ));
    }
    t.push_str(&format!(
        "\nWerte:\n{}",
        serde_json::to_string_pretty(&js.werte).unwrap_or_default()
    ));
    if !js.meldungen.is_empty() {
        t.push_str("\nMeldungen:");
        for m in &js.meldungen {
            let ort: Vec<String> = m
                .pfad
                .iter()
                .map(|s| match s {
                    tftn_formular::Schritt::Name(n) => n.clone(),
                    tftn_formular::Schritt::Index(i) => i.to_string(),
                })
                .collect();
            t.push_str(&format!(
                "\n- {} ({}): {}",
                ort.join("."),
                if m.fehler { "Fehler" } else { "Hinweis" },
                m.text
            ));
        }
    }
    Ergebnis::text(t)
}

fn formular_lesen(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_mut().ok_or(KEIN_DOKUMENT)?;
    let js = s.formular.laden(crate::stile());
    if js.formular.is_none() {
        return Ok(Ergebnis::text(format!(
            "Dieser Dokumenttyp hat kein Formular.{}",
            js.dateifehler
                .map(|e| format!(" ({e})"))
                .unwrap_or_default()
        )));
    }
    Ok(formular_antwort(
        format!("Formular, Datendatei {}:", js.datei),
        &js,
        true,
    ))
}

fn formular_setzen(
    app: &AppHandle,
    werte: &serde_json::Map<String, Value>,
) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_mut().ok_or(KEIN_DOKUMENT)?;
    let js = s
        .formular
        .mehrere_setzen(werte, crate::stile())
        .map_err(|e| format!("Nichts geändert. {e}"))?;
    Ok(formular_antwort(
        format!(
            "Gespeichert in {}; das Fach „Dokument“ zeigt es, die Vorschau wird neu übersetzt \
             (Ergebnis mit `status`).",
            js.datei
        ),
        &js,
        false,
    ))
}

/// Text mit Zeilennummern (ab `erste`), wie bei `quelltext_lesen`.
fn nummeriert(text: &str, erste: usize) -> String {
    let mut t = String::new();
    for (i, z) in text.lines().enumerate() {
        t.push_str(&format!("{}\t{z}\n", i + erste));
    }
    t
}

/// Höchstens so viele Zeichen der Auswahl gehen an den Assistenten.
const AUSWAHL_GRENZE: usize = 20_000;

fn auswahl_lesen(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let auswahl = zustand.mcp.lock().unwrap().auswahl;
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let a = auswahl
        .ok_or("TFTN hat noch keine Auswahl gemeldet; der Benutzer soll in den Text klicken.")?;
    let quelle = s.spiegel.quelle();
    let z = quelle.lines();
    let stelle = |u: usize| {
        z.utf16_to_byte(u)
            .ok_or("Die gemeldete Auswahl passt nicht mehr zum Text; bitte noch einmal.")
    };
    let (von, bis, kopf) = (stelle(a.von)?, stelle(a.bis)?, stelle(a.kopf)?);
    let (kz, ks) = z.byte_to_line_column(kopf).unwrap_or((0, 0));
    let mut t = format!(
        "{}: Schreibmarke in Zeile {}, Spalte {}.",
        s.projekt.konfig.haupt,
        kz + 1,
        ks + 1
    );
    if a.rev != s.spiegel.rev() {
        t.push_str(
            " (Seit der Meldung wurde der Text geändert; die Stelle kann leicht abweichen.)",
        );
    }
    if von == bis {
        t.push_str("\nNichts markiert.");
        return Ok(Ergebnis::text(t));
    }
    let text = &quelle.text()[von..bis];
    let (vz, _) = z.byte_to_line_column(von).unwrap_or((0, 0));
    let bz = z.byte_to_line(bis).unwrap_or(vz);
    let zeichen = text.chars().count();
    t.push_str(&format!(
        "\nMarkiert: Zeile {} bis {}, {zeichen} Zeichen. Wörtlich (für `text_ersetzen`):\n",
        vz + 1,
        bz + 1
    ));
    if zeichen > AUSWAHL_GRENZE {
        let ende = text
            .char_indices()
            .nth(AUSWAHL_GRENZE)
            .map_or(text.len(), |(i, _)| i);
        t.push_str(&text[..ende]);
        t.push_str(&format!(
            "\n[… gekürzt; der Rest mit `quelltext_lesen` ab Zeile {}]",
            z.byte_to_line(von + ende).unwrap_or(bz) + 1
        ));
    } else {
        t.push_str(text);
    }
    Ok(Ergebnis::text(t))
}

/// Überschriften und Kommentare einer Quelle, je mit Zeile. `einschuebe`: weitere Zeilen (etwa
/// eingebundene Kapitel), die nach ihrer Zeile zwischen die Überschriften kommen.
fn gliederung_text(quelle: &typst::syntax::Source, einschuebe: Vec<(usize, String)>) -> String {
    let z = quelle.lines();
    let zeile = |b: usize| z.byte_to_line(b).map_or(0, |l| l + 1);
    let mut zeilen: Vec<(usize, String)> = tftn_edit::gliederung(quelle)
        .into_iter()
        .map(|u| {
            (
                zeile(u.von),
                format!(
                    "{}{} {} (Zeile {}, {} Wörter)\n",
                    "  ".repeat(u.ebene.saturating_sub(1)),
                    "=".repeat(u.ebene),
                    u.titel,
                    zeile(u.von),
                    tftn_edit::zaehlen(quelle, u.abschnitt.clone()).woerter
                ),
            )
        })
        .collect();
    zeilen.extend(einschuebe);
    zeilen.sort_by_key(|(z, _)| *z);
    let mut t: String = zeilen.into_iter().map(|(_, s)| s).collect();
    let kommentare = tftn_edit::kommentare(quelle);
    if !kommentare.is_empty() {
        t.push_str("Kommentare:\n");
        for k in kommentare {
            t.push_str(&format!(
                "- Zeile {}: {}\n",
                zeile(k.von),
                k.text.replace('\n', " ")
            ));
        }
    }
    if t.is_empty() {
        t.push_str("(keine Überschriften, keine Kommentare)\n");
    }
    t
}

fn gliederung_lesen(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let quelle = s.spiegel.quelle();
    // Eingebundene Dateien relativ zur Hauptdatei, an ihrer Stelle; Zeilen darin gelten in der Datei.
    let basis = Path::new(&s.projekt.konfig.haupt)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .filter(|p| !p.is_empty());
    let einschuebe = crate::projektdateien::eingebunden(quelle.text())
        .into_iter()
        .map(|(zeile, rel)| {
            let pfad = match &basis {
                Some(b) => format!("{b}/{rel}"),
                None => rel,
            };
            let innen = match crate::projektdateien::lesen(&s.projekt.wurzel, &pfad) {
                Ok(text) => gliederung_text(&typst::syntax::Source::detached(text), Vec::new()),
                Err(e) => format!("nicht lesbar: {e}\n"),
            };
            let eingerueckt: String = innen.lines().map(|l| format!("    {l}\n")).collect();
            (
                zeile,
                format!(
                    "#include {pfad} (Zeile {zeile}), darin (Zeilen der Datei):\n{eingerueckt}"
                ),
            )
        })
        .collect();
    Ok(Ergebnis::text(format!(
        "{} (Hauptdatei):\n{}",
        s.projekt.konfig.haupt,
        gliederung_text(quelle, einschuebe)
    )))
}

fn dateien_auflisten(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let daten = relativ(&s.projekt.wurzel, s.formular.datei());
    let mut t = format!("Projektordner {}:\n", s.projekt.wurzel.display());
    for e in crate::projektdateien::auflisten(&s.projekt.wurzel) {
        let art = if e.pfad == s.projekt.konfig.haupt {
            "Hauptdatei (über text_ersetzen)"
        } else if Some(&e.pfad) == daten.as_ref() {
            "Formulardaten (über formular_setzen)"
        } else if e.schreibbar {
            "Text"
        } else if e.text {
            "Text, nur lesen"
        } else {
            "nicht lesbar (Bild o. Ä., im Text mit Pfad einbinden)"
        };
        t.push_str(&format!("- {} ({} Bytes): {art}\n", e.pfad, e.bytes));
    }
    Ok(Ergebnis::text(t))
}

/// `datei` relativ zu `wurzel`, mit `/`.
fn relativ(wurzel: &Path, datei: &Path) -> Option<String> {
    let r = datei.strip_prefix(wurzel).ok()?;
    Some(
        r.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn datei_lesen(app: &AppHandle, rel: &str) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let wurzel = &s.projekt.wurzel;
    let p = crate::projektdateien::pfad(wurzel, rel)?;
    let (text, kopf) = if crate::projektdateien::pfad(wurzel, &s.projekt.konfig.haupt)? == p {
        (
            s.spiegel.text().to_string(),
            format!(
                "{rel} (Hauptdatei, Stand im Editor, Rev {})",
                s.spiegel.rev()
            ),
        )
    } else {
        (crate::projektdateien::lesen(wurzel, rel)?, rel.to_string())
    };
    Ok(Ergebnis::text(format!(
        "{kopf}, Zeilennummer und Tabulator stehen nicht im Text:\n{}",
        nummeriert(&text, 1)
    )))
}

/// Liest die Datei (falls vorhanden), wendet `aenderung` an, schreibt mit Sicherung und lässt neu
/// übersetzen (bei Literatur auch neu laden).
fn datei_aendern(
    app: &AppHandle,
    rel: &str,
    aenderung: impl FnOnce(Option<&str>) -> Result<String, String>,
) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_mut().ok_or(KEIN_DOKUMENT)?;
    let wurzel = s.projekt.wurzel.clone();
    let daten = relativ(&wurzel, s.formular.datei()).unwrap_or_default();
    let ziel =
        crate::projektdateien::schreiben_pruefen(&wurzel, rel, &s.projekt.konfig.haupt, &daten)?;
    let alt = if ziel.exists() {
        Some(crate::projektdateien::lesen(&wurzel, rel)?)
    } else {
        None
    };
    let neu = aenderung(alt.as_deref())?;
    let zeit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let sicherung = crate::projektdateien::schreiben(&wurzel, rel, &ziel, &neu, zeit)?;
    if alt.as_deref() == Some(neu.as_str()) {
        return Ok(Ergebnis::text(format!("{rel}: unverändert.")));
    }
    crate::protokollieren(&s.projekt, &format!("MCP: {rel} geschrieben"));
    if ziel
        .extension()
        .is_some_and(|e| ["bib", "yaml", "yml"].contains(&&*e.to_string_lossy()))
    {
        s.literatur = crate::literatur_laden(&s.projekt);
    }
    crate::kompilieren_lassen(&zustand, s);
    let mut t = match (&alt, &sicherung) {
        (None, _) => format!("{rel}: neu angelegt."),
        (Some(_), Some(sich)) => format!("{rel}: geschrieben; der alte Inhalt liegt in {sich}."),
        (Some(_), None) => format!("{rel}: geschrieben."),
    };
    t.push_str(
        " Die Vorschau wird neu übersetzt (Ergebnis mit `status`). Rückgängig im Editor gilt hier \
         nicht.",
    );
    if alt.is_none() && ziel.extension().is_some_and(|e| e == "typ") {
        t.push_str(" Eine neue .typ-Datei erscheint erst, wenn die Hauptdatei sie mit `#include` einbindet.");
    }
    Ok(Ergebnis::text(t))
}

fn literatur_lesen(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let dateien = crate::literatur_dateien_js(s);
    if dateien.is_empty() {
        return Ok(Ergebnis::text(
            "Dieses Dokument hat keine Literaturdatei (bei Typen mit Formular: Feld literatur.dateien).",
        ));
    }
    let mut t = String::new();
    for d in dateien {
        t.push_str(&format!(
            "{} ({}):\n",
            d.pfad,
            if d.schreibbar {
                "schreibbar"
            } else {
                "nur lesen"
            }
        ));
        if let Some(f) = &d.fehler {
            t.push_str(&format!("  {f}\n"));
        }
        for e in &d.eintraege {
            t.push_str(&format!(
                "@{}{{{}}} – {}\n",
                e.eintrag.art,
                e.eintrag.schluessel,
                if e.zitiert {
                    "zitiert"
                } else {
                    "nicht zitiert"
                }
            ));
            for f in &e.eintrag.felder {
                t.push_str(&format!(
                    "    {} = {}{}\n",
                    f.name,
                    f.wert,
                    if f.roh { "  (BibTeX roh)" } else { "" }
                ));
            }
        }
    }
    Ok(Ergebnis::text(t))
}

fn literatur_speichern(app: &AppHandle, a: &Value) -> Result<Ergebnis, String> {
    use tftn_literatur::verwaltung::{BibEintrag, Feld};
    let text = |n: &str| a.get(n).and_then(Value::as_str).map(str::to_string);
    let art = text("art").ok_or("`art` fehlt")?.to_lowercase();
    let schluessel = text("schluessel").ok_or("`schluessel` fehlt")?;
    let felder = a
        .get("felder")
        .and_then(Value::as_object)
        .ok_or("`felder` fehlt (ein Objekt Feldname → Wert)")?;
    let alt = text("alt").filter(|s| !s.trim().is_empty());
    let zustand = app.state::<Zustand>();
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_mut().ok_or(KEIN_DOKUMENT)?;
    let schreibbar: Vec<String> = crate::literatur_dateien_js(s)
        .into_iter()
        .filter(|d| d.schreibbar)
        .map(|d| d.pfad)
        .collect();
    let datei = match text("datei") {
        Some(d) => d,
        None if schreibbar.len() == 1 => schreibbar[0].clone(),
        None if schreibbar.is_empty() => {
            return Err("Das Dokument hat keine .bib-Datei, die sich bearbeiten lässt.".into());
        }
        None => {
            return Err(format!(
                "Mehrere Literaturdateien – bitte `datei` angeben: {}",
                schreibbar.join(", ")
            ));
        }
    };
    let pfad = crate::bib_datei(&s.projekt, &datei)?;
    let vorher = std::fs::read(&pfad).unwrap_or_default();
    // Beim Ändern bleiben unveränderte rohe Werte (Zahlen ohne Klammern, @string-Kürzel) roh.
    let bisher: Vec<Feld> = alt
        .as_ref()
        .and_then(|k| {
            tftn_literatur::verwaltung::lesen(&String::from_utf8_lossy(&vorher))
                .ok()?
                .into_iter()
                .find(|g| &g.eintrag.schluessel == k)
                .map(|g| g.eintrag.felder)
        })
        .unwrap_or_default();
    let mut liste = Vec::new();
    for (name, wert) in felder {
        let wert = match wert {
            Value::String(w) => w.clone(),
            Value::Number(n) => n.to_string(),
            Value::Null => continue,
            _ => return Err(format!("Feld `{name}`: bitte als Text angeben")),
        };
        let name = name.to_lowercase();
        let roh = bisher
            .iter()
            .any(|f| f.roh && f.name == name && f.wert == wert);
        liste.push(Feld { name, wert, roh });
    }
    let eintrag = BibEintrag {
        art,
        schluessel: schluessel.trim().to_string(),
        felder: liste,
    };
    let zeit = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let wurzel = s.projekt.wurzel.clone();
    crate::literatur_speichern(&zustand, s, &datei, alt.clone(), &eintrag)?;
    let sicherung = if vorher.is_empty() {
        None
    } else {
        crate::projektdateien::sichern(&wurzel, &datei, &vorher, zeit).ok()
    };
    let mut t = match &alt {
        Some(a) if a != &eintrag.schluessel => {
            format!(
                "@{a} geändert und umbenannt in @{} ({datei}).",
                eintrag.schluessel
            )
        }
        Some(_) => format!("@{} geändert ({datei}).", eintrag.schluessel),
        None => format!(
            "@{} angelegt ({datei}); im Text mit `@{}` zitieren.",
            eintrag.schluessel, eintrag.schluessel
        ),
    };
    if let Some(sich) = sicherung {
        t.push_str(&format!(" Alte Fassung der Datei: {sich}."));
    }
    if alt.as_ref().is_some_and(|a| a != &eintrag.schluessel) {
        t.push_str(" Zitate mit dem alten Schlüssel im Text müssen mit geändert werden.");
    }
    Ok(Ergebnis::text(t))
}

fn fassung_sichern(app: &AppHandle) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
    let ordner = s.projekt.wurzel.join(crate::fassungen::ORDNER);
    let neu = crate::fassungen::ablegen(&ordner, s.spiegel.text(), crate::fassungen::jetzt(), true)
        .map_err(|e| format!("Fassung nicht gesichert: {e}"))?;
    Ok(Ergebnis::text(if neu {
        format!(
            "Fassung von {} gesichert (Rev {}); der Benutzer holt sie in TFTN über „Fassungen“ zurück.",
            s.projekt.konfig.haupt,
            s.spiegel.rev()
        )
    } else {
        "Der Text ist derselbe wie in der letzten Fassung; nichts Neues gesichert.".to_string()
    }))
}

/// Höchstens so viele unbekannte Wörter gehen an den Assistenten.
const RECHTSCHREIBUNG_GRENZE: usize = 200;

fn rechtschreibung_pruefen(app: &AppHandle, pfad: Option<String>) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let (haupt, quelle, name) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
        let haupt = s.spiegel.quelle().clone();
        match pfad.filter(|p| !p.trim().is_empty()) {
            Some(p)
                if crate::projektdateien::pfad(&s.projekt.wurzel, &p)?
                    != crate::projektdateien::pfad(&s.projekt.wurzel, &s.projekt.konfig.haupt)? =>
            {
                let text = crate::projektdateien::lesen(&s.projekt.wurzel, &p)?;
                (haupt, typst::syntax::Source::detached(text), p)
            }
            _ => (haupt.clone(), haupt, s.projekt.konfig.haupt.clone()),
        }
    };
    let (sprache, falsch) = crate::falsche_woerter(app, &zustand, &haupt, &quelle)?;
    if falsch.is_empty() {
        return Ok(Ergebnis::text(format!(
            "{name}: keine unbekannten Wörter ({sprache})."
        )));
    }
    let z = quelle.lines();
    let stand = zustand.rechtschreibung.lock().unwrap();
    let mut t = format!(
        "{name}, Wörterbuch {sprache}: {} unbekannte Wörter (Zeile: Wort – Vorschläge):\n",
        falsch.len()
    );
    for r in falsch.iter().take(RECHTSCHREIBUNG_GRENZE) {
        let wort = &quelle.text()[r.clone()];
        let vorschlaege = stand
            .as_ref()
            .map(|st| st.pruefer.vorschlaege(wort))
            .unwrap_or_default();
        t.push_str(&format!(
            "- {}: {wort} – {}\n",
            z.byte_to_line(r.start).map_or(0, |l| l + 1),
            if vorschlaege.is_empty() {
                "keine".to_string()
            } else {
                vorschlaege
                    .into_iter()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
    }
    if falsch.len() > RECHTSCHREIBUNG_GRENZE {
        t.push_str(&format!(
            "[… und {} weitere]\n",
            falsch.len() - RECHTSCHREIBUNG_GRENZE
        ));
    }
    Ok(Ergebnis::text(t))
}

/// Schickt dem Editor einen Auftrag und wartet auf seine Antwort (`Ok(false)`: nicht ausgeführt).
fn rufen(app: &AppHandle, mut auftrag: Value, geduld: Duration) -> Result<bool, String> {
    let (tx, rx) = mpsc::channel();
    let id = {
        let zustand = app.state::<Zustand>();
        let mut m = zustand.mcp.lock().unwrap();
        m.naechste += 1;
        let id = m.naechste;
        m.offen.insert(id, tx);
        id
    };
    auftrag["id"] = json!(id);
    app.emit("mcp-auftrag", auftrag)
        .map_err(|e| e.to_string())?;
    let antwort = rx.recv_timeout(geduld);
    app.state::<Zustand>().mcp.lock().unwrap().offen.remove(&id);
    antwort.map_err(|_| "TFTN hat nicht rechtzeitig geantwortet.".to_string())?
}

/// Antwort des Editors auf einen Auftrag (Befehl `mcp_antwort`).
pub fn antwort(app: &AppHandle, id: u64, ok: bool, fehler: Option<String>) {
    let tx = app.state::<Zustand>().mcp.lock().unwrap().offen.remove(&id);
    if let Some(tx) = tx {
        let _ = tx.send(match fehler {
            Some(f) => Err(f),
            None => Ok(ok),
        });
    }
}

/// Wartet, bis `fertig` gilt (höchstens `geduld`).
fn warten(geduld: Duration, mut fertig: impl FnMut() -> bool) -> bool {
    let ende = Instant::now() + geduld;
    loop {
        if fertig() {
            return true;
        }
        if Instant::now() >= ende {
            return false;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

/// Ändert den Text über den Editor; tippt der Benutzer dazwischen, wird neu gerechnet.
fn aendern(
    app: &AppHandle,
    neuer_text: impl Fn(&str) -> Result<String, String>,
) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    for _ in 0..5 {
        let (patch, rev) = {
            let sitzung = zustand.sitzung.lock().unwrap();
            let s = sitzung.as_ref().ok_or(KEIN_DOKUMENT)?;
            let alt = s.spiegel.text();
            let neu = neuer_text(alt)?;
            if neu == alt {
                return Ok(Ergebnis::text("Nichts zu ändern: Der Text ist schon so."));
            }
            let (bereich, text) = tftn_mcp::kleinster_bereich(alt, &neu);
            let rev = s.spiegel.rev();
            let patch =
                crate::patch_js(s.spiegel.quelle(), rev, tftn_edit::Patch { bereich, text })?;
            (patch, rev)
        };
        let auftrag = json!({ "art": "patch", "patch": patch });
        if !rufen(app, auftrag, Duration::from_secs(10))? {
            std::thread::sleep(Duration::from_millis(100));
            continue; // veraltet
        }
        // Der Editor meldet die Änderung über `aendern`; dann läuft Typst.
        let mut neu_rev = rev;
        warten(Duration::from_secs(5), || {
            neu_rev = zustand
                .sitzung
                .lock()
                .unwrap()
                .as_ref()
                .map_or(rev, |s| s.spiegel.rev());
            neu_rev > rev
        });
        if neu_rev > rev {
            warten(Duration::from_secs(30), || {
                zustand
                    .lauf
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|l| l.rev >= neu_rev)
            });
        }
        let lauf = zustand.lauf.lock().unwrap().clone();
        return Ok(Ergebnis::text(aenderung_text(rev, neu_rev, lauf.as_ref())));
    }
    Err("Der Benutzer schreibt gerade; bitte gleich noch einmal versuchen.".into())
}

/// Antwort auf eine Änderung. Kam die neue Revision nicht rechtzeitig an, ist der letzte Lauf noch
/// der des alten Texts und kein Ergebnis der Änderung.
fn aenderung_text(rev: u64, neu_rev: u64, lauf: Option<&LaufStand>) -> String {
    if neu_rev <= rev {
        return format!(
            "Der Editor hat die Änderung angenommen, aber noch nicht gemeldet (Rev {rev}); das \
             Ergebnis bitte gleich mit `status` prüfen."
        );
    }
    format!(
        "Geändert (jetzt Rev {neu_rev}); im Editor mit Rückgängig zurückzunehmen.\n{}",
        lauf_text(lauf, neu_rev)
    )
}

fn seite_ansehen(app: &AppHandle, seite: usize, ppi: f64) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    let (bild, anzahl) = {
        let anzeige = zustand.anzeige.lock().unwrap();
        let a = anzeige
            .as_ref()
            .ok_or("Es gibt noch keine Vorschau (Typst-Lauf fehlgeschlagen oder läuft noch).")?;
        let seiten = a.dokument.pages();
        let s = seite
            .checked_sub(1)
            .and_then(|i| seiten.get(i))
            .ok_or_else(|| {
                format!(
                    "Seite {seite} gibt es nicht; das Dokument hat {} Seite(n).",
                    seiten.len()
                )
            })?
            .clone();
        (s, seiten.len())
    };
    let png = tftn_core::vorschau::rastern(&bild, ppi.clamp(30.0, 300.0));
    Ok(Ergebnis {
        inhalt: vec![
            Inhalt::Text {
                text: format!("Seite {seite} von {anzahl} (letzter guter Stand der Vorschau)"),
            },
            Inhalt::Image {
                data: typen::base64(&png),
                mime_type: "image/png".into(),
            },
        ],
        fehler: false,
    })
}

fn typen_auflisten(app: &AppHandle) -> Result<Ergebnis, String> {
    let (quellen, eigen) = crate::typquellen(app)?;
    let liste = typen::typen_js(&quellen, &eigen, None);
    let mut t = String::from("Dokumenttypen (Name für `dokument_anlegen`):");
    for x in liste.typen {
        t.push_str(&format!("\n- {} ({}): {}", x.name, x.version, x.titel));
        if let Some(b) = x.beschreibung {
            t.push_str(&format!(" – {b}"));
        }
    }
    Ok(Ergebnis::text(t))
}

/// Wartezeiten eines Wechsels: auf den Benutzer (Speichern, Name für ein Dokument ohne Namen), auf das
/// Öffnen und auf den ersten Typst-Lauf. Zusammen kürzer als die Geduld der Brücke
/// (`tftn_mcp::GEDULD`), damit der Assistent eine Antwort bekommt statt eines Abbruchs.
const WECHSEL_RUFEN: Duration = Duration::from_secs(230);
const WECHSEL_OFFEN: Duration = Duration::from_secs(40);
const WECHSEL_LAUF: Duration = Duration::from_secs(15);

/// Öffnet `ziel` wie „Öffnen …“: Der Editor speichert (fragt bei einem Dokument ohne Namen nach
/// einem) und lädt neu; dann wartet der Aufruf, bis das neue Dokument offen ist.
fn wechseln(app: &AppHandle, ziel: PathBuf) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    zustand.mcp.lock().unwrap().ziel = Some(ziel.clone());
    let gewechselt = rufen(app, json!({ "art": "wechseln" }), WECHSEL_RUFEN);
    if !matches!(gewechselt, Ok(true)) {
        zustand.mcp.lock().unwrap().ziel = None;
        let mut naechstes = zustand.naechstes.lock().unwrap();
        if naechstes.as_ref() == Some(&ziel) {
            *naechstes = None;
            *zustand.wechsel.lock().unwrap() = Wechsel::Benutzer;
        }
        return Err(match gewechselt {
            Err(e) => e,
            _ => "Der Benutzer hat den Wechsel abgebrochen; das bisherige Dokument bleibt offen."
                .into(),
        });
    }
    let wurzel = ziel.canonicalize().unwrap_or(ziel.clone());
    let offen = warten(WECHSEL_OFFEN, || {
        zustand
            .sitzung
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.projekt.wurzel == wurzel)
    });
    if !offen {
        return Err(format!("{} wurde nicht geöffnet.", ziel.display()));
    }
    // Erst antworten, wenn Typst das neue Dokument einmal übersetzt hat (`oeffnen` leert den Lauf).
    warten(WECHSEL_LAUF, || zustand.lauf.lock().unwrap().is_some());
    let lauf = zustand.lauf.lock().unwrap().clone();
    let mut t = format!(
        "Geöffnet: {}.\n{}",
        wurzel.display(),
        lauf_text(lauf.as_ref(), lauf.as_ref().map_or(0, |l| l.rev))
    );
    if zustand.mcp.lock().unwrap().wartet {
        t.push_str(
            "\nMCP ist pausiert, bis der Benutzer in TFTN bestätigt, dass es für dieses Dokument aktiv \
             bleibt; bis dahin werden Aufrufe abgelehnt.",
        );
    }
    Ok(Ergebnis::text(t))
}

/// Konfigurationsordner der App ohne Tauri (für die Brücke), wie `app_config_dir` mit der Kennung
/// aus `tauri.conf.json`.
pub fn konfig_ordner() -> Option<PathBuf> {
    const KENNUNG: &str = "org.tftn.editor";
    let umgebung = |n: &str| {
        std::env::var_os(n)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let basis = if cfg!(target_os = "macos") {
        umgebung("HOME").map(|h| h.join("Library/Application Support"))
    } else if cfg!(windows) {
        umgebung("APPDATA")
    } else {
        umgebung("XDG_CONFIG_HOME").or_else(|| umgebung("HOME").map(|h| h.join(".config")))
    };
    Some(basis?.join(KENNUNG))
}

/// `tftn-app --mcp`: Brücke zwischen Assistent (stdin/stdout) und laufender App.
pub fn bruecke() -> std::process::ExitCode {
    let Some(ordner) = konfig_ordner() else {
        eprintln!("{}", tr!("Konfigurationsordner nicht gefunden"));
        return std::process::ExitCode::FAILURE;
    };
    let ein = std::io::stdin().lock();
    let aus = std::io::stdout().lock();
    match tftn_mcp::bruecke(ein, aus, |w, a| tftn_mcp::weiterreichen(&ordner, w, a)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("MCP: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn im_appimage_die_appimage_datei() {
        let exe = PathBuf::from("/tmp/.mount_TFTNab/usr/bin/tftn-app");
        let datei = std::env::temp_dir().join(format!("tftn-mcp-appimage-{}", std::process::id()));
        std::fs::write(&datei, b"").unwrap();
        assert_eq!(
            programm_aus(Some(datei.clone()), Some(exe.clone())),
            datei.display().to_string()
        );
        // Ohne AppImage (oder wenn es die Datei nicht gibt): das Programm selbst.
        assert_eq!(
            programm_aus(None, Some(exe.clone())),
            exe.display().to_string()
        );
        assert_eq!(
            programm_aus(
                Some(PathBuf::from("/gibt/es/nicht.AppImage")),
                Some(exe.clone())
            ),
            exe.display().to_string()
        );
        let _ = std::fs::remove_file(&datei);
    }

    #[test]
    fn vormerken_gewinnt_gegen_das_benennen_des_entwurfs() {
        // Der Entwurf wurde vor dem Wechsel benannt: das merkt sich den umbenannten Ordner.
        let mut naechstes = Some(PathBuf::from("/Dokumente/Benannt"));
        let mut wechsel = Wechsel::Umbenannt;
        vormerken(Some(PathBuf::from("/ziel")), &mut naechstes, &mut wechsel).unwrap();
        assert_eq!(naechstes, Some(PathBuf::from("/ziel")));
        assert_eq!(wechsel, Wechsel::Mcp);

        let mut naechstes = None;
        let mut wechsel = Wechsel::Benutzer;
        assert!(vormerken(None, &mut naechstes, &mut wechsel).is_err());
        assert_eq!((naechstes, wechsel), (None, Wechsel::Benutzer));
    }

    #[test]
    fn lauf_text_nennt_seiten_meldungen_und_rueckstand() {
        assert!(lauf_text(None, 1).contains("noch nicht"));
        let l = LaufStand {
            rev: 4,
            gelungen: false,
            seiten: 0,
            meldungen: vec!["Fehler (Zeile 3, Spalte 1): unknown variable: x".into()],
        };
        let t = lauf_text(Some(&l), 5);
        assert!(t.contains("fehlgeschlagen"));
        assert!(t.contains("Rev 5 wird noch"));
        assert!(t.contains("\n- Fehler (Zeile 3"));
        let l = LaufStand {
            gelungen: true,
            seiten: 2,
            meldungen: vec![],
            ..l
        };
        assert_eq!(
            lauf_text(Some(&l), 4),
            "Typst-Lauf (Rev 4): gelungen, 2 Seite(n)."
        );
    }

    #[test]
    fn wechsel_antwortet_bevor_die_bruecke_aufgibt() {
        let summe = WECHSEL_RUFEN + WECHSEL_OFFEN + WECHSEL_LAUF;
        assert!(
            summe + Duration::from_secs(10) <= tftn_mcp::GEDULD,
            "{summe:?} gegen {:?}",
            tftn_mcp::GEDULD
        );
    }

    #[test]
    fn aenderung_ohne_neue_revision_sagt_das() {
        let l = LaufStand {
            rev: 4,
            gelungen: true,
            seiten: 1,
            meldungen: vec![],
        };
        let t = aenderung_text(4, 4, Some(&l));
        assert!(t.contains("noch nicht"), "{t}");
        assert!(
            !t.contains("Typst-Lauf (Rev 4)"),
            "kein alter Lauf als Ergebnis: {t}"
        );
        let t = aenderung_text(4, 5, Some(&LaufStand { rev: 5, ..l }));
        assert!(t.starts_with("Geändert (jetzt Rev 5)"), "{t}");
        assert!(t.contains("Typst-Lauf (Rev 5): gelungen"), "{t}");
    }

    #[test]
    fn konfig_ordner_endet_auf_kennung() {
        let o = konfig_ordner().expect("HOME/APPDATA gesetzt");
        assert!(o.ends_with("org.tftn.editor"));
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["identifier"], "org.tftn.editor",
            "Kennung in tauri.conf.json geändert"
        );
    }

    #[test]
    fn gliederung_mit_zeilen_und_kommentaren() {
        let q = typst::syntax::Source::detached(
            "= Eins\nText eins zwei.\n// zu kurz\n== Unter\nmehr\n/* lang\n   zweite */\n",
        );
        let t = gliederung_text(&q, vec![(2, "#include x\n".into())]);
        assert!(t.contains("= Eins (Zeile 1, "), "{t}");
        assert!(
            t.contains(" Wörter)\n#include x\n  == Unter"),
            "an seiner Stelle: {t}"
        );
        assert!(t.contains("  == Unter (Zeile 4, "), "{t}");
        assert!(t.contains("- Zeile 3: zu kurz"), "{t}");
        assert!(t.contains("- Zeile 6: lang zweite"), "{t}");
        assert_eq!(
            gliederung_text(&typst::syntax::Source::detached("nur Text"), Vec::new()),
            "(keine Überschriften, keine Kommentare)\n"
        );
    }

    #[test]
    fn felder_knapp_fuer_den_assistenten() {
        let schema = include_str!("../../types/tftn-fachartikel/tftn/formular.schema.json");
        let f = tftn_formular::formular_lesen(schema).unwrap();
        let j = felder_js(&f.felder);
        let feld = |n: &str| {
            j.as_array()
                .unwrap()
                .iter()
                .find(|x| x["name"] == n)
                .unwrap()
                .clone()
        };
        assert_eq!(feld("titel")["art"], "text");
        assert_eq!(feld("datum")["art"], "datum (JJJJ-MM-TT)");
        let autoren = feld("autoren");
        assert_eq!(autoren["art"], "eintraege");
        assert_eq!(autoren["felder"][0]["name"], "name");
        assert_eq!(autoren["felder"][0]["pflicht"], true);
        assert_eq!(feld("literatur")["art"], "gruppe");
    }
}
