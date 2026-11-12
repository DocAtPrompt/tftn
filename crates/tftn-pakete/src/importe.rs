//! Universe-Pakete, die ein Paket importiert (Festlegung 4 des Plans AP9).
//!
//! Erkannt wird über den Syntaxbaum: nur `import`/`include` mit einer String-Quelle, in jeder Tiefe.
//! Ein String in einem Kommentar oder als gewöhnliches Argument zählt nicht.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use typst_syntax::{LinkedNode, Source, ast};

use crate::{PaketFehler, PaketSpec};

/// Alle `@preview`-Angaben in `#import`/`#include` der `.typ`-Dateien unter `ordner`; Warnungen für
/// `@local/…` und unlesbare Angaben.
pub fn importe(ordner: &Path) -> Result<(BTreeSet<PaketSpec>, Vec<String>), PaketFehler> {
    let mut gefunden = BTreeSet::new();
    let mut warnungen = Vec::new();
    for datei in typ_dateien(ordner)? {
        let text =
            std::fs::read_to_string(&datei).map_err(|e| PaketFehler::Io(datei.clone(), e))?;
        let quelle = Source::detached(text);
        let mut quellen = Vec::new();
        sammeln(&LinkedNode::new(quelle.root()), &mut quellen);
        let relativ = datei.strip_prefix(ordner).unwrap_or(&datei).display();
        for q in quellen {
            if q.starts_with("@preview/") {
                match PaketSpec::lesen(&q) {
                    Ok(s) => {
                        gefunden.insert(s);
                    }
                    Err(e) => warnungen.push(tr!(
                        "{datei}: {paket} wird nicht geholt: {fehler}",
                        datei = relativ,
                        paket = q,
                        fehler = e
                    )),
                }
            } else if q.starts_with("@local/") {
                warnungen.push(tr!(
                    "{datei}: {paket} liegt nicht im Universe und wird nicht geholt",
                    datei = relativ,
                    paket = q
                ));
            }
        }
    }
    Ok((gefunden, warnungen))
}

/// Die `.typ`-Dateien des Projekts selbst: ohne `packages/` und ohne Ordner, die mit `_` oder `.`
/// beginnen.
pub(crate) fn projekt_dateien(wurzel: &Path) -> Vec<PathBuf> {
    let mut aus = Vec::new();
    let Ok(eintraege) = std::fs::read_dir(wurzel) else {
        return aus;
    };
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        let pfad = e.path();
        if pfad.is_dir() {
            if name != "packages" && !name.starts_with(['_', '.']) {
                aus.extend(typ_dateien(&pfad).unwrap_or_default());
            }
        } else if pfad.extension().is_some_and(|x| x == "typ") {
            aus.push(pfad);
        }
    }
    aus.sort();
    aus
}

/// Was die Datei importiert oder einbindet; Unlesbares zählt nicht.
pub(crate) fn importe_der_datei(datei: &Path) -> Vec<PaketSpec> {
    let Ok(text) = std::fs::read_to_string(datei) else {
        return Vec::new();
    };
    let quelle = Source::detached(text);
    let mut quellen = Vec::new();
    sammeln(&LinkedNode::new(quelle.root()), &mut quellen);
    quellen
        .iter()
        .filter_map(|q| PaketSpec::lesen(q).ok())
        .collect()
}

/// Was die `.typ`-Dateien des Projekts selbst importieren.
pub fn projektimporte(wurzel: &Path) -> BTreeSet<PaketSpec> {
    projekt_dateien(wurzel)
        .iter()
        .flat_map(|d| importe_der_datei(d))
        .collect()
}

fn sammeln(knoten: &LinkedNode, aus: &mut Vec<String>) {
    let quelle = if let Some(i) = knoten.get().cast::<ast::ModuleImport>() {
        Some(i.source())
    } else {
        knoten
            .get()
            .cast::<ast::ModuleInclude>()
            .map(|i| i.source())
    };
    if let Some(ast::Expr::Str(s)) = quelle {
        aus.push(s.get().to_string());
    }
    for kind in knoten.children() {
        sammeln(&kind, aus);
    }
}

/// Alle `.typ`-Dateien unter `ordner`, sortiert; Verknüpfungen werden nicht verfolgt.
fn typ_dateien(ordner: &Path) -> Result<Vec<PathBuf>, PaketFehler> {
    let mut aus = Vec::new();
    let mut stapel = vec![ordner.to_path_buf()];
    while let Some(o) = stapel.pop() {
        for e in std::fs::read_dir(&o).map_err(|e| PaketFehler::Io(o.clone(), e))? {
            let e = e.map_err(|e| PaketFehler::Io(o.clone(), e))?;
            let art = e.file_type().map_err(|f| PaketFehler::Io(e.path(), f))?;
            let pfad = e.path();
            if art.is_dir() {
                stapel.push(pfad);
            } else if art.is_file() && pfad.extension().is_some_and(|x| x == "typ") {
                aus.push(pfad);
            }
        }
    }
    aus.sort();
    Ok(aus)
}
