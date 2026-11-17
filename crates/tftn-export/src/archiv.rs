//! Archivpaket: ein Zip unter `export/`, mit dem sich das Dokument noch in Jahren ohne TFTN genau
//! so setzen lässt. Darin: der Projektordner (ohne `_lokal`, `_renders`, `export` und versteckte
//! Dateien), alle benutzten Schriften in `fonts/`, die Pakete in `packages/`, das PDF/A und eine
//! Liesmich-Datei mit der typst-Zeile für macOS/Linux und Windows samt SHA-256.
//!
//! Das PDF entsteht in einer Kopie des Projekts, nur aus dem, was ins Zip kommt (`--font-path
//! fonts`, ohne die Schriften der App). Was fehlt, fällt also hier auf und nicht erst in zehn Jahren.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use tftn_core::gegenprobe::{gegenprobe_argumente_eigen, gegenprobe_zeile_fuer};
use tftn_core::{Projekt, ProjektWorld, kompilieren, schriften};
use tftn_sprachen::tr;

use crate::{ExportFehler, STANDARD_FRIST, TypstCli, ausfuehren_argumente, vorpruefung};

/// Ordner, die nie ins Archiv kommen (in jeder Tiefe): nicht gesichert bzw. erzeugt.
const AUSSER: [&str; 2] = ["_lokal", "_renders"];
/// Ordner, die auf oberster Ebene nicht ins Archiv kommen: frühere Exporte und Archive.
const AUSSER_OBEN: [&str; 1] = ["export"];
/// PDF-Standard, wenn der Dokumenttyp kein Profil `archiv` hat.
const STANDARD: &str = "a-2b";

/// Ergebnis von `archiv_erzeugen`.
#[derive(Debug)]
pub struct Archiviert {
    /// Absoluter Pfad des Zips.
    pub pfad: PathBuf,
    /// Pfad relativ zum Projekt (`export/…zip`).
    pub relativ: String,
    /// Name des PDFs im Archiv.
    pub pdf: String,
    /// PDF-Standard(s), etwa `a-2b`.
    pub standard: String,
    pub sha256: String,
    /// Anzahl der Dateien im Zip.
    pub dateien: usize,
    /// Byte
    pub groesse: u64,
    /// Warnungen der Vorprüfung und der typst-CLI.
    pub warnungen: Vec<String>,
}

/// Räumt den Arbeitsordner auch bei einem Fehler weg.
struct Arbeitsordner(PathBuf);

