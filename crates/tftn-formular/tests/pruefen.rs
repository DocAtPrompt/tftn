//! Werte am Feld prüfen (PLAN-M2 AP7 Task 2).

use serde_json::{Value, json};
use tftn_formular::{Feld, Kontext, Schritt, alles_pruefen, formular_lesen, wert_pruefen};

/// Ein Feld `x` mit dem Schema-Knoten `knoten`; `pflicht` setzt `required`.
fn feld(knoten: &str, pflicht: bool) -> Feld {
    let req = if pflicht { r#""required": ["x"],"# } else { "" };
    let f = formular_lesen(&format!(
        r#"{{ "type": "object", {req} "properties": {{ "x": {knoten} }} }}"#
    ))
    .unwrap();
    f.felder.into_iter().next().unwrap()
}

const OHNE: Kontext = Kontext {
    wurzel: None,
    stile: &["american-psychological-association", "ieee"],
};

fn pruefe(f: &Feld, w: Value) -> Result<(), String> {
    wert_pruefen(f, Some(&w), &OHNE)
}

#[test]
fn pflicht_und_leer() {
    let p = feld(r#"{ "type": "string" }"#, true);
    assert_eq!(wert_pruefen(&p, None, &OHNE), Err("Pflichtfeld".into()));
    let o = feld(r#"{ "type": "string" }"#, false);
    assert_eq!(wert_pruefen(&o, None, &OHNE), Ok(()));
    // Leerer Text im Pflichtfeld ist nur ein Hinweis (Festlegung 7), kein Fehler.
    assert_eq!(pruefe(&p, json!("")), Ok(()));
}

#[test]
fn datum_ungueltig() {
    let d = feld(r#"{ "type": "string", "format": "date" }"#, false);
    assert_eq!(
        pruefe(&d, json!("2026-02-31")),
        Err("kein gültiges Datum".into())
    );
    assert_eq!(
        pruefe(&d, json!("31.02.2026")),
        Err("kein gültiges Datum".into())
    );
    assert_eq!(pruefe(&d, json!("2026-02-28")), Ok(()));
    assert_eq!(pruefe(&d, json!("2028-02-29")), Ok(()));
    assert_eq!(
        pruefe(&d, json!("2026-02-29")),
        Err("kein gültiges Datum".into())
    );
}

#[test]
fn formate() {
    let f = |format: &str| {
        feld(
            &format!(r#"{{ "type": "string", "format": "{format}" }}"#),
            false,
        )
    };
    let orcid = f("orcid");
    assert_eq!(pruefe(&orcid, json!("0000-0002-1825-0097")), Ok(()));
    assert_eq!(
        pruefe(&orcid, json!("0000-0002-1825-0098")),
        Err("keine gültige ORCID".into())
    );
    let iban = f("iban");
    assert_eq!(pruefe(&iban, json!("AT611904300234573201")), Ok(()));
    assert_eq!(pruefe(&iban, json!("AT61 1904 3002 3457 3201")), Ok(()));
    assert_eq!(
        pruefe(&iban, json!("AT611904300234573210")),
        Err("keine gültige IBAN".into())
    );
    let uid = f("uid-at");
    assert_eq!(pruefe(&uid, json!("ATU12345675")), Ok(()));
    assert_eq!(
        pruefe(&uid, json!("ATU12345676")),
        Err("keine gültige UID (ATU + 8 Ziffern)".into())
    );
    let sv = f("sv-nummer");
    assert_eq!(pruefe(&sv, json!("1237010180")), Ok(()));
    assert_eq!(
        pruefe(&sv, json!("1238010180")),
        Err("keine gültige Sozialversicherungsnummer".into())
    );
    let mail = f("email");
    assert_eq!(pruefe(&mail, json!("a@b.at")), Ok(()));
    assert_eq!(
        pruefe(&mail, json!("a@")),
        Err("keine gültige E-Mail-Adresse".into())
    );
    let uri = f("uri");
    assert_eq!(pruefe(&uri, json!("https://example.org/x")), Ok(()));
    assert_eq!(
        pruefe(&uri, json!("kein uri")),
        Err("keine gültige Adresse (URI)".into())
    );
}

#[test]
fn meldung_ohne_wert() {
    let f = feld(
        r#"{ "type": "string", "maxLength": 3, "x-tftn-sensibel": true }"#,
        false,
    );
    let m = pruefe(&f, json!("Geheimname")).unwrap_err();
    assert_eq!(m, "höchstens 3 Zeichen");
    assert!(!m.contains("Geheimname"));
}

#[test]
fn laenge_und_muster() {
    let f = feld(
        r#"{ "type": "string", "minLength": 2, "pattern": "^[a-z]+$" }"#,
        false,
    );
    assert_eq!(pruefe(&f, json!("ä")), Err("mindestens 2 Zeichen".into()));
    assert_eq!(
        pruefe(&f, json!("AB")),
        Err("entspricht nicht dem Muster".into())
    );
    assert_eq!(pruefe(&f, json!("ab")), Ok(()));
}

#[test]
fn einzeilig_ohne_umbruch() {
    let f = feld(r#"{ "type": "string" }"#, false);
    assert_eq!(pruefe(&f, json!("a\nb")), Err("nur eine Zeile".into()));
    let m = feld(
        r#"{ "type": "string", "x-tftn-widget": "mehrzeilig" }"#,
        false,
    );
    assert_eq!(pruefe(&m, json!("a\nb")), Ok(()));
}

#[test]
fn zahlen_schalter_wahl() {
    let z = feld(
        r#"{ "type": "integer", "minimum": 1, "maximum": 6 }"#,
        false,
    );
    assert_eq!(pruefe(&z, json!(3)), Ok(()));
    assert_eq!(pruefe(&z, json!(2.5)), Err("ganze Zahl erwartet".into()));
    assert_eq!(pruefe(&z, json!(7)), Err("Zahl zwischen 1 und 6".into()));
    assert_eq!(pruefe(&z, json!("3")), Err("Zahl erwartet".into()));
    let s = feld(r#"{ "type": "boolean" }"#, false);
    assert_eq!(pruefe(&s, json!(true)), Ok(()));
    assert_eq!(pruefe(&s, json!("ja")), Err("ja oder nein erwartet".into()));
    let w = feld(r#"{ "type": "string", "enum": ["a", "b"] }"#, false);
    assert_eq!(pruefe(&w, json!("b")), Ok(()));
    assert_eq!(pruefe(&w, json!("c")), Err("kein Wert der Auswahl".into()));
}

#[test]
fn listen() {
    let l = feld(
        r#"{ "type": "array", "minItems": 1, "maxItems": 2, "items": { "type": "string", "maxLength": 2 } }"#,
        false,
    );
    assert_eq!(pruefe(&l, json!(["a", "b"])), Ok(()));
    assert_eq!(pruefe(&l, json!([])), Err("mindestens 1 Eintrag".into()));
    assert_eq!(
        pruefe(&l, json!(["a", "b", "c"])),
        Err("höchstens 2 Einträge".into())
    );
    assert_eq!(
        pruefe(&l, json!(["abc"])),
        Err("Eintrag 1: höchstens 2 Zeichen".into())
    );
}

#[test]
fn markup_pruefen() {
    let f = feld(r#"{ "type": "string", "x-tftn-widget": "markup" }"#, false);
    assert_eq!(pruefe(&f, json!("*fett* und _kursiv_")), Ok(()));
    let m = pruefe(&f, json!("#let x = (")).unwrap_err();
    assert!(m.starts_with("Markup fehlerhaft"), "{m}");
}

#[test]
fn literaturdatei_und_stil() {
    let w = std::env::temp_dir().join(format!("tftn-formular-pruefen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&w);
    std::fs::create_dir_all(w.join("lit")).unwrap();
    std::fs::write(w.join("a.bib"), "").unwrap();
    std::fs::write(w.join("lit/b.yaml"), "").unwrap();
    std::fs::write(w.join("a.txt"), "").unwrap();
    std::fs::write(w.join("stil.csl"), "").unwrap();
    let k = Kontext {
        wurzel: Some(&w),
        stile: &["american-psychological-association"],
    };
    let d = feld(r#"{ "type": "string", "format": "literaturdatei" }"#, false);
    let p = |f: &Feld, v: &str| wert_pruefen(f, Some(&json!(v)), &k);
    assert_eq!(p(&d, "a.bib"), Ok(()));
    assert_eq!(p(&d, "lit/b.yaml"), Ok(()));
    assert_eq!(
        p(&d, "fehlt.bib"),
        Err("Datei nicht im Projekt gefunden".into())
    );
    assert_eq!(
        p(&d, "../a.bib"),
        Err("Datei nicht im Projekt gefunden".into())
    );
    assert_eq!(
        p(&d, "/etc/a.bib"),
        Err("Datei nicht im Projekt gefunden".into())
    );
    assert_eq!(
        p(&d, "a.txt"),
        Err("Literaturdatei muss .bib, .yaml oder .yml sein".into())
    );
    let s = feld(r#"{ "type": "string", "format": "zitierstil" }"#, false);
    assert_eq!(p(&s, "american-psychological-association"), Ok(()));
    assert_eq!(p(&s, "stil.csl"), Ok(()));
    assert_eq!(
        p(&s, "fehlt.csl"),
        Err("Datei nicht im Projekt gefunden".into())
    );
    assert_eq!(p(&s, "erfunden"), Err("unbekannter Zitierstil".into()));
    let _ = std::fs::remove_dir_all(&w);
}

#[test]
fn alles_pruefen_mit_pfaden() {
    let f = formular_lesen(
        r#"{ "type": "object", "required": ["titel", "autoren"], "properties": {
             "titel": { "type": "string" },
             "datum": { "type": "string", "format": "date" },
             "autoren": { "type": "array", "items": { "type": "object", "required": ["name"],
                          "properties": { "name": { "type": "string" } } } } } }"#,
    )
    .unwrap();
    let m = alles_pruefen(
        &f,
        &json!({ "titel": "T", "autoren": [{ "name": "" }] }),
        &OHNE,
    );
    assert_eq!(m.len(), 1, "{m:?}");
    assert_eq!(
        m[0].pfad,
        vec![
            Schritt::Name("autoren".into()),
            Schritt::Index(0),
            Schritt::Name("name".into())
        ]
    );
    assert_eq!(m[0].text, "Pflichtfeld leer");
    assert!(!m[0].fehler);

    let m = alles_pruefen(&f, &json!({ "datum": "x", "autoren": [] }), &OHNE);
    let texte: Vec<(&str, bool)> = m.iter().map(|m| (m.text.as_str(), m.fehler)).collect();
    assert_eq!(
        texte,
        [("Pflichtfeld", true), ("kein gültiges Datum", true)],
        "{m:?}"
    );
}

#[test]
fn pfad_als_json() {
    let p: Vec<Schritt> = serde_json::from_value(json!(["autoren", 0, "name"])).unwrap();
    assert_eq!(p[1], Schritt::Index(0));
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        json!(["autoren", 0, "name"])
    );
}
