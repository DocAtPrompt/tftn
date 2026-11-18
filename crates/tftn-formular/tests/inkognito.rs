//! Inkognito: Ersatztexte für KI-Assistenten und genaues Zurücksetzen.
//! Alle Daten sind erfunden.

use proptest::prelude::*;
use serde_json::json;
use tftn_formular::formular_lesen;
use tftn_formular::inkognito::*;

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "patient": {
      "type": "object",
      "properties": {
        "name": { "type": "string", "title": "Name", "x-tftn-sensibel": true,
          "x-tftn-ersatz": { "nach": "patient.geschlecht",
            "je": { "weiblich": "Patientin", "männlich": "Patient" }, "sonst": "Patient/in" } },
        "geschlecht": { "type": "string", "title": "Geschlecht", "enum": ["weiblich", "männlich", "divers"] },
        "geburtsdatum": { "type": "string", "title": "Geburtsdatum", "format": "date",
          "x-tftn-sensibel": true, "x-tftn-ersatz": "geb. {jahr}" },
        "svnr": { "type": "string", "title": "Sozialversicherungsnummer", "x-tftn-sensibel": true,
          "x-tftn-ersatz": "SV-Nummer" }
      }
    },
    "empfaenger": { "type": "array", "title": "Empfänger", "items": { "type": "string" } },
    "ort": { "type": "string", "title": "Ort" }
  }
}"#;

fn daten() -> serde_json::Value {
    json!({
        "patient": { "name": "Erika Musterfrau", "geschlecht": "weiblich",
                     "geburtsdatum": "1970-02-01", "svnr": "1234 010270" },
        "empfaenger": ["Frau", "Dr. Hanna Beispielärztin", "Probegasse 3", "1010 Wien"],
        "ort": "Wien"
    })
}

fn tabelle(m: &Markierungen) -> (Vec<Regel>, Tabelle) {
    let f = formular_lesen(SCHEMA).unwrap();
    let r = regeln(&f, m);
    let mut t = Tabelle::default();
    t.werte_setzen(&werte(&r, &[daten()])).unwrap();
    t.regeln_merken(&r);
    (r, t)
}

#[test]
fn sensible_felder_sind_von_haus_aus_inkognito() {
    let f = formular_lesen(SCHEMA).unwrap();
    let r = regeln(&f, &Markierungen::new());
    let namen: Vec<String> = r.iter().map(Regel::schluessel).collect();
    assert_eq!(
        namen,
        ["patient.name", "patient.geburtsdatum", "patient.svnr"]
    );
    assert!(r[1].datum);
}

#[test]
fn markierungen_im_projekt_gehen_vor() {
    let f = formular_lesen(SCHEMA).unwrap();
    let m = markierungen_lesen(
        "tftn = 1\n[inkognito]\n\"patient.svnr\" = false\nempfaenger = true\n\"patient.name\" = \"[Frau X]\"\n",
    );
    assert_eq!(
        m.get("patient.name"),
        Some(&Markierung::Text("Frau X".into()))
    );
    let r = regeln(&f, &m);
    let namen: Vec<String> = r.iter().map(Regel::schluessel).collect();
    assert_eq!(
        namen,
        ["patient.name", "patient.geburtsdatum", "empfaenger"]
    );
    let w = werte(&r, &[daten()]);
    assert_eq!(
        w[0].basis, "Frau X",
        "eigener Ersatz statt dem nach Geschlecht"
    );
    // Empfänger ohne Ersatz der Vorlage: die Feldbeschriftung, je Zeile
    assert!(
        w.iter()
            .any(|w| w.basis == "Empfänger, Zeile 2" && w.echt == "Dr. Hanna Beispielärztin")
    );

    let st = feld_staende(&f, &m);
    assert!(!st["patient.svnr"].an && st["patient.svnr"].sensibel);
    assert_eq!(st["patient.name"].eigen.as_deref(), Some("Frau X"));
    assert_eq!(
        st["patient.name"].vorgabe,
        "Patientin / Patient / Patient/in"
    );
    assert!(!st["ort"].an);
}

#[test]
fn markierung_schreiben_laesst_den_rest_stehen() {
    let alt = "# Projekt\ntftn = 1\ntypst = \"0.14.2\"\n";
    let t = markierung_schreiben(
        alt,
        "patient.name",
        Some(&Markierung::Text("Patientin".into())),
    )
    .unwrap();
    let t = markierung_schreiben(&t, "ort", Some(&Markierung::An)).unwrap();
    assert!(t.starts_with(alt), "{t}");
    let m = markierungen_lesen(&t);
    assert_eq!(m.len(), 2);
    assert_eq!(m["ort"], Markierung::An);
    let t = markierung_schreiben(&t, "ort", None).unwrap();
    let t = markierung_schreiben(&t, "patient.name", None).unwrap();
    assert_eq!(
        t.trim_end(),
        alt.trim_end(),
        "leerer Abschnitt verschwindet"
    );
    assert!(markierung_schreiben(alt, "x", Some(&Markierung::Text(" [ ] ".into()))).is_err());
}

