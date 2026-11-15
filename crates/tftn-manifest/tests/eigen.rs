//! „Eigener Dokumenttyp …“: Kopie eines vorhandenen Typs unter eigenem Namen.

use std::path::{Path, PathBuf};

use tftn_manifest::anlegen::projekt_anlegen_aus;
use tftn_manifest::eigen::{eigenen_typ_anlegen, kennung_aus_titel};
use tftn_manifest::typen::{Herkunft, Typquelle, verfuegbar};

fn quellen(name: &str) -> (PathBuf, Vec<Typquelle>) {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("eigen-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    let eigen = o.join("typen");
    let q = vec![
        Typquelle {
            ordner: eigen,
            herkunft: Herkunft::Eigen,
        },
        Typquelle {
            ordner: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types"),
            herkunft: Herkunft::Tftn,
        },
    ];
    (o, q)
}

#[test]
fn kennung_aus_dem_titel() {
    assert_eq!(
        kennung_aus_titel("Mein Brief").as_deref(),
        Some("mein-brief")
    );
    assert_eq!(
        kennung_aus_titel("  Ärztlicher Befund (kurz) 2 ").as_deref(),
        Some("aerztlicher-befund-kurz-2")
    );
    assert_eq!(kennung_aus_titel("Straße").as_deref(), Some("strasse"));
    assert_eq!(
        kennung_aus_titel("2 Spalten").as_deref(),
        Some("eigen-2-spalten")
    );
    assert_eq!(kennung_aus_titel(" – "), None);
    // Unter Windows reservierte Ordnernamen
    assert_eq!(kennung_aus_titel("Con").as_deref(), Some("eigen-con"));
    assert_eq!(kennung_aus_titel("NUL").as_deref(), Some("eigen-nul"));
    assert_eq!(kennung_aus_titel("com1").as_deref(), Some("eigen-com1"));
}

#[test]
fn kopie_ist_ein_eigener_waehlbarer_typ() {
    let (o, q) = quellen("brief");
    let ordner = eigenen_typ_anlegen(&q, "tftn-brief", "Mein Brief").unwrap();
    assert_eq!(ordner, q[0].ordner.join("mein-brief"));

    let (liste, warnungen) = verfuegbar(&q);
    assert!(warnungen.is_empty(), "{warnungen:?}");
    let t = liste
        .iter()
        .find(|t| t.name == "mein-brief")
        .expect("in der Liste");
    assert_eq!(t.titel, "Mein Brief");
    assert_eq!(t.version.to_string(), "0.1.0");
    assert_eq!(t.herkunft, Herkunft::Eigen);
    // Das Original bleibt wählbar.
    assert!(liste.iter().any(|t| t.name == "tftn-brief"));

    // Schriften liegen jetzt im Typ selbst, nicht mehr neben ihm.
    assert!(ordner.join("schriften/SourceSans3-Regular.otf").is_file());
    let manifest = std::fs::read_to_string(ordner.join("typst.toml")).unwrap();
    assert!(!manifest.contains("../"), "{manifest}");
    assert!(
        manifest.contains("# Manifest für TFTN"),
        "Kommentare bleiben"
    );

    // Ein Projekt daraus importiert den neuen Typ, in beiden Sprachen der Vorlage.
    let p = projekt_anlegen_aus(&q, "mein-brief", &o.join("projekt"), Some(1_793_404_800)).unwrap();
    let main = std::fs::read_to_string(p.projekt.wurzel.join("main.typ")).unwrap();
    assert!(main.contains("@local/mein-brief:0.1.0"), "{main}");
    let en = std::fs::read_to_string(ordner.join("vorlage-en/main.typ")).unwrap();
    assert!(
        en.contains("@local/mein-brief:0.1.0") && !en.contains("tftn-brief"),
        "{en}"
    );
}

#[test]
fn vorhandene_namen_und_unbekannte_vorbilder_abgelehnt() {
    let (_, q) = quellen("doppelt");
    eigenen_typ_anlegen(&q, "tftn-brief", "Mein Brief").unwrap();
    let e = eigenen_typ_anlegen(&q, "tftn-leer", "Mein Brief").unwrap_err();
    assert!(e.contains("mein-brief"), "{e}");
    assert!(eigenen_typ_anlegen(&q, "tftn-brief", "tftn brief").is_err());
    assert!(eigenen_typ_anlegen(&q, "gibt-es-nicht", "Neu").is_err());
    assert!(eigenen_typ_anlegen(&q, "tftn-brief", "  ").is_err());
}

/// Ein eigener Typ lässt sich wieder als Vorbild nehmen.
#[test]
fn eigener_typ_als_vorbild() {
    let (_, q) = quellen("kette");
    eigenen_typ_anlegen(&q, "tftn-brief", "Mein Brief").unwrap();
    let zweiter = eigenen_typ_anlegen(&q, "mein-brief", "Brief Ordination").unwrap();
    let main = std::fs::read_to_string(zweiter.join("vorlage/main.typ")).unwrap();
    assert!(main.contains("@local/brief-ordination:0.1.0"), "{main}");
    assert!(zweiter.join("schriften/SourceSans3-Regular.otf").is_file());
}

/// Finder-Reste im Vorbild kommen nicht mit.
#[test]
fn ds_store_kommt_nicht_mit() {
    let (_, q) = quellen("ds-store");
    let erster = eigenen_typ_anlegen(&q, "tftn-brief", "Mein Brief").unwrap();
    std::fs::write(erster.join(".DS_Store"), b"x").unwrap();
    std::fs::write(erster.join("vorlage/.DS_Store"), b"x").unwrap();
    let zweiter = eigenen_typ_anlegen(&q, "mein-brief", "Zweiter Brief").unwrap();
    assert!(!zweiter.join(".DS_Store").exists());
    assert!(!zweiter.join("vorlage/.DS_Store").exists());
    assert!(zweiter.join("vorlage/main.typ").is_file());
}

/// Ein Manifest, dessen Vorlagenpfad aus dem Typ hinausführt, darf beim Kopieren keine fremden
/// Dateien umschreiben.
#[test]
fn vorlagenpfad_ausserhalb_des_typs_abgelehnt() {
    let (o, q) = quellen("ausserhalb");
    let erster = eigenen_typ_anlegen(&q, "tftn-brief", "Mein Brief").unwrap();
    let fremd = o.join("fremd.typ");
    std::fs::write(&fremd, "#import \"@local/mein-brief:0.1.0\": *\n").unwrap();
    let manifest = std::fs::read_to_string(erster.join("typst.toml")).unwrap();
    let manifest = manifest.replace("path = \"vorlage\"", "path = \"../../..\"");
    assert!(manifest.contains("../../.."), "{manifest}");
    std::fs::write(erster.join("typst.toml"), manifest).unwrap();
    let e = eigenen_typ_anlegen(&q, "mein-brief", "Zweiter Brief");
    assert!(e.is_err(), "{e:?}");
    assert!(
        std::fs::read_to_string(&fremd)
            .unwrap()
            .contains("mein-brief"),
        "fremde Datei unverändert"
    );
}
