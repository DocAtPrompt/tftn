//! Weitere Schriften: Familie aus einer Datei nach `fonts/` kopieren, Zeichen prüfen, Ersatz und
//! leere Kästchen im gesetzten Dokument melden.

use std::path::{Path, PathBuf};

use tftn_core::schriften::{
    dateifamilien, einbettung_gesperrt, familie_einbinden, fehlende_zeichen,
};
use tftn_core::{Projekt, ProjektWorld, kompilieren};
use typst::text::{Coverage, FontFlags, FontInfo, FontVariant};

fn typschriften(datei: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../types/tftn-fachartikel/schriften")
        .join(datei)
        .canonicalize()
        .unwrap()
}

fn projekt(name: &str, text: &str) -> Projekt {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(o.join("main.typ"), text).unwrap();
    Projekt::oeffnen(&o).unwrap()
}

#[test]
fn familie_aus_datei_kommt_ganz_ins_projekt() {
    let p = projekt("schriften-familie", "Hallo\n");
    let familien = dateifamilien(&typschriften("LibertinusSerif-Regular.otf"));
    assert_eq!(familien.len(), 1, "{familien:?}");
    let f = &familien[0];
    assert_eq!(f.name, "Libertinus Serif");
    // Regular, Italic, Bold, BoldItalic, Semibold, SemiboldItalic aus demselben Ordner.
    assert_eq!(f.dateien.len(), 6, "{:?}", f.dateien);
    assert!(fehlende_zeichen(&f.info, "de").is_empty());

    let neu = familie_einbinden(&p, f).unwrap();
    assert_eq!(neu.len(), 6);
    assert!(p.schriften().join("LibertinusSerif-Bold.otf").is_file());
    // Ein zweites Mal kopiert nichts.
    assert!(familie_einbinden(&p, f).unwrap().is_empty());
}

#[test]
fn fehlende_zeichen_je_sprache() {
    let nur_ascii = FontInfo {
        family: "Nur ASCII".into(),
        variant: FontVariant::default(),
        flags: FontFlags::empty(),
        axes: Vec::new(),
        coverage: Coverage::from_vec((0x20..0x7f).collect()),
    };
    let de = fehlende_zeichen(&nur_ascii, "de");
    assert!(
        de.contains(&'ß') && de.contains(&'ü') && de.contains(&'€'),
        "{de:?}"
    );
    assert!(!de.contains(&'a'));
    let en = fehlende_zeichen(&nur_ascii, "en");
    assert!(!en.contains(&'ß') && en.contains(&'“'), "{en:?}");
}

/// Kleinste sfnt-Datei mit nur einer OS/2-Tabelle, deren `fsType` gesetzt ist.
fn sfnt_mit_fs_type(fs_type: u16) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend(0x0001_0000u32.to_be_bytes());
    d.extend(1u16.to_be_bytes()); // numTables
    d.extend([0u8; 6]);
    d.extend(*b"OS/2");
    d.extend(0u32.to_be_bytes()); // checksum
    d.extend(28u32.to_be_bytes()); // offset
    d.extend(10u32.to_be_bytes()); // length
    d.extend([0u8; 8]); // version, xAvgCharWidth, usWeightClass, usWidthClass
    d.extend(fs_type.to_be_bytes());
    d
}

#[test]
fn einbettungssperre_aus_fs_type() {
    assert!(einbettung_gesperrt(&sfnt_mit_fs_type(0x0002), 0));
    assert!(einbettung_gesperrt(&sfnt_mit_fs_type(0x0200), 0));
    assert!(!einbettung_gesperrt(&sfnt_mit_fs_type(0x0000), 0));
    assert!(!einbettung_gesperrt(&sfnt_mit_fs_type(0x0004), 0)); // Preview & Print
    assert!(!einbettung_gesperrt(&sfnt_mit_fs_type(0x0008), 0)); // Editable
    let echt = std::fs::read(typschriften("LibertinusSerif-Regular.otf")).unwrap();
    assert!(!einbettung_gesperrt(&echt, 0));
    assert!(!einbettung_gesperrt(b"kurz", 0));
}

