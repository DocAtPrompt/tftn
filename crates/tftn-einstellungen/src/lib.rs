//! Einstellungen von TFTN (Entwurf 2026-11-04 Abschnitt 7, PLAN-M2 AP3).
//!
//! Je Rechner (im Konfigurationsordner der App): `rechner.toml` mit dem Zeiger auf den Ordner im
//! Tresor und „ganz offline“. Synchronisiert (im gewählten Ordner, sonst im Konfigurationsordner):
//! `tasten.toml` (eigene Kürzel) und `einstellungen.toml`. Geschrieben wird mit `toml_edit`, damit
//! Kommentare und fremde Einträge bleiben, und atomar über eine temporäre Datei.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};

/// Eigene Kürzel, synchronisiert.
pub const TASTEN: &str = "tasten.toml";
/// Helfer-Optionen, synchronisiert (Felder ab AP8/AP10).
pub const EINSTELLUNGEN: &str = "einstellungen.toml";
/// Unterordner für eigene Paketbeschreibungen (PLAN-M2 AP10).
pub const BESCHREIBUNGEN: &str = "beschreibungen";
/// Je Rechner, im Konfigurationsordner der App.
pub const RECHNER: &str = "rechner.toml";

const KOPF: &str =
    "# TFTN-Tastenkürzel (Mod = Cmd am Mac, Strg unter Windows). \"\" = keine Belegung.\n";

/// Was nur auf diesem Rechner gilt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rechner {
    /// Ordner im Tresor für `tasten.toml` und `einstellungen.toml`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordner: Option<PathBuf>,
    /// Keine Verbindung ins Netz, auch nicht auf Klick (Entwurf 8.2)
    #[serde(default)]
    pub ganz_offline: bool,
    /// Sprache der Oberfläche (`de`, `en`, …); fehlt sie, gilt die des Systems.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprache: Option<String>,
}

impl Rechner {
    /// Fehlt die Datei oder ist sie fehlerhaft, gilt der Standard.
    pub fn laden(app_konfig: &Path) -> Rechner {
        std::fs::read_to_string(app_konfig.join(RECHNER))
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn speichern(&self, app_konfig: &Path) -> io::Result<()> {
        let text = toml::to_string(self).map_err(io::Error::other)?;
        schreiben(&app_konfig.join(RECHNER), &text)
    }
}

/// Wo die synchronisierten Dateien gerade liegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ort {
    pub ordner: PathBuf,
    pub synchronisiert: bool,
    pub hinweis: Option<String>,
}

/// Der gewählte Ordner, wenn es ihn gibt; sonst der Konfigurationsordner der App mit Hinweis.
pub fn ort(app_konfig: &Path, rechner: &Rechner) -> Ort {
    match &rechner.ordner {
        Some(o) if o.is_dir() => Ort {
            ordner: o.clone(),
            synchronisiert: true,
            hinweis: None,
        },
        Some(o) => Ort {
            ordner: app_konfig.to_path_buf(),
            synchronisiert: false,
            hinweis: Some(tr!(
                "Ordner {ordner} nicht gefunden – Kürzel gelten nur auf diesem Rechner.",
                ordner = o.display()
            )),
        },
        None => Ort {
            ordner: app_konfig.to_path_buf(),
            synchronisiert: false,
            hinweis: Some(tr!(
                "Kein Ordner im Tresor gewählt – Kürzel werden nicht synchronisiert."
            )),
        },
    }
}

/// Inhalt von `tasten.toml`: Aktion → Kürzel (`""` = keine Belegung).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tasten {
    pub kuerzel: BTreeMap<String, String>,
    pub fehler: Option<String>,
}

#[derive(Deserialize)]
struct TastenDatei {
    #[serde(default)]
    kuerzel: toml::Table,
}

/// Fehlt die Datei: leer. Ist sie fehlerhaft: leer mit Fehler (Datei und Zeile).
pub fn tasten_lesen(ordner: &Path) -> Tasten {
    let Ok(text) = std::fs::read_to_string(ordner.join(TASTEN)) else {
        return Tasten::default();
    };
    let datei: TastenDatei = match toml::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            return Tasten {
                kuerzel: BTreeMap::new(),
                fehler: Some(fehlertext(&text, &e)),
            };
        }
    };
    let mut kuerzel = BTreeMap::new();
    let mut falsch = Vec::new();
    for (aktion, wert) in datei.kuerzel {
        match wert {
            toml::Value::String(k) => {
                kuerzel.insert(aktion, k);
            }
            _ => falsch.push(aktion),
        }
    }
    Tasten {
        kuerzel,
        fehler: (!falsch.is_empty()).then(|| {
            tr!(
                "{datei}: kein Text bei {aktionen} – übersprungen",
                datei = TASTEN,
                aktionen = falsch.join(", ")
            )
        }),
    }
}