#[test]
fn ersatz_folgt_dem_geschlecht_und_nennt_das_jahr() {
    let (r, _) = tabelle(&Markierungen::new());
    let mut d = daten();
    let w = werte(&r, &[d.clone()]);
    assert_eq!(w[0].basis, "Patientin");
    assert_eq!(w[1].basis, "geb. 1970");
    assert_eq!(w[2].basis, "SV-Nummer");
    assert_eq!(w[2].art, WertArt::Nummer);
    d["patient"]["geschlecht"] = json!("divers");
    assert_eq!(werte(&r, &[d.clone()])[0].basis, "Patient/in");
    d["patient"]["geschlecht"] = json!("männlich");
    assert_eq!(werte(&r, &[d])[0].basis, "Patient");
}

#[test]
fn text_wird_verborgen_und_genau_zurueckgesetzt() {
    let (_, mut t) = tabelle(&Markierungen::new());
    let echt = "Befund für Erika Musterfrau, geb. 01.02.1970 (SVNr 1234010270).\n\
                Frau MUSTERFRAU kam am 1. Februar 1970 zur Welt; Musterfraus Werte sind gut.\n\
                In Amerika lebt Erika~Musterfrau nicht. Datum: 1970-02-01, Erika.";
    let p = t.verbergen(echt);
    for verboten in [
        "Musterfrau",
        "MUSTERFRAU",
        "02.1970",
        "Februar",
        "-02-",
        "010270",
        "Erika ",
    ] {
        assert!(!p.contains(verboten), "{verboten} in {p}");
    }
    assert!(p.contains("Befund für [Patientin], geb. [geb. 1970"), "{p}");
    assert!(
        p.contains("[Patientin, Teil 2 · Schreibweise 2]s Werte"),
        "Genitiv-s bleibt: {p}"
    );
    assert!(p.contains("Frau [Patientin, Teil 2] kam"), "{p}");
    assert!(p.contains("In Amerika lebt"), "kein Teil eines Worts: {p}");
    assert!(p.contains("[Patientin, Teil 1]."), "{p}");
    assert_eq!(t.aufdecken(&p).unwrap(), echt);

    // Dieselbe Schreibweise behält ihren Platzhalter.
    assert_eq!(t.verbergen("Erika Musterfrau"), "[Patientin]");
}

#[test]
fn neuer_text_der_ki_bekommt_die_echten_werte() {
    let (_, mut t) = tabelle(&Markierungen::new());
    t.verbergen("Erika Musterfrau, 1970-02-01");
    let neu = t
        .aufdecken("Die [Patientin] ([geb. 1970]) wirkt gesund.")
        .unwrap();
    assert_eq!(neu, "Die Erika Musterfrau (1970-02-01) wirkt gesund.");
    let e = t.aufdecken("Die [Patientin 7] ist da.").unwrap_err();
    assert!(e.contains("„Patientin 7“"), "{e}");
    // Die Meldung läuft selbst durch den Filter; danach ist der Platzhalter nicht „wörtlich“.
    t.verbergen(&e);
    assert!(t.aufdecken("Die [Patientin 7] ist da.").is_err());
    for falsch in [
        "[patientin]",
        "[Patient]",
        "[geb. 1971]",
        "[Patientin, Teil 9]",
        "[SV-Nummer 2]",
    ] {
        assert!(t.aufdecken(falsch).is_err(), "{falsch} durchgelassen");
    }
    // Eckige Klammern in Typst bleiben, was sie sind.
    assert_eq!(
        t.aufdecken("#strong[[Patientin]]").unwrap(),
        "#strong[Erika Musterfrau]"
    );
    assert_eq!(
        t.aufdecken("#text(red)[Hallo]").unwrap(),
        "#text(red)[Hallo]"
    );
}

#[test]
fn anreden_und_kurze_woerter_bleiben() {
    let (_, mut t) = tabelle(&Markierungen::from([(
        "empfaenger".to_string(),
        Markierung::An,
    )]));
    let p = t.verbergen("Frau Dr. Hanna Beispielärztin, Probegasse 3, 1010 Wien. Wien ist schön.");
    assert!(p.starts_with("Frau [Empfänger, Zeile 2]"), "{p}");
    assert!(p.contains("[Empfänger, Zeile 3]"), "{p}");
    assert!(
        p.contains("[Empfänger, Zeile 4]. Wien ist schön."),
        "Anschrift nicht zerlegt: {p}"
    );
}