#[test]
fn ersatz_und_leere_kaestchen_werden_gemeldet() {
    let p = projekt(
        "schriften-ersatz",
        "#set text(font: \"Libertinus Serif\")\nStraße ─ 漢 und noch etwas Text.\n\n`Code` und $a + b = sum_i x_i$.\n",
    );
    for datei in [
        "LibertinusSerif-Regular.otf",
        "DejaVuSansMono.ttf",
        "NewCMMath-Regular.otf",
    ] {
        for f in dateifamilien(&typschriften(datei)) {
            familie_einbinden(&p, &f).unwrap();
        }
    }
    let libertinus = &dateifamilien(&typschriften("LibertinusSerif-Regular.otf"))[0];
    let mono = &dateifamilien(&typschriften("DejaVuSansMono.ttf"))[0];
    // Voraussetzungen des Tests: ─ nur in DejaVu, 漢 in keiner der beiden.
    assert!(!libertinus.info.coverage.contains('─' as u32));
    assert!(mono.info.coverage.contains('─' as u32));
    assert!(!mono.info.coverage.contains('漢' as u32));

    let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
    assert!(k.dokument.is_some(), "{:?}", k.meldungen);
    let texte: Vec<&str> = k.meldungen.iter().map(|m| m.text.as_str()).collect();
    let strich = texte
        .iter()
        .find(|t| t.contains("„─“"))
        .unwrap_or_else(|| panic!("{texte:?}"));
    // Woher genau, entscheidet Typst nach der Reihenfolge der Schriften.
    assert!(
        strich.contains("„Libertinus Serif“") && strich.contains("es kommt aus"),
        "{strich}"
    );
    assert!(
        texte
            .iter()
            .any(|t| t.contains("„漢“") && t.contains("Kästchen")),
        "{texte:?}"
    );
    // ß hat Libertinus; Code in Monospace ist kein Ersatz.
    assert!(
        !texte.iter().any(|t| t.contains("„ß“") || t.contains("„C“")),
        "{texte:?}"
    );
    // Die Meldung zeigt auf die Stelle in main.typ.
    let m = k.meldungen.iter().find(|m| m.text.contains("„─“")).unwrap();
    assert_eq!(m.ort.as_deref(), Some("main.typ:2:8"));
    let m = k
        .meldungen
        .iter()
        .find(|m| m.text.contains("„漢“"))
        .unwrap();
    assert_eq!(m.ort.as_deref(), Some("main.typ:2:10"));
    let (_, bereich) = m.stelle.clone().unwrap();
    assert_eq!(bereich.len(), '漢'.len_utf8());
}

#[test]
fn ohne_schriften_gibt_es_eine_klare_meldung() {
    let p = projekt("schriften-keine", "Hallo\n");
    let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
    assert!(
        k.meldungen
            .iter()
            .any(|m| m.text.contains("Keine Schriften gefunden")),
        "{:?}",
        k.meldungen
    );
}

/// Messung auf dem eigenen Rechner, nicht in CI: `cargo test -p tftn-core --test schriften -- --ignored --nocapture`.
#[test]
#[ignore]
fn systemschriften_messen() {
    let start = std::time::Instant::now();
    let familien = tftn_core::schriften::systemfamilien();
    let dauer = start.elapsed();
    let mit_luecken = familien
        .iter()
        .filter(|f| !fehlende_zeichen(&f.info, "de").is_empty())
        .count();
    let gesperrt: Vec<&str> = familien
        .iter()
        .filter(|f| {
            f.dateien
                .iter()
                .any(|d| std::fs::read(d).is_ok_and(|daten| einbettung_gesperrt(&daten, 0)))
        })
        .map(|f| f.name.as_str())
        .collect();
    println!(
        "{} Familien in {dauer:?}; {mit_luecken} mit Lücken für Deutsch; gesperrt: {gesperrt:?}",
        familien.len()
    );
}
