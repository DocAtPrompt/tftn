//! Eigenschaftstest: Werte mit Sonderzeichen überstehen Schreiben und Lesen, und alle anderen
//! Zeilen bleiben byte-gleich (PLAN-M2 AP7 Task 3, Review Focus 1). `PROPTEST_CASES` wirkt.

use proptest::prelude::*;
use serde_json::json;
use tftn_formular::{Daten, Kontext, Schritt, formular_lesen};

const SCHEMA: &str = r#"{ "type": "object", "properties": {
    "titel": { "type": "string" },
    "abstract": { "type": "string", "x-tftn-widget": "markup" } } }"#;

const VORHER: &str = "# Kopf\nvorne = 1 # bleibt\n";
const NACHHER: &str = "\n[[autoren]]\nname = \"X\" # auch\n";

const K: Kontext = Kontext {
    wurzel: None,
    stile: &[],
};

/// Text aus beliebigem Unicode, angereichert mit Zeichen, die TOML-Strings brechen könnten.
fn text(mit_umbruch: bool) -> impl Strategy<Value = String> {
    let besondere = if mit_umbruch {
        prop_oneof![
            Just("\"".to_string()),
            Just("\\".to_string()),
            Just("'''".to_string()),
            Just("\"\"\"".to_string()),
            Just("\n".to_string()),
            Just("\t".to_string()),
            Just("ä€😀".to_string()),
            "\\PC{1,4}",
        ]
        .boxed()
    } else {
        prop_oneof![
            Just("\"".to_string()),
            Just("\\".to_string()),
            Just("'''".to_string()),
            Just("\"\"\"".to_string()),
            Just("\t".to_string()),
            Just("ä€😀".to_string()),
            "\\PC{1,4}",
        ]
        .boxed()
    };
    prop::collection::vec(besondere, 0..12).prop_map(|t| t.concat())
}

/// `crlf`: Die Datei hat Windows-Zeilenenden (Abschlussprüfung AP7); sie bleiben erhalten.
fn pruefen(feld: &str, wert: String, ausgang: &str, crlf: bool) -> Result<(), TestCaseError> {
    let umwandeln = |t: &str| {
        if crlf {
            t.replace('\n', "\r\n")
        } else {
            t.to_string()
        }
    };
    let ausgang = &umwandeln(ausgang);
    let f = formular_lesen(SCHEMA).unwrap();
    let mut d = Daten::lesen(ausgang).unwrap();
    let pfad = vec![Schritt::Name(feld.into())];
    match d.setzen(&f, &pfad, Some(json!(wert)), &K) {
        // Ungültiges Markup darf abgelehnt werden; dann bleibt alles, wie es war.
        Err(_) if feld == "abstract" => {
            prop_assert_eq!(&d.text(), ausgang);
            return Ok(());
        }
        Err(e) => return Err(TestCaseError::fail(format!("abgelehnt: {e}"))),
        Ok(()) => {}
    }
    prop_assert_eq!(&d.werte()[feld], &json!(wert));
    let neu = d.text();
    let wieder = Daten::lesen(&neu).unwrap();
    prop_assert_eq!(&wieder.werte()[feld], &json!(wert));
    prop_assert!(
        neu.starts_with(&umwandeln(VORHER)),
        "Anfang geändert: {neu:?}"
    );
    prop_assert!(neu.ends_with(&umwandeln(NACHHER)), "Ende geändert: {neu:?}");
    if crlf {
        prop_assert!(
            !neu.replace("\r\n", "").contains('\n'),
            "gemischte Zeilenenden: {neu:?}"
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|n| n.parse().ok()).unwrap_or(256)
    ))]

    #[test]
    fn p_setzen_round_trip_titel(wert in text(false), vorhanden in any::<bool>(), crlf in any::<bool>()) {
        let ausgang = if vorhanden {
            format!("{VORHER}titel = \"alt\" # nach titel\n{NACHHER}")
        } else {
            format!("{VORHER}{NACHHER}")
        };
        pruefen("titel", wert, &ausgang, crlf)?;
    }

    #[test]
    fn p_setzen_round_trip_abstract(wert in text(true), vorhanden in any::<bool>(), crlf in any::<bool>()) {
        let ausgang = if vorhanden {
            format!("{VORHER}abstract = \"\"\"\nalt\\\n  weiter\"\"\"\n{NACHHER}")
        } else {
            format!("{VORHER}{NACHHER}")
        };
        pruefen("abstract", wert, &ausgang, crlf)?;
    }
}
