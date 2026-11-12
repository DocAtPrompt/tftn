//! PLAN-M2 AP6 Task 1: positionale Parameter, Pfad der umschließenden Bausteine, Angebote in einem
//! Bereich, Serde-Formen für die Oberfläche.

use proptest::prelude::*;
use tftn_edit::{
    Farbraum, Ort, PatchFehler, Wert, WidgetArt, WidgetParameter, angebote, angebote_im_bereich,
    anwenden, ebenen, wert_patch,
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

fn parameter() -> Vec<WidgetParameter> {
    vec![
        WidgetParameter {
            funktion: "image".into(),
            parameter: "pfad".into(),
            art: WidgetArt::Datei { endungen: vec![] },
            position: Some(0),
        },
        WidgetParameter {
            funktion: "image".into(),
            parameter: "width".into(),
            art: WidgetArt::Prozent {
                min: None,
                max: None,
            },
            position: None,
        },
        WidgetParameter {
            funktion: "figure".into(),
            parameter: "caption".into(),
            art: WidgetArt::Inhalt { rumpf: false },
            position: None,
        },
        WidgetParameter {
            funktion: "hinweis".into(),
            parameter: "farbe".into(),
            art: WidgetArt::Farbe {
                raum: Farbraum::Rgb,
            },
            position: None,
        },
    ]
}

fn quelle(t: &str) -> Source {
    Source::detached(t.to_string())
}

#[test]
fn positional_lesen_und_schreiben() {
    let t = "#image(\"a.png\", width: 50%)";
    let q = quelle(t);
    let a = angebote(&q, t.find("a.png").unwrap(), &parameter());
    let pfad = a.iter().find(|a| a.parameter == "pfad").unwrap();
    assert_eq!(pfad.gesperrt, None);
    assert_eq!(pfad.wert, Some(Wert::Text("a.png".into())));
    let p = wert_patch(&q, pfad, &pfad.art, &Wert::Text("b.png".into())).unwrap();
    assert_eq!(&t[p.bereich.clone()], "\"a.png\"");
    assert_eq!(anwenden(t, &p), "#image(\"b.png\", width: 50%)");
}

#[test]
fn positional_fehlt_oder_berechnet() {
    for (t, grund) in [
        ("#image(width: 50%)", "fehlt – im Text ergänzen"),
        ("#image(..args)", "berechnet – im Text bearbeiten"),
    ] {
        let q = quelle(t);
        let a = angebote(&q, 2, &parameter());
        let pfad = a.iter().find(|a| a.parameter == "pfad").unwrap();
        assert_eq!(pfad.gesperrt.as_deref(), Some(grund), "{t}");
        assert!(matches!(
            wert_patch(&q, pfad, &pfad.art, &Wert::Text("b.png".into())),
            Err(PatchFehler::Gesperrt(_))
        ));
    }
}

#[test]
fn pfad_verschachtelt() {
    let t = "#figure(image(\"a.png\"), caption: [x])";
    let q = quelle(t);
    let e = ebenen(&q, t.find("a.png").unwrap(), &parameter());
    let namen: Vec<&str> = e.iter().map(|e| e.funktion.as_str()).collect();
    assert_eq!(namen, ["image", "figure"]);
    assert_eq!(&t[e[0].aufruf.clone()], "image(\"a.png\")");
    assert_eq!(e[0].angebote.len(), 2);
    assert_eq!(e[1].angebote.len(), 1);

    let e = ebenen(&q, t.find("x]").unwrap(), &parameter());
    let namen: Vec<&str> = e.iter().map(|e| e.funktion.as_str()).collect();
    assert_eq!(namen, ["figure"]);
    assert_eq!(
        e[0].angebote[0].ort,
        Ort::Ersetzen {
            bereich: t.find("x]").unwrap()..t.find("x]").unwrap() + 1
        }
    );
}

#[test]
fn angebote_im_bereich_findet_alle() {
    let t = "#hinweis(farbe: luma(1))[a] Text #hinweis[b] Text #hinweis(farbe: luma(3))[c]";
    let q = quelle(t);
    let ende = t.find("[b]").unwrap() + 3;
    let a = angebote_im_bereich(&q, 0..ende, &parameter());
    assert_eq!(a.len(), 2);
    assert_eq!(a[0].aufruf.start, 1);
    assert!(a[1].aufruf.start > a[0].aufruf.start);
    let alle = angebote_im_bereich(&q, 0..t.len(), &parameter());
    assert_eq!(alle.len(), 3);
    let keine = angebote_im_bereich(
        &q,
        t.find("Text").unwrap()..t.find("Text").unwrap() + 4,
        &parameter(),
    );
    assert!(keine.is_empty());
}

#[test]
fn serde_formen() {
    let art = WidgetArt::Zahl {
        min: Some(1.0),
        max: None,
        ganzzahl: true,
    };
    let j = serde_json::to_value(&art).unwrap();
    assert_eq!(j["art"], "zahl");
    assert_eq!(j["mit"]["min"], 1.0);
    assert_eq!(j["mit"]["ganzzahl"], true);
    let j = serde_json::to_value(WidgetArt::Schalter).unwrap();
    assert_eq!(j["art"], "schalter");
    let j = serde_json::to_value(WidgetArt::Farbe {
        raum: Farbraum::Cmyk,
    })
    .unwrap();
    assert_eq!(j["mit"]["raum"], "cmyk");

    let w = Wert::Laenge {
        betrag: 1.5,
        einheit: "pt".into(),
    };
    let j = serde_json::to_value(&w).unwrap();
    assert_eq!(
        j,
        serde_json::json!({"art": "laenge", "wert": {"betrag": 1.5, "einheit": "pt"}})
    );
    let zurueck: Wert = serde_json::from_value(j).unwrap();
    assert_eq!(zurueck, w);
    let farbe: Wert =
        serde_json::from_value(serde_json::json!({"art": "farbe", "wert": {"rgb": [1, 2, 3]}}))
            .unwrap();
    assert_eq!(farbe, Wert::Farbe(tftn_edit::Farbe::Rgb([1, 2, 3])));
}

fn fehler(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::Error) + k.children().map(fehler).sum::<usize>()
}

