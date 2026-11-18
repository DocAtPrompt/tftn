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

/// Eine Rechnung mit Logo als Serie: Der Pfad des Logos kommt aus `dokument.toml` des Projekts,
/// die Datensätze liegen unter `_lokal/serie/`; aufgelöst wird er in `main.typ` (`pfad`), also
/// relativ zum Projekt, in der Vorprüfung wie in der CLI.
#[test]
fn rechnung_mit_logo_als_serie() {
    let wurzel = std::env::temp_dir().join(format!("tftn-serie-logo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    let projekt = tftn_manifest::anlegen::projekt_anlegen(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types"),
        "tftn-rechnung",
        &wurzel.join("projekt"),
        Some(1_793_404_800),
    )
    .unwrap()
    .projekt;
    std::fs::create_dir_all(projekt.wurzel.join("bilder")).unwrap();
    std::fs::write(
        projekt.wurzel.join("bilder/logo.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40"><rect width="120" height="40" fill="blue"/></svg>"#,
    )
    .unwrap();
    let daten = projekt.wurzel.join("dokument.toml");
    let alt = std::fs::read_to_string(&daten).unwrap();
    std::fs::write(&daten, format!("logo = \"bilder/logo.svg\"\n{alt}")).unwrap();
    let quelle_datei = wurzel.join("daten/logo.toml");
    std::fs::create_dir_all(quelle_datei.parent().unwrap()).unwrap();
    std::fs::write(
        &quelle_datei,
        "[[serie]]\ndatei = \"mit-logo\"\nnummer = \"2026-201\"\n",
    )
    .unwrap();
    let quelle = serie::lesen(&quelle_datei).unwrap();
    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let ergebnis = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen::default(),
        |_, _, _| {},
    )
    .unwrap();
    let s = &ergebnis.stuecke[0];
    let pdf = s
        .pdf
        .as_ref()
        .unwrap_or_else(|| panic!("{}: {:?}", s.name, s.fehler));
    let satz = std::fs::read_to_string(projekt.wurzel.join("_lokal/serie/mit-logo.toml")).unwrap();
    assert!(satz.contains("bilder/logo.svg"), "{satz}");
    // Mit Logo ist das PDF größer als ohne (gleicher Datensatz, Logo entfernt).
    let mit = std::fs::metadata(pdf).unwrap().len();
    std::fs::write(&daten, &alt).unwrap();
    let ohne = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen::default(),
        |_, _, _| {},
    )
    .unwrap();
    let ohne = std::fs::metadata(ohne.stuecke[0].pdf.as_ref().unwrap())
        .unwrap()
        .len();
    assert!(mit > ohne, "mit Logo {mit} B, ohne {ohne} B");
    let _ = std::fs::remove_dir_all(&wurzel);
}

