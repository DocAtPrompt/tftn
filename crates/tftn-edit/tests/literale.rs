//! Widget-Arten lesen und schreiben (PLAN-M2 AP4, Task 2): P5 für alle Arten und die Ränder.
//! Standard 1 000 Fälle; mehr mit `PROPTEST_CASES=20000`.

use proptest::prelude::*;
use tftn_edit::{Farbe, Farbraum, PatchFehler, Wert, WidgetArt, lesen, schreiben};
use typst_syntax::{SyntaxNode, parse_code};

fn konfig() -> ProptestConfig {
    let faelle = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    ProptestConfig {
        cases: faelle,
        ..ProptestConfig::default()
    }
}

/// Der Ausdruck in `code` (erster Knoten, der kein Leerraum ist).
fn ausdruck(code: &str) -> SyntaxNode {
    let wurzel = parse_code(code);
    assert!(
        wurzel.errors_and_warnings().0.is_empty(),
        "{code:?} parst nicht"
    );
    wurzel
        .children()
        .find(|k| !k.kind().is_trivia())
        .cloned()
        .unwrap_or_else(|| panic!("kein Ausdruck in {code:?}"))
}

fn zurueck(art: &WidgetArt, code: &str) -> Option<Wert> {
    lesen(art, &ausdruck(code))
}

const TEXT: WidgetArt = WidgetArt::Text { mehrzeilig: true };
const FLOAT: WidgetArt = WidgetArt::Zahl {
    min: None,
    max: None,
    ganzzahl: false,
};
const GANZ: WidgetArt = WidgetArt::Zahl {
    min: None,
    max: None,
    ganzzahl: true,
};

fn laenge() -> WidgetArt {
    WidgetArt::Laenge { einheiten: vec![] }
}

fn art_und_wert() -> impl Strategy<Value = (WidgetArt, Wert)> {
    let einheit = proptest::sample::select(vec!["pt", "mm", "cm", "in", "em"]);
    let laenge_wert = (-1e6f64..1e6, einheit.clone()).prop_map(|(b, e)| Wert::Laenge {
        betrag: b,
        einheit: e.into(),
    });
    prop_oneof![
        "[\\x00-\\x7f äöüß„“😀\"\\\\\n\t]{0,20}".prop_map(|t| (TEXT, Wert::Text(t))),
        (-1e15f64..1e15).prop_map(|z| (FLOAT, Wert::Zahl(z))),
        (-1_000_000_000_000_000i64..1_000_000_000_000_000)
            .prop_map(|z| (GANZ, Wert::Zahl(z as f64))),
        laenge_wert.clone().prop_map(|w| (laenge(), w)),
        (-1000f64..1000.0).prop_map(|p| (
            WidgetArt::Prozent {
                min: None,
                max: None
            },
            Wert::Prozent(p)
        )),
        any::<bool>().prop_map(|b| (WidgetArt::Schalter, Wert::Schalter(b))),
        any::<[u8; 3]>().prop_map(|c| (
            WidgetArt::Farbe {
                raum: Farbraum::Rgb
            },
            Wert::Farbe(Farbe::Rgb(c))
        )),
        [0u8..=100, 0u8..=100, 0u8..=100, 0u8..=100].prop_map(|c| (
            WidgetArt::Farbe {
                raum: Farbraum::Cmyk
            },
            Wert::Farbe(Farbe::Cmyk(c))
        )),
        (1i32..=9999, 1u8..=12, 1u8..=28)
            .prop_map(|(jahr, monat, tag)| (WidgetArt::Datum, Wert::Datum { jahr, monat, tag })),
        "[a-z][a-z0-9_-]{0,10}".prop_map(|l| (WidgetArt::Label, Wert::Label(l))),
        (laenge_wert.clone(), laenge_wert)
            .prop_map(|(a, b)| (WidgetArt::Punkt, Wert::Punkt(vec![a, b]))),
        proptest::collection::vec("[a-zäö \"]{0,6}".prop_map(Wert::Text), 0..4).prop_map(|v| (
            WidgetArt::Liste {
                element: Box::new(TEXT)
            },
            Wert::Liste(v)
        )),
    ]
}

