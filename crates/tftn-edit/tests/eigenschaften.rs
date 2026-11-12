//! Eigenschaftstests P3–P6 (PLAN-M1 4.1) für das Farb-Widget und alle Literal-Arten (PLAN-M2 AP4).
//! Standard 1 000 Fälle je Eigenschaft; mehr mit `PROPTEST_CASES=100000`.

use proptest::prelude::*;
use tftn_edit::{
    Farbe, Farbraum, Ort, PatchFehler, Wert, WidgetArt, WidgetParameter, angebote, anwenden,
    farbe_lesen, farbe_literal, farbe_patch, schreiben, wert_patch,
};
use typst_syntax::{Source, SyntaxKind, SyntaxNode};

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

fn hinweis() -> Vec<WidgetParameter> {
    vec![WidgetParameter {
        position: None,
        funktion: "hinweis".into(),
        parameter: "farbe".into(),
        art: WidgetArt::Farbe {
            raum: Farbraum::Rgb,
        },
    }]
}

/// Umgebung aus gültigen Typst-Schnipseln (Markup, Code, Mathe, Kommentare, Umlaute).
fn umgebung() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text ",
            "Größe und Maß ",
            "*fett* ",
            "_kursiv_ ",
            "$x^2 + y$ ",
            "#text(fill: red)[rot] ",
            "= Titel\n",
            "\n\n",
            "// Kommentar\n",
            "#hinweis(farbe: luma(100))[anderer Kasten] ",
            "#let farbe = blue\n",
            "`raw` ",
            "„Zitat“ ",
            "😀 ",
            "- Punkt\n",
            "#footnote[Fuß] ",
        ]),
        0..12,
    )
    .prop_map(|v| v.concat())
}

/// Der Aufruf in allen Schreibweisen des Arguments; `ZIEL` markiert den Inhalt.
fn aufruf() -> impl Strategy<Value = String> {
    let hex = "[0-9a-fA-F]{6}";
    prop_oneof![
        hex.prop_map(|h| format!("#hinweis(farbe: rgb(\"#{h}\"))[ZIEL]")),
        hex.prop_map(|h| format!("#hinweis(art: \"merke\", farbe: rgb(\"#{h}\"))[ZIEL *x*]")),
        "[0-9a-f]{3}".prop_map(|h| format!("#hinweis(farbe: rgb(\"#{h}\"), art: \"info\")[ZIEL]")),
        (0u8..=255).prop_map(|l| format!("#hinweis(farbe: luma({l}))[ZIEL]")),
        (0u8..=255, 0u8..=255, 0u8..=255)
            .prop_map(|(r, g, b)| format!("#hinweis(farbe: rgb({r}, {g}, {b}))[ZIEL]")),
        Just("#hinweis[ZIEL]".to_string()),
        Just("#hinweis()[ZIEL]".to_string()),
        Just("#hinweis(art: \"warnung\")[ZIEL]".to_string()),
    ]
}

fn fall() -> impl Strategy<Value = (String, String)> {
    (umgebung(), aufruf(), umgebung(), "[0-9a-f]{6}")
        .prop_map(|(v, a, n, hex)| (format!("{v}{a}{n}"), hex))
}

fn fehler(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::Error) + k.children().map(fehler).sum::<usize>()
}

