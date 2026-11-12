//! Seiteneinstellungen: Format, Schrift, Sprache als `#set`-Regeln oben im Dokument.

use tftn_edit::{PatchFehler, SeitenFeld as F, Zeile, anwenden, seite_lesen, seite_patch};
use typst_syntax::Source;

fn setzen(t: &str, f: F) -> String {
    let q = Source::detached(t);
    anwenden(t, &seite_patch(&q, &f).unwrap())
}

fn sprache(lang: &str, region: Option<&str>) -> F {
    F::Sprache {
        lang: lang.into(),
        region: region.map(Into::into),
    }
}

#[test]
fn liest_literale() {
    let t = "#set page(paper: \"a5\", flipped: true)\n#set text(font: \"X\", size: 12pt, lang: \"de\", region: \"AT\")\nHallo";
    let s = seite_lesen(&Source::detached(t));
    assert!(!s.gesperrt);
    assert_eq!(s.format.as_deref(), Some("a5"));
    assert!(s.quer);
    assert_eq!(s.schrift.as_deref(), Some("X"));
    assert_eq!(s.groesse, Some(12.0));
    assert_eq!(s.sprache.as_deref(), Some("de"));
    assert_eq!(s.region.as_deref(), Some("AT"));
}

#[test]
fn ohne_regeln_nichts_gesetzt() {
    let s = seite_lesen(&Source::detached("Hallo"));
    assert_eq!(s.format, None);
    assert!(!s.quer);
    assert_eq!(s.groesse, None);
}

#[test]
fn neue_regel_ganz_oben() {
    assert_eq!(
        setzen("Hallo\n", F::Format("a5".into())),
        "#set page(paper: \"a5\")\nHallo\n"
    );
}

#[test]
fn neue_regel_nach_den_importen() {
    assert_eq!(
        setzen("#import \"x.typ\": a\n\nHallo\n", F::Groesse(12.0)),
        "#import \"x.typ\": a\n#set text(size: 12pt)\n\nHallo\n"
    );
}

#[test]
fn import_ohne_zeilenende() {
    assert_eq!(
        setzen("#import \"x.typ\": a", F::Groesse(12.0)),
        "#import \"x.typ\": a\n#set text(size: 12pt)\n"
    );
}

#[test]
fn ergaenzt_und_behaelt_andere_argumente() {
    let t = "#set text(lang: \"de\", weight: \"bold\")\nHallo\n";
    assert_eq!(
        setzen(t, F::Schrift("DejaVu Sans Mono".into())),
        "#set text(lang: \"de\", weight: \"bold\", font: \"DejaVu Sans Mono\")\nHallo\n"
    );
}

#[test]
fn ersetzt_vorhandenen_wert() {
    let t = "#set page(paper: \"a4\")\nHallo\n";
    assert_eq!(
        setzen(t, F::Format("us-letter".into())),
        "#set page(paper: \"us-letter\")\nHallo\n"
    );
}

#[test]
fn quer_an_und_aus() {
    let t = "#set page(paper: \"a4\")\n";
    let an = setzen(t, F::Quer(true));
    assert_eq!(an, "#set page(paper: \"a4\", flipped: true)\n");
    assert_eq!(setzen(&an, F::Quer(false)), t);
}

#[test]
fn sprache_setzt_und_entfernt_region() {
    let t = "#set text(lang: \"de\", region: \"AT\")\n";
    assert_eq!(
        setzen(t, sprache("de", Some("CH"))),
        "#set text(lang: \"de\", region: \"CH\")\n"
    );
    assert_eq!(setzen(t, sprache("fr", None)), "#set text(lang: \"fr\")\n");
}

#[test]
fn berechneter_wert_wird_ersetzt() {
    let t = "#set text(size: 2em)\n";
    assert_eq!(seite_lesen(&Source::detached(t)).groesse, None);
    assert_eq!(setzen(t, F::Groesse(10.5)), "#set text(size: 10.5pt)\n");
}

#[test]
fn nur_die_erste_regel_der_obersten_ebene() {
    let t = "#set text(size: 9pt)\n#[#set text(size: 20pt)]\n#set text(size: 14pt)\n";
    assert_eq!(seite_lesen(&Source::detached(t)).groesse, Some(9.0));
    assert_eq!(
        setzen(t, F::Groesse(10.0)),
        "#set text(size: 10pt)\n#[#set text(size: 20pt)]\n#set text(size: 14pt)\n"
    );
}

#[test]
fn vorlage_legt_selbst_fest() {
    let t = "#import \"@local/x:1.0.0\": *\n#show: fachartikel.with(titel: 1)\nHallo\n";
    let q = Source::detached(t);
    assert!(seite_lesen(&q).gesperrt);
    assert!(matches!(
        seite_patch(&q, &F::Groesse(12.0)),
        Err(PatchFehler::Gesperrt(_))
    ));
    // Eine Show-Regel mit Ziel ist keine Vorlage.
    assert!(!seite_lesen(&Source::detached("#show heading: set text(red)\nHallo")).gesperrt);
}

#[test]
fn ungueltige_werte() {
    let q = Source::detached("Hallo");
    for f in [
        F::Groesse(0.0),
        F::Groesse(f64::NAN),
        F::Groesse(1000.0),
        F::Format(String::new()),
        F::Schrift(" ".into()),
        sprache("", None),
    ] {
        assert!(
            matches!(seite_patch(&q, &f), Err(PatchFehler::UngueltigerWert(_))),
            "{f:?}"
        );
    }
}

