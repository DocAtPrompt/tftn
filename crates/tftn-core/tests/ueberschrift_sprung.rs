//! Quelle → Vorschau bei Überschriften: Der Text einer Überschrift steht auch im Inhaltsverzeichnis
//! (und in Kolumnentiteln). Die Vorschau soll zur Überschrift selbst springen, nicht dorthin.

use std::path::Path;

use tftn_core::Projekt;
use tftn_core::vorschau::Vorschau;
use typst::syntax::Source;

#[test]
fn ueberschrift_vor_inhaltsverzeichnis() {
    let projekt = Projekt::oeffnen(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel"),
    )
    .unwrap();
    let mut vorschau = Vorschau::neu(&projekt).unwrap();
    let original = std::fs::read_to_string(projekt.haupt()).unwrap();
    // Inhaltsverzeichnis auf einer eigenen ersten Seite, nach den Einstellungen am Anfang.
    let ende_kopf = original
        .find("\n= ")
        .expect("eine Überschrift erster Ebene")
        + 1;
    let text = format!(
        "{}#outline()\n#pagebreak()\n{}",
        &original[..ende_kopf],
        &original[ende_kopf..]
    );
    let quelle = Source::new(vorschau.world().haupt(), text.clone());
    assert!(vorschau.lauf(1, Some(quelle.clone())).gelungen);
    let verzeichnis = vorschau
        .stellen(&quelle, text.find("= Methode").unwrap() + 3)
        .iter()
        .map(|s| s.0)
        .collect::<Vec<_>>();
    let seite_verzeichnis = 0;
    assert!(
        verzeichnis.contains(&seite_verzeichnis),
        "Der Test setzt voraus, dass „Methode“ im Verzeichnis steht: {verzeichnis:?}"
    );
    for byte in [
        text.find("= Methode").unwrap(),     // vor dem Gleichheitszeichen
        text.find("= Methode").unwrap() + 4, // im Wort
    ] {
        let stellen = vorschau.stellen(&quelle, byte);
        assert!(
            stellen.first().is_some_and(|s| s.0 != seite_verzeichnis),
            "erste Stelle im Inhaltsverzeichnis: {stellen:?}"
        );
    }
}
