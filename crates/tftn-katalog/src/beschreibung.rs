//! Paketbeschreibungen: Bausteine für Universe-Pakete ohne `[tool.tftn]` (Entwurf 5, Quelle 3).
//!
//! Eine Datei je Paket und Versionsbereich, flach in einem Ordner:
//! `paket`, `versionen` (semver-Bereich), `import` (mit `{version}`), optional `titel`, `empfohlen`
//! (die geprüfte Version) und `auch` (zusätzlich zu importierende Pakete), dann `[[baustein]]` wie im
//! Manifest.

use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::Deserialize;
use tftn_manifest::Baustein;
use typst_syntax::{SyntaxKind, ast};

#[derive(Debug, Clone)]
pub struct Beschreibung {
    pub paket: String,
    pub versionen: semver::VersionReq,
    pub import: String,
    /// Anzeigename, z. B. „Diagramme aus Daten“
    pub titel: Option<String>,
    /// Die in `tftn typ-pruefen` geprüfte Version; so holt die App das Paket.
    pub empfohlen: Option<semver::Version>,
    /// Pakete, die zusätzlich importiert werden müssen (in der Version, die dieses Paket nutzt)
    pub auch: Vec<String>,
    pub baustein: Vec<Baustein>,
    pub datei: PathBuf,
}

#[derive(Deserialize)]
struct Datei {
    paket: String,
    versionen: String,
    import: String,
    titel: Option<String>,
    empfohlen: Option<String>,
    #[serde(default)]
    auch: Vec<String>,
    #[serde(default)]
    baustein: Vec<Baustein>,
}

/// Alle `*.toml` in `ordner`, nach Dateiname sortiert. Unlesbare Dateien werden zu Warnungen.
pub fn beschreibungen_lesen(ordner: &Path) -> (Vec<Beschreibung>, Vec<String>) {
    let mut dateien: Vec<PathBuf> = match std::fs::read_dir(ordner) {
        Ok(liste) => liste
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect(),
        // Ein eigener Ordner muss nicht da sein.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Vec::new(), Vec::new()),
        Err(e) => return (Vec::new(), vec![format!("{}: {e}", ordner.display())]),
    };
    dateien.sort();
    let mut aus = Vec::new();
    let mut warnungen = Vec::new();
    for datei in dateien {
        match lesen(&datei) {
            Ok(b) => aus.push(b),
            Err(e) => warnungen.push(format!("{}: {e}", datei.display())),
        }
    }
    (aus, warnungen)
}

fn lesen(datei: &Path) -> Result<Beschreibung, String> {
    let text = std::fs::read_to_string(datei).map_err(|e| e.to_string())?;
    let d: Datei = toml::from_str(&text).map_err(|e| e.to_string())?;
    let versionen = semver::VersionReq::parse(&d.versionen).map_err(|e| {
        tr!(
            "versionen = {wert}: {fehler}",
            wert = format!("{:?}", d.versionen),
            fehler = e
        )
    })?;
    import_pruefen(&d.import, &d.paket)?;
    let empfohlen = d
        .empfohlen
        .as_deref()
        .map(|t| {
            semver::Version::parse(t).map_err(|e| {
                tr!(
                    "empfohlen = {wert}: {fehler}",
                    wert = format!("{t:?}"),
                    fehler = e
                )
            })
        })
        .transpose()?;
    if let Some(v) = empfohlen.as_ref().filter(|v| !versionen.matches(v)) {
        return Err(tr!(
            "empfohlen = {v} liegt außerhalb von {bereich}",
            v,
            bereich = d.versionen
        ));
    }
    for name in &d.auch {
        tftn_pakete::PaketSpec::neu(name, "0.0.0")
            .map_err(|e| tr!("auch: {fehler}", fehler = e))?;
    }
    Ok(Beschreibung {
        titel: d.titel,
        empfohlen,
        auch: d.auch,
        paket: d.paket,
        versionen,
        import: d.import,
        baustein: d.baustein,
        datei: datei.to_path_buf(),
    })
}

/// `import` muss genau ein `#import` von `@preview/<paket>:{version}` sein: Eine Beschreibung darf
/// nichts anderes in das Dokument schreiben.
fn import_pruefen(import: &str, paket: &str) -> Result<(), String> {
    let text = import.replace("{version}", "0.0.0");
    let baum = typst_syntax::parse(&text);
    let teile: Vec<_> = baum.children().filter(|k| !k.kind().is_trivia()).collect();
    let fehler = || {
        tr!(
            "import muss genau ein #import von @preview/{paket}:{version} sein",
            paket
        )
    };
    let [raute, anweisung] = teile.as_slice() else {
        return Err(fehler());
    };
    if raute.kind() != SyntaxKind::Hash || baum.diagnosis().either() {
        return Err(fehler());
    }
    let quelle = anweisung
        .cast::<ast::ModuleImport>()
        .map(|i| i.source())
        .ok_or_else(fehler)?;
    match quelle {
        ast::Expr::Str(s) if s.get() == format!("@preview/{paket}:0.0.0").as_str() => Ok(()),
        _ => Err(fehler()),
    }
}

/// Universe-Pakete im Paketordner des Projekts (`Projekt::pakete()`):
/// `preview/<name>/<version>/typst.toml`.
pub fn projektpakete(pakete: &Path) -> Vec<(String, semver::Version)> {
    let mut aus = Vec::new();
    let Ok(namen) = std::fs::read_dir(pakete.join("preview")) else {
        return aus;
    };
    for name in namen.filter_map(Result::ok) {
        let Ok(versionen) = std::fs::read_dir(name.path()) else {
            continue;
        };
        for v in versionen.filter_map(Result::ok) {
            let version = v.file_name().to_string_lossy().into_owned();
            if let (true, Ok(version)) = (
                v.path().join("typst.toml").is_file(),
                semver::Version::parse(&version),
            ) {
                aus.push((name.file_name().to_string_lossy().into_owned(), version));
            }
        }
    }
    aus.sort();
    aus
}
