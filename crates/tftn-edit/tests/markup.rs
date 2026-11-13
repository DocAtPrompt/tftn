//! Markup-Helfer (PLAN-M2 AP4, Task 5; Entwurf 4 „Was danach ändern umfasst“ 3): Ebene einer
//! Überschrift, Liste ↔ Aufzählung, Formel Block ↔ inline. Geprüft nach (a)–(c).

use proptest::prelude::*;
use tftn_edit::{
    MarkupAktion, MarkupStelle, PatchFehler, anwenden, formel_einfuegen, formel_patch, formel_text,
    markup_patch, markup_stellen, zitat_patch,
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

/// Erste Stelle am Cursor (`cursor` = Text, dessen Anfang der Cursor ist), Aktion angewendet.
fn helfer(text: &str, cursor: &str, aktion: MarkupAktion) -> Result<String, PatchFehler> {
    let q = Source::detached(text);
    let stellen = markup_stellen(&q, text.find(cursor).unwrap());
    let stelle = stellen.first().expect("Stelle");
    markup_patch(&q, stelle, aktion).map(|p| anwenden(text, &p))
}

#[test]
fn ebene_plus_minus_und_grenzen() {
    assert_eq!(
        helfer("= A\nText", "A", MarkupAktion::EbenePlus).unwrap(),
        "== A\nText"
    );
    assert_eq!(
        helfer("=== A", "A", MarkupAktion::EbeneMinus).unwrap(),
        "== A"
    );
    assert!(matches!(
        helfer("= A", "A", MarkupAktion::EbeneMinus),
        Err(PatchFehler::UngueltigerWert(_))
    ));
    assert!(matches!(
        helfer("====== A", "A", MarkupAktion::EbenePlus),
        Err(PatchFehler::UngueltigerWert(_))
    ));
    // Abschlussprüfung AP4: Eine tiefere Überschrift aus dem Text lässt sich zurückstufen.
    assert_eq!(
        helfer("======== A", "A", MarkupAktion::EbeneMinus).unwrap(),
        "======= A"
    );
}

#[test]
fn liste_wird_aufzaehlung_und_zurueck() {
    let t = "- a\n- b\n\n- c\nText";
    let aufz = helfer(t, "b", MarkupAktion::ListeUmschalten).unwrap();
    assert_eq!(aufz, "+ a\n+ b\n\n+ c\nText");
    assert_eq!(
        helfer(&aufz, "b", MarkupAktion::ListeUmschalten).unwrap(),
        t
    );
}

#[test]
fn nummerierte_aufzaehlung_mit_folgezeile() {
    let t = "1. eins\n   weiter\n10. zehn";
    let neu = helfer(t, "eins", MarkupAktion::ListeUmschalten).unwrap();
    assert_eq!(neu, "- eins\n   weiter\n- zehn");
    let q = Source::detached(neu.as_str());
    let Some(MarkupStelle::Liste {
        marken,
        aufzaehlung,
    }) = markup_stellen(&q, neu.find("weiter").unwrap())
        .into_iter()
        .next()
    else {
        panic!("keine Liste bei „weiter“");
    };
    assert!(!aufzaehlung);
    assert_eq!(marken.len(), 2, "„weiter“ gehört weiter zum ersten Punkt");
}

#[test]
fn verschachtelte_liste_nur_die_ebene_am_cursor() {
    assert_eq!(
        helfer("- a\n  - b\n- c", "b", MarkupAktion::ListeUmschalten).unwrap(),
        "- a\n  + b\n- c"
    );
}

#[test]
fn formel_inline_block() {
    assert_eq!(
        helfer("Es gilt $x^2$.", "x", MarkupAktion::FormelUmschalten).unwrap(),
        "Es gilt $ x^2 $."
    );
    assert_eq!(
        helfer("$\n  a + b\n$", "a", MarkupAktion::FormelUmschalten).unwrap(),
        "$a + b$"
    );
}

/// Grenze: Grenzt eine Liste der neuen Art an, würden beide verschmelzen; das ändert die Struktur,
/// also verwirft (b) den Patch.
#[test]
fn angrenzende_liste_wird_nicht_verschmolzen() {
    assert!(matches!(
        helfer("- a\n\n+ b", "b", MarkupAktion::ListeUmschalten),
        Err(PatchFehler::Invariante(_))
    ));
}

/// Typst verwirft Kommentare: Eine Liste mit Kommentarzeile ist eine Liste, und ein Kommentar
/// trennt zwei Listen nicht.
#[test]
fn kommentar_zwischen_punkten() {
    assert_eq!(
        helfer("- a\n// c\n- b", "a", MarkupAktion::ListeUmschalten).unwrap(),
        "+ a\n// c\n+ b"
    );
    assert!(matches!(
        helfer("+ x\n// Notiz\n- a", "a", MarkupAktion::ListeUmschalten),
        Err(PatchFehler::Invariante(_))
    ));
}

#[test]
fn ausserhalb_keine_stelle() {
    let q = Source::detached("Nur Text.");
    assert!(markup_stellen(&q, 2).is_empty());
}

fn fehler(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::Error) + k.children().map(fehler).sum::<usize>()
}