proptest! {
    #![proptest_config(konfig())]

    /// P3: Außerhalb des Patch-Bereichs ändert sich kein Byte.
    #[test]
    fn p3_minimaler_patch((text, hex) in fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &hinweis());
        prop_assert_eq!(a.len(), 1);
        let p = farbe_patch(&q, &a[0], &hex).unwrap();
        let neu = anwenden(&text, &p);
        prop_assert_eq!(&neu[..p.bereich.start], &text[..p.bereich.start]);
        prop_assert_eq!(&neu[p.bereich.start + p.text.len()..], &text[p.bereich.end..]);
        if let Ort::Ersetzen { bereich } = &a[0].ort {
            prop_assert_eq!(&p.bereich, bereich);
        }
    }

    /// P4: Struktur gleich bis auf das Literal; kein Fehlerknoten; der Wert ist danach gesetzt.
    #[test]
    fn p4_strukturgleich((text, hex) in fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &hinweis());
        let p = farbe_patch(&q, &a[0], &hex).unwrap();
        let neu = Source::detached(anwenden(&text, &p));
        prop_assert_eq!(fehler(neu.root()), fehler(q.root()));
        let b = angebote(&neu, neu.text().find("ZIEL").unwrap(), &hinweis());
        prop_assert_eq!(b.len(), 1);
        prop_assert_eq!(b[0].wert.as_ref().and_then(|w| match w { Wert::Farbe(f) => f.hex(), _ => None }), Some(hex.clone()));
        // Alle anderen Aufrufe unverändert: gleiche Anzahl FuncCall-Knoten.
        fn aufrufe(k: &SyntaxNode) -> usize {
            usize::from(k.kind() == SyntaxKind::FuncCall) + k.children().map(aufrufe).sum::<usize>()
        }
        let ergaenzt = usize::from(matches!(a[0].ort, Ort::Einfuegen { .. }));
        prop_assert_eq!(aufrufe(neu.root()), aufrufe(q.root()) + ergaenzt);
    }

    /// P5: parse(render(farbe)) == farbe.
    #[test]
    fn p5_literal_round_trip(hex in "[0-9a-fA-F]{6}") {
        let literal = farbe_literal(&hex).unwrap();
        let q = Source::detached(format!("#{literal}"));
        let aufruf = q.root().children().find(|k| k.kind() == SyntaxKind::FuncCall).unwrap();
        prop_assert_eq!(farbe_lesen(aufruf), Some(hex.to_ascii_lowercase()));
    }

    /// P6: Liegt das Literal auf einer Zeile, ändert der Patch genau eine Zeile.
    #[test]
    fn p6_zeilentreue((text, hex) in fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &hinweis());
        if let Some(Wert::Farbe(alt)) = &a[0].wert {
            prop_assume!(alt.hex().as_deref() != Some(hex.as_str()));
        }
        let neu = anwenden(&text, &farbe_patch(&q, &a[0], &hex).unwrap());
        let alt_z: Vec<&str> = text.split('\n').collect();
        let neu_z: Vec<&str> = neu.split('\n').collect();
        prop_assert_eq!(alt_z.len(), neu_z.len());
        let geaendert = alt_z.iter().zip(&neu_z).filter(|(x, y)| x != y).count();
        prop_assert_eq!(geaendert, 1);
    }
}

// --- Alle Literal-Arten am Testbaustein `kasten` (PLAN-M2 AP4, Task 3) ------------------------

fn kasten_art(parameter: &str) -> WidgetArt {
    match parameter {
        "farbe" => WidgetArt::Farbe {
            raum: Farbraum::Rgb,
        },
        "art" => WidgetArt::Wahl {
            werte: vec!["info".into(), "warnung".into(), "merke".into()],
        },
        "titel" => WidgetArt::Text { mehrzeilig: false },
        "breite" => WidgetArt::Laenge { einheiten: vec![] },
        "anteil" => WidgetArt::Prozent {
            min: Some(0.0),
            max: Some(100.0),
        },
        "sichtbar" => WidgetArt::Schalter,
        "am" => WidgetArt::Datum,
        "ziel" => WidgetArt::Label,
        _ => unreachable!(),
    }
}

fn kasten(parameter: &str) -> Vec<WidgetParameter> {
    vec![WidgetParameter {
        position: None,
        funktion: "kasten".into(),
        parameter: parameter.into(),
        art: kasten_art(parameter),
    }]
}

/// Zufälliger gültiger Wert für einen Parameter von `kasten`.
fn kasten_wert(parameter: &'static str) -> BoxedStrategy<Wert> {
    match parameter {
        "farbe" => any::<[u8; 3]>()
            .prop_map(|c| Wert::Farbe(Farbe::Rgb(c)))
            .boxed(),
        "art" => proptest::sample::select(vec!["info", "warnung", "merke"])
            .prop_map(|w| Wert::Text(w.into()))
            .boxed(),
        "titel" => "[a-zäöüß „“\"\\\\]{0,10}".prop_map(Wert::Text).boxed(),
        "breite" => (
            -100f64..100.0,
            proptest::sample::select(vec!["pt", "mm", "cm", "in", "em"]),
        )
            .prop_map(|(b, e)| Wert::Laenge {
                betrag: b,
                einheit: e.into(),
            })
            .boxed(),
        "anteil" => (0f64..100.0).prop_map(Wert::Prozent).boxed(),
        "sichtbar" => any::<bool>().prop_map(Wert::Schalter).boxed(),
        "am" => (1900i32..2100, 1u8..=12, 1u8..=28)
            .prop_map(|(jahr, monat, tag)| Wert::Datum { jahr, monat, tag })
            .boxed(),
        "ziel" => "[a-z][a-z0-9-]{0,6}".prop_map(Wert::Label).boxed(),
        _ => unreachable!(),
    }
}

