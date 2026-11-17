//! Serie über die typst-CLI: Rechnung aus der Vorlage, drei Datensätze, einer fehlerhaft.
//! Braucht die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

use std::path::Path;

use tftn_export::TypstCli;
use tftn_export::serie::{self, Hauptdatei};

#[test]
fn rechnungen_als_serie() {
    let wurzel = std::env::temp_dir().join(format!("tftn-serie-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    let projekt = tftn_manifest::anlegen::projekt_anlegen(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types"),
        "tftn-rechnung",
        &wurzel.join("projekt"),
        Some(1_793_404_800),
    )
    .unwrap()
    .projekt;
    let haupt = std::fs::read_to_string(projekt.haupt()).unwrap();
    assert_eq!(
        serie::hauptdatei_pruefen(&haupt, &serie::datendatei(&projekt)),
        Hauptdatei::Bereit,
        "neue Rechnungen sind serienfähig"
    );
    let daten = wurzel.join("daten");
    std::fs::create_dir_all(&daten).unwrap();
    let datei = daten.join("november.toml");
    std::fs::write(
        &datei,
        r#"
[[serie]]
datei = "2026-101 Huber"
nummer = "2026-101"
empfaenger = ["Frau", "Lena Huber", "Gasse 2", "4020 Linz"]
anrede = "Sehr geehrte Frau Huber,"
posten = [{ text = "Beratung", menge = 2, einheit = "Std.", preis = 100 }]

[[serie]]
datei = "2026-102 Gruber"
nummer = "2026-102"
empfaenger = ["Herr", "Paul Gruber", "Platz 3", "5020 Salzburg"]
anrede = "Sehr geehrter Herr Gruber,"

[[serie]]
datei = "kaputt"
nummer = "2026-103"
datum = "kein Datum"
"#,
    )
    .unwrap();
    let quelle = serie::lesen(&datei).unwrap();
    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let mut meldungen = Vec::new();
    let stuecke = serie::erzeugen(&projekt, &cli, &quelle, |f, g, n| {
        meldungen.push(format!("{f}/{g} {n}"))
    })
    .unwrap();
    assert_eq!(
        meldungen,
        ["1/3 2026-101 Huber", "2/3 2026-102 Gruber", "3/3 kaputt"]
    );
    for s in &stuecke[..2] {
        let pdf = s
            .pdf
            .as_ref()
            .unwrap_or_else(|| panic!("{}: {:?}", s.name, s.fehler));
        assert!(
            std::fs::read(pdf).unwrap().starts_with(b"%PDF"),
            "{}",
            pdf.display()
        );
        assert!(
            s.gegenprobe.contains("--input 'daten=_lokal/serie/"),
            "{}",
            s.gegenprobe
        );
    }
    assert!(
        stuecke[2].pdf.is_none() && stuecke[2].fehler.is_some(),
        "{:?}",
        stuecke[2]
    );
    assert!(!daten.join("kaputt.pdf").exists());
    // Datensatz liegt fürs Nachbauen im Projekt, mit den Projektdaten zusammengeführt.
    let satz =
        std::fs::read_to_string(projekt.wurzel.join("_lokal/serie/2026-102 Gruber.toml")).unwrap();
    assert!(
        satz.contains("Paul Gruber") && satz.contains("[bank]"),
        "{satz}"
    );
    // Gruber hat keine eigenen Posten: die aus dem Projekt
    assert!(satz.contains("Fahrtkosten"), "{satz}");
    let _ = std::fs::remove_dir_all(&wurzel);
}