fn umgebung() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text ",
            "*fett* ",
            "$y$ ",
            "\n= Andere\n",
            "\n\n",
            "// Kommentar\n",
            "#footnote[Fuß] ",
            "😀 ",
            "\n- Punkt\n",
            "\n+ Nummer\n",
            "Größe ",
        ]),
        0..8,
    )
    .prop_map(|v| v.concat())
}

#[derive(Debug, Clone)]
enum Ziel {
    Ueberschrift(usize),
    Liste(&'static str, usize),
    Formel(bool),
}

fn ziel() -> impl Strategy<Value = Ziel> {
    prop_oneof![
        (1usize..=5).prop_map(Ziel::Ueberschrift),
        (proptest::sample::select(vec!["-", "+", "N."]), 1usize..=5)
            .prop_map(|(m, n)| Ziel::Liste(m, n)),
        any::<bool>().prop_map(Ziel::Formel),
    ]
}

fn ziel_text(z: &Ziel) -> (String, MarkupAktion) {
    match z {
        Ziel::Ueberschrift(k) => (
            format!("\n{} ZIEL Titel\n", "=".repeat(*k)),
            MarkupAktion::EbenePlus,
        ),
        Ziel::Liste(m, n) => {
            let punkte: Vec<String> = (0..*n)
                .map(|i| {
                    let marke = if *m == "N." {
                        format!("{}.", i + 1)
                    } else {
                        m.to_string()
                    };
                    let inhalt = if i == 0 {
                        "ZIEL eins".to_string()
                    } else {
                        format!("Punkt {i}")
                    };
                    format!("{marke} {inhalt}")
                })
                .collect();
            // Textzeilen davor und danach: Eine angrenzende Liste der neuen Art würde sonst
            // verschmelzen (siehe `angrenzende_liste_wird_nicht_verschmolzen`).
            (
                format!("\nVorher\n{}\nNachher\n", punkte.join("\n")),
                MarkupAktion::ListeUmschalten,
            )
        }
        Ziel::Formel(block) => (
            if *block {
                "$ ZIEL + 1 $".into()
            } else {
                "$ZIEL + 1$".into()
            },
            MarkupAktion::FormelUmschalten,
        ),
    }
}

proptest! {
    #![proptest_config(konfig())]

    /// (a)–(c) und neuer Zustand für alle drei Helfer in zufälliger Umgebung.
    #[test]
    fn p_markup_eigenschaften((v, z, n) in (umgebung(), ziel(), umgebung())) {
        let (zt, aktion) = ziel_text(&z);
        let text = format!("{v}{zt}{n}");
        let q = Source::detached(text.clone());
        let cursor = text.find("ZIEL").unwrap();
        let stellen = markup_stellen(&q, cursor);
        let stelle = stellen.first().expect("Stelle am Ziel").clone();
        let p = markup_patch(&q, &stelle, aktion).unwrap();
        let neu = anwenden(&text, &p);
        // (a)
        prop_assert_eq!(&neu[..p.bereich.start], &text[..p.bereich.start]);
        prop_assert_eq!(&neu[p.bereich.start + p.text.len()..], &text[p.bereich.end..]);
        // (c)
        let nq = Source::detached(neu.clone());
        prop_assert!(fehler(nq.root()) <= fehler(q.root()));
        // neuer Zustand am selben Ziel
        let danach = markup_stellen(&nq, neu.find("ZIEL").unwrap());
        match (&stelle, danach.first()) {
            (MarkupStelle::Ueberschrift { ebene, .. }, Some(MarkupStelle::Ueberschrift { ebene: e, .. })) => {
                prop_assert_eq!(*e, ebene + 1);
            }
            (MarkupStelle::Liste { aufzaehlung, marken }, Some(MarkupStelle::Liste { aufzaehlung: a, marken: m })) => {
                prop_assert_eq!(*a, !aufzaehlung);
                prop_assert_eq!(m.len(), marken.len());
            }
            (MarkupStelle::Formel { block, .. }, Some(MarkupStelle::Formel { block: b, .. })) => {
                prop_assert_eq!(*b, !block);
            }
            (alt, neu) => prop_assert!(false, "{:?} → {:?}", alt, neu),
        }
        // Überschrift und einzeilige Formel: genau eine Zeile geändert.
        if !matches!(z, Ziel::Liste(..)) {
            let geaendert = text.split('\n').zip(neu.split('\n')).filter(|(a, b)| a != b).count();
            prop_assert_eq!(geaendert, 1);
        }
    }
}

// --- Formeltext (PLAN-M2 AP10, Fach „Formel“) ---------------------------------------------------

fn formel_stelle(q: &Source, cursor: usize) -> MarkupStelle {
    markup_stellen(q, cursor)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Formel { .. }))
        .expect("Formel")
}