/// (Text mit `ZIEL` im Rumpf, Parameter, neuer Wert)
fn kasten_fall() -> impl Strategy<Value = (String, &'static str, Wert)> {
    let parameter = proptest::sample::select(vec![
        "farbe", "art", "titel", "breite", "anteil", "sichtbar", "am", "ziel",
    ]);
    parameter.prop_flat_map(|p| {
        let aufruf = prop_oneof![
            kasten_wert(p).prop_map(move |alt| {
                let l = schreiben(&kasten_art(p), &alt).unwrap();
                format!("#kasten({p}: {l})[ZIEL]")
            }),
            kasten_wert(p).prop_map(move |alt| {
                let l = schreiben(&kasten_art(p), &alt).unwrap();
                format!("#kasten(nr: 3, {p}: {l})[ZIEL *x*]")
            }),
            Just("#kasten[ZIEL]".to_string()),
            Just("#kasten()[ZIEL]".to_string()),
            Just("#kasten(nr: 3)[ZIEL]".to_string()),
        ];
        (umgebung(), aufruf, umgebung(), kasten_wert(p))
            .prop_map(move |(v, a, n, neu)| (format!("{v}{a}{n}"), p, neu))
    })
}

fn aufrufe(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::FuncCall) + k.children().map(aufrufe).sum::<usize>()
}

proptest! {
    #![proptest_config(konfig())]

    /// P3 für alle Literal-Arten: außerhalb des Bereichs kein Byte geändert.
    #[test]
    fn p3_alle_arten((text, p, neu) in kasten_fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &kasten(p));
        prop_assert_eq!(a.len(), 1);
        let patch = wert_patch(&q, &a[0], &kasten_art(p), &neu).unwrap();
        let ergebnis = anwenden(&text, &patch);
        prop_assert_eq!(&ergebnis[..patch.bereich.start], &text[..patch.bereich.start]);
        prop_assert_eq!(&ergebnis[patch.bereich.start + patch.text.len()..], &text[patch.bereich.end..]);
    }

    /// P4 für alle Literal-Arten: kein Fehlerknoten, der Wert ist danach gesetzt, Aufrufe gleich.
    #[test]
    fn p4_alle_arten((text, p, neu) in kasten_fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &kasten(p));
        let patch = wert_patch(&q, &a[0], &kasten_art(p), &neu).unwrap();
        let n = Source::detached(anwenden(&text, &patch));
        prop_assert_eq!(fehler(n.root()), fehler(q.root()));
        let b = angebote(&n, n.text().find("ZIEL").unwrap(), &kasten(p));
        prop_assert_eq!(b.len(), 1);
        prop_assert_eq!(b[0].wert.as_ref(), Some(&neu));
        let ergaenzt = matches!(a[0].ort, Ort::Einfuegen { .. }) && matches!(p, "farbe" | "am");
        prop_assert_eq!(aufrufe(n.root()), aufrufe(q.root()) + usize::from(ergaenzt));
    }

    /// P6 für alle Literal-Arten: genau eine Zeile ändert sich.
    #[test]
    fn p6_alle_arten((text, p, neu) in kasten_fall()) {
        let q = Source::detached(text.clone());
        let a = angebote(&q, text.find("ZIEL").unwrap(), &kasten(p));
        prop_assume!(a[0].wert.as_ref() != Some(&neu));
        let ergebnis = anwenden(&text, &wert_patch(&q, &a[0], &kasten_art(p), &neu).unwrap());
        let alt_z: Vec<&str> = text.split('\n').collect();
        let neu_z: Vec<&str> = ergebnis.split('\n').collect();
        prop_assert_eq!(alt_z.len(), neu_z.len());
        prop_assert_eq!(alt_z.iter().zip(&neu_z).filter(|(x, y)| x != y).count(), 1);
    }
}

#[test]
fn berechnete_werte_bleiben_gesperrt() {
    let t = "#kasten(breite: 1em + 2pt)[x]";
    let q = Source::detached(t);
    let a = &angebote(&q, t.find('x').unwrap(), &kasten("breite"))[0];
    assert!(a.gesperrt.is_some());
    let neu = Wert::Laenge {
        betrag: 1.0,
        einheit: "pt".into(),
    };
    assert!(matches!(
        wert_patch(&q, a, &kasten_art("breite"), &neu),
        Err(PatchFehler::Gesperrt(_))
    ));
}
