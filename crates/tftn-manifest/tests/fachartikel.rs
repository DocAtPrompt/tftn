//! Fachartikel ab 0.2.0 (PLAN-M2 AP7 Task 4 und 5): Literatur aus dokument.toml, Feldmarken in der
//! Titelei und der Klick darauf in der Vorschau.

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seitenzeilen};
use tftn_manifest::Typpaket;
use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn neues_projekt(name: &str) -> Projekt {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    projekt_anlegen(
        &wurzel().join("types"),
        "tftn-fachartikel",
        &ordner,
        Some(ZEITSTEMPEL),
    )
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

fn dokument_aendern(projekt: &Projekt, alt: &str, neu: &str) {
    let datei = projekt.wurzel.join("dokument.toml");
    let t = std::fs::read_to_string(&datei).unwrap();
    assert!(t.contains(alt), "{alt} fehlt in:\n{t}");
    std::fs::write(&datei, t.replace(alt, neu)).unwrap();
}

#[test]
fn neu_aus_0_3_kompiliert() {
    let projekt = neues_projekt("ap7-neu");
    assert_eq!(
        projekt.konfig.typ.as_deref(),
        Some("@local/tftn-fachartikel:0.3.0")
    );
    let t = text(&projekt);
    assert!(t.contains("Literatur"), "{t}");
    assert!(t.contains("Eine erfundene Einführung"), "{t}");
    assert!(
        !std::fs::read_to_string(projekt.haupt())
            .unwrap()
            .contains("#bibliography")
    );
}

#[test]
fn stil_aus_dokument_toml() {
    let projekt = neues_projekt("ap7-stil");
    let apa = text(&projekt);
    assert!(apa.contains("(Muster, 2026"), "{apa}");
    dokument_aendern(
        &projekt,
        r#"stil = "american-psychological-association""#,
        r#"stil = "ieee""#,
    );
    let ieee = text(&projekt);
    assert!(ieee.contains("[1]"), "{ieee}");
    assert!(!ieee.contains("(Muster, 2026"), "{ieee}");
}

#[test]
fn eigene_csl_datei() {
    let projekt = neues_projekt("ap7-csl");
    std::fs::write(
        projekt.wurzel.join("stil.csl"),
        r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>Probe</title><id>probe</id><updated>2026-01-01T00:00:00+00:00</updated></info>
  <citation><layout prefix="‹" suffix="›"><text variable="citation-number"/></layout></citation>
  <bibliography><layout><text variable="title"/></layout></bibliography>
</style>
"#,
    )
    .unwrap();
    dokument_aendern(
        &projekt,
        r#"stil = "american-psychological-association""#,
        r#"stil = "stil.csl""#,
    );
    let t = text(&projekt);
    assert!(t.contains("‹1›"), "{t}");
}

#[test]
fn vorgabe_gleich_manifest() {
    let typ = wurzel().join("types/tftn-fachartikel");
    let paket = Typpaket::lesen(&typ).unwrap();
    let lit = paket.manifest().unwrap().literatur.clone().unwrap();
    let daten: toml::Table =
        toml::from_str(&std::fs::read_to_string(typ.join("vorlage/dokument.toml")).unwrap())
            .unwrap();
    let vorgabe = daten["literatur"].as_table().unwrap();
    let dateien: Vec<&str> = vorgabe["dateien"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(dateien, lit.dateien);
    assert_eq!(vorgabe["stil"].as_str(), lit.stil.as_deref());
}

#[test]
fn schema_besteht_profil() {
    let json =
        std::fs::read_to_string(wurzel().join("types/tftn-fachartikel/tftn/formular.schema.json"))
            .unwrap();
    let f = tftn_formular::formular_lesen(&json).unwrap();
    let lit = f.felder.iter().find(|f| f.name == "literatur").unwrap();
    assert!(matches!(lit.art, tftn_formular::Art::Gruppe { .. }));
    assert_eq!(lit.gruppe.as_deref(), Some("Literatur"));
    assert_eq!(f.felder.last().unwrap().name, "literatur");
}

#[test]
fn vorlage_besteht_eigenes_schema() {
    let typ = wurzel().join("types/tftn-fachartikel");
    let f = tftn_formular::formular_lesen(
        &std::fs::read_to_string(typ.join("tftn/formular.schema.json")).unwrap(),
    )
    .unwrap();
    let d = tftn_formular::Daten::lesen(
        &std::fs::read_to_string(typ.join("vorlage/dokument.toml")).unwrap(),
    )
    .unwrap();
    let k = tftn_formular::Kontext {
        wurzel: Some(&typ.join("vorlage")),
        stile: &["american-psychological-association"],
    };
    let m = tftn_formular::alles_pruefen(&f, &d.werte(), &k);
    assert!(m.is_empty(), "{m:?}");
}

#[test]
fn testartikel_bleibt_0_1() {
    let t = std::fs::read_to_string(wurzel().join("tests/korpus/testartikel/tftn.toml")).unwrap();
    assert!(t.contains(r#"typ = "@local/tftn-fachartikel:0.1.0""#));
}

// --- Task 5: Klick auf die Titelei ergibt das Formularfeld ----------------------------------

use tftn_core::vorschau::{Sprung, Vorschau};

/// Mitte des ersten Textstücks auf Seite 1, das `nadel` enthält (x, y in pt).
fn stelle(v: &Vorschau, nadel: &str) -> (f64, f64) {
    use typst::layout::{Frame, FrameItem};
    fn suchen(frame: &Frame, dx: f64, dy: f64, nadel: &str) -> Option<(f64, f64)> {
        for (pos, item) in frame.items() {
            let x = dx + pos.x.to_pt();
            let y = dy + pos.y.to_pt();
            match item {
                FrameItem::Group(g) => {
                    if let Some(p) = suchen(
                        &g.frame,
                        x + g.transform.tx.to_pt(),
                        y + g.transform.ty.to_pt(),
                        nadel,
                    ) {
                        return Some(p);
                    }
                }
                FrameItem::Text(t) if t.text.contains(nadel) => {
                    return Some((x + t.width().to_pt() / 2.0, y - t.size.to_pt() * 0.3));
                }
                _ => {}
            }
        }
        None
    }
    let gut = v.gut().expect("guter Stand");
    suchen(&gut.dokument.pages()[0].frame, 0.0, 0.0, nadel)
        .unwrap_or_else(|| panic!("{nadel} nicht auf Seite 1"))
}

fn vorschau(projekt: &Projekt) -> Vorschau {
    let mut v = Vorschau::neu(projekt).unwrap();
    assert!(v.lauf(1, None).gelungen);
    v
}

fn klick(v: &Vorschau, nadel: &str) -> Sprung {
    let (x, y) = stelle(v, nadel);
    v.klick(0, x, y)
}

#[test]
fn klick_auf_titel_und_autor() {
    let projekt = neues_projekt("ap7-klick");
    let v = vorschau(&projekt);
    assert_eq!(
        klick(&v, "Titel des Artikels"),
        Sprung::Feld("titel".into())
    );
    assert_eq!(
        klick(&v, "Vorname Nachname"),
        Sprung::Feld("autoren".into())
    );
    assert_eq!(klick(&v, "Institution"), Sprung::Feld("autoren".into()));
    assert_eq!(klick(&v, "Jänner"), Sprung::Feld("datum".into()));
    assert_eq!(
        klick(&v, "Schlagwort eins"),
        Sprung::Feld("schlagworte".into())
    );
}

#[test]
fn klick_auf_zusammenfassung() {
    let projekt = neues_projekt("ap7-klick-abstract");
    let v = vorschau(&projekt);
    assert_eq!(
        klick(&v, "Zusammenfassung."),
        Sprung::Feld("abstract".into())
    );
    // Auch Text aus eval(…, mode: "markup") trägt Stellen in der Feldmarke (Probe Task 5).
    assert_eq!(klick(&v, "kurze"), Sprung::Feld("abstract".into()));
}

#[test]
fn testartikel_bleibt_vorlage() {
    let projekt = Projekt::oeffnen(&wurzel().join("tests/korpus/testartikel")).unwrap();
    let v = vorschau(&projekt);
    assert_eq!(klick(&v, "Reproduzierbarkeit"), Sprung::Vorlage);
}