fn formel_aendern(text: &str, cursor: &str, neu: &str) -> Result<String, PatchFehler> {
    let q = Source::detached(text);
    let s = formel_stelle(&q, text.find(cursor).unwrap());
    formel_patch(&q, &s, neu).map(|p| anwenden(text, &p))
}

#[test]
fn formel_text_und_patch() {
    let t = "a $ x + 1 $ b";
    let q = Source::detached(t);
    let s = formel_stelle(&q, t.find('x').unwrap());
    assert_eq!(formel_text(&q, &s).as_deref(), Some("x + 1"));
    assert_eq!(formel_aendern(t, "x", "y").unwrap(), "a $ y $ b");
    assert_eq!(formel_aendern("$x$", "x", "y").unwrap(), "$y$");
    assert_eq!(
        formel_aendern("$\n  a\n$", "a", "b + c").unwrap(),
        "$\n  b + c\n$"
    );
    assert_eq!(formel_aendern("$ x $", "x", "  y  ").unwrap(), "$ y $");
    // `$ $` gilt bei Typst nicht als Block.
    assert_eq!(formel_aendern("$ $", " ", "y").unwrap(), "$y$");
    assert_eq!(
        formel_aendern("$ x $", "x", "a &= b \\\n &= c").unwrap(),
        "$ a &= b \\\n &= c $"
    );
}

/// Review Focus 1: Es wird abgelehnt, und die Meldung kommt als Fehler zurück.
#[test]
fn formel_patch_lehnt_ab() {
    for neu in [
        "$", "a $ b", "a$ + $b", "frac(a", "a // b", "a \\", "", "   ",
    ] {
        assert!(formel_aendern("$x$", "x", neu).is_err(), "{neu:?}");
    }
    let keine = Source::detached("= A");
    let s = markup_stellen(&keine, 2).remove(0);
    assert_eq!(formel_text(&keine, &s), None);
    assert!(formel_patch(&keine, &s, "x").is_err());
}