proptest! {
    #![proptest_config(konfig())]

    /// P5: lesen(parse(schreiben(w))) == w für alle Arten.
    #[test]
    fn p5_round_trip_alle_arten((art, wert) in art_und_wert()) {
        let code = schreiben(&art, &wert).unwrap();
        prop_assert_eq!(zurueck(&art, &code), Some(wert), "{}", code);
    }
}

#[test]
fn zahlen_an_den_raendern() {
    // Ganze Werte als Int: Typst nimmt Int auch für Float-Parameter, umgekehrt nicht.
    assert_eq!(schreiben(&FLOAT, &Wert::Zahl(3.0)).unwrap(), "3");
    assert_eq!(schreiben(&FLOAT, &Wert::Zahl(-2.0)).unwrap(), "-2");
    let punkt = Wert::Punkt(vec![Wert::Zahl(1.0), Wert::Zahl(2.0)]);
    assert_eq!(schreiben(&WidgetArt::Punkt, &punkt).unwrap(), "(1, 2)");
    assert_eq!(schreiben(&FLOAT, &Wert::Zahl(-0.5)).unwrap(), "-0.5");
    assert_eq!(schreiben(&FLOAT, &Wert::Zahl(1e-7)).unwrap(), "0.0000001");
    assert_eq!(schreiben(&GANZ, &Wert::Zahl(12.0)).unwrap(), "12");
    assert!(matches!(
        schreiben(&FLOAT, &Wert::Zahl(2e15)),
        Err(PatchFehler::UngueltigerWert(_))
    ));
    assert!(matches!(
        schreiben(&FLOAT, &Wert::Zahl(f64::NAN)),
        Err(PatchFehler::UngueltigerWert(_))
    ));
    assert!(matches!(
        schreiben(&GANZ, &Wert::Zahl(1.5)),
        Err(PatchFehler::UngueltigerWert(_))
    ));
    assert_eq!(zurueck(&FLOAT, "1"), Some(Wert::Zahl(1.0)));
    let l = Wert::Laenge {
        betrag: 12.0,
        einheit: "pt".into(),
    };
    assert_eq!(schreiben(&laenge(), &l).unwrap(), "12pt");
    let p = WidgetArt::Prozent {
        min: Some(0.0),
        max: Some(100.0),
    };
    assert_eq!(schreiben(&p, &Wert::Prozent(70.0)).unwrap(), "70%");
    assert!(matches!(
        schreiben(&p, &Wert::Prozent(101.0)),
        Err(PatchFehler::UngueltigerWert(_))
    ));
}

#[test]
fn arrays_mit_einem_und_ohne_element() {
    let art = WidgetArt::Liste {
        element: Box::new(TEXT),
    };
    let eins = Wert::Liste(vec![Wert::Text("a".into())]);
    assert_eq!(schreiben(&art, &eins).unwrap(), "(\"a\",)");
    assert_eq!(zurueck(&art, "(\"a\",)"), Some(eins));
    assert_eq!(schreiben(&art, &Wert::Liste(vec![])).unwrap(), "()");
    assert_eq!(zurueck(&art, "()"), Some(Wert::Liste(vec![])));
    // `("a")` ist kein Array, sondern ein geklammerter String.
    assert_eq!(zurueck(&art, "(\"a\")"), None);
}

#[test]
fn liest_alte_farbformen() {
    let art = WidgetArt::Farbe {
        raum: Farbraum::Rgb,
    };
    assert_eq!(
        zurueck(&art, "rgb(\"#abc\")"),
        Some(Wert::Farbe(Farbe::Rgb([0xaa, 0xbb, 0xcc])))
    );
    assert_eq!(
        zurueck(&art, "rgb(1, 2, 3)"),
        Some(Wert::Farbe(Farbe::Rgb([1, 2, 3])))
    );
    assert_eq!(
        zurueck(&art, "luma(230)"),
        Some(Wert::Farbe(Farbe::Luma(230)))
    );
    assert_eq!(Farbe::Luma(230).hex().as_deref(), Some("e6e6e6"));
    assert_eq!(
        zurueck(&art, "cmyk(10%, 20%, 30%, 40%)"),
        Some(Wert::Farbe(Farbe::Cmyk([10, 20, 30, 40])))
    );
    assert_eq!(Farbe::Cmyk([1, 2, 3, 4]).hex(), None);
}