#[test]
fn platzhalter_der_schon_im_text_steht_wird_nicht_vergeben() {
    let (_, mut t) = tabelle(&Markierungen::new());
    let echt = "Vorlage: [Patientin]. Name: Erika Musterfrau.";
    let p = t.verbergen(echt);
    assert_eq!(p, "Vorlage: [wörtlich: Patientin]. Name: [Patientin].");
    assert_eq!(
        t.aufdecken(&p).unwrap(),
        echt,
        "wörtliches [Patientin] ist kein Fehler"
    );
}

#[test]
fn mehrere_personen_bekommen_nummern() {
    let f = formular_lesen(SCHEMA).unwrap();
    let r = regeln(&f, &Markierungen::new());
    let mut zweite = daten();
    zweite["patient"]["name"] = json!("Maria Probefrau");
    let mut t = Tabelle::default();
    t.werte_setzen(&werte(&r, &[daten(), zweite])).unwrap();
    assert_eq!(
        t.verbergen("Maria Probefrau und Erika Musterfrau"),
        "[Patientin 2] und [Patientin]"
    );
    assert_eq!(
        t.felder(),
        ["Name", "Geburtsdatum", "Sozialversicherungsnummer"]
    );
}

#[test]
fn datensaetze_einer_serie_in_einer_datei() {
    let s = datensaetze(
        "[[serie]]\ndatei = \"a\"\n[serie.patient]\nname = \"A B\"\n[[serie]]\ndatei = \"b\"\n",
    )
    .unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(s[0]["patient"]["name"], "A B");
    assert_eq!(datensaetze("x = 1").unwrap().len(), 1);
    assert!(datensaetze("x = ").is_err());
}

#[test]
fn toml_fuer_das_seitenbild() {
    let (r, mut t) = tabelle(&Markierungen::new());
    let toml = "anrede = \"Sehr geehrte Frau Musterfrau,\"\n[patient]\nname = \"Erika Musterfrau\"\n\
                geschlecht = \"weiblich\"\ngeburtsdatum = 1970-02-01\nsvnr = \"1234 010270\"\n";
    let p = t.toml_verbergen(toml, &r, false).unwrap();
    assert!(p.contains("name = \"[Patientin]\""), "{p}");
    assert!(
        p.contains("geburtsdatum = 1970-01-01"),
        "Jahr bleibt, weil der Ersatz es nennt: {p}"
    );
    assert!(p.contains("svnr = \"[SV-Nummer]\""), "{p}");
    assert!(p.contains("Frau [Patientin, Teil 2],"), "{p}");
    assert!(p.parse::<toml_edit::DocumentMut>().is_ok());

    // Ohne {jahr} im Ersatz: 1900
    let m = Markierungen::from([(
        "patient.geburtsdatum".to_string(),
        Markierung::Text("Geburtstag".into()),
    )]);
    let (r, mut t) = tabelle(&m);
    let p = t.toml_verbergen(toml, &r, false).unwrap();
    assert!(p.contains("geburtsdatum = 1900-01-01"), "{p}");
    let p = t.toml_verbergen(toml, &r, true).unwrap();
    assert!(
        p.contains("geburtsdatum = \"[Geburtstag]\""),
        "Datum als Text: {p}"
    );

    // Serie in einer Datei
    let serie =
        "[[serie]]\n[serie.patient]\nname = \"Erika Musterfrau\"\ngeburtsdatum = 1970-02-01\n";
    let p = t.toml_verbergen(serie, &r, false).unwrap();
    assert!(!p.contains("Musterfrau") && !p.contains("1970"), "{p}");
}

#[test]
fn json_baum() {
    let (_, mut t) = tabelle(&Markierungen::new());
    let mut j = daten();
    t.verbergen_json(&mut j);
    assert_eq!(j["patient"]["name"], "[Patientin]");
    assert_eq!(j["patient"]["geburtsdatum"], "[geb. 1970]");
    assert_eq!(j["patient"]["geschlecht"], "weiblich");
    t.aufdecken_json(&mut j).unwrap();
    assert_eq!(j, daten());
}