#[test]
fn formel_einfuegen_faelle() {
    let neu = |t: &str, an: usize, block: bool| {
        let q = Source::detached(t);
        formel_einfuegen(&q, an, "y", block).map(|p| anwenden(t, &p))
    };
    assert_eq!(neu("a  b", 2, true).unwrap(), "a $ y $ b");
    assert_eq!(neu("a  b", 2, false).unwrap(), "a $y$ b");
    assert_eq!(neu("#[a ]", 4, false).unwrap(), "#[a $y$]");
    assert!(neu("#{ 1 }", 3, true).is_err());
    assert!(neu("$ a $", 3, true).is_err());
    assert!(neu("// a", 3, true).is_err());
    let q = Source::detached("a");
    assert!(formel_einfuegen(&q, 1, " ", true).is_err());
    assert!(formel_einfuegen(&q, 1, "$ b", true).is_err());
}

fn zahl_formeln(n: &SyntaxNode) -> usize {
    usize::from(n.kind() == SyntaxKind::Equation) + n.children().map(zahl_formeln).sum::<usize>()
}

fn mathe_text(block: bool) -> impl Strategy<Value = String> {
    let mut atome = vec![
        "x",
        "y",
        "12",
        "+",
        "-",
        "=",
        "x_1",
        "x^2",
        "frac(a, b)",
        "sqrt(x)",
        "alpha",
        "\"Text\"",
    ];
    if block {
        atome.push("\\\n");
        atome.push("&=");
    }
    proptest::collection::vec(proptest::sample::select(atome), 1..12).prop_map(|v| v.join(" "))
}

fn formel_patch_fall(vor: &str, block: bool, nach: &str, neu: &str) -> Result<(), TestCaseError> {
    let alt = if block { "$ a + b $" } else { "$a + b$" };
    let text = format!("{vor}{alt}{nach}");
    let q = Source::detached(text.clone());
    let s = formel_stelle(&q, vor.len() + alt.find('a').unwrap());
    let patch = formel_patch(&q, &s, neu);
    prop_assert!(patch.is_ok(), "{text:?} {neu:?}: {patch:?}");
    let patch = patch.unwrap();
    let ergebnis = anwenden(&text, &patch);
    prop_assert_eq!(
        &ergebnis[..patch.bereich.start],
        &text[..patch.bereich.start]
    );
    let ende = patch.bereich.start + patch.text.len();
    prop_assert_eq!(&ergebnis[ende..], &text[patch.bereich.end..]);
    let n = Source::detached(ergebnis);
    prop_assert!(!n.root().diagnosis().either());
    prop_assert_eq!(zahl_formeln(n.root()), zahl_formeln(q.root()));
    let danach = formel_stelle(&n, patch.bereich.start);
    let MarkupStelle::Formel { block: b, .. } = &danach else {
        unreachable!();
    };
    prop_assert_eq!(*b, block);
    prop_assert_eq!(formel_text(&n, &danach).unwrap(), neu.trim());
    Ok(())
}

proptest! {
    #![proptest_config(konfig())]

    #[test]
    fn formel_patch_eigenschaften_block(
        vor in proptest::sample::select(vec!["", "Text ", "= Kopf\n"]),
        nach in proptest::sample::select(vec!["", " mehr", "\n\nAbsatz"]),
        neu in mathe_text(true),
    ) {
        formel_patch_fall(vor, true, nach, &neu)?;
    }

    #[test]
    fn formel_patch_eigenschaften_im_satz(
        vor in proptest::sample::select(vec!["", "Text ", "= Kopf\n"]),
        nach in proptest::sample::select(vec!["", " mehr", "\n\nAbsatz"]),
        neu in mathe_text(false),
    ) {
        formel_patch_fall(vor, false, nach, &neu)?;
    }
}

// --- Zitat ändern (PLAN-M2 AP11) -----------------------------------------------------------------

fn zitat_stelle(q: &Source, byte: usize) -> Option<MarkupStelle> {
    markup_stellen(q, byte)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Zitat { .. }))
}

fn zitat_lesen(text: &str, byte: usize) -> Option<(String, Option<String>)> {
    match zitat_stelle(&Source::detached(text), byte)? {
        MarkupStelle::Zitat {
            schluessel, zusatz, ..
        } => Some((schluessel, zusatz)),
        _ => unreachable!(),
    }
}

