//! Patch-Klasse Inhalt (PLAN-M2 AP4, Task 4; Entwurf 4 „Was danach ändern umfasst“ 1):
//! `caption: […]` und der Rumpf `[…]` werden als Markup geändert, genau innerhalb der Klammern.

use proptest::prelude::*;
use tftn_edit::{
    Ort, PatchFehler, Wert, WidgetArt, WidgetParameter, angebote, anwenden, wert_patch,
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

const CAPTION: WidgetArt = WidgetArt::Inhalt { rumpf: false };
const RUMPF: WidgetArt = WidgetArt::Inhalt { rumpf: true };

fn parameter() -> Vec<WidgetParameter> {
    vec![
        WidgetParameter {
            position: None,
            funktion: "figure".into(),
            parameter: "caption".into(),
            art: CAPTION,
        },
        WidgetParameter {
            position: None,
            funktion: "footnote".into(),
            parameter: "body".into(),
            art: RUMPF,
        },
        WidgetParameter {
            position: None,
            funktion: "hinweis".into(),
            parameter: "body".into(),
            art: RUMPF,
        },
    ]
}

fn art(funktion: &str) -> WidgetArt {
    if funktion == "figure" { CAPTION } else { RUMPF }
}

/// Patch für `neu` am Aufruf, in dem `cursor` steht.
fn patch(text: &str, cursor: &str, neu: &str) -> Result<(String, tftn_edit::Patch), PatchFehler> {
    let q = Source::detached(text);
    let a = angebote(&q, text.find(cursor).unwrap(), &parameter());
    let a = a.first().expect("Angebot");
    let p = wert_patch(&q, a, &art(&a.funktion), &Wert::Inhalt(neu.into()))?;
    Ok((anwenden(text, &p), p))
}

#[test]
fn ersetzt_nur_innerhalb_der_klammern() {
    let t = "#figure(image(\"a.png\"), caption: [Alt])";
    let (neu, p) = patch(t, "Alt", "Neu *fett*").unwrap();
    assert_eq!(neu, "#figure(image(\"a.png\"), caption: [Neu *fett*])");
    assert_eq!(&t[p.bereich.clone()], "Alt");
}

#[test]
fn rumpf_von_footnote_und_hinweis() {
    assert_eq!(
        patch("Satz#footnote[Fuß].", "Fuß", "Neue Fußnote")
            .unwrap()
            .0,
        "Satz#footnote[Neue Fußnote]."
    );
    assert_eq!(
        patch("#hinweis(art: \"info\")[Text]", "Text", "Anderer _Text_")
            .unwrap()
            .0,
        "#hinweis(art: \"info\")[Anderer _Text_]"
    );
}

#[test]
fn fehlende_caption_wird_ergaenzt() {
    let t = "#figure(image(\"a.png\"))";
    // Wie in M1 kommt ein fehlendes benanntes Argument direkt nach `(`.
    assert_eq!(
        patch(t, "a.png", "Neu").unwrap().0,
        "#figure(caption: [Neu], image(\"a.png\"))"
    );
}

#[test]
fn fehlender_rumpf_wird_angehaengt() {
    let t = "#hinweis(art: \"info\") danach";
    let q = Source::detached(t);
    let a = &angebote(&q, t.find("art").unwrap(), &parameter())[0];
    assert!(matches!(a.ort, Ort::Anhaengen { .. }), "{:?}", a.ort);
    let p = wert_patch(&q, a, &RUMPF, &Wert::Inhalt("Neu".into())).unwrap();
    assert_eq!(anwenden(t, &p), "#hinweis(art: \"info\")[Neu] danach");
}

#[test]
fn berechneter_inhalt_ist_gesperrt() {
    let t = "#let c = [x]\n#figure(image(\"a.png\"), caption: c)";
    let q = Source::detached(t);
    let a = &angebote(&q, t.find("a.png").unwrap(), &parameter())[0];
    assert_eq!(
        a.gesperrt.as_deref(),
        Some("berechnet – im Text bearbeiten")
    );
}

/// Steht der Inhalt schon positional in den Klammern (kein Block), würde ein angehängter Rumpf
/// ein zweites Argument; das Angebot ist deshalb gesperrt.
#[test]
fn kein_zweiter_rumpf_neben_positionalem_argument() {
    for (t, cursor) in [
        ("#let t = [Hallo]\n#hinweis(t)", "(t)"),
        ("#footnote(\"x\")", "x"),
    ] {
        let q = Source::detached(t);
        let a = &angebote(&q, t.find(cursor).unwrap() + 1, &parameter())[0];
        assert_eq!(
            a.gesperrt.as_deref(),
            Some("berechnet – im Text bearbeiten"),
            "{t}"
        );
        assert!(wert_patch(&q, a, &RUMPF, &Wert::Inhalt("Neu".into())).is_err());
    }
}

#[test]
fn unausgeglichene_klammer_wird_verworfen() {
    let t = "#footnote[Fuß]";
    for neu in ["a]b", "a[b", "#emph[", "a\\"] {
        let f = patch(t, "Fuß", neu).unwrap_err();
        assert!(
            matches!(
                f,
                PatchFehler::UngueltigerWert(_) | PatchFehler::Invariante(_)
            ),
            "{neu:?}: {f:?}"
        );
    }
}

fn fehler(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::Error) + k.children().map(fehler).sum::<usize>()
}

