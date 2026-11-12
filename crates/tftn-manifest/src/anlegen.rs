//! `tftn neu`: ein Projekt aus einem Dokumenttyp anlegen (KONZEPT 2.5, 5.7, 5.8; PLAN-M1 AP2).
//!
//! - Vorlage (`[template]`) in den Projektordner kopieren,
//! - Schriften aus `[tool.tftn] schriften` nach `fonts/`,
//! - den Typ und alle `@local`-Pakete, die er importiert, nach `packages/local/<name>/<version>/`
//!   vendoren (ohne Vorlage und Schriften, die liegen schon im Projekt),
//! - `tftn.toml`, `README.md` mit Gegenprobe-Zeile, `.gitignore`, `.editorconfig` schreiben.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tftn_sprachen::tr;

use tftn_core::projekt::iso_utc;
use tftn_core::{Projekt, gegenprobe::gegenprobe_zeile};

use crate::typen::{Herkunft, Typquelle, finden_in};
use crate::{ManifestFehler, Typpaket};

/// Typst-Version dieses TFTN-Releases (KONZEPT E2).
pub const TYPST_VERSION: &str = "0.15.1";

/// Was angelegt wurde.
#[derive(Debug)]
pub struct Anlage {
    pub projekt: Projekt,
    /// Vendorte Pakete als `@local/<name>:<version>`, in Reihenfolge des Kopierens.
    pub pakete: Vec<String>,
}

pub(crate) fn io(pfad: &Path) -> impl Fn(std::io::Error) -> ManifestFehler + '_ {
    move |e| ManifestFehler::Lesen(pfad.to_path_buf(), e)
}

/// Legt im leeren oder neuen Ordner `ziel` ein Projekt vom Typ `typ` an.
/// `typ` ist ein Paketname (`tftn-fachartikel`) oder `@local/<name>:<version>`.
/// `zeitstempel` in Unix-Sekunden; ohne Angabe die aktuelle Zeit.
pub fn projekt_anlegen(
    typen: &Path,
    typ: &str,
    ziel: &Path,
    zeitstempel: Option<i64>,
) -> Result<Anlage, ManifestFehler> {
    projekt_anlegen_sprache(typen, typ, ziel, zeitstempel, None)
}

/// Wie `projekt_anlegen`, mit der Vorlage für `sprache` (`en`, `en-GB` …), wenn der Typ eine hat.
pub fn projekt_anlegen_sprache(
    typen: &Path,
    typ: &str,
    ziel: &Path,
    zeitstempel: Option<i64>,
    sprache: Option<&str>,
) -> Result<Anlage, ManifestFehler> {
    let quelle = Typquelle {
        ordner: typen.to_path_buf(),
        herkunft: Herkunft::Tftn,
    };
    projekt_anlegen_aus_sprache(&[quelle], typ, ziel, zeitstempel, sprache)
}

/// Wie `projekt_anlegen`, mit mehreren Quellen in Vorrang-Reihenfolge (PLAN-M2 AP8). Scheitert das
/// Anlegen, wird aufgeräumt: Einen Ordner, den TFTN angelegt hat, entfernt es wieder; einen schon
/// vorhandenen (leeren) Ordner leert es.
pub fn projekt_anlegen_aus(
    quellen: &[Typquelle],
    typ: &str,
    ziel: &Path,
    zeitstempel: Option<i64>,
) -> Result<Anlage, ManifestFehler> {
    projekt_anlegen_aus_sprache(quellen, typ, ziel, zeitstempel, None)
}

/// Ordner der Vorlage für `sprache`: erst die ganze Kennung (`en-GB`), dann die Sprache allein
/// (`en`); ohne Treffer der aus `[template]`.
pub fn vorlage_fuer<'a>(
    manifest: &'a crate::Manifest,
    standard: &'a str,
    sprache: Option<&str>,
) -> &'a str {
    let Some(s) = sprache.map(str::trim).filter(|s| !s.is_empty()) else {
        return standard;
    };
    let kurz = s.split(['-', '_']).next().unwrap_or(s);
    manifest
        .vorlagen
        .get(s)
        .or_else(|| manifest.vorlagen.get(kurz))
        .map(String::as_str)
        .unwrap_or(standard)
}

/// `projekt_anlegen_aus` mit der Vorlage für `sprache`.
pub fn projekt_anlegen_aus_sprache(
    quellen: &[Typquelle],
    typ: &str,
    ziel: &Path,
    zeitstempel: Option<i64>,
    sprache: Option<&str>,
) -> Result<Anlage, ManifestFehler> {
    let (name, version) = spec_lesen(typ);
    let paket = finden_in(quellen, name, version)?;
    if ziel.exists() && std::fs::read_dir(ziel).map_err(io(ziel))?.next().is_some() {
        return Err(ManifestFehler::Inhalt(tr!(
            "{ordner} ist nicht leer",
            ordner = ziel.display()
        )));
    }
    let war_da = ziel.exists();
    let ergebnis = anlegen_innen(quellen, &paket, ziel, zeitstempel, sprache);
    if ergebnis.is_err() {
        if war_da {
            if let Ok(eintraege) = std::fs::read_dir(ziel) {
                for e in eintraege.filter_map(Result::ok) {
                    let p = e.path();
                    let _ = if p.is_dir() {
                        std::fs::remove_dir_all(&p)
                    } else {
                        std::fs::remove_file(&p)
                    };
                }
            }
        } else {
            let _ = std::fs::remove_dir_all(ziel);
        }
    }
    ergebnis
}

