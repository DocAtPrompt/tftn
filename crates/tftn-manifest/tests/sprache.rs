//! Dokumenttypen in einer Sprache anlegen: eigene Vorlage je Sprache (`[tool.tftn.vorlagen]`),
//! Beschriftungen des Satzes nach der Dokumentsprache.

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seitenzeilen};
use tftn_manifest::Typpaket;
use tftn_manifest::anlegen::projekt_anlegen_sprache;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn typen() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types")
}

fn anlegen(name: &str, typ: &str, sprache: Option<&str>) -> Projekt {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("sprache-{name}"));
    let _ = std::fs::remove_dir_all(&ordner);
    projekt_anlegen_sprache(&typen(), typ, &ordner, Some(ZEITSTEMPEL), sprache)
        .unwrap()
        .projekt
}

/// Übersetzt und gibt die Zeilen aller Seiten zurück; Fehler lassen den Test scheitern.
fn text(projekt: &Projekt) -> String {
    let world = ProjektWorld::neu(projekt).unwrap();
    let ergebnis = kompilieren(&world);
    let fehler: Vec<_> = ergebnis.meldungen.iter().filter(|m| m.fehler).collect();
    assert!(fehler.is_empty(), "Fehler: {fehler:?}");
    seitenzeilen(&ergebnis.dokument.expect("kompiliert"))
        .concat()
        .join("\n")
}

#[test]
fn manifest_nennt_vorlagen_je_sprache() {
    for typ in ["tftn-fachartikel", "tftn-leer"] {
        let p = Typpaket::lesen(&typen().join(typ)).unwrap();
        let m = p.manifest().unwrap();
        assert_eq!(
            m.vorlagen.get("en").map(String::as_str),
            Some("vorlage-en"),
            "{typ}"
        );
        assert!(typen().join(typ).join("vorlage-en").is_dir(), "{typ}");
    }
}

#[test]
fn fachartikel_englisch() {
    let p = anlegen("fa-en", "tftn-fachartikel", Some("en"));
    let t = text(&p);
    for erwartet in ["Abstract.", "Keywords:", "References", "Note:"] {
        assert!(t.contains(erwartet), "{erwartet} fehlt:\n{t}");
    }
    assert!(t.contains("15 January 2026"), "{t}");
    for deutsch in [
        "Zusammenfassung",
        "Schlagwörter",
        "Literatur",
        "Jänner",
        "Hinweis:",
    ] {
        assert!(!t.contains(deutsch), "{deutsch} steht noch da:\n{t}");
    }
    let daten = std::fs::read_to_string(p.wurzel.join("dokument.toml")).unwrap();
    assert!(daten.contains("sprache = \"en\""), "{daten}");
    let readme = std::fs::read_to_string(p.wurzel.join("README.md")).unwrap();
    assert!(readme.contains("Created with TFTN"), "{readme}");
    assert!(readme.contains("Build without TFTN"), "{readme}");
    assert!(!readme.contains("Angelegt"), "{readme}");
}

#[test]
fn fachartikel_ohne_sprache_bleibt_deutsch() {
    let t = text(&anlegen("fa-de", "tftn-fachartikel", None));
    for erwartet in [
        "Zusammenfassung.",
        "Schlagwörter:",
        "Literatur",
        "15. Jänner 2026",
    ] {
        assert!(t.contains(erwartet), "{erwartet} fehlt:\n{t}");
    }
}

#[test]
fn readme_ohne_sprache_deutsch() {
    let p = anlegen("readme-de", "tftn-leer", None);
    let readme = std::fs::read_to_string(p.wurzel.join("README.md")).unwrap();
    assert!(readme.contains("Angelegt mit TFTN"), "{readme}");
}

#[test]
fn unbekannte_sprache_nimmt_die_vorgabe() {
    let t = text(&anlegen("fa-fr", "tftn-fachartikel", Some("fr")));
    assert!(t.contains("Zusammenfassung."), "{t}");
}

#[test]
fn sprache_mit_region_findet_die_vorlage() {
    let t = text(&anlegen("fa-en-gb", "tftn-fachartikel", Some("en-GB")));
    assert!(t.contains("Abstract."), "{t}");
}

#[test]
fn sprache_im_dokument_umschalten() {
    let p = anlegen("fa-um", "tftn-fachartikel", Some("en"));
    let datei = p.wurzel.join("dokument.toml");
    let d = std::fs::read_to_string(&datei).unwrap();
    std::fs::write(&datei, d.replace("sprache = \"en\"", "sprache = \"de\"")).unwrap();
    let t = text(&p);
    assert!(t.contains("Zusammenfassung."), "{t}");
    assert!(t.contains("Hinweis:"), "{t}");
}

#[test]
fn leer_englisch() {
    let p = anlegen("leer-en", "tftn-leer", Some("en"));
    let haupt = std::fs::read_to_string(p.haupt()).unwrap();
    assert!(haupt.contains("lang: \"en\""), "{haupt}");
    assert!(!haupt.contains("\"de\""), "{haupt}");
    text(&p);
}

#[test]
fn readme_nennt_fonts_nur_wenn_da() {
    use tftn_manifest::anlegen::readme_schriften_abgleichen;
    for (sprache, zeile) in [
        (None, "- `fonts/`: frei lizenzierte"),
        (Some("en"), "- `fonts/`: freely licensed"),
    ] {
        let p = anlegen(
            &format!("readme-fonts-{}", sprache.unwrap_or("de")),
            "tftn-fachartikel",
            sprache,
        );
        let readme = || std::fs::read_to_string(p.wurzel.join("README.md")).unwrap();
        assert!(readme().contains(zeile), "{}", readme());
        std::fs::remove_dir_all(p.wurzel.join("fonts")).unwrap();
        readme_schriften_abgleichen(&p.wurzel).unwrap();
        assert!(!readme().contains("fonts/"), "{}", readme());
        assert!(
            readme().contains("packages/local/"),
            "Rest bleibt: {}",
            readme()
        );
        // Später eingebunden: die Zeile kommt zurück, einmal.
        std::fs::create_dir_all(p.wurzel.join("fonts")).unwrap();
        readme_schriften_abgleichen(&p.wurzel).unwrap();
        readme_schriften_abgleichen(&p.wurzel).unwrap();
        assert_eq!(readme().matches(zeile).count(), 1, "{}", readme());
    }
}