/// `schluessel`/`zusatz` an dem Zitat, das bei `cursor` beginnt; Ergebnis als Text.
fn zitat_aendern(
    text: &str,
    cursor: &str,
    schluessel: &str,
    zusatz: Option<&str>,
) -> Result<String, PatchFehler> {
    let q = Source::detached(text);
    let s = zitat_stelle(&q, text.find(cursor).unwrap()).expect("Zitat");
    zitat_patch(&q, &s, schluessel, zusatz).map(|p| anwenden(text, &p))
}

#[test]
fn zitat_stelle_gelesen() {
    let mit = "Text @mei2020[S. 3] mehr.";
    let erwartet = Some(("mei2020".to_string(), Some("S. 3".to_string())));
    for cursor in ["@mei", "mei2020", "S. 3"] {
        assert_eq!(zitat_lesen(mit, mit.find(cursor).unwrap()), erwartet);
    }
    let ohne = "Text @mei2020 mehr.";
    assert_eq!(
        zitat_lesen(ohne, ohne.find("@").unwrap()),
        Some(("mei2020".to_string(), None))
    );
    assert_eq!(zitat_lesen(ohne, 1), None);
}

#[test]
fn zitat_cursor_direkt_dahinter() {
    // Nach dem Einfügen steht der Cursor genau hinter dem Zitat.
    for (t, ende) in [("Text @a mehr", 7), ("Text @a[S. 3] mehr", 13), ("@a", 2)] {
        assert_eq!(zitat_lesen(t, ende).unwrap().0, "a", "{t}");
    }
    // Ein Leerzeichen dazwischen: nicht mehr das Zitat
    assert_eq!(zitat_lesen("Text @a mehr", 8), None);
}

#[test]
fn zitat_patch_setzt_und_entfernt() {
    let t = "Vor @a mehr";
    assert_eq!(
        zitat_aendern(t, "@a", "a", Some("S. 12")).unwrap(),
        "Vor @a[S. 12] mehr"
    );
    let t = "Vor @a[S. 3] mehr";
    assert_eq!(zitat_aendern(t, "@a", "a", None).unwrap(), "Vor @a mehr");
    assert_eq!(
        zitat_aendern(t, "@a", "a", Some("  ")).unwrap(),
        "Vor @a mehr"
    );
    assert_eq!(
        zitat_aendern(t, "@a", "b", Some("S. 3")).unwrap(),
        "Vor @b[S. 3] mehr"
    );
}

#[test]
fn zitat_zusatz_sonderzeichen() {
    let t = "Vor @a mehr";
    for falsch in ["x]", "[x", "#(", "][", "#x", "Nr. #5"] {
        assert!(
            matches!(
                zitat_aendern(t, "@a", "a", Some(falsch)),
                Err(PatchFehler::UngueltigerWert(_))
            ),
            "{falsch}"
        );
    }
    for gut in ["S. 3–5", "*Kap.* 2", "Tab. 4, ä"] {
        let neu = zitat_aendern(t, "@a", "a", Some(gut)).unwrap();
        assert_eq!(zitat_lesen(&neu, 4).unwrap().1.as_deref(), Some(gut));
    }
}

/// Beim Tippen steht `[` oder `$` kurz offen am Dokumentende (Abschlussprüfung AP11).
#[test]
fn offene_klammer_am_ende_kein_absturz() {
    for t in ["@a[", "Text @a[ä", "@a[S. 1", "Text @a[ä "] {
        let q = Source::detached(t);
        for byte in (0..=t.len()).filter(|&b| t.is_char_boundary(b)) {
            assert_eq!(zitat_lesen(t, byte), None, "{t:?} bei {byte}");
            for s in markup_stellen(&q, byte) {
                let _ = formel_text(&q, &s);
            }
        }
    }
}