fn anlegen_innen(
    quellen: &[Typquelle],
    paket: &Typpaket,
    ziel: &Path,
    zeitstempel: Option<i64>,
    sprache: Option<&str>,
) -> Result<Anlage, ManifestFehler> {
    let manifest = paket.manifest()?;
    let vorlage = paket.toml.template.as_ref().ok_or_else(|| {
        ManifestFehler::Inhalt(tr!(
            "{paket} hat keine Vorlage ([template])",
            paket = paket.spec()
        ))
    })?;

    std::fs::create_dir_all(ziel).map_err(io(ziel))?;

    // Vorlage
    let vorlage_ordner = paket
        .ordner
        .join(vorlage_fuer(manifest, &vorlage.path, sprache));
    kopieren(&vorlage_ordner, ziel, &BTreeSet::new())?;

    schriften_kopieren(&paket.ordner, &manifest.schriften, ziel)?;

    // Pakete vendoren: der Typ selbst und alles, was er oder die Vorlage per @local importiert.
    let erledigt = vendoren(ziel, paket, quellen, importe(&vorlage_ordner)?)?;

    // Projektdateien
    let zeit = zeitstempel.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    });
    schreiben(
        &ziel.join("tftn.toml"),
        &format!(
            "tftn = 1\ntypst = \"{TYPST_VERSION}\"\ntyp = \"{}\"\nhaupt = \"{}\"\nzeitstempel = {}\n",
            paket.spec(),
            vorlage.entrypoint,
            iso_utc(zeit)
        ),
    )?;
    schreiben(&ziel.join(".gitignore"), "_renders/\n_lokal/\n.DS_Store\n")?;
    schreiben(
        &ziel.join(".editorconfig"),
        "root = true\n\n[*]\ncharset = utf-8\nend_of_line = lf\ninsert_final_newline = true\n",
    )?;
    let projekt = Projekt::oeffnen(ziel).map_err(|e| ManifestFehler::Inhalt(e.to_string()))?;
    let stamm = Path::new(&vorlage.entrypoint)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dokument".into());
    let zeile = gegenprobe_zeile(&projekt, &format!("_renders/{stamm}.pdf"), &[]);
    let englisch = sprache.is_some_and(|s| s.trim().to_lowercase().starts_with("en"));
    let readme = if englisch {
        format!(
            "# {titel}\n\nCreated with TFTN as “{typname}” ({spec}).\n\n\
             ## Build without TFTN (cross-check)\n\n\
             With typst {TYPST_VERSION}, this line in the project folder produces the same PDF as TFTN:\n\n\
             ```bash\nmkdir -p _renders\n{zeile}\n```\n\n\
             ## Origin\n\n\
             - `packages/local/`: document type and its dependencies, copied from TFTN `typen/`.\n\
             - `fonts/`: freely licensed fonts, licenses in `fonts/LIZENZEN.txt`.\n",
            titel = tftn_sprachen::uebersetzen(&manifest.titel),
            typname = tftn_sprachen::uebersetzen(&manifest.titel),
            spec = paket.spec(),
        )
    } else {
        format!(
            "# {titel}\n\nAngelegt mit TFTN als „{typname}“ ({spec}).\n\n\
             ## Ohne TFTN bauen (Gegenprobe)\n\n\
             Mit typst {TYPST_VERSION} ergibt diese Zeile im Projektordner dasselbe PDF wie TFTN:\n\n\
             ```bash\nmkdir -p _renders\n{zeile}\n```\n\n\
             ## Herkunft\n\n\
             - `packages/local/`: Dokumenttyp und seine Abhängigkeiten, kopiert aus TFTN `typen/`.\n\
             - `fonts/`: frei lizenzierte Schriften, Lizenzen in `fonts/LIZENZEN.txt`.\n",
            titel = manifest.titel,
            typname = manifest.titel,
            spec = paket.spec(),
        )
    };
    schreiben(&ziel.join("README.md"), &readme)?;

    Ok(Anlage {
        projekt,
        pakete: erledigt,
    })
}

