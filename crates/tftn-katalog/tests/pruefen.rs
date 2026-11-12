//! `tftn typ-pruefen` (PLAN-M2 AP10, Task 3): Bausteine aller Quellen im leeren Dokument prüfen.

use std::path::{Path, PathBuf};

use tftn_core::Projekt;
use tftn_katalog::{Katalog, Quellen, pruefen};
use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

const LIB: &str = "#let kasten(farbe: none, body) = rect(fill: farbe, body)\n";

/// Projekt mit dem Testpaket `demo` und einer Beschreibung mit genau diesem Baustein.
fn fall(name: &str, baustein: &str) -> (Projekt, Katalog) {
    let basis = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pruefen-{name}"));
    let _ = std::fs::remove_dir_all(&basis);
    let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typen");
    let projekt = projekt_anlegen(
        &typen,
        "tftn-fachartikel",
        &basis.join("p"),
        Some(ZEITSTEMPEL),
    )
    .unwrap()
    .projekt;
    let paket = projekt.pakete().join("preview/demo/0.1.3");
    std::fs::create_dir_all(&paket).unwrap();
    std::fs::write(
        paket.join("typst.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.3\"\nentrypoint = \"lib.typ\"\n",
    )
    .unwrap();
    std::fs::write(paket.join("lib.typ"), LIB).unwrap();
    let ordner: PathBuf = basis.join("b");
    std::fs::create_dir_all(&ordner).unwrap();
    std::fs::write(
        ordner.join("demo.toml"),
        format!(
            "paket = \"demo\"\nversionen = \"0.1\"\nimport = '#import \"@preview/demo:{{version}}\"'\n\n[[baustein]]\nid = \"b\"\ntitel = \"B\"\n{baustein}"
        ),
    )
    .unwrap();
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[ordner],
    });
    assert!(w.is_empty(), "{w:?}");
    (projekt, k)
}

fn ein_befund(name: &str, baustein: &str, erwartet: &str) {
    let (p, k) = fall(name, baustein);
    let b = pruefen(&p, &k).unwrap();
    assert_eq!(b.len(), 1, "{b:?}");
    assert_eq!(b[0].id, "@preview/demo/b");
    assert!(b[0].grund.contains(erwartet), "{b:?}");
}

#[test]
fn eingebaute_und_typ_bestehen() {
    let (p, k) = fall(
        "gut",
        "funktion = \"demo.kasten\"\neinfuegen = \"#demo.kasten[x]\"\n",
    );
    assert_eq!(pruefen(&p, &k).unwrap(), vec![]);
}

#[test]
fn parameter_vorhanden_besteht() {
    let (p, k) = fall(
        "param-gut",
        "funktion = \"demo.kasten\"\neinfuegen = \"#demo.kasten[x]\"\n[baustein.parameter.farbe]\nwidget = \"farbe\"\n",
    );
    assert_eq!(pruefen(&p, &k).unwrap(), vec![]);
}

#[test]
fn falsche_funktion() {
    ein_befund(
        "funktion",
        "funktion = \"demo.nichtda\"\neinfuegen = \"#demo.nichtda[x]\"\n",
        "nichtda",
    );
}

#[test]
fn falscher_parameter() {
    ein_befund(
        "parameter",
        "funktion = \"demo.kasten\"\neinfuegen = \"#demo.kasten[x]\"\n[baustein.parameter.nichtda]\nwidget = \"text\"\n",
        "unexpected argument: nichtda",
    );
}

#[test]
fn schnipsel_kaputt() {
    ein_befund(
        "kaputt",
        "funktion = \"demo.kasten\"\neinfuegen = \"#demo.kasten(\"\n",
        "unclosed",
    );
}

#[test]
fn fehlende_datei_kein_befund() {
    let (p, k) = fall(
        "datei",
        "funktion = \"image\"\neinfuegen = '#image(\"gibtsnicht.png\")'\n",
    );
    assert_eq!(pruefen(&p, &k).unwrap(), vec![]);
}

#[test]
fn aufruf_fehlt() {
    ein_befund(
        "aufruf",
        "funktion = \"demo.kasten\"\neinfuegen = \"Nur Text\"\n",
        "demo.kasten",
    );
}

#[test]
fn modus_mathe_und_code_werden_eingewickelt() {
    let (p, k) = fall(
        "modi",
        "funktion = \"demo.kasten\"\nmodus = [\"code\"]\neinfuegen = \"demo.kasten[x]\"\n",
    );
    assert_eq!(pruefen(&p, &k).unwrap(), vec![]);
}
