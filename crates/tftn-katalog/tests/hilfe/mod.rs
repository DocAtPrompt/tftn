//! Gemeinsames der Diagramm- und Abnahmetests: neues Fachartikel-Projekt mit den Paketen des Korpus,
//! Bausteine anhängen wie das Menü, Übersetzen.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren};
use tftn_edit::WidgetParameter;
use tftn_katalog::{Katalog, Quellen, import_fuer};
use tftn_manifest::anlegen::projekt_anlegen;
use typst_syntax::Source;

pub const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let e = e.unwrap();
        let ziel = nach.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            kopieren(&e.path(), &ziel);
        } else {
            std::fs::copy(e.path(), ziel).unwrap();
        }
    }
}

/// Neues Fachartikel-Projekt mit den Paketen des Korpus; Katalog mit den Beschreibungen aus dem Repo.
pub fn fall(name: &str) -> (Projekt, Katalog, Vec<String>) {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("diagramme-{name}"));
    let _ = std::fs::remove_dir_all(&ordner);
    let projekt = projekt_anlegen(
        &repo().join("types"),
        "tftn-fachartikel",
        &ordner,
        Some(ZEITSTEMPEL),
    )
    .unwrap()
    .projekt;
    kopieren(
        &repo().join("tests/korpus/diagramme/packages/preview"),
        &projekt.pakete().join("preview"),
    );
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[repo().join("package-descriptions")],
    });
    (projekt, k, w)
}

/// Hängt den Baustein samt nötigem Import an `text`, wie es das Menü täte.
pub fn anhaengen(text: &mut String, k: &Katalog, id: &str) {
    let e = k.finden(id).unwrap_or_else(|| panic!("{id} fehlt"));
    if let Some(f) = import_fuer(&Source::detached(text.clone()), e) {
        text.insert_str(f.byte, &f.text);
    }
    text.push_str("\n\n");
    text.push_str(&e.schnipsel.einsetzen("").text);
    text.push('\n');
}

pub fn uebersetzt(p: &Projekt, text: &str) {
    let mut world = ProjektWorld::neu(p).unwrap();
    world.quelle_setzen(Source::new(world.haupt(), text.to_string()));
    let e = kompilieren(&world);
    assert!(
        e.meldungen.iter().all(|m| !m.fehler),
        "{:?}\n{text}",
        e.meldungen
    );
    assert!(e.dokument.is_some());
}

pub fn parameter(k: &Katalog) -> Vec<WidgetParameter> {
    k.widgets().into_iter().map(|w| w.parameter).collect()
}

pub fn hauptdatei(p: &Projekt) -> String {
    std::fs::read_to_string(p.wurzel.join(&p.konfig.haupt)).unwrap()
}