/// Vendort `paket` und alles, was es (oder `zusatz`) per `@local` importiert, nach
/// `<ziel>/packages/local/<name>/<version>/` – ohne Vorlage und Schriften. Gibt die Specs zurück.
pub(crate) fn vendoren(
    ziel: &Path,
    paket: &Typpaket,
    quellen: &[Typquelle],
    zusatz: BTreeSet<(String, String)>,
) -> Result<Vec<String>, ManifestFehler> {
    let mut offen = vec![(
        paket.toml.package.name.clone(),
        Some(paket.toml.package.version.clone()),
    )];
    offen.extend(zusatz.into_iter().map(|(n, v)| (n, Some(v))));
    let mut erledigt = Vec::new();
    while let Some((name, version)) = offen.pop() {
        let p = finden_in(quellen, &name, version.as_deref())?;
        if erledigt.contains(&p.spec()) {
            continue;
        }
        let mut aus = BTreeSet::new();
        if let Some(v) = &p.toml.template {
            aus.insert(PathBuf::from(&v.path));
        }
        if let Some(m) = &p.toml.tool.tftn {
            aus.extend(m.schriften.iter().map(PathBuf::from));
            aus.extend(m.vorlagen.values().map(PathBuf::from));
        }
        let ablage = ziel
            .join("packages/local")
            .join(&p.toml.package.name)
            .join(&p.toml.package.version);
        kopieren(&p.ordner, &ablage, &aus)?;
        offen.extend(importe(&ablage)?.into_iter().map(|(n, v)| (n, Some(v))));
        erledigt.push(p.spec());
    }
    Ok(erledigt)
}

/// `@local/name:1.2.3` → (`name`, Some(`1.2.3`)); `name` → (`name`, None).
pub(crate) fn spec_lesen(typ: &str) -> (&str, Option<&str>) {
    let rest = typ.strip_prefix("@local/").unwrap_or(typ);
    match rest.split_once(':') {
        Some((n, v)) => (n, Some(v)),
        None => (rest, None),
    }
}

/// Findet `"@local/<name>:<version>"` in allen `.typ`-Dateien unter `ordner`.
/// Universe-Pakete (`@preview/…`) werden in M1 noch nicht vendort und sind ein Fehler.
fn importe(ordner: &Path) -> Result<BTreeSet<(String, String)>, ManifestFehler> {
    let mut gefunden = BTreeSet::new();
    for datei in dateien(ordner)? {
        if datei.extension().is_none_or(|e| e != "typ") {
            continue;
        }
        let text = std::fs::read_to_string(&datei).map_err(io(&datei))?;
        if text.contains("\"@preview/") {
            return Err(ManifestFehler::Inhalt(tr!(
                "{datei}: Universe-Pakete (@preview) werden in M1 noch nicht vendort",
                datei = datei.display()
            )));
        }
        for teil in text.split("\"@local/").skip(1) {
            let Some(ende) = teil.find('"') else { continue };
            if let Some((n, v)) = teil[..ende].split_once(':') {
                gefunden.insert((n.to_string(), v.to_string()));
            }
        }
    }
    Ok(gefunden)
}

/// Alle Dateien unter `ordner`, sortiert.
pub(crate) fn dateien(ordner: &Path) -> Result<Vec<PathBuf>, ManifestFehler> {
    let mut alle = Vec::new();
    let mut stapel = vec![ordner.to_path_buf()];
    while let Some(o) = stapel.pop() {
        for eintrag in std::fs::read_dir(&o).map_err(io(&o))? {
            let pfad = eintrag.map_err(io(&o))?.path();
            if pfad.is_dir() {
                stapel.push(pfad);
            } else {
                alle.push(pfad);
            }
        }
    }
    alle.sort();
    Ok(alle)
}

/// Kopiert `von` nach `nach`; Pfade in `aus` (relativ zu `von`) und Systemdateien bleiben weg.
pub(crate) fn kopieren(
    von: &Path,
    nach: &Path,
    aus: &BTreeSet<PathBuf>,
) -> Result<(), ManifestFehler> {
    for datei in dateien(von)? {
        let rel = datei.strip_prefix(von).expect("liegt unter von");
        if aus.iter().any(|a| rel.starts_with(a))
            || rel.file_name().is_some_and(|n| n == ".DS_Store")
        {
            continue;
        }
        let ziel = nach.join(rel);
        if let Some(eltern) = ziel.parent() {
            std::fs::create_dir_all(eltern).map_err(io(eltern))?;
        }
        std::fs::copy(&datei, &ziel).map_err(io(&datei))?;
    }
    Ok(())
}

fn schreiben(pfad: &Path, text: &str) -> Result<(), ManifestFehler> {
    std::fs::write(pfad, text).map_err(io(pfad))
}

/// Kopiert die Schriften des Typs nach `<ziel>/fonts`.
fn schriften_kopieren(
    paket: &Path,
    schriften: &[String],
    ziel: &Path,
) -> Result<(), ManifestFehler> {
    if schriften.is_empty() {
        return Ok(());
    }
    let fonts = ziel.join("fonts");
    std::fs::create_dir_all(&fonts).map_err(io(&fonts))?;
    for s in schriften {
        let quelle = paket.join(s);
        let datei = Path::new(s)
            .file_name()
            .ok_or_else(|| ManifestFehler::Inhalt(tr!("schriften: {s} ist kein Dateipfad", s)))?;
        std::fs::copy(&quelle, fonts.join(datei)).map_err(io(&quelle))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_varianten() {
        assert_eq!(spec_lesen("tftn-fachartikel"), ("tftn-fachartikel", None));
        assert_eq!(
            spec_lesen("@local/tftn-kern:0.1.0"),
            ("tftn-kern", Some("0.1.0"))
        );
    }
}