/// Serie aus einer CSV wie aus deutschem Excel (Semikolon, Windows-1252, CRLF): Spalten finden
/// ihre Felder über Titel und Nummern, eine Spalte über die eigene Zuordnung in `tftn.toml`; eine
/// Zeile mit falschem Datum ergibt kein PDF, aber eine Meldung mit Zeile und Spalte.
#[test]
fn rechnungen_aus_csv() {
    let wurzel = std::env::temp_dir().join(format!("tftn-serie-csv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    let projekt = tftn_manifest::anlegen::projekt_anlegen(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types"),
        "tftn-rechnung",
        &wurzel.join("projekt"),
        Some(1_793_404_800),
    )
    .unwrap()
    .projekt;
    let tftn = projekt.wurzel.join("tftn.toml");
    let alt = std::fs::read_to_string(&tftn).unwrap();
    std::fs::write(
        &tftn,
        format!(
            "{alt}\n[serie-csv.\"adressen.csv\"]\nTelefon = \"absender.telefon\"\nNotiz = \"\"\n"
        ),
    )
    .unwrap();
    let daten = wurzel.join("daten");
    std::fs::create_dir_all(&daten).unwrap();
    let csv = daten.join("adressen.csv");
    let mut bytes: Vec<u8> = Vec::new();
    for zeile in [
        "Dateiname;Rechnungsnummer;Empfänger 1;Empfänger 2;Anrede;Rechnungsdatum;Telefon;Notiz",
        "2026-301 Müller;2026-301;Frau;Mag. Anna Müller;Sehr geehrte Frau Müller,;15.01.2026;+43 1 999;privat",
        "2026-302 Groß;2026-302;\"Herr\r\nJörg Groß\r\nGasse 2\";;Sehr geehrter Herr Groß,;2026-01-16;;",
        "kaputt;2026-303;Frau;X;;31.02.2026;;",
        ";;;;;;;",
    ] {
        // Windows-1252: Die Umlaute sind hier Latin-1 (ein Byte).
        bytes.extend(zeile.chars().map(|c| u8::try_from(u32::from(c)).unwrap()));
        bytes.extend(b"\r\n");
    }
    std::fs::write(&csv, &bytes).unwrap();
    let (quelle, bericht) = serie::quelle_lesen(&projekt, &csv).unwrap();
    let bericht = bericht.expect("CSV hat einen Bericht");
    assert_eq!(bericht.trennzeichen, ';');
    assert!(
        bericht.zuordnungsfehler.is_empty(),
        "{:?}",
        bericht.zuordnungsfehler
    );
    let zuordnung: Vec<(&str, Option<&str>)> = bericht
        .spalten
        .iter()
        .map(|s| (s.name.as_str(), s.feld.as_deref()))
        .collect();
    assert_eq!(
        zuordnung,
        [
            ("Dateiname", Some("datei")),
            ("Rechnungsnummer", Some("nummer")),
            ("Empfänger 1", Some("empfaenger")),
            ("Empfänger 2", Some("empfaenger")),
            ("Anrede", Some("anrede")),
            ("Rechnungsdatum", Some("datum")),
            ("Telefon", Some("absender.telefon")),
            ("Notiz", None),
        ]
    );
    let namen: Vec<&str> = quelle.datensaetze.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(namen, ["2026-301 Müller", "2026-302 Groß", "kaputt"]);
    assert_eq!(quelle.zielordner, daten);
    let fehler = quelle.datensaetze[2].fehler.as_deref().unwrap_or("");
    assert!(
        fehler.contains("Zeile 4") && fehler.contains("Rechnungsdatum"),
        "{fehler}"
    );

    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let ergebnis = serie::erzeugen(
        &projekt,
        &cli,
        &quelle,
        serie::Optionen {
            ausgabe: serie::Ausgabe::Beides,
            beidseitig: false,
        },
        |_, _, _| {},
    )
    .unwrap();
    let s = &ergebnis.stuecke;
    for st in &s[..2] {
        let pdf = st
            .pdf
            .as_ref()
            .unwrap_or_else(|| panic!("{}: {:?}", st.name, st.fehler));
        assert!(std::fs::read(pdf).unwrap().starts_with(b"%PDF"));
    }
    assert!(s[2].pdf.is_none() && s[2].fehler.as_deref() == Some(fehler));
    assert!(daten.join("2026-301 Müller.pdf").exists());
    let g = ergebnis.gesamt.expect("Sammel-PDF");
    assert_eq!(seiten(g.pdf.as_ref().unwrap()), 2, "ohne die fehlerhafte");
    // Derselbe Weg wie [[serie]]: Datensatz im Projekt, mit den Projektdaten zusammengeführt.
    let satz =
        std::fs::read_to_string(projekt.wurzel.join("_lokal/serie/2026-302 Groß.toml")).unwrap();
    let t: toml::Table = satz.parse().unwrap();
    assert_eq!(t["datum"].to_string(), "2026-01-16", "Datum als TOML-Datum");
    assert_eq!(
        t["empfaenger"].as_array().unwrap().len(),
        3,
        "Zeilen einer Zelle"
    );
    assert_eq!(t["anrede"].as_str(), Some("Sehr geehrter Herr Groß,"));
    assert!(
        t["bank"].is_table() && satz.contains("Fahrtkosten"),
        "{satz}"
    );
    let m: toml::Table =
        std::fs::read_to_string(projekt.wurzel.join("_lokal/serie/2026-301 Müller.toml"))
            .unwrap()
            .parse()
            .unwrap();
    assert_eq!(m["absender"]["telefon"].as_str(), Some("+43 1 999"));
    assert_eq!(m["absender"]["name"].as_str(), Some("Vorname Nachname"));
    assert!(!m.contains_key("datei") && !m.contains_key("Notiz"));
    let _ = std::fs::remove_dir_all(&wurzel);
}