impl Drop for Arbeitsordner {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fehler(e: impl std::fmt::Display) -> ExportFehler {
    ExportFehler::Sonstiges(e.to_string())
}

/// Ein Dateiname aus dem Projektordner: Buchstaben, Ziffern, `-`, `_`, `.`; sonst `-`.
fn sicherer_name(roh: &str) -> String {
    let name: String = roh
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = name.trim_matches(|c| c == '.' || c == '-').to_string();
    if name.is_empty() {
        "dokument".into()
    } else {
        name
    }
}

/// Kopiert `von` nach `nach` ohne versteckte Dateien, Verknüpfungen und die ausgenommenen Ordner.
fn kopieren(von: &Path, nach: &Path, oben: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(nach)?;
    for eintrag in std::fs::read_dir(von)? {
        let eintrag = eintrag?;
        let name = eintrag.file_name();
        let name_text = name.to_string_lossy();
        if name_text.starts_with('.')
            || AUSSER.contains(&name_text.as_ref())
            || (oben && AUSSER_OBEN.contains(&name_text.as_ref()))
        {
            continue;
        }
        let art = eintrag.file_type()?;
        if art.is_dir() {
            kopieren(&eintrag.path(), &nach.join(&name), false)?;
        } else if art.is_file() {
            std::fs::copy(eintrag.path(), nach.join(&name))?;
        }
    }
    Ok(())
}

/// Alle Dateien unter `ordner`, relativ und mit `/`, sortiert.
fn dateien(ordner: &Path) -> std::io::Result<Vec<String>> {
    fn sammeln(basis: &Path, ordner: &Path, aus: &mut Vec<String>) -> std::io::Result<()> {
        for eintrag in std::fs::read_dir(ordner)? {
            let pfad = eintrag?.path();
            if pfad.is_dir() {
                sammeln(basis, &pfad, aus)?;
            } else {
                let relativ = pfad.strip_prefix(basis).expect("unter basis");
                aus.push(
                    relativ
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
        Ok(())
    }
    let mut aus = Vec::new();
    sammeln(ordner, ordner, &mut aus)?;
    aus.sort();
    Ok(aus)
}

/// Die Pakete in `packages/` als `@namensraum/name:version`.
fn pakete(ordner: &Path) -> Vec<String> {
    let lesen = |p: &Path| -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(p)
            .map(|e| {
                e.filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    };
    let mut aus = Vec::new();
    for raum in lesen(ordner) {
        for name in lesen(&ordner.join(&raum)) {
            for version in lesen(&ordner.join(&raum).join(&name)) {
                aus.push(format!("@{raum}/{name}:{version}"));
            }
        }
    }
    aus
}

/// Der PDF-Standard aus dem Profil `archiv` des Dokumenttyps, sonst PDF/A-2b.
fn standard(projekt: &Projekt) -> String {
    crate::profil::profile(projekt)
        .ok()
        .and_then(|liste| liste.into_iter().find(|p| p.id == "archiv"))
        .filter(|p| p.manifest.art == "pdf" && !p.manifest.pdf_standards.is_empty())
        .map(|p| p.manifest.pdf_standards.join(","))
        .unwrap_or_else(|| STANDARD.into())
}

struct Liesmich<'a> {
    titel: &'a str,
    jetzt: &'a str,
    tftn: &'a str,
    typst: &'a str,
    pdf: &'a str,
    standard: &'a str,
    sha256: &'a str,
    unix: &'a str,
    windows: &'a str,
    schriften: usize,
    pakete: &'a [String],
}

fn liesmich(l: &Liesmich) -> String {
    let mut t = String::new();
    t.push_str(&tr!(
        "{titel} – Archiv eines Typst-Dokuments",
        titel = l.titel
    ));
    t.push_str("\n\n");
    t.push_str(&tr!(
        "Dieser Ordner enthält alles, um {pdf} ohne TFTN neu zu setzen: den Typst-Quelltext mit Daten, Bildern und Literatur, alle benutzten Schriften (fonts/) und Typst-Pakete (packages/). Nichts davon wird aus dem Netz geladen.",
        pdf = l.pdf
    ));
    t.push_str("\n\n");
    t.push_str(&tr!(
        "Erzeugt: {jetzt} mit TFTN {tftn} und typst {typst}",
        jetzt = l.jetzt,
        tftn = l.tftn,
        typst = l.typst
    ));
    t.push('\n');
    t.push_str(&tr!(
        "PDF: {pdf} ({standard}), SHA-256 {sha256}",
        pdf = l.pdf,
        standard = l.standard,
        sha256 = l.sha256
    ));
    t.push_str("\n\n");
    t.push_str(&tr!("So entsteht das PDF neu:"));
    t.push_str("\n\n");
    t.push_str(&tr!(
        "1. typst {typst} besorgen, genau diese Version: https://github.com/typst/typst/releases/tag/v{typst}",
        typst = l.typst
    ));
    t.push('\n');
    t.push_str(&tr!(
        "2. In diesem Ordner eine Eingabeaufforderung öffnen und ausführen:"
    ));
    t.push_str("\n\n   macOS, Linux:\n   ");
    t.push_str(l.unix);
    t.push_str("\n\n   Windows (PowerShell):\n   ");
    t.push_str(l.windows);
    t.push_str("\n\n");
    t.push_str(&tr!(
        "Mit derselben typst-Version entsteht dasselbe PDF, Byte für Byte: Die SHA-256 oben weist es nach (macOS, Linux: shasum -a 256 {pdf}; Windows: Get-FileHash {pdf}).",
        pdf = l.pdf
    ));
    t.push_str("\n\n");
    t.push_str(&tr!(
        "Schriften: {anzahl} Dateien in fonts/, Lizenzen in fonts/LIZENZEN*.txt.",
        anzahl = l.schriften
    ));
    t.push('\n');
    if l.pakete.is_empty() {
        t.push_str(&tr!("Pakete: keine."));
    } else {
        t.push_str(&tr!(
            "Pakete in packages/: {liste}",
            liste = l.pakete.join(", ")
        ));
    }
    t.push('\n');
    t
}

/// Baut das Archivpaket nach `export/<ordner>-archiv-<datum>.zip` (ein gleichnamiges wird ersetzt).
/// `tftn` ist die Version von TFTN für die Liesmich-Datei.
pub fn archiv_erzeugen(
    projekt: &Projekt,
    cli: &TypstCli,
    tftn: &str,
) -> Result<Archiviert, ExportFehler> {
    let mut warnungen: Vec<String> = vorpruefung(projekt)?
        .iter()
        .map(|m| m.to_string())
        .collect();
    let jetzt = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let jetzt_text = tftn_core::projekt::iso_utc(jetzt);
    let titel = sicherer_name(
        &projekt
            .wurzel
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let paket = tr!(
        "{name}-archiv-{datum}",
        name = titel,
        datum = &jetzt_text[..10]
    );

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let arbeit = Arbeitsordner(
        std::env::temp_dir().join(format!("tftn-archiv-{}-{nanos}", std::process::id())),
    );
    let innen = arbeit.0.join(&paket);
    kopieren(&projekt.wurzel, &innen, true).map_err(fehler)?;

    // Was das Dokument an Schriften benutzt, kommt aus dem Vorrat der App nach fonts/.
    let kopie = Projekt::oeffnen(&innen).map_err(fehler)?;
    let world = ProjektWorld::neu(&kopie).map_err(fehler)?;
    let k = kompilieren(&world);
    let doc = k.dokument.ok_or(ExportFehler::Kompilieren(k.meldungen))?;
    std::fs::create_dir_all(kopie.schriften()).map_err(fehler)?;
    schriften::einbinden(&kopie, &doc).map_err(fehler)?;
    drop(world);

    // Das PDF nur aus der Kopie, nur mit ihren Schriften.
    let kopie = Projekt::oeffnen(&innen).map_err(fehler)?;
    let standard = standard(projekt);
    let pdf = format!("{titel}.pdf");
    let zusatz = vec!["--pdf-standard".to_string(), standard.clone()];
    let argumente = gegenprobe_argumente_eigen(&kopie, &pdf, &zusatz);
    let ausgabe = ausfuehren_argumente(&kopie, cli, &argumente, STANDARD_FRIST).map_err(|e| {
        ExportFehler::Sonstiges(tr!(
            "Das PDF für das Archiv ({standard}) ließ sich nicht setzen: {fehler}",
            standard = standard,
            fehler = e
        ))
    })?;
    warnungen.extend(
        ausgabe
            .lines()
            .map(str::trim)
            .filter(|z| !z.is_empty())
            .map(String::from),
    );
    let sha256: String = Sha256::digest(std::fs::read(innen.join(&pdf)).map_err(fehler)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let schrift_dateien = std::fs::read_dir(kopie.schriften())
        .map(|e| {
            e.filter_map(Result::ok)
                .filter(|e| {
                    e.path().extension().is_some_and(|x| {
                        matches!(
                            x.to_string_lossy().to_lowercase().as_str(),
                            "otf" | "ttf" | "otc" | "ttc"
                        )
                    })
                })
                .count()
        })
        .unwrap_or(0);
    let liste = pakete(&kopie.pakete());
    let liesmich_name = tr!("LIESMICH.txt");
    let text = liesmich(&Liesmich {
        titel: &titel,
        jetzt: &jetzt_text,
        tftn,
        typst: &cli.version,
        pdf: &pdf,
        standard: &standard,
        sha256: &sha256,
        unix: &gegenprobe_zeile_fuer(&kopie, &pdf, &zusatz, false),
        windows: &gegenprobe_zeile_fuer(&kopie, &pdf, &zusatz, true),
        schriften: schrift_dateien,
        pakete: &liste,
    });
    std::fs::write(innen.join(&liesmich_name), text).map_err(fehler)?;

    // Liesmich und PDF zuerst, dann alles andere in fester Reihenfolge.
    let mut namen = vec![liesmich_name.clone(), pdf.clone()];
    namen.extend(
        dateien(&innen)
            .map_err(fehler)?
            .into_iter()
            .filter(|n| *n != liesmich_name && *n != pdf),
    );
    let eintraege: Vec<(String, PathBuf)> = namen
        .iter()
        .map(|n| (format!("{paket}/{n}"), innen.join(n)))
        .collect();
    let relativ = format!("export/{paket}.zip");
    let pfad = projekt.wurzel.join(&relativ);
    std::fs::create_dir_all(pfad.parent().expect("hat Ordner")).map_err(fehler)?;
    let vorlaeufig = pfad.with_extension("zip.teil");
    let groesse =
        crate::zip::schreiben(&vorlaeufig, &eintraege, projekt.epoche()).map_err(|e| {
            let _ = std::fs::remove_file(&vorlaeufig);
            fehler(e)
        })?;
    std::fs::rename(&vorlaeufig, &pfad).map_err(fehler)?;
    Ok(Archiviert {
        pfad,
        relativ,
        pdf,
        standard,
        sha256,
        dateien: eintraege.len(),
        groesse,
        warnungen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sichere_namen() {
        assert_eq!(sicherer_name("Befund Huber"), "Befund-Huber");
        assert_eq!(sicherer_name("Grüße"), "Grüße");
        assert_eq!(sicherer_name("..."), "dokument");
    }
}
