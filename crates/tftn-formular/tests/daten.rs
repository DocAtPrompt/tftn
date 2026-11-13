//! dokument.toml lesen, schreiben, Historie (PLAN-M2 AP7 Task 3).

use serde_json::json;
use tftn_formular::{
    Daten, EintragAktion, Formular, Historie, Kontext, Schritt, atomar_schreiben, formular_lesen,
    literatur_dateien, stand,
};

const TEXT: &str = r#"# Kopfkommentar
titel = "Alt" # hinter dem Wert
fremd = 1
datum = 2026-01-15

[[autoren]]
name = "A. Muster"
"#;

const K: Kontext = Kontext {
    wurzel: None,
    stile: &[],
};

pub fn fachartikel() -> Formular {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/korpus/testartikel/packages/local/tftn-fachartikel/0.1.0/tftn/formular.schema.json"
    ))
    .unwrap();
    formular_lesen(&json).unwrap()
}

fn p(teile: &[&str]) -> Vec<Schritt> {
    teile
        .iter()
        .map(|t| match t.parse() {
            Ok(i) => Schritt::Index(i),
            Err(_) => Schritt::Name(t.to_string()),
        })
        .collect()
}

#[test]
fn setzen_erhaelt_kommentare() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    d.setzen(&f, &p(&["titel"]), Some(json!("Neu")), &K)
        .unwrap();
    assert_eq!(d.text(), TEXT.replace(r#""Alt""#, r#""Neu""#));
}

#[test]
fn leer_entfernt_schluessel() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    d.setzen(&f, &p(&["untertitel"]), Some(json!("U")), &K)
        .unwrap();
    assert!(d.text().contains(r#"untertitel = "U""#), "{}", d.text());
    assert_eq!(d.werte()["untertitel"], "U");
    d.setzen(&f, &p(&["untertitel"]), None, &K).unwrap();
    assert_eq!(d.text(), TEXT);
}

#[test]
fn datum_als_toml_datum() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    d.setzen(&f, &p(&["datum"]), Some(json!("2026-03-01")), &K)
        .unwrap();
    assert!(d.text().contains("datum = 2026-03-01\n"), "{}", d.text());
    assert_eq!(d.werte()["datum"], "2026-03-01");
}

#[test]
fn ungueltig_schreibt_nichts() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    assert_eq!(
        d.setzen(&f, &p(&["datum"]), Some(json!("x")), &K),
        Err("kein gültiges Datum".into())
    );
    assert_eq!(
        d.setzen(&f, &p(&["titel"]), None, &K),
        Err("Pflichtfeld".into())
    );
    assert_eq!(
        d.setzen(&f, &p(&["gibtsnicht"]), Some(json!(1)), &K),
        Err("unbekanntes Feld".into())
    );
    assert_eq!(d.text(), TEXT);
}

#[test]
fn feld_im_eintrag_und_liste() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    d.setzen(
        &f,
        &p(&["autoren", "0", "name"]),
        Some(json!("B. Beispiel")),
        &K,
    )
    .unwrap();
    d.setzen(
        &f,
        &p(&["autoren", "0", "institution"]),
        Some(json!(["Erfundene Uni"])),
        &K,
    )
    .unwrap();
    d.setzen(&f, &p(&["schlagworte"]), Some(json!(["a", "b"])), &K)
        .unwrap();
    let w = d.werte();
    assert_eq!(w["autoren"][0]["name"], "B. Beispiel");
    assert_eq!(w["autoren"][0]["institution"], json!(["Erfundene Uni"]));
    assert_eq!(w["schlagworte"], json!(["a", "b"]));
    assert!(
        d.text().contains(r#"schlagworte = ["a", "b"]"#),
        "{}",
        d.text()
    );
    assert_eq!(
        d.setzen(&f, &p(&["autoren", "3", "name"]), Some(json!("X")), &K),
        Err("Eintrag fehlt".into())
    );
}

#[test]
fn gruppe_wird_tabelle() {
    let f = formular_lesen(
        r#"{ "type": "object", "properties": { "literatur": { "type": "object",
             "properties": { "stil": { "type": "string" } } } } }"#,
    )
    .unwrap();
    let mut d = Daten::lesen("titel = \"T\"\n").unwrap();
    d.setzen(&f, &p(&["literatur", "stil"]), Some(json!("ieee")), &K)
        .unwrap();
    assert_eq!(d.text(), "titel = \"T\"\n\n[literatur]\nstil = \"ieee\"\n");
}