fn umgebung() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text ",
            "*fett* ",
            "$x^2$ ",
            "= Titel\n",
            "\n\n",
            "// Kommentar\n",
            "#footnote[andere] ",
            "#text(fill: red)[rot] ",
            "😀 ",
            "- Punkt\n",
        ]),
        0..8,
    )
    .prop_map(|v| v.concat())
}

fn markup() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text",
            " ",
            "*fett*",
            "_kursiv_",
            "$x$",
            "#emph[e]",
            "\\]",
            "Größe",
            "😀",
            "\n",
            "„Zitat“",
        ]),
        0..6,
    )
    .prop_map(|v| v.concat())
}

fn aufruf() -> impl Strategy<Value = String> {
    prop_oneof![
        markup().prop_map(|m| format!("#figure(image(\"ZIEL.png\"), caption: [{m}])")),
        Just("#figure(image(\"ZIEL.png\"))".to_string()),
        markup().prop_map(|m| format!("#footnote[ZIEL {m}]")),
        markup().prop_map(|m| format!("#hinweis(art: \"info\")[ZIEL {m}]")),
    ]
}

proptest! {
    #![proptest_config(konfig())]

    /// P3/P4 für Inhalt: kein Byte außerhalb, kein neuer Fehlerknoten, neu gelesen gleich.
    #[test]
    fn p3_p4_inhalt((v, a, n, neu) in (umgebung(), aufruf(), umgebung(), markup())) {
        let text = format!("{v}{a}{n}");
        let q = Source::detached(text.clone());
        let angebot = angebote(&q, text.find("ZIEL").unwrap(), &parameter());
        prop_assert_eq!(angebot.len(), 1);
        let art = art(&angebot[0].funktion);
        // Gültiges Markup wird angenommen, ungültiges abgelehnt (nie angewendet).
        let gueltig = typst_syntax::parse_code(&format!("[{neu}]")).errors_and_warnings().0.is_empty();
        let p = match wert_patch(&q, &angebot[0], &art, &Wert::Inhalt(neu.clone())) {
            Ok(p) => p,
            Err(f) => {
                prop_assert!(!gueltig, "gültiges Markup {:?} abgelehnt: {:?}", neu, f);
                return Ok(());
            }
        };
        prop_assert!(gueltig, "ungültiges Markup {:?} angenommen", neu);
        let ergebnis = anwenden(&text, &p);
        prop_assert_eq!(&ergebnis[..p.bereich.start], &text[..p.bereich.start]);
        prop_assert_eq!(&ergebnis[p.bereich.start + p.text.len()..], &text[p.bereich.end..]);
        let nq = Source::detached(ergebnis.clone());
        prop_assert!(fehler(nq.root()) <= fehler(q.root()));
        let b = angebote(&nq, p.bereich.start.min(ergebnis.len()), &parameter());
        prop_assert!(b.iter().any(|b| b.wert == Some(Wert::Inhalt(neu.clone()))), "{}", ergebnis);
    }
}

/// PLAN-M2 AP10 Task 1: Inhalt an positionaler Stelle (`fletcher.node((0, 0), [Start])`).
#[test]
fn positionaler_inhalt() {
    let p = vec![WidgetParameter {
        position: Some(1),
        funktion: "fletcher.node".into(),
        parameter: "text".into(),
        art: CAPTION,
    }];
    let text = "#fletcher.node((0, 0), [Start])";
    let q = Source::detached(text);
    let a = angebote(&q, text.find("Start").unwrap(), &p);
    let a = a.first().expect("Angebot");
    assert_eq!(a.gesperrt, None);
    assert_eq!(a.wert, Some(Wert::Inhalt("Start".into())));
    let Ort::Ersetzen { bereich } = &a.ort else {
        panic!("{:?}", a.ort);
    };
    assert_eq!(&text[bereich.clone()], "Start");
    let patch = wert_patch(&q, a, &CAPTION, &Wert::Inhalt("Neu *x*".into())).unwrap();
    assert_eq!(anwenden(text, &patch), "#fletcher.node((0, 0), [Neu *x*])");
}
