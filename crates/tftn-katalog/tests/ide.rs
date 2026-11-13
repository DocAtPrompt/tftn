//! Vervollständigung über `typst_ide::autocomplete` (PLAN-M2 AP2, Task 4).

use std::path::Path;

use tftn_core::vorschau::Vorschau;
use tftn_katalog::ide::vervollstaendigen;
use tftn_manifest::anlegen::projekt_anlegen;
use typst::World;
use typst_syntax::Source;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn vorschau(name: &str) -> (Vorschau, String) {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types");
    let projekt = projekt_anlegen(&typen, "tftn-fachartikel", &ordner, Some(ZEITSTEMPEL))
        .unwrap()
        .projekt;
    let haupt = std::fs::read_to_string(projekt.haupt()).unwrap();
    (Vorschau::neu(&projekt).unwrap(), haupt)
}

#[test]
fn funktion_mit_feld() {
    let (v, haupt) = vorschau("ide-funktion");
    let text = format!("{haupt}\n#fig");
    let quelle = Source::new(v.world().main(), text.clone());
    let (von, vorschlaege) = vervollstaendigen(v.world(), None, &quelle, text.len()).unwrap();
    assert_eq!(von, text.len() - 3);
    let figure = vorschlaege.iter().find(|v| v.titel == "figure").unwrap();
    assert_eq!(figure.eingesetzt.text, "figure(\n  \n)");
    assert_eq!(figure.eingesetzt.felder.len(), 1);
    assert_eq!(figure.art, "function");
}

#[test]
fn label_aus_dem_dokument() {
    let (mut v, _) = vorschau("ide-label");
    let text = "= Einleitung <einl>\nSiehe @";
    let quelle = Source::new(v.world().main(), text.into());
    assert!(v.lauf(1, Some(quelle.clone())).gelungen);
    let dokument = &v.gut().unwrap().dokument;
    let (_, vorschlaege) =
        vervollstaendigen(v.world(), Some(dokument), &quelle, text.len()).unwrap();
    assert!(
        vorschlaege.iter().any(|v| v.titel == "einl"),
        "{:?}",
        vorschlaege.iter().map(|v| &v.titel).collect::<Vec<_>>()
    );
}