#[test]
fn eintraege() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    d.eintrag(&f, &p(&["autoren"]), EintragAktion::Hinzufuegen)
        .unwrap();
    let w = d.werte();
    assert_eq!(
        w["autoren"][1],
        json!({ "name": "", "korrespondierend": false })
    );
    assert!(
        d.text()
            .ends_with("[[autoren]]\nname = \"\"\nkorrespondierend = false\n"),
        "{}",
        d.text()
    );
    d.eintrag(
        &f,
        &p(&["autoren"]),
        EintragAktion::Verschieben { index: 1, nach: 0 },
    )
    .unwrap();
    let w = d.werte();
    assert_eq!(w["autoren"][0]["name"], "");
    assert_eq!(w["autoren"][1]["name"], "A. Muster");
    // Die Reihenfolge steht auch im Text so.
    let t = d.text();
    assert!(
        t.find("name = \"\"").unwrap() < t.find("A. Muster").unwrap(),
        "{t}"
    );
    d.eintrag(&f, &p(&["autoren"]), EintragAktion::Entfernen { index: 0 })
        .unwrap();
    assert_eq!(d.werte()["autoren"], json!([{ "name": "A. Muster" }]));
}

#[test]
fn hinzufuegen_ohne_vorhandene_liste() {
    let f = fachartikel();
    let mut d = Daten::lesen("titel = \"T\"\n").unwrap();
    d.eintrag(&f, &p(&["autoren"]), EintragAktion::Hinzufuegen)
        .unwrap();
    assert_eq!(d.werte()["autoren"].as_array().unwrap().len(), 1);
}

#[test]
fn min_items_entfernen_abgelehnt() {
    let f = fachartikel();
    let mut d = Daten::lesen(TEXT).unwrap();
    assert_eq!(
        d.eintrag(&f, &p(&["autoren"]), EintragAktion::Entfernen { index: 0 }),
        Err("mindestens 1 Eintrag".into())
    );
    assert_eq!(d.text(), TEXT);
}

#[test]
fn lesen_fehlerhaft() {
    let e = Daten::lesen("titel = ").err().unwrap();
    assert!(e.contains("Zeile 1"), "{e}");
}

#[test]
fn literatur_dateien_lesen() {
    assert_eq!(
        literatur_dateien("[literatur]\ndateien = [\"a.bib\"]\n"),
        Some(vec!["a.bib".to_string()])
    );
    assert_eq!(literatur_dateien("titel = \"T\"\n"), None);
    assert_eq!(literatur_dateien("kaputt = "), None);
}

