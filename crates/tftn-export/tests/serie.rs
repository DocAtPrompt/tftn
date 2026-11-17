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
    let ergebnis = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen::default(),
        |f, g, n| meldungen.push(format!("{f}/{g} {n}")),
    )
    .unwrap();
    assert!(ergebnis.gesamt.is_none(), "ohne Sammel-PDF verlangt");
    let stuecke = ergebnis.stuecke;
    assert_eq!(stuecke[0].seiten, 1);
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

    // Beides, beidseitig: Einzelne bei den Daten (außerhalb des Projekts, also kopiert) und
    // ein Sammel-PDF mit je einer Leerseite nach den einseitigen Rechnungen.
    for f in ["2026-101 Huber.pdf", "2026-102 Gruber.pdf"] {
        std::fs::remove_file(daten.join(f)).unwrap();
    }
    let beides = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen {
            ausgabe: serie::Ausgabe::Beides,
            beidseitig: true,
        },
        |_, _, _| {},
    )
    .unwrap();
    assert!(daten.join("2026-101 Huber.pdf").exists());
    let gesamt = beides.gesamt.expect("Sammel-PDF verlangt");
    let pdf = gesamt
        .pdf
        .as_ref()
        .unwrap_or_else(|| panic!("{:?}", gesamt.fehler));
    assert_eq!(*pdf, daten.join("november-alle.pdf"));
    assert_eq!(gesamt.seiten, 4);
    assert_eq!(seiten(pdf), 4, "zwei Rechnungen, je mit Leerseite");
    assert!(
        gesamt.gegenprobe.contains("_lokal/serie/alle.typ"),
        "{}",
        gesamt.gegenprobe
    );

    // Nur gesamt, einseitig: keine Einzelnen bei den Daten.
    std::fs::remove_file(daten.join("2026-101 Huber.pdf")).unwrap();
    let nur = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen {
            ausgabe: serie::Ausgabe::Gesamt,
            beidseitig: false,
        },
        |_, _, _| {},
    )
    .unwrap();
    assert!(!daten.join("2026-101 Huber.pdf").exists());
    let g = nur.gesamt.unwrap();
    assert_eq!(seiten(g.pdf.as_ref().unwrap()), 2);
    let _ = std::fs::remove_dir_all(&wurzel);
}

/// Seiten eines von typst geschriebenen PDFs (Seitenobjekte zählen).
fn seiten(pdf: &Path) -> usize {
    let d = std::fs::read(pdf).unwrap();
    let muster = b"/Type /Page";
    let muster2 = b"/Type/Page";
    let mut n = 0;
    for m in [&muster[..], &muster2[..]] {
        n += d
            .windows(m.len() + 1)
            .filter(|w| &w[..m.len()] == m && w[m.len()] != b's')
            .count();
    }
    n
}
