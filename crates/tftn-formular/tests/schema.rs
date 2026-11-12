//! Schema lesen und Profil „TFTN-Schema 1“ prüfen (PLAN-M2 AP7 Task 1).

use std::path::Path;

use tftn_formular::{Art, TextWidget, formular_lesen, sensibel};

fn wurzel() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn namen(felder: &[tftn_formular::Feld]) -> Vec<&str> {
    felder.iter().map(|f| f.name.as_str()).collect()
}

#[test]
fn fachartikel_0_1_liest() {
    let json = std::fs::read_to_string(wurzel().join(
        "tests/korpus/testartikel/packages/local/tftn-fachartikel/0.1.0/tftn/formular.schema.json",
    ))
    .unwrap();
    let f = formular_lesen(&json).unwrap();
    assert_eq!(f.titel, "Fachartikel");
    assert_eq!(
        namen(&f.felder),
        [
            "titel",
            "untertitel",
            "autoren",
            "datum",
            "abstract",
            "schlagworte"
        ]
    );
    let autoren = &f.felder[2];
    match &autoren.art {
        Art::Eintraege { felder, min, .. } => {
            assert_eq!(*min, Some(1));
            assert_eq!(
                namen(felder),
                ["name", "orcid", "institution", "korrespondierend", "email"]
            );
            assert!(felder[0].pflicht);
            assert!(!felder[1].pflicht);
        }
        a => panic!("autoren: {a:?}"),
    }
    assert!(f.felder[0].pflicht);
    assert_eq!(f.felder[0].pandoc.as_deref(), Some("title"));
    assert!(matches!(f.felder[3].art, Art::Datum { .. }));
    assert!(matches!(
        f.felder[4].art,
        Art::Text {
            widget: TextWidget::Markup,
            ..
        }
    ));
    match &f.felder[5].art {
        Art::Liste { element, .. } => assert!(matches!(**element, Art::Text { .. })),
        a => panic!("schlagworte: {a:?}"),
    }
}

#[test]
fn profil_fehler_mit_ort() {
    let fehler = formular_lesen(
        r##"{
          "type": "object",
          "properties": {
            "a": { "$ref": "#/x" },
            "b": { "type": "string", "format": "farbe" },
            "c": { "type": "array" }
          }
        }"##,
    )
    .unwrap_err();
    let mut orte: Vec<&str> = fehler.iter().map(|f| f.ort.as_str()).collect();
    orte.sort();
    assert_eq!(
        orte,
        ["/properties/a", "/properties/b", "/properties/c"],
        "{fehler:?}"
    );
}

#[test]
fn one_of_wird_wahl() {
    let f = formular_lesen(
        r#"{ "type": "object", "properties": { "w": { "type": "string", "oneOf": [
            { "const": "a", "title": "A" }, { "const": "b", "title": "B" } ] } } }"#,
    )
    .unwrap();
    match &f.felder[0].art {
        Art::Wahl {
            werte,
            beschriftungen,
            ..
        } => {
            assert_eq!(werte, &[serde_json::json!("a"), serde_json::json!("b")]);
            assert_eq!(beschriftungen, &["A", "B"]);
        }
        a => panic!("{a:?}"),
    }
    let fehler = formular_lesen(
        r#"{ "type": "object", "properties": { "w": { "oneOf": [ { "type": "string" } ] } } }"#,
    )
    .unwrap_err();
    assert_eq!(fehler[0].ort, "/properties/w/oneOf/0", "{fehler:?}");
}

#[test]
fn enum_wird_wahl() {
    let f = formular_lesen(
        r#"{ "type": "object", "properties": { "w": { "type": "string", "enum": ["x", "y"] } } }"#,
    )
    .unwrap();
    assert!(matches!(&f.felder[0].art, Art::Wahl { werte, .. } if werte.len() == 2));
}

#[test]
fn widget_passt_zum_typ() {
    let fehler = formular_lesen(
        r#"{ "type": "object", "properties": { "n": { "type": "integer", "x-tftn-widget": "markup" } } }"#,
    )
    .unwrap_err();
    assert_eq!(fehler[0].ort, "/properties/n");
}

#[test]
fn reihenfolge_unbekannt() {
    let fehler = formular_lesen(
        r#"{ "type": "object", "x-tftn-reihenfolge": ["gibtsnicht"],
             "properties": { "a": { "type": "string" } } }"#,
    )
    .unwrap_err();
    assert_eq!(fehler[0].ort, "");
}

#[test]
fn reihenfolge_dann_schema() {
    let f = formular_lesen(
        r#"{ "type": "object", "x-tftn-reihenfolge": ["c"],
             "properties": { "b": { "type": "string" }, "a": { "type": "string" }, "c": { "type": "string" } } }"#,
    )
    .unwrap();
    assert_eq!(namen(&f.felder), ["c", "b", "a"]);
}

#[test]
fn wurzel_muss_objekt_sein() {
    assert!(formular_lesen(r#"{ "type": "string" }"#).is_err());
    assert!(formular_lesen("kein json").is_err());
}

#[test]
fn array_von_arrays_ist_fehler() {
    let fehler = formular_lesen(
        r#"{ "type": "object", "properties": { "m": { "type": "array",
             "items": { "type": "array", "items": { "type": "string" } } } } }"#,
    )
    .unwrap_err();
    assert_eq!(fehler[0].ort, "/properties/m/items");
}

#[test]
fn sensibel_erkannt() {
    let mit = formular_lesen(
        r#"{ "type": "object", "properties": { "p": { "type": "array", "items": {
             "type": "object", "properties": { "sv": { "type": "string", "x-tftn-sensibel": true } } } } } }"#,
    )
    .unwrap();
    assert!(sensibel(&mit));
    let ohne =
        formular_lesen(r#"{ "type": "object", "properties": { "t": { "type": "string" } } }"#)
            .unwrap();
    assert!(!sensibel(&ohne));
}

#[test]
fn alle_fehler_gesammelt() {
    let fehler = formular_lesen(
        r#"{ "type": "object", "properties": {
             "a": { "type": "string", "unbekannt": 1 },
             "b": { "type": "string", "format": "x" } } }"#,
    )
    .unwrap_err();
    assert_eq!(fehler.len(), 2, "{fehler:?}");
}
