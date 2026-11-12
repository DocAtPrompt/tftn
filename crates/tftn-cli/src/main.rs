//! `tftn` – Kommandozeile (KONZEPT 2.11 Punkt 2, PLAN-M1 AP1).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tftn_core::{Projekt, ProjektWorld, kompilieren};
use tftn_export::{TypstCli, exportieren, gegenprobe_zeile};

const HILFE: &str = "\
tftn – Typst for the Noobs

Aufruf:
  tftn neu --typ TYP ORDNER [--typen PFAD]       Projekt aus einem Dokumenttyp anlegen
  tftn build [ORDNER] [-o ZIEL] [-- SCHALTER…]   PDF über die typst-CLI erzeugen
  tftn build [ORDNER] --png [--ppi N] [-o ZIEL]  Seitenbilder statt PDF (Standard 144 ppi)
  tftn export [ORDNER] --profil ID [--abnehmen]  Export-Profil des Dokumenttyps (ohne ID: Liste)
  tftn gegenprobe [ORDNER] [-o ZIEL]             die gleichwertige typst-Zeile ausgeben
  tftn check [ORDNER]                            mit der Vorschau-World prüfen
  tftn katalog [ORDNER] [--beschreibungen PFAD]… Bausteine des Projekts auflisten
  tftn typ-pruefen [ORDNER] [--beschreibungen PFAD]…
                                                 Bausteine aller Quellen prüfen (Funktion,
                                                 Parameter, Schnipsel übersetzt)

TYP ist ein Paket aus typen/, z. B. tftn-fachartikel (Ordner aus --typen, TFTN_TYPEN
oder dem TFTN-Quellordner).
--abnehmen setzt den Zeitstempel in tftn.toml auf jetzt und legt das Ergebnis mit
Gegenprobe-Notiz unter export/ ab; sonst landet es unter _renders/.
ORDNER ist der Projektordner mit tftn.toml (Standard: aktueller Ordner).
ZIEL ist relativ zum Projektordner (Standard: _renders/<hauptdatei>.pdf, mit --png
_renders/<hauptdatei>-{0p}.png; {0p} wird zur Seitennummer).
SCHALTER nach -- gehen unverändert an typst compile, z. B. -- --pdf-standard a-2b.
Die typst-CLI kommt aus TFTN_TYPST, neben tftn oder vom PATH und muss die Version
aus tftn.toml haben.";

struct Aufruf {
    befehl: String,
    ordner: PathBuf,
    ziel: Option<String>,
    zusatz: Vec<String>,
    typ: Option<String>,
    typen: Option<PathBuf>,
    profil: Option<String>,
    abnehmen: bool,
    /// `--png`: Seitenbilder mit dieser Auflösung (PLAN-M1 4.3).
    png: Option<f32>,
    /// `--beschreibungen`: Ordner mit Paketbeschreibungen, der erste hat Vorrang.
    beschreibungen: Vec<PathBuf>,
}

fn lesen(args: Vec<String>) -> Result<Aufruf, String> {
    let mut it = args.into_iter();
    let befehl = it.next().ok_or_else(|| HILFE.to_string())?;
    let mut ordner = None;
    let mut ziel = None;
    let mut zusatz = Vec::new();
    let mut typ = None;
    let mut typen = None;
    let mut profil = None;
    let mut abnehmen = false;
    let mut png = false;
    let mut ppi = None;
    let mut beschreibungen = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" => ziel = Some(it.next().ok_or("-o braucht ein Ziel")?),
            "--typ" => typ = Some(it.next().ok_or("--typ braucht einen Namen")?),
            "--profil" => profil = Some(it.next().ok_or("--profil braucht eine ID")?),
            "--abnehmen" => abnehmen = true,
            "--png" => png = true,
            "--ppi" => {
                let wert = it.next().ok_or("--ppi braucht eine Zahl")?;
                ppi = Some(
                    wert.parse::<f32>()
                        .ok()
                        .filter(|p| *p > 0.0)
                        .ok_or(format!("--ppi: keine gültige Zahl: {wert}"))?,
                );
            }
            "--typen" => {
                typen = Some(PathBuf::from(
                    it.next().ok_or("--typen braucht einen Pfad")?,
                ))
            }
            "--beschreibungen" => beschreibungen.push(PathBuf::from(
                it.next().ok_or("--beschreibungen braucht einen Pfad")?,
            )),
            "--" => zusatz.extend(it.by_ref()),
            "-h" | "--help" => return Err(HILFE.to_string()),
            _ if ordner.is_none() && !a.starts_with('-') => ordner = Some(PathBuf::from(a)),
            _ => return Err(format!("Unbekanntes Argument: {a}\n\n{HILFE}")),
        }
    }
    if ppi.is_some() && !png {
        return Err("--ppi gilt nur zusammen mit --png".into());
    }
    Ok(Aufruf {
        befehl,
        ordner: ordner.unwrap_or_else(|| ".".into()),
        ziel,
        zusatz,
        typ,
        typen,
        profil,
        abnehmen,
        png: png.then(|| ppi.unwrap_or(144.0)),
        beschreibungen,
    })
}

fn standard_ziel(projekt: &Projekt, png: bool) -> String {
    let stamm = Path::new(&projekt.konfig.haupt)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dokument".into());
    if png {
        format!("_renders/{stamm}-{{0p}}.png")
    } else {
        format!("_renders/{stamm}.pdf")
    }
}

fn main() -> ExitCode {
    let aufruf = match lesen(std::env::args().skip(1).collect()) {
        Ok(a) => a,
        Err(text) => {
            eprintln!("{text}");
            return ExitCode::from(2);
        }
    };
    match ausfuehren(aufruf) {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("{text}");
            ExitCode::FAILURE
        }
    }
}

