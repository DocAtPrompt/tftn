//! Export über die beigelegte typst-CLI (KONZEPT 4.8, E12).
//!
//! Jedes Export-Ergebnis entsteht mit genau der Gegenprobe-Zeile (KONZEPT 2.1).
//! Vorher prüft die netzlose World, dass alle Pakete im Projekt liegen, damit
//! die CLI nie etwas herunterlädt.

pub mod profil;

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tftn_sprachen::tr;

pub use tftn_core::Meldung;
pub use tftn_core::gegenprobe::{gegenprobe_argumente, gegenprobe_zeile};
use tftn_core::{Projekt, ProjektWorld, kompilieren};

/// Obergrenze für einen CLI-Lauf (Projektregel: jede lange Arbeit hat eine Frist).
pub const STANDARD_FRIST: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub enum ExportFehler {
    CliNichtGefunden,
    FalscheVersion {
        pfad: PathBuf,
        gefunden: String,
        erwartet: String,
    },
    FehlendePakete(Vec<String>),
    Kompilieren(Vec<Meldung>),
    Cli {
        status: String,
        ausgabe: String,
    },
    Frist(Duration),
    Sonstiges(String),
}

impl fmt::Display for ExportFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CliNichtGefunden => f.write_str(&tr!(
                "typst-CLI nicht gefunden. TFTN_TYPST setzen oder typst auf den PATH legen."
            )),
            Self::FalscheVersion {
                pfad,
                gefunden,
                erwartet,
            } => f.write_str(&tr!(
                "{pfad} ist typst {gefunden}, das Projekt verlangt typst {erwartet}. Export abgebrochen.",
                pfad = pfad.display(),
                gefunden,
                erwartet
            )),
            Self::FehlendePakete(p) => f.write_str(&tr!(
                "Diese Pakete fehlen in packages/ (die CLI würde sie aus dem Netz laden): {pakete}",
                pakete = p.join(", ")
            )),
            Self::Kompilieren(m) => {
                writeln!(f, "{}", tr!("Das Dokument hat Fehler:"))?;
                for meldung in m {
                    writeln!(f, "  {meldung}")?;
                }
                Ok(())
            }
            Self::Cli { status, ausgabe } => {
                f.write_str(&tr!("typst-CLI meldet {status}:\n{ausgabe}", status, ausgabe))
            }
            Self::Frist(d) => f.write_str(&tr!(
                "typst-CLI nach {sekunden} s abgebrochen",
                sekunden = d.as_secs()
            )),
            Self::Sonstiges(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for ExportFehler {}

/// Eine geprüfte typst-CLI.
#[derive(Debug, Clone)]
pub struct TypstCli {
    pub pfad: PathBuf,
    pub version: String,
}

impl TypstCli {
    /// Sucht die CLI: `TFTN_TYPST`, dann neben dem laufenden Programm (Tauri-Sidecar),
    /// dann auf dem `PATH`. Prüft, dass sie genau die verlangte Version hat.
    pub fn finden(erwartet: &str) -> Result<Self, ExportFehler> {
        let mut kandidaten = Vec::new();
        if let Some(p) = std::env::var_os("TFTN_TYPST") {
            kandidaten.push(PathBuf::from(p));
        } else {
            if let Some(ordner) = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
            {
                kandidaten.push(ordner.join(format!("typst{}", std::env::consts::EXE_SUFFIX)));
            }
            kandidaten.push(PathBuf::from("typst"));
        }
        for pfad in kandidaten {
            if let Some(cli) = Self::pruefen(&pfad, erwartet)? {
                return Ok(cli);
            }
        }
        Err(ExportFehler::CliNichtGefunden)
    }

    /// Prüft eine bestimmte CLI. `Ok(None)`, wenn dort keine lauffähige typst-CLI liegt.
    pub fn pruefen(pfad: &Path, erwartet: &str) -> Result<Option<Self>, ExportFehler> {
        let Some(version) = version_von(pfad) else {
            return Ok(None);
        };
        if version != erwartet {
            return Err(ExportFehler::FalscheVersion {
                pfad: pfad.to_path_buf(),
                gefunden: version,
                erwartet: erwartet.to_string(),
            });
        }
        Ok(Some(Self {
            pfad: pfad.to_path_buf(),
            version,
        }))
    }
}

/// Liest die Version aus `typst --version` („typst 0.15.1 (…)“).
fn version_von(pfad: &Path) -> Option<String> {
    let aus = Command::new(pfad)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !aus.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&aus.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Prüft das Projekt mit der netzlosen World: keine fehlenden Pakete, keine Fehler.
pub fn vorpruefung(projekt: &Projekt) -> Result<Vec<Meldung>, ExportFehler> {
    let world = ProjektWorld::neu(projekt).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let ergebnis = kompilieren(&world);
    let fehlende = world.fehlende_pakete();
    if !fehlende.is_empty() {
        return Err(ExportFehler::FehlendePakete(
            fehlende.iter().map(|p| p.to_string()).collect(),
        ));
    }
    if ergebnis.dokument.is_none() {
        return Err(ExportFehler::Kompilieren(ergebnis.meldungen));
    }
    Ok(ergebnis.meldungen)
}

/// Führt die Gegenprobe-Zeile mit der CLI aus. `ziel` ist relativ zum Projekt oder absolut.
/// Liefert die Ausgabe der CLI auf stderr (Warnungen).
pub fn ausfuehren(
    projekt: &Projekt,
    cli: &TypstCli,
    ziel: &Path,
    zusatz: &[String],
    frist: Duration,
) -> Result<String, ExportFehler> {
    let ziel_text = ziel.to_str().ok_or_else(|| {
        ExportFehler::Sonstiges(tr!(
            "Zielpfad ist kein gültiger Text: {pfad}",
            pfad = ziel.display()
        ))
    })?;
    let mut befehl = Command::new(&cli.pfad);
    befehl
        .args(gegenprobe_argumente(projekt, ziel_text, zusatz))
        .current_dir(&projekt.wurzel)
        .env("SOURCE_DATE_EPOCH", projekt.epoche().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // Keine TYPST_*-Variablen der Umgebung durchreichen: Die Zeile allein bestimmt das Ergebnis.
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("TYPST_") {
            befehl.env_remove(name);
        }
    }
    let mut kind = befehl
        .spawn()
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    // stderr in einem eigenen Faden lesen, damit ein voller Puffer die CLI nicht blockiert.
    let mut stderr = kind.stderr.take().expect("stderr ist umgeleitet");
    let leser = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
        text
    });
    let start = Instant::now();
    let status = loop {
        match kind
            .try_wait()
            .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?
        {
            Some(status) => break status,
            None if start.elapsed() > frist => {
                let _ = kind.kill();
                let _ = kind.wait();
                return Err(ExportFehler::Frist(frist));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let ausgabe = leser.join().unwrap_or_default();
    if !status.success() {
        return Err(ExportFehler::Cli {
            status: status.to_string(),
            ausgabe,
        });
    }
    Ok(ausgabe)
}

/// Export: erst Vorprüfung, dann CLI. Gibt Warnungen der Vorprüfung zurück.
pub fn exportieren(
    projekt: &Projekt,
    cli: &TypstCli,
    ziel: &Path,
    zusatz: &[String],
) -> Result<Vec<Meldung>, ExportFehler> {
    let warnungen = vorpruefung(projekt)?;
    ausfuehren(projekt, cli, ziel, zusatz, STANDARD_FRIST)?;
    Ok(warnungen)
}