fn fehlertext(text: &str, e: &toml::de::Error) -> String {
    let zeile = e
        .span()
        .map(|s| text[..s.start.min(text.len())].matches('\n').count() + 1);
    match zeile {
        Some(z) => tr!(
            "{datei}, Zeile {zeile}: {meldung}",
            datei = TASTEN,
            zeile = z,
            meldung = e.message()
        ),
        None => tr!("{datei}: {meldung}", datei = TASTEN, meldung = e.message()),
    }
}

/// Setzt (`Some`) oder entfernt (`None`) die eigene Belegung einer Aktion. Eine fehlerhafte Datei
/// wird nicht angefasst.
pub fn kuerzel_setzen(ordner: &Path, aktion: &str, kuerzel: Option<&str>) -> Result<(), String> {
    let pfad = ordner.join(TASTEN);
    let text = match std::fs::read_to_string(&pfad) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => format!("{KOPF}[kuerzel]\n"),
        Err(e) => return Err(format!("{}: {e}", pfad.display())),
    };
    let fehlerhaft = |grund: String| {
        tr!(
            "{datei} ist fehlerhaft ({grund}) – bitte erst reparieren",
            datei = TASTEN,
            grund
        )
    };
    if let Err(e) = toml::from_str::<TastenDatei>(&text) {
        return Err(fehlerhaft(fehlertext(&text, &e)));
    }
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e| fehlerhaft(format!("{e}")))?;
    if !doc.contains_key("kuerzel") {
        doc["kuerzel"] = toml_edit::table();
    }
    let tabelle = doc["kuerzel"]
        .as_table_mut()
        .ok_or_else(|| fehlerhaft(tr!("[kuerzel] ist keine Tabelle")))?;
    match kuerzel {
        Some(k) => match tabelle.get_mut(aktion).and_then(|i| i.as_value_mut()) {
            // Wert ersetzen, Schmuck (Kommentar dahinter, Abstände) behalten.
            Some(v) => {
                let schmuck = v.decor().clone();
                *v = toml_edit::Value::from(k);
                *v.decor_mut() = schmuck;
            }
            None => {
                tabelle.insert(aktion, toml_edit::value(k));
            }
        },
        None => {
            tabelle.remove(aktion);
        }
    }
    schreiben(&pfad, &doc.to_string()).map_err(|e| format!("{}: {e}", pfad.display()))
}

/// Kopiert die synchronisierten Dateien, die in `nach` fehlen; vorhandene gewinnen.
pub fn umziehen(von: &Path, nach: &Path) -> io::Result<Vec<String>> {
    let mut kopiert = Vec::new();
    for name in [TASTEN, EINSTELLUNGEN] {
        let quelle = von.join(name);
        if quelle.is_file() && !nach.join(name).exists() {
            let text = std::fs::read(&quelle)?;
            schreiben_bytes(&nach.join(name), &text)?;
            kopiert.push(name.to_string());
        }
    }
    Ok(kopiert)
}

/// Atomar: erst `.<name>.tmp` im selben Ordner, dann umbenennen.
fn schreiben(pfad: &Path, text: &str) -> io::Result<()> {
    schreiben_bytes(pfad, text.as_bytes())
}

fn schreiben_bytes(pfad: &Path, daten: &[u8]) -> io::Result<()> {
    let ordner = pfad
        .parent()
        .ok_or_else(|| io::Error::other("Pfad ohne Ordner"))?;
    std::fs::create_dir_all(ordner)?;
    let name = pfad
        .file_name()
        .ok_or_else(|| io::Error::other("Pfad ohne Dateinamen"))?;
    let tmp = ordner.join(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::write(&tmp, daten)?;
    std::fs::rename(&tmp, pfad)
}

/// Meldet Änderungen an `tasten.toml` und `einstellungen.toml` in `ordner` (nicht rekursiv), auch
/// wenn ein Sync-Programm die Datei durch Umbenennen ersetzt. Andere Dateien, auch die eigenen
/// temporären `.<name>.tmp`, lösen nichts aus.
pub fn beobachten(
    ordner: &Path,
    bei_aenderung: impl Fn() + Send + 'static,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::Watcher;
    let mut w = notify::recommended_watcher(move |e: notify::Result<notify::Event>| {
        let Ok(e) = e else { return };
        if e.kind.is_access() {
            return;
        }
        let betrifft = e.paths.iter().any(|p| {
            let beschreibung = p.extension().is_some_and(|x| x == "toml")
                && p.parent()
                    .and_then(Path::file_name)
                    .is_some_and(|n| n == BESCHREIBUNGEN);
            beschreibung
                || p.file_name()
                    .is_some_and(|n| n == TASTEN || n == EINSTELLUNGEN)
        });
        if betrifft {
            bei_aenderung();
        }
    })?;
    // Rekursiv, damit auch `beschreibungen/` ankommt, wenn der Ordner erst später entsteht.
    w.watch(ordner, notify::RecursiveMode::Recursive)?;
    Ok(w)
}