fn umgebung() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text ",
            "Größe ",
            "*fett* ",
            "$x^2$ ",
            "= Titel\n",
            "\n\n",
            "// Kommentar\n",
            "#figure(image(\"z.png\"), caption: [Bild])\n",
            "😀 ",
            "#footnote[Fuß] ",
        ]),
        0..6,
    )
    .prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(konfig())]

    #[test]
    fn p_positional(vor in umgebung(), nach in umgebung(), alt in "[a-z]{1,8}\\.png", neu in "[a-zäöü_/.-]{1,12}") {
        let aufruf = format!("#image(\"{alt}\")");
        let t = format!("{vor}{aufruf}{nach}");
        let q = quelle(&t);
        let byte = vor.len() + 2;
        let a = angebote(&q, byte, &parameter());
        let pfad = a.iter().find(|a| a.parameter == "pfad").unwrap();
        prop_assert_eq!(pfad.wert.clone(), Some(Wert::Text(alt.clone())));
        let p = wert_patch(&q, pfad, &pfad.art, &Wert::Text(neu.clone())).unwrap();
        let n = anwenden(&t, &p);
        prop_assert_eq!(&n[..p.bereich.start], &t[..p.bereich.start]);
        prop_assert_eq!(&n[p.bereich.start + p.text.len()..], &t[p.bereich.end..]);
        let nq = quelle(&n);
        prop_assert!(fehler(nq.root()) <= fehler(q.root()));
        let wieder = angebote(&nq, byte, &parameter());
        let pfad = wieder.iter().find(|a| a.parameter == "pfad").unwrap();
        prop_assert_eq!(pfad.wert.clone(), Some(Wert::Text(neu)));
    }
}