/// Ordner der Dokumenttypen: `--typen`, `TFTN_TYPEN` oder `typen/` im TFTN-Quellordner.
fn typen_ordner(aufruf: &Aufruf) -> PathBuf {
    aufruf
        .typen
        .clone()
        .or_else(|| std::env::var_os("TFTN_TYPEN").map(PathBuf::from))
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typen"))
}

fn ausfuehren(aufruf: Aufruf) -> Result<(), String> {
    if aufruf.befehl == "neu" {
        let typ = aufruf.typ.as_deref().ok_or("neu braucht --typ")?;
        let anlage = tftn_manifest::anlegen::projekt_anlegen(
            &typen_ordner(&aufruf),
            typ,
            &aufruf.ordner,
            None,
        )
        .map_err(|e| e.to_string())?;
        println!("angelegt: {}", anlage.projekt.wurzel.display());
        for p in anlage.pakete {
            println!("  vendort: {p}");
        }
        return Ok(());
    }
    let projekt = Projekt::oeffnen(&aufruf.ordner).map_err(|e| e.to_string())?;
    let ziel = aufruf
        .ziel
        .clone()
        .unwrap_or_else(|| standard_ziel(&projekt, aufruf.png.is_some()));
    match aufruf.befehl.as_str() {
        "build" => {
            let cli = TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
            let ziel_pfad = projekt.wurzel.join(&ziel);
            if let Some(eltern) = ziel_pfad.parent() {
                std::fs::create_dir_all(eltern).map_err(|e| e.to_string())?;
            }
            let mut zusatz = aufruf.zusatz.clone();
            if let Some(ppi) = aufruf.png {
                zusatz.extend([
                    "--format".into(),
                    "png".into(),
                    "--ppi".into(),
                    ppi.to_string(),
                ]);
            }
            let warnungen = exportieren(&projekt, &cli, Path::new(&ziel), &zusatz)
                .map_err(|e| e.to_string())?;
            for w in warnungen {
                eprintln!("{w}");
            }
            println!("{}", ziel_pfad.display());
            Ok(())
        }
        "export" => {
            let profile = tftn_export::profil::profile(&projekt).map_err(|e| e.to_string())?;
            let Some(id) = aufruf.profil.as_deref() else {
                for p in profile {
                    let std = p.manifest.pdf_standards.join(", ");
                    println!("{:<14} {} {}", p.id, p.manifest.art, std);
                }
                return Ok(());
            };
            let cli = TypstCli::finden(&projekt.konfig.typst).map_err(|e| e.to_string())?;
            let e = tftn_export::profil::profil_exportieren(&projekt, &cli, id, aufruf.abnehmen)
                .map_err(|e| e.to_string())?;
            for w in e.warnungen {
                eprintln!("{w}");
            }
            println!("{}", e.pfad.display());
            println!("Gegenprobe: {}", e.gegenprobe);
            if let Some(h) = e.sha256 {
                println!("SHA-256: {h}");
            }
            Ok(())
        }
        "gegenprobe" => {
            println!("cd {}", projekt.wurzel.display());
            if let Some(eltern) = Path::new(&ziel)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
            {
                println!("mkdir -p {}", eltern.display());
            }
            println!("{}", gegenprobe_zeile(&projekt, &ziel, &aufruf.zusatz));
            Ok(())
        }
        "check" => {
            let world = ProjektWorld::neu(&projekt).map_err(|e| e.to_string())?;
            let ergebnis = kompilieren(&world);
            for m in &ergebnis.meldungen {
                eprintln!("{m}");
            }
            let fehlende = world.fehlende_pakete();
            if !fehlende.is_empty() {
                let liste: Vec<String> = fehlende.iter().map(|p| p.to_string()).collect();
                return Err(format!(
                    "Fehlende Pakete in packages/: {}",
                    liste.join(", ")
                ));
            }
            match ergebnis.dokument {
                Some(d) => {
                    println!("in Ordnung: {} Seiten", d.pages().len());
                    Ok(())
                }
                None => Err("Das Dokument hat Fehler.".into()),
            }
        }
        "katalog" => {
            let (katalog, warnungen) = tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
                projekt: Some(&projekt),
                beschreibungen: &aufruf.beschreibungen,
            });
            for e in katalog.eintraege() {
                let herkunft = match e.herkunft {
                    tftn_katalog::Herkunft::Typst => "typst",
                    tftn_katalog::Herkunft::Dokumenttyp { .. } => "dokumenttyp",
                    tftn_katalog::Herkunft::Paket { .. } => "paket",
                };
                println!("{}\t{}\t{herkunft}", e.id, e.titel);
            }
            for w in warnungen {
                eprintln!("Warnung: {w}");
            }
            Ok(())
        }
        "typ-pruefen" => {
            let (katalog, warnungen) = tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
                projekt: Some(&projekt),
                beschreibungen: &aufruf.beschreibungen,
            });
            let befunde = tftn_katalog::pruefen(&projekt, &katalog)?;
            for w in &warnungen {
                eprintln!("Warnung: {w}");
            }
            for b in &befunde {
                eprintln!("{}: {}", b.id, b.grund);
            }
            if warnungen.is_empty() && befunde.is_empty() {
                println!("in Ordnung: {} Bausteine", katalog.eintraege().len());
                Ok(())
            } else {
                Err(format!(
                    "{} Befunde, {} Warnungen",
                    befunde.len(),
                    warnungen.len()
                ))
            }
        }
        anders => Err(format!("Unbekannter Befehl: {anders}\n\n{HILFE}")),
    }
}