#[test]
fn berechnet_wird_nicht_gelesen() {
    assert_eq!(zurueck(&laenge(), "1em + 2pt"), None);
    assert_eq!(zurueck(&TEXT, "x"), None);
    assert_eq!(zurueck(&TEXT, "\"a\" + \"b\""), None);
    assert_eq!(
        zurueck(
            &WidgetArt::Farbe {
                raum: Farbraum::Rgb
            },
            "rgb(\"#abcd\")"
        ),
        None
    );
    assert_eq!(zurueck(&WidgetArt::Tabelle, "table()"), None);
}

#[test]
fn pruefungen_beim_schreiben() {
    let ungueltig = |art: WidgetArt, wert: Wert| {
        assert!(
            matches!(schreiben(&art, &wert), Err(PatchFehler::UngueltigerWert(_))),
            "{art:?} {wert:?}"
        );
    };
    ungueltig(
        WidgetArt::Wahl {
            werte: vec!["info".into()],
        },
        Wert::Text("x".into()),
    );
    ungueltig(
        WidgetArt::Datei {
            endungen: vec!["png".into()],
        },
        Wert::Text("a.jpg".into()),
    );
    ungueltig(
        WidgetArt::Datum,
        Wert::Datum {
            jahr: 2026,
            monat: 2,
            tag: 30,
        },
    );
    ungueltig(WidgetArt::Label, Wert::Label("mit leer".into()));
    ungueltig(
        WidgetArt::Text { mehrzeilig: false },
        Wert::Text("a\nb".into()),
    );
    ungueltig(
        WidgetArt::Laenge {
            einheiten: vec!["pt".into()],
        },
        Wert::Laenge {
            betrag: 1.0,
            einheit: "mm".into(),
        },
    );
    assert!(
        schreiben(
            &WidgetArt::Datei {
                endungen: vec!["png".into()]
            },
            &Wert::Text("b/A.PNG".into())
        )
        .is_ok()
    );
    assert!(matches!(
        schreiben(&WidgetArt::Tabelle, &Wert::Text("x".into())),
        Err(PatchFehler::Gesperrt(_))
    ));
}

#[test]
fn inhalt_liest_und_schreibt_markup() {
    let art = WidgetArt::Inhalt { rumpf: false };
    assert_eq!(
        zurueck(&art, "[Text *fett*]"),
        Some(Wert::Inhalt("Text *fett*".into()))
    );
    assert_eq!(
        schreiben(&art, &Wert::Inhalt("Text *fett*".into())).unwrap(),
        "[Text *fett*]"
    );
}

/// Abschlussprüfung AP4: Zeilentrenner U+2028/U+2029 sind auch Umbrüche; Typst kennt Jahre bis 9999.
#[test]
fn zeilentrenner_und_jahr_ueber_9999() {
    let ungueltig = |art: WidgetArt, wert: Wert| {
        assert!(
            matches!(schreiben(&art, &wert), Err(PatchFehler::UngueltigerWert(_))),
            "{wert:?} gilt als gültig"
        );
    };
    for t in ["a\u{2028}b", "a\u{2029}b", "a\u{85}b"] {
        ungueltig(WidgetArt::Text { mehrzeilig: false }, Wert::Text(t.into()));
    }
    ungueltig(
        WidgetArt::Datum,
        Wert::Datum {
            jahr: 10000,
            monat: 1,
            tag: 1,
        },
    );
    ungueltig(
        WidgetArt::Datum,
        Wert::Datum {
            jahr: -1,
            monat: 1,
            tag: 1,
        },
    );
}