#[test]
fn schema_prueft_x_tftn_ersatz() {
    let mit = |e: &str| {
        formular_lesen(&format!(
            r#"{{"type":"object","properties":{{"a":{{"type":"string","x-tftn-ersatz":{e}}}}}}}"#
        ))
    };
    assert!(mit(r#""Patientin""#).is_ok());
    assert!(mit(r#"{"nach":"b","je":{"x":"Y"},"sonst":"Z"}"#).is_ok());
    assert!(mit(r#""[Patientin]""#).is_err());
    assert!(mit(r#""""#).is_err());
    assert!(mit(r#"{"nach":"b","je":{"x":"Y"}}"#).is_err());
    assert!(mit(r#"3"#).is_err());
    let gruppe = formular_lesen(
        r#"{"type":"object","properties":{"g":{"type":"object","x-tftn-ersatz":"X","properties":{"a":{"type":"string"}}}}}"#,
    );
    assert!(gruppe.is_err());
}

fn stuecke() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(
        prop_oneof![
            Just("Erika Musterfrau".to_string()),
            Just("MUSTERFRAU".to_string()),
            Just("Musterfrauen".to_string()),
            Just("erika".to_string()),
            Just("01.02.1970".to_string()),
            Just("1234 010270".to_string()),
            Just("[Patientin]".to_string()),
            Just("[".to_string()),
            Just("]".to_string()),
            "[a-zäöüß ,.\n~_*-]{0,12}",
        ],
        0..12,
    )
}

proptest! {
    /// Was immer im Text steht: verbergen und aufdecken ergibt ihn Zeichen für Zeichen wieder.
    #[test]
    fn hin_und_zurueck(teile in stuecke()) {
        let (_, mut t) = tabelle(&Markierungen::new());
        let echt: String = teile.concat();
        let p = t.verbergen(&echt);
        prop_assert_eq!(t.aufdecken(&p).unwrap(), echt);
    }

    /// Steht der Name für sich (hier: durch Leerzeichen getrennt), bleibt nichts davon übrig.
    #[test]
    fn nichts_bleibt_uebrig(teile in stuecke()) {
        let (_, mut t) = tabelle(&Markierungen::new());
        let p = t.verbergen(&teile.join(" "));
        prop_assert!(!p.to_lowercase().contains("musterfrau"), "{}", p);
        prop_assert!(!p.contains("010270") && !p.contains("02.1970"), "{}", p);
    }
}

#[test]
fn platzhalter_gleich_nach_dem_setzen_bekannt() {
    let (_, t) = tabelle(&Markierungen::new());
    assert_eq!(
        t.aufdecken("[Patientin], [geb. 1970]").unwrap(),
        "Erika Musterfrau, 1970-02-01"
    );
    assert_eq!(t.beispiele(2), ["[Patientin]", "[geb. 1970]"]);
}

#[test]
fn nummern_trennzeichen_zeilennummern_und_zusammengesetzte_umlaute() {
    let f = formular_lesen(SCHEMA).unwrap();
    let m = Markierungen::from([
        ("ort".to_string(), Markierung::Text("Konto".into())),
        ("empfaenger".to_string(), Markierung::Text("Telefon".into())),
    ]);
    let r = regeln(&f, &m);
    let mut d = daten();
    d["ort"] = json!("AT61 1904 3002 3457 3201");
    d["empfaenger"] = json!(["(0660) 123 45 67"]);
    d["patient"]["name"] = json!("Jörg Müller");
    let mut t = Tabelle::default();
    t.werte_setzen(&werte(&r, &[d])).unwrap();
    let echt = "IBAN AT61 1904 3002 3457 3201, Tel. 0660/1234567, SVNr 1234~010270.\n\
                12\tgeb. am 1.\n13\tFebruar 1970; Ordner Jo\u{308}rg Mu\u{308}ller; \
                #panic: datetime(year: 1970, month: 2, day: 1)";
    let p = t.verbergen(echt);
    for verboten in [
        "3457", "AT61", "1234567", "010270", "Februar", "rg Mu", "ller", "month: 2",
    ] {
        assert!(!p.contains(verboten), "{verboten} in {p}");
    }
    assert_eq!(t.aufdecken(&p).unwrap(), echt);
}

#[test]
fn ersetzen_im_platzhaltertext_trifft_nur_was_die_ki_sah() {
    let (_, mut t) = tabelle(&Markierungen::new());
    let echt = "Amerika. Liebe Erika, Erika Musterfrau.";
    let neu = t
        .im_platzhaltertext(
            echt,
            |p| Ok(p.replace("[Patientin, Teil 1]", "[Patientin]")),
        )
        .unwrap();
    assert_eq!(neu, "Amerika. Liebe Erika Musterfrau, Erika Musterfrau.");
    assert!(t.betrifft("Musterfrau") && !t.betrifft("Wien"));
}

#[test]
fn grenzen_der_erkennung() {
    let (_, mut t) = tabelle(&Markierungen::new());
    assert_eq!(
        t.verbergen("_Musterfrau_ und *Erika*"),
        "_[Patientin, Teil 2]_ und *[Patientin, Teil 1]*"
    );
    assert_eq!(t.verbergen("Musterfrauen"), "[Patientin, Teil 2]en");
    assert_eq!(
        t.verbergen("Musterfrauenhaus Amerika"),
        "Musterfrauenhaus Amerika"
    );
}