#[test]
fn anfuehrungszeichen_im_namen() {
    let t = setzen("Hallo", F::Schrift("A\"B".into()));
    assert_eq!(
        seite_lesen(&Source::detached(t)).schrift.as_deref(),
        Some("A\"B")
    );
}

const MIT_IMPORT: &str = "#import \"@local/tftn-leer:0.2.0\": *\n";

fn zeile(innen: &str, mitte: &str, aussen: &str, erste: bool) -> Zeile {
    Zeile {
        innen: innen.into(),
        mitte: mitte.into(),
        aussen: aussen.into(),
        erste,
    }
}

#[test]
fn raender_einseitig_und_doppelseitig() {
    let r = |doppelseitig| F::Raender {
        oben: 3.0,
        unten: 2.5,
        innen: 2.0,
        aussen: 4.0,
        doppelseitig,
    };
    let t = setzen("Hallo", r(false));
    assert_eq!(
        t,
        "#set page(margin: (top: 3cm, bottom: 2.5cm, left: 2cm, right: 4cm))\nHallo"
    );
    let s = seite_lesen(&Source::detached(t.as_str()));
    let g = s.raender.unwrap();
    assert_eq!((g.oben, g.unten, g.innen, g.aussen), (3.0, 2.5, 2.0, 4.0));
    assert!(!s.doppelseitig);
    let t = setzen(&t, r(true));
    assert_eq!(
        t,
        "#set page(margin: (top: 3cm, bottom: 2.5cm, inside: 2cm, outside: 4cm))\nHallo"
    );
    let s = seite_lesen(&Source::detached(t.as_str()));
    assert!(s.doppelseitig);
    assert_eq!(s.raender.unwrap().innen, 2.0);
}

#[test]
fn einzelner_rand_gilt_ueberall() {
    let s = seite_lesen(&Source::detached("#set page(margin: 2cm)\n"));
    let g = s.raender.unwrap();
    assert_eq!((g.oben, g.aussen), (2.0, 2.0));
}

#[test]
fn raender_ungueltig() {
    let q = Source::detached("Hallo");
    let f = F::Raender {
        oben: -1.0,
        unten: 1.0,
        innen: 1.0,
        aussen: 1.0,
        doppelseitig: false,
    };
    assert!(matches!(
        seite_patch(&q, &f),
        Err(PatchFehler::UngueltigerWert(_))
    ));
}

#[test]
fn spalten_blocksatz_abstand_nummern() {
    let t = setzen("Hallo", F::Spalten(2));
    assert_eq!(t, "#set page(columns: 2)\nHallo");
    assert_eq!(seite_lesen(&Source::detached(t.as_str())).spalten, 2);
    assert_eq!(setzen(&t, F::Spalten(1)), "Hallo");

    let t = setzen("Hallo", F::Blocksatz(true));
    assert_eq!(t, "#set par(justify: true)\nHallo");
    assert!(seite_lesen(&Source::detached(t.as_str())).blocksatz);
    assert_eq!(setzen(&t, F::Blocksatz(false)), "Hallo");
    assert_eq!(setzen("Hallo", F::Blocksatz(false)), "Hallo");

    let t = setzen("Hallo", F::Zeilenabstand(0.8));
    assert_eq!(t, "#set par(leading: 0.8em)\nHallo");
    assert_eq!(
        seite_lesen(&Source::detached(t.as_str())).zeilenabstand,
        Some(0.8)
    );

    let t = setzen("Hallo", F::Nummerierung(true));
    assert_eq!(t, "#set heading(numbering: \"1.1\")\nHallo");
    assert!(seite_lesen(&Source::detached(t.as_str())).nummeriert);
}

#[test]
fn kopf_und_fuss_schreiben_und_lesen() {
    let t = format!("{MIT_IMPORT}Hallo");
    assert!(seite_lesen(&Source::detached(t.as_str())).zeilen_moeglich);
    let neu = setzen(&t, F::Kopf(zeile("{kapitel}", "", "Ein \"Titel\"", false)));
    assert_eq!(
        neu,
        format!(
            "{MIT_IMPORT}#set page(header: kopfzeile(innen: \"{{kapitel}}\", aussen: \"Ein \\\"Titel\\\"\", erste: false))\nHallo"
        )
    );
    let s = seite_lesen(&Source::detached(neu.as_str()));
    assert_eq!(s.kopf, zeile("{kapitel}", "", "Ein \"Titel\"", false));
    assert_eq!(s.fuss, zeile("", "", "", true));

    let neu = setzen(&neu, F::Fuss(zeile("", "{seite}", "", true)));
    assert_eq!(
        seite_lesen(&Source::detached(neu.as_str())).fuss.mitte,
        "{seite}"
    );

    // Alles leer: Das Argument verschwindet.
    let weg = setzen(&neu, F::Kopf(zeile("", "", "", true)));
    assert!(!weg.contains("header"), "{weg}");
    assert!(weg.contains("footer"), "{weg}");
}

#[test]
fn kopf_braucht_den_import() {
    let q = Source::detached("#set text(lang: \"de\")\nHallo");
    assert!(!seite_lesen(&q).zeilen_moeglich);
    assert!(matches!(
        seite_patch(&q, &F::Kopf(zeile("x", "", "", true))),
        Err(PatchFehler::Gesperrt(_))
    ));
}