#[test]
fn stand_ist_hex() {
    assert_eq!(stand("a"), stand("a"));
    assert_ne!(stand("a"), stand("b"));
    assert!(stand("a").chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn historie() {
    let mut h = Historie::default();
    assert_eq!(h.kann(), (false, false));
    h.geschrieben("A".into(), "B");
    assert_eq!(h.kann(), (true, false));
    assert_eq!(h.rueckgaengig("B"), Some("A".into()));
    assert_eq!(h.kann(), (false, true));
    assert_eq!(h.wiederholen("A"), Some("B".into()));
    // Fremd geändert: nichts zurückdrehen, Historie leeren.
    assert_eq!(h.rueckgaengig("fremd"), None);
    assert_eq!(h.kann(), (false, false));
    for i in 0..101 {
        h.geschrieben(format!("{i}"), &format!("{}", i + 1));
    }
    let mut n = 0;
    let mut jetzt = "101".to_string();
    while let Some(alt) = h.rueckgaengig(&jetzt) {
        jetzt = alt;
        n += 1;
    }
    assert_eq!(n, 100);
}

#[test]
fn historie_neuer_schritt_leert_vor() {
    let mut h = Historie::default();
    h.geschrieben("A".into(), "B");
    h.rueckgaengig("B");
    h.geschrieben("A".into(), "C");
    assert_eq!(h.kann(), (true, false));
}

#[test]
fn atomar_ohne_reste() {
    let o = std::env::temp_dir().join(format!("tftn-formular-atomar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    let datei = o.join("dokument.toml");
    atomar_schreiben(&datei, "a = 1\n").unwrap();
    atomar_schreiben(&datei, "a = 2\n").unwrap();
    assert_eq!(std::fs::read_to_string(&datei).unwrap(), "a = 2\n");
    let namen: Vec<_> = std::fs::read_dir(&o)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(namen.len(), 1, "{namen:?}");
    let _ = std::fs::remove_dir_all(&o);
}

/// Abschlussprüfung: Eine Datei mit CRLF (Windows) behält ihre Zeilenenden.
#[test]
fn crlf_bleibt() {
    let f = fachartikel();
    let text = "titel = \"a\"\r\n# k\r\nfremd = 1\r\n\r\n[[autoren]]\r\nname = \"A\"\r\n";
    let mut d = Daten::lesen(text).unwrap();
    d.setzen(&f, &p(&["titel"]), Some(json!("b")), &K).unwrap();
    assert_eq!(d.text(), text.replace("\"a\"", "\"b\""));
    d.setzen(&f, &p(&["abstract"]), Some(json!("x\ny")), &K)
        .unwrap();
    let neu = d.text();
    assert!(!neu.replace("\r\n", "").contains('\n'), "{neu:?}");
    assert_eq!(Daten::lesen(&neu).unwrap().werte()["abstract"], "x\ny");
}

/// Abschlussprüfung: Leeren eines Felds lässt den Kommentar darüber stehen.
#[test]
fn leeren_behaelt_kommentar() {
    let f = fachartikel();
    // Kommentar über einem Schlüssel mit Nachfolger in derselben Tabelle
    let a = "titel = \"T\"\n# nur für die Zeitschrift\nuntertitel = \"U\"\ndatum = 2026-01-15\n";
    let mut d = Daten::lesen(a).unwrap();
    d.setzen(&f, &p(&["untertitel"]), None, &K).unwrap();
    assert_eq!(
        d.text(),
        "titel = \"T\"\n# nur für die Zeitschrift\ndatum = 2026-01-15\n"
    );
    // Kommentar über dem letzten Schlüssel der Wurzel, danach eine Tabelle
    let b = "titel = \"T\"\n# Notiz\nuntertitel = \"U\"\n\n[[autoren]]\nname = \"A\"\n";
    let mut d = Daten::lesen(b).unwrap();
    d.setzen(&f, &p(&["untertitel"]), None, &K).unwrap();
    assert!(d.text().contains("# Notiz\n"), "{}", d.text());
    assert_eq!(d.werte()["autoren"][0]["name"], "A");
    // Kommentar über dem letzten Schlüssel der Datei
    let c = "titel = \"T\"\n# am Ende\nuntertitel = \"U\"\n";
    let mut d = Daten::lesen(c).unwrap();
    d.setzen(&f, &p(&["untertitel"]), None, &K).unwrap();
    assert!(d.text().contains("# am Ende\n"), "{}", d.text());
    assert!(Daten::lesen(&d.text()).is_ok());
}

/// Ein UTF-8-BOM am Dateianfang bleibt beim Schreiben erhalten.
#[test]
fn bom_bleibt() {
    let f = fachartikel();
    let text = "\u{feff}titel = \"a\"\nfremd = 1\n";
    let mut d = Daten::lesen(text).unwrap();
    assert_eq!(d.werte()["titel"], "a");
    d.setzen(&f, &p(&["titel"]), Some(json!("b")), &K).unwrap();
    assert_eq!(d.text(), "\u{feff}titel = \"b\"\nfremd = 1\n");
}

/// Autoren als Inline-Array (`autoren = [{…}]`): Hinzufügen, Ändern, Verschieben, Entfernen gehen.
#[test]
fn eintraege_als_inline_array() {
    let f = fachartikel();
    let text = "titel = \"T\"\nautoren = [{ name = \"A\" }, { name = \"B\" }]\n";
    let mut d = Daten::lesen(text).unwrap();
    d.eintrag(&f, &p(&["autoren"]), EintragAktion::Hinzufuegen)
        .unwrap();
    assert_eq!(d.werte()["autoren"].as_array().unwrap().len(), 3);
    d.setzen(&f, &p(&["autoren", "2", "name"]), Some(json!("C")), &K)
        .unwrap();
    d.eintrag(
        &f,
        &p(&["autoren"]),
        EintragAktion::Verschieben { index: 2, nach: 0 },
    )
    .unwrap();
    d.eintrag(&f, &p(&["autoren"]), EintragAktion::Entfernen { index: 1 })
        .unwrap();
    let namen: Vec<_> = d.werte()["autoren"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(namen, ["C", "B"]);
    let neu = d.text();
    assert!(neu.starts_with("titel = \"T\"\nautoren = ["), "{neu}");
    assert!(Daten::lesen(&neu).is_ok(), "{neu}");
}
