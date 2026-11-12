//! Abnahmeprüfungen AP2 (PLAN-M1): Projekt aus tftn-fachartikel anlegen und bauen,
//! österreichisches Datum, Trennausnahmen.
//!
//! Der Export-Test braucht die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seitenzeilen};
use tftn_export::{TypstCli, exportieren};
use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn typen() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typen")
}

fn neues_projekt(name: &str) -> Projekt {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    projekt_anlegen(&typen(), "tftn-fachartikel", &ordner, Some(ZEITSTEMPEL))
        .unwrap()
        .projekt
}

fn zeilen(projekt: &Projekt) -> Vec<String> {
    let world = ProjektWorld::neu(projekt).unwrap();
    let ergebnis = kompilieren(&world);
    for m in &ergebnis.meldungen {
        eprintln!("{m}");
    }
    let dokument = ergebnis.dokument.expect("kompiliert");
    seitenzeilen(&dokument).concat()
}

#[test]
fn angelegtes_projekt_baut_ohne_meldungen_und_wiederholbar() {
    let projekt = neues_projekt("ap2-bauen");
    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let eins = projekt.wurzel.join("_renders/eins.pdf");
    let zwei = projekt.wurzel.join("_renders/zwei.pdf");
    std::fs::create_dir_all(eins.parent().unwrap()).unwrap();
    let meldungen = exportieren(&projekt, &cli, &eins, &[]).unwrap();
    assert!(meldungen.is_empty(), "Warnungen: {meldungen:?}");
    exportieren(&projekt, &cli, &zwei, &[]).unwrap();
    assert_eq!(std::fs::read(eins).unwrap(), std::fs::read(zwei).unwrap());
    assert_eq!(
        std::fs::read_to_string(projekt.wurzel.join("tftn.toml"))
            .unwrap()
            .lines()
            .nth(2),
        Some("typ = \"@local/tftn-fachartikel:0.2.0\"")
    );
    assert!(
        projekt
            .wurzel
            .join("packages/local/tftn-kern/0.1.0/lib.typ")
            .exists()
    );
    assert!(
        !projekt
            .wurzel
            .join("packages/local/tftn-fachartikel/0.2.0/vorlage")
            .exists()
    );
    assert!(
        !projekt
            .wurzel
            .join("packages/local/tftn-fachartikel/0.2.0/schriften")
            .exists()
    );
}

#[test]
fn datum_auf_oesterreichisch() {
    let projekt = neues_projekt("ap2-datum");
    let text = zeilen(&projekt).join("\n");
    assert!(text.contains("15. Jänner 2026"), "Datum fehlt in:\n{text}");
}

const WORT: &str = "Sozialversicherungsträgerdokumentationspflicht";

/// Setzt `WORT` dreimal in eine schmale Spalte und gibt die Wortanfänge zurück, nach denen
/// eine Zeile endet (also die tatsächlich genutzten Trennstellen).
fn trennstellen(projekt: &Projekt, ausnahmen: &str) -> Vec<String> {
    std::fs::write(projekt.wurzel.join("trennungen.txt"), ausnahmen).unwrap();
    std::fs::write(
        projekt.wurzel.join("main.typ"),
        format!(
            "#import \"@local/tftn-kern:0.1.0\": trennungen\n\
             #set page(width: 5cm, height: auto, margin: 3mm)\n\
             #set text(lang: \"de\", font: \"Libertinus Serif\")\n\
             #set par(justify: true)\n\
             #show: trennungen(read(\"trennungen.txt\"))\n\
             Die {WORT} gilt. Die {WORT} gilt. Die {WORT} gilt.\n"
        ),
    )
    .unwrap();
    // Typst legt Trennstriche (am Zeilenende wie im Wort) als U+00AD in den Text.
    // Zeilenumbrüche als „|“ markieren, dann im Fließtext die Umbrüche innerhalb von WORT suchen.
    let text: String = zeilen(projekt)
        .iter()
        .map(|z| z.trim_end().replace('\u{ad}', ""))
        .collect::<Vec<_>>()
        .join("|");
    let mut stellen = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("Sozi") {
        let mut wort = String::new();
        for c in rest[start..].chars() {
            if c == '|' && wort.chars().count() < WORT.chars().count() {
                stellen.push(wort.clone());
            } else if c != '|' && wort.chars().count() < WORT.chars().count() {
                wort.push(c);
            } else {
                break;
            }
        }
        assert_eq!(wort, WORT, "Wort nicht vollständig gefunden in {text:?}");
        rest = &rest[start + 4..];
    }
    stellen
}

#[test]
fn trennausnahme_greift() {
    let projekt = neues_projekt("ap2-trennung");
    let erlaubt = [
        "Sozial",
        "Sozialversicherungs",
        "Sozialversicherungsträger",
        "Sozialversicherungsträgerdokumentations",
    ];

    // Gegenprobe: Ohne Ausnahme trennt Typst auch an anderen Stellen.
    let ohne = trennstellen(&projekt, "");
    eprintln!("ohne Ausnahme: {ohne:?}");
    assert!(
        ohne.iter().any(|s| !erlaubt.contains(&s.as_str())),
        "Gegenprobe wirkungslos: {ohne:?}"
    );

    let mit = trennstellen(
        &projekt,
        "# Test\nSozial-versicherungs-träger-dokumentations-pflicht\n",
    );
    eprintln!("mit Ausnahme: {mit:?}");
    assert!(!mit.is_empty(), "keine Trennung, Test ohne Aussage");
    for s in &mit {
        assert!(
            erlaubt.contains(&s.as_str()),
            "unerlaubte Trennstelle {s}- in {mit:?}"
        );
    }
}
