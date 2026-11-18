//! Mehrere offene Dateien (Reiter im Editor): Die Vorschau setzt das ganze Dokument mit allen
//! ungespeicherten Kapiteln; was nicht mehr offen ist, kommt wieder von der Platte.

use std::path::Path;

use tftn_core::Projekt;
use tftn_core::vorschau::Vorschau;
use typst::syntax::Source;

fn projekt(name: &str) -> Projekt {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(o.join("kapitel")).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(
        o.join("main.typ"),
        "Anfang\n#include \"kapitel/eins.typ\"\n",
    )
    .unwrap();
    std::fs::write(o.join("kapitel/eins.typ"), "Eine Seite.\n").unwrap();
    Projekt::oeffnen(&o).unwrap()
}

#[test]
fn ungespeichertes_kapitel_kommt_in_die_vorschau() {
    let p = projekt("offene-dateien");
    let mut v = Vorschau::neu(&p).unwrap();
    let haupt = Source::new(
        v.world().haupt(),
        std::fs::read_to_string(p.haupt()).unwrap(),
    );
    let kapitel = v
        .world()
        .datei_id(&p.wurzel.join("kapitel/eins.typ"))
        .expect("Kapitel liegt im Projekt");
    assert_eq!(kapitel.vpath().get_without_slash(), "kapitel/eins.typ");

    // Auf der Platte: eine Seite.
    let lauf = v.lauf_offen(1, haupt.clone(), Vec::new());
    assert!(lauf.gelungen, "{:?}", lauf.meldungen);
    assert_eq!(lauf.seiten.len(), 1);

    // Das Kapitel ist offen und ungespeichert, mit einem Seitenumbruch: zwei Seiten. Aktiv ist
    // das Kapitel, die Hauptdatei liegt daneben.
    let neu = Source::new(kapitel, "Erste.\n#pagebreak()\nZweite.\n".into());
    let lauf = v.lauf_offen(7, neu, vec![haupt.clone()]);
    assert!(lauf.gelungen, "{:?}", lauf.meldungen);
    assert_eq!(lauf.seiten.len(), 2);
    assert_eq!(v.quelle_datei(), Some(kapitel));
    assert_eq!(v.quelle_rev(), 7);

    // Ohne Speichern geschlossen: wieder die Fassung von der Platte.
    let lauf = v.lauf_offen(8, haupt, Vec::new());
    assert_eq!(lauf.seiten.len(), 1);
    assert_eq!(v.quelle_datei(), Some(v.world().haupt()));
}

#[test]
fn keine_kennung_ausserhalb_des_projekts() {
    let p = projekt("offene-dateien-aussen");
    let v = Vorschau::neu(&p).unwrap();
    assert!(v.world().datei_id(&std::env::temp_dir()).is_none());
    assert!(
        v.world()
            .datei_id(&p.wurzel.join("gibt-es-nicht.typ"))
            .is_none()
    );
}
