//! `cargo xtask typstyle [--check] [PFAD…]`: Typst-Quellen mit typstyle-core formatieren
//! (PLAN-M1 4.2), ohne Pfade die Vorlagen in `types/`. Nur als Werkzeug, nie beim Speichern.
//! Einstellungen wie im Test `crates/tftn-edit/tests/typstyle.rs` (dort geprüft: S1–S3).

use std::path::{Path, PathBuf};

use typstyle_core::{Config, LineEnding, Typstyle};

use crate::projektordner;

pub fn typstyle(args: &[String]) -> Result<(), String> {
    let pruefen = args.iter().any(|a| a == "--check");
    let mut pfade: Vec<PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .collect();
    if pfade.is_empty() {
        pfade.push(projektordner().join("types"));
    }
    let konfig = Config {
        max_width: 100,
        tab_spaces: 2,
        line_ending: LineEnding::Lf,
        ..Config::default()
    };
    let mut abweichend = Vec::new();
    for datei in pfade.iter().flat_map(|p| typ_dateien(p)) {
        let text =
            std::fs::read_to_string(&datei).map_err(|e| format!("{}: {e}", datei.display()))?;
        let neu = Typstyle::new(konfig.clone())
            .format_text(text.as_str())
            .render()
            .map_err(|e| format!("{}: {e}", datei.display()))?;
        if neu == text {
            continue;
        }
        if !pruefen {
            std::fs::write(&datei, &neu).map_err(|e| e.to_string())?;
            println!("formatiert: {}", datei.display());
        }
        abweichend.push(datei);
    }
    if pruefen && !abweichend.is_empty() {
        for d in &abweichend {
            println!("nicht formatiert: {}", d.display());
        }
        return Err("cargo xtask typstyle formatiert sie".into());
    }
    Ok(())
}

/// `.typ`-Dateien in `pfad` (Datei oder Ordner), ohne Ordner mit `_` am Anfang.
fn typ_dateien(pfad: &Path) -> Vec<PathBuf> {
    if pfad.is_file() {
        return vec![pfad.to_path_buf()];
    }
    let mut liste = Vec::new();
    for e in std::fs::read_dir(pfad).into_iter().flatten().flatten() {
        let p = e.path();
        if p.file_name().unwrap().to_string_lossy().starts_with('_') {
            continue;
        }
        if p.is_dir() {
            liste.extend(typ_dateien(&p));
        } else if p.extension().is_some_and(|e| e == "typ") {
            liste.push(p);
        }
    }
    liste.sort();
    liste
}
