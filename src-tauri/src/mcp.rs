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
use std::io::{BufRead, BufReader, Write};
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
    port: u16,
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
        programm: std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        fehler,
    }
}

fn melden(app: &AppHandle) {
    let _ = app.emit("mcp-geaendert", js(app, None));
}

/// Schaltet ein oder aus und merkt es für den nächsten Start.
pub fn setzen(app: &AppHandle, an: bool) -> Result<(), String> {
    let konfig = app_konfig(app)?;
    let mut rechner = tftn_einstellungen::Rechner::laden(&konfig);
    if an {
        starten(app, &konfig)?;
    } else {
        stoppen(app, &konfig);
    }
    app.state::<Zustand>().mcp.lock().unwrap().wartet = false;
    if rechner.mcp != an {
        rechner.mcp = an;
        rechner.speichern(&konfig).map_err(|e| e.to_string())?;
    }
    melden(app);
    Ok(())
}

pub fn beim_wechsel_aus_setzen(app: &AppHandle, aus: bool) -> Result<(), String> {
    let konfig = app_konfig(app)?;
    let mut rechner = tftn_einstellungen::Rechner::laden(&konfig);
    rechner.mcp_beim_wechsel_aus = aus;
    rechner.speichern(&konfig).map_err(|e| e.to_string())?;
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

/// Beim Start: an, wenn es zuletzt an war; eine liegengebliebene Verbindungsdatei weg.
pub fn beim_start(app: &AppHandle) {
    let Ok(konfig) = app_konfig(app) else { return };
    Verbindung::entfernen(&konfig);
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
    Verbindung {
        port,
        schluessel: schluessel.clone(),
    }
    .schreiben(konfig)
    .map_err(|e| e.to_string())?;
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
    m.horcher = Some(Horcher { port, stopp });
    Ok(())
}

fn stoppen(app: &AppHandle, konfig: &Path) {
    let horcher = app.state::<Zustand>().mcp.lock().unwrap().horcher.take();
    Verbindung::entfernen(konfig);
    if let Some(h) = horcher {
        h.stopp.store(true, Ordering::SeqCst);
        // `accept` aufwecken, damit der Faden endet.
        let _ = TcpStream::connect_timeout(
            &(Ipv4Addr::LOCALHOST, h.port).into(),
            Duration::from_millis(500),
        );
    }
}

/// Eine Verbindung der Brücke: ein Auftrag, eine Antwort.
fn bedienen(app: &AppHandle, strom: TcpStream, schluessel: &str) {
    let _ = strom.set_read_timeout(Some(Duration::from_secs(10)));
    let mut zeile = String::new();
    if BufReader::new(&strom).read_line(&mut zeile).is_err() {
        return;
    }
    let ergebnis = match serde_json::from_str::<Auftrag>(&zeile) {
        Err(e) => Ergebnis::fehler(format!("unlesbarer Auftrag: {e}")),
        Ok(a) if a.schluessel != schluessel => Ergebnis::fehler("falscher Schlüssel"),
        Ok(a) => {
            let (an, wartet) = {
                let zustand = app.state::<Zustand>();
                let m = zustand.mcp.lock().unwrap();
                (m.horcher.is_some(), m.wartet)
            };
            if !an {
                Ergebnis::fehler("MCP ist in TFTN ausgeschaltet.")
            } else if wartet {
                Ergebnis::fehler(
                    "MCP ist pausiert: Der Benutzer muss in TFTN bestätigen, dass MCP für das neu \
                     geöffnete Dokument aktiv bleibt. Bitte den Benutzer darum bitten.",
                )
            } else {
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
        warten(Duration::from_secs(30), || {
            zustand
                .lauf
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|l| l.rev >= neu_rev)
        });
        let lauf = zustand.lauf.lock().unwrap().clone();
        return Ok(Ergebnis::text(format!(
            "Geändert (jetzt Rev {neu_rev}); im Editor mit Rückgängig zurückzunehmen.\n{}",
            lauf_text(lauf.as_ref(), neu_rev)
        )));
    }
    Err("Der Benutzer schreibt gerade; bitte gleich noch einmal versuchen.".into())
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

/// Öffnet `ziel` wie „Öffnen …“: Der Editor speichert (fragt bei einem Dokument ohne Namen nach
/// einem) und lädt neu; dann wartet der Aufruf, bis das neue Dokument offen ist.
fn wechseln(app: &AppHandle, ziel: PathBuf) -> Result<Ergebnis, String> {
    let zustand = app.state::<Zustand>();
    zustand.mcp.lock().unwrap().ziel = Some(ziel.clone());
    let gewechselt = rufen(app, json!({ "art": "wechseln" }), Duration::from_secs(280));
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
    let offen = warten(Duration::from_secs(60), || {
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
    let mut t = format!("Geöffnet: {}.", wurzel.display());
    if zustand.mcp.lock().unwrap().wartet {
        t.push_str(
            " MCP ist pausiert, bis der Benutzer in TFTN bestätigt, dass es für dieses Dokument aktiv \
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
    fn konfig_ordner_endet_auf_kennung() {
        let o = konfig_ordner().expect("HOME/APPDATA gesetzt");
        assert!(o.ends_with("org.tftn.editor"));
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["identifier"], "org.tftn.editor",
            "Kennung in tauri.conf.json geändert"
        );
    }
}