#[test]
fn offene_formel_am_ende_kein_absturz() {
    for t in ["$", "a $ä", "$ x", "Text $x + ä"] {
        let q = Source::detached(t);
        for byte in (0..=t.len()).filter(|&b| t.is_char_boundary(b)) {
            let stellen = markup_stellen(&q, byte);
            assert!(
                !stellen
                    .iter()
                    .any(|s| matches!(s, MarkupStelle::Formel { .. })),
                "{t:?} bei {byte}"
            );
        }
    }
}

#[test]
fn zusatz_entfernen_vor_buchstabe() {
    // `@ab` wäre ein anderer Schlüssel.
    assert!(matches!(
        zitat_aendern("@a[S. 1]b", "@a", "a", None),
        Err(PatchFehler::UngueltigerWert(_))
    ));
}

#[test]
fn schluessel_nicht_als_verweis() {
    for falsch in ["a/b", "a.", "a:", "a b", ""] {
        assert!(
            matches!(
                zitat_aendern("Vor @a mehr", "@a", falsch, Some("S. 1")),
                Err(PatchFehler::UngueltigerWert(_))
            ),
            "{falsch:?}"
        );
    }
}

#[test]
fn zitat_in_verschachtelung() {
    for (vor, nach) in [
        ("#footnote[", "]"),
        ("#table(columns: 1, [", "])"),
        ("= Kopf ", "\n"),
        ("#hinweis[", "]"),
    ] {
        let t = format!("{vor}@ziel[S. 1] @anderes{nach}");
        assert_eq!(
            zitat_aendern(&t, "@ziel", "ziel", Some("S. 9")).unwrap(),
            format!("{vor}@ziel[S. 9] @anderes{nach}")
        );
        let q = Source::detached(t.clone());
        assert!(matches!(
            markup_stellen(&q, t.find("@ziel").unwrap())[0],
            MarkupStelle::Zitat { .. }
        ));
    }
}

fn zahl_verweise(k: &SyntaxNode) -> usize {
    usize::from(k.kind() == SyntaxKind::Ref) + k.children().map(zahl_verweise).sum::<usize>()
}

fn zusatz_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "S.", " ", "3", "–", "]", "[", "#", "*", "_", "$", "@", "ä", "😀", "x", "Kap", "\\",
        ]),
        0..8,
    )
    .prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(konfig())]

    #[test]
    fn zitat_patch_eigenschaften(
        (vor, nach) in proptest::sample::select(vec![
            ("", ""),
            ("Text ", " mehr"),
            ("#footnote[", "]"),
            ("= Kopf ", "\n"),
            ("Text ", "b"),
        ]),
        alt_zusatz in proptest::bool::ANY,
        schluessel in "[a-z][a-z0-9_-]{0,8}",
        zusatz in zusatz_text(),
    ) {
        let klammer = if alt_zusatz { "[S. 1]" } else { "[]" };
        let text = format!("{vor}@alt{klammer}{nach}");
        let q = Source::detached(text.clone());
        let stelle = zitat_stelle(&q, vor.len());
        prop_assert!(stelle.is_some(), "{text:?}");
        match zitat_patch(&q, &stelle.unwrap(), &schluessel, Some(&zusatz)) {
            Ok(p) => {
                let neu = anwenden(&text, &p);
                prop_assert_eq!(&neu[..p.bereich.start], &text[..p.bereich.start]);
                let ende = p.bereich.start + p.text.len();
                prop_assert_eq!(&neu[ende..], &text[p.bereich.end..]);
                let n = Source::detached(neu.clone());
                prop_assert!(fehler(n.root()) <= fehler(q.root()), "{neu:?}");
                prop_assert_eq!(zahl_verweise(n.root()), zahl_verweise(q.root()));
                let danach = zitat_lesen(&neu, vor.len());
                let z = zusatz.trim();
                let erwartet = (schluessel.clone(), (!z.is_empty()).then(|| z.to_string()));
                prop_assert_eq!(danach, Some(erwartet), "{:?}", neu);
            }
            Err(PatchFehler::UngueltigerWert(_)) => {}
            Err(andere) => prop_assert!(false, "{text:?} {zusatz:?}: {andere:?}"),
        }
    }
}
