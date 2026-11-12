//! Visuelle Regression (PLAN-M1 4.3): jedes Korpus-Dokument mit `tftn build --png --ppi 144`
//! rendern und pixelgenau mit `tests/referenz/<dokument>/` vergleichen. Abweichungen kommen als
//! Diff-Bild (rot markiert) nach `target/visual-diff/`; übernommen wird nur ausdrücklich.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use tiny_skia::Pixmap;

use crate::{mit_frist, projektordner, tftn_bauen};

const PPI: &str = "144";
/// Ausgabe innerhalb des Korpus-Projekts (`_renders/` wird nicht gesichert).
const AUSGABE: &str = "_renders/referenz";

/// Rendert, vergleicht und übernimmt die genannten Pfade (`<dokument>` oder
/// `<dokument>/<datei>`). Fehler, wenn danach noch etwas abweicht.
pub fn referenzen(uebernehmen: &[String]) -> Result<(), String> {
    let wurzel = projektordner();
    let tftn = tftn_bauen()?;
    let diff_ordner = wurzel.join("target/visual-diff");
    let _ = std::fs::remove_dir_all(&diff_ordner);

    let mut fehler = 0;
    for dokument in korpus(&wurzel)? {
        let name = dokument.file_name().unwrap().to_string_lossy().into_owned();
        let ausgabe = dokument.join(AUSGABE);
        let _ = std::fs::remove_dir_all(&ausgabe);
        mit_frist(
            Command::new(&tftn)
                .arg("build")
                .arg(&dokument)
                .args(["--png", "--ppi", PPI, "-o"])
                .arg(format!("{AUSGABE}/seite-{{0p}}.png")),
            120,
        )?;
        let referenz = wurzel.join("tests/referenz").join(&name);
        for p in uebernehmen {
            if let Some(rest) = p.strip_prefix(&name) {
                uebernehme(&ausgabe, &referenz, rest.trim_start_matches(['/', '\\']))?;
            }
        }
        fehler += vergleichen(&name, &ausgabe, &referenz, &diff_ordner.join(&name))?;
    }
    if fehler == 0 {
        println!("Visuelle Regression: alles pixelgleich.");
        Ok(())
    } else {
        Err(format!(
            "Visuelle Regression: {fehler} Abweichung(en). Diff-Bilder unter target/visual-diff/.\n\
             Nach Sichtprüfung übernehmen: cargo xtask referenzen --uebernehmen <dokument>[/<datei>]"
        ))
    }
}

/// Korpus-Dokumente: Unterordner von `tests/korpus/` mit `tftn.toml`.
pub(crate) fn korpus(wurzel: &Path) -> Result<Vec<PathBuf>, String> {
    let mut liste: Vec<PathBuf> = std::fs::read_dir(wurzel.join("tests/korpus"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("tftn.toml").is_file())
        .collect();
    liste.sort();
    Ok(liste)
}

fn pngs(ordner: &Path) -> BTreeSet<String> {
    std::fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".png"))
        .collect()
}

/// `datei` leer: das ganze Dokument (auch Referenzen entfernter Seiten löschen).
fn uebernehme(ausgabe: &Path, referenz: &Path, datei: &str) -> Result<(), String> {
    std::fs::create_dir_all(referenz).map_err(|e| e.to_string())?;
    let neu = pngs(ausgabe);
    let auswahl: Vec<String> = if datei.is_empty() {
        for alt in pngs(referenz).difference(&neu) {
            std::fs::remove_file(referenz.join(alt)).map_err(|e| e.to_string())?;
            println!("entfernt: {}", referenz.join(alt).display());
        }
        neu.into_iter().collect()
    } else if neu.contains(datei) {
        vec![datei.to_string()]
    } else if pngs(referenz).contains(datei) {
        // Seite gibt es nicht mehr: Referenz entfernen.
        std::fs::remove_file(referenz.join(datei)).map_err(|e| e.to_string())?;
        println!("entfernt: {}", referenz.join(datei).display());
        vec![]
    } else {
        return Err(format!(
            "{datei}: weder gerendert noch als Referenz vorhanden"
        ));
    };
    for d in auswahl {
        std::fs::copy(ausgabe.join(&d), referenz.join(&d)).map_err(|e| e.to_string())?;
        println!("übernommen: {}", referenz.join(&d).display());
    }
    Ok(())
}

/// Zählt die Abweichungen eines Dokuments und schreibt Diff-Bilder.
fn vergleichen(name: &str, ausgabe: &Path, referenz: &Path, diff: &Path) -> Result<usize, String> {
    let neu = pngs(ausgabe);
    let alt = pngs(referenz);
    let mut fehler = 0;
    for d in alt.difference(&neu) {
        println!("{name}/{d}: Seite fehlt (Referenz vorhanden, nicht mehr gerendert)");
        fehler += 1;
    }
    for d in neu.difference(&alt) {
        println!("{name}/{d}: keine Referenz");
        fehler += 1;
    }
    for d in neu.intersection(&alt) {
        let a = laden(&referenz.join(d))?;
        let b = laden(&ausgabe.join(d))?;
        if (a.width(), a.height()) != (b.width(), b.height()) {
            println!(
                "{name}/{d}: Größe {}×{} statt {}×{}",
                b.width(),
                b.height(),
                a.width(),
                a.height()
            );
            fehler += 1;
            continue;
        }
        let (anzahl, bild) = diff_bild(&a, &b);
        if anzahl == 0 {
            println!("{name}/{d}: gleich");
        } else {
            std::fs::create_dir_all(diff).map_err(|e| e.to_string())?;
            let ziel = diff.join(d);
            bild.save_png(&ziel).map_err(|e| e.to_string())?;
            println!(
                "{name}/{d}: {anzahl} Pixel verschieden → {}",
                ziel.display()
            );
            fehler += 1;
        }
    }
    Ok(fehler)
}

fn laden(pfad: &Path) -> Result<Pixmap, String> {
    Pixmap::load_png(pfad).map_err(|e| format!("{}: {e}", pfad.display()))
}

/// Referenz blass, abweichende Pixel rot. Gibt die Zahl der abweichenden Pixel zurück.
fn diff_bild(alt: &Pixmap, neu: &Pixmap) -> (usize, Pixmap) {
    let mut bild = alt.clone();
    let mut anzahl = 0;
    let paare = alt.data().chunks_exact(4).zip(neu.data().chunks_exact(4));
    for (ziel, (a, b)) in bild.data_mut().chunks_exact_mut(4).zip(paare) {
        if a != b {
            anzahl += 1;
            ziel.copy_from_slice(&[255, 0, 0, 255]);
        } else {
            let grau = (a[0] as u32 * 3 + a[1] as u32 * 6 + a[2] as u32) / 10;
            let blass = (255 - (255 - grau) / 4) as u8;
            ziel.copy_from_slice(&[blass, blass, blass, 255]);
        }
    }
    (anzahl, bild)
}
