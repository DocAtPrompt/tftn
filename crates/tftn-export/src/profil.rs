//! Export-Profile aus dem Manifest (KONZEPT 5.6, PLAN-M1 AP8).
//!
//! Ein Profil wird zu Schaltern der typst-CLI. „Probe“ legt das Ergebnis unter `_renders/` ab
//! (nicht synchronisiert). „Abnehmen“ setzt zuerst den Zeitstempel in `tftn.toml` auf jetzt
//! (KONZEPT 5.7: nur hier und bei „Datum aktualisieren“), legt das Ergebnis unter `export/` ab und
//! schreibt daneben eine Gegenprobe-Notiz mit Befehlszeile und SHA-256.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tftn_sprachen::tr;

use sha2::{Digest, Sha256};
use tftn_core::Projekt;
use tftn_core::gegenprobe::gegenprobe_zeile;
use tftn_manifest::{ExportManifest, Typpaket};

use crate::{ExportFehler, Meldung, TypstCli, exportieren};

#[derive(Debug, Clone)]
pub struct ExportProfil {
    pub id: String,
    pub manifest: ExportManifest,
}

/// Die Export-Profile des Dokumenttyps eines Projekts, nach Namen sortiert.
pub fn profile(projekt: &Projekt) -> Result<Vec<ExportProfil>, ExportFehler> {
    let paket =
        Typpaket::vom_projekt(projekt).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let manifest = paket
        .manifest()
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    Ok(manifest
        .export
        .iter()
        .map(|(id, m)| ExportProfil {
            id: id.clone(),
            manifest: m.clone(),
        })
        .collect())
}

impl ExportProfil {
    /// Schalter für `typst compile` (zusätzlich zur Gegenprobe-Zeile).
    pub fn schalter(&self) -> Result<Vec<String>, ExportFehler> {
        let m = &self.manifest;
        let mut s = Vec::new();
        match m.art.as_str() {
            "pdf" => {
                if !m.pdf_standards.is_empty() {
                    s.push("--pdf-standard".into());
                    s.push(m.pdf_standards.join(","));
                }
                if m.getaggt == Some(false) {
                    s.push("--no-pdf-tags".into());
                }
            }
            "png" => s.extend([
                "--format".into(),
                "png".into(),
                "--ppi".into(),
                "144".into(),
            ]),
            anders => {
                return Err(ExportFehler::Sonstiges(tr!(
                    "Export-Art „{anders}“ (Profil {profil}) kommt nach M1",
                    anders,
                    profil = self.id
                )));
            }
        }
        if let Some(seiten) = &m.seiten {
            s.push("--pages".into());
            s.push(seiten.clone());
        }
        Ok(s)
    }

    /// Dateiname nach dem Muster `dateiname` (Platzhalter `{stamm}`, `{profil}`, `{datum}`),
    /// Standard `{stamm}-{profil}-{datum}`; `{datum}` ist das Datum des Zeitstempels.
    pub fn dateiname(&self, projekt: &Projekt) -> Result<String, ExportFehler> {
        let muster = self
            .manifest
            .dateiname
            .clone()
            .unwrap_or_else(|| "{stamm}-{profil}-{datum}".into());
        let stamm = Path::new(&projekt.konfig.haupt)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dokument".into());
        let datum = projekt
            .konfig
            .zeitstempel
            .date
            .map(|d| d.to_string())
            .unwrap_or_default();
        let mut name = muster
            .replace("{stamm}", &stamm)
            .replace("{profil}", &self.id)
            .replace("{datum}", &datum);
        if name.contains('{') {
            return Err(ExportFehler::Sonstiges(tr!(
                "dateiname: unbekannter Platzhalter in {muster}",
                muster
            )));
        }
        let endung = if self.manifest.art == "png" {
            "-{0p}.png"
        } else {
            ".pdf"
        };
        if !name.ends_with(".pdf") && !name.ends_with(".png") {
            name.push_str(endung);
        }
        Ok(name)
    }
}

/// Ergebnis eines Exports.
#[derive(Debug)]
pub struct Exportiert {
    /// Absoluter Pfad der Datei.
    pub pfad: PathBuf,
    /// Pfad relativ zum Projekt, wie in der Gegenprobe-Zeile.
    pub relativ: String,
    pub gegenprobe: String,
    pub sha256: Option<String>,
    pub warnungen: Vec<Meldung>,
}

/// Exportiert mit Profil `id`. `abnehmen`: Zeitstempel auf jetzt, Ablage in `export/`, Notiz.
pub fn profil_exportieren(
    projekt: &Projekt,
    cli: &TypstCli,
    id: &str,
    abnehmen: bool,
) -> Result<Exportiert, ExportFehler> {
    let projekt = if abnehmen {
        let jetzt = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        zeitstempel_setzen(projekt, jetzt)?
    } else {
        projekt.clone()
    };
    let profil = profile(&projekt)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| ExportFehler::Sonstiges(tr!("Export-Profil „{id}“ gibt es nicht", id)))?;
    let ordner = if abnehmen { "export" } else { "_renders" };
    let relativ = format!("{ordner}/{}", profil.dateiname(&projekt)?);
    let pfad = projekt.wurzel.join(&relativ);
    std::fs::create_dir_all(pfad.parent().expect("hat Ordner"))
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let schalter = profil.schalter()?;
    let warnungen = exportieren(&projekt, cli, Path::new(&relativ), &schalter)?;
    let gegenprobe = gegenprobe_zeile(&projekt, &relativ, &schalter);
    let sha256 = (profil.manifest.art == "pdf")
        .then(|| std::fs::read(&pfad).ok())
        .flatten()
        .map(|d| {
            Sha256::digest(&d)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        });
    if abnehmen {
        let notiz = pfad.with_extension("gegenprobe.txt");
        let text = format!(
            "Abgenommen mit TFTN, typst {} (Profil {id}).\n\
             Gegenprobe im Projektordner:\n\n{gegenprobe}\n\nSHA-256: {}\n",
            cli.version,
            sha256.as_deref().unwrap_or("–")
        );
        std::fs::write(&notiz, text).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    }
    Ok(Exportiert {
        pfad,
        relativ,
        gegenprobe,
        sha256,
        warnungen,
    })
}

/// Setzt `zeitstempel` in `tftn.toml` (nur diese Zeile, Kommentare bleiben) und liest das Projekt neu.
pub fn zeitstempel_setzen(projekt: &Projekt, sekunden: i64) -> Result<Projekt, ExportFehler> {
    let pfad = projekt.wurzel.join("tftn.toml");
    let text =
        std::fs::read_to_string(&pfad).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let neu_zeile = format!("zeitstempel = {}", tftn_core::projekt::iso_utc(sekunden));
    let mut gefunden = false;
    let neu: Vec<String> = text
        .lines()
        .map(|z| {
            if z.trim_start().starts_with("zeitstempel") && !gefunden {
                gefunden = true;
                neu_zeile.clone()
            } else {
                z.to_string()
            }
        })
        .collect();
    if !gefunden {
        return Err(ExportFehler::Sonstiges(tr!("tftn.toml ohne zeitstempel")));
    }
    std::fs::write(&pfad, neu.join("\n") + "\n")
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    Projekt::oeffnen(&projekt.wurzel).map_err(|e| ExportFehler::Sonstiges(e.to_string()))
}
