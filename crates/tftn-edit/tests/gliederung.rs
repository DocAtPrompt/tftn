use tftn_edit::{abschnitt_verschieben, anwenden, gliederung};
use typst_syntax::Source;

const TEXT: &str = "#set text(lang: \"de\")\n\
Titelei\n\
\n\
= Einleitung\n\
Text A.\n\
\n\
== Frage *fett*\n\
Text B.\n\
\n\
= Methode\n\
Text C.\n\
\n\
= Ergebnis\n\
Text D.\n";

fn titel(q: &str) -> Vec<(usize, String)> {
    gliederung(&Source::detached(q))
        .into_iter()
        .map(|u| (u.ebene, u.titel))
        .collect()
}

fn verschieben(q: &str, nr: usize, vor: Option<usize>) -> String {
    let s = Source::detached(q);
    let p = abschnitt_verschieben(&s, nr, vor).unwrap();
    anwenden(q, &p)
}

#[test]
fn ueberschriften_mit_ebene_und_titel() {
    assert_eq!(
        titel(TEXT),
        vec![
            (1, "Einleitung".to_string()),
            (2, "Frage fett".to_string()),
            (1, "Methode".to_string()),
            (1, "Ergebnis".to_string()),
        ]
    );
}

#[test]
fn abschnitt_reicht_bis_zur_naechsten_gleich_hohen() {
    let g = gliederung(&Source::detached(TEXT));
    let einleitung = &TEXT[g[0].abschnitt.clone()];
    assert!(einleitung.starts_with("= Einleitung\n"));
    assert!(einleitung.ends_with("Text B.\n\n"), "{einleitung:?}");
    assert_eq!(&TEXT[g[3].abschnitt.clone()], "= Ergebnis\nText D.\n");
}

#[test]
fn titel_ohne_code_und_marken() {
    assert_eq!(
        titel("= Über #emph[uns] <ziel>\n== Teil \\#2\n"),
        vec![(1, "Über uns".to_string()), (2, "Teil #2".to_string())]
    );
}

#[test]
fn ueberschriften_in_bloecken_zaehlen_nicht() {
    assert_eq!(
        titel("= A\n#block[\n= Innen\n]\n= B\n"),
        vec![(1, "A".to_string()), (1, "B".to_string())]
    );
}

#[test]
fn nach_oben_mit_unterabschnitten() {
    let neu = verschieben(TEXT, 2, Some(0));
    assert_eq!(
        titel(&neu),
        vec![
            (1, "Methode".to_string()),
            (1, "Einleitung".to_string()),
            (2, "Frage fett".to_string()),
            (1, "Ergebnis".to_string()),
        ]
    );
    assert!(
        neu.starts_with("#set text(lang: \"de\")\nTitelei\n\n= Methode\nText C.\n\n= Einleitung")
    );
}

#[test]
fn nach_unten_und_ans_ende() {
    let neu = verschieben(TEXT, 0, Some(3));
    assert_eq!(
        titel(&neu).iter().map(|t| t.1.as_str()).collect::<Vec<_>>(),
        ["Methode", "Einleitung", "Frage fett", "Ergebnis"]
    );
    let neu = verschieben(TEXT, 0, None);
    assert_eq!(
        titel(&neu).iter().map(|t| t.1.as_str()).collect::<Vec<_>>(),
        ["Methode", "Ergebnis", "Einleitung", "Frage fett"]
    );
    // Alles andere bleibt, nur die Reihenfolge ändert sich.
    assert_eq!(neu.len(), TEXT.len());
}

#[test]
fn letzter_ohne_zeilenende_bekommt_eines() {
    let q = "= A\nText A.\n= B\nText B.";
    assert_eq!(verschieben(q, 1, Some(0)), "= B\nText B.\n= A\nText A.\n");
    assert_eq!(verschieben(q, 0, None), "= B\nText B.\n= A\nText A.\n");
}

#[test]
fn literaturverzeichnis_bleibt_am_ende() {
    let q = "= A\nText A.\n\n= B\nText B.\n\n#bibliography(\"lit.bib\")\n";
    let neu = verschieben(q, 0, None);
    assert_eq!(
        neu,
        "= B\nText B.\n\n= A\nText A.\n\n#bibliography(\"lit.bib\")\n"
    );
    let neu = verschieben(q, 1, Some(0));
    assert_eq!(
        neu,
        "= B\nText B.\n\n= A\nText A.\n\n#bibliography(\"lit.bib\")\n"
    );
}

#[test]
fn an_die_eigene_stelle_aendert_nichts() {
    assert_eq!(verschieben(TEXT, 2, Some(2)), TEXT);
    assert_eq!(verschieben(TEXT, 2, Some(3)), TEXT);
    assert_eq!(verschieben(TEXT, 3, None), TEXT);
}

#[test]
fn nicht_in_den_eigenen_unterabschnitt() {
    let s = Source::detached(TEXT);
    assert!(abschnitt_verschieben(&s, 0, Some(1)).is_err());
    assert!(abschnitt_verschieben(&s, 9, Some(1)).is_err());
    assert!(abschnitt_verschieben(&s, 0, Some(9)).is_err());
}

/// Der Testartikel: alle Kapitel, Marken nicht im Titel, das Literaturverzeichnis bleibt am Schluss.
#[test]
fn testartikel() {
    let q = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/korpus/testartikel/main.typ"
    ))
    .unwrap()
    .replace("\r\n", "\n");
    let t: Vec<String> = titel(&q).into_iter().map(|t| t.1).collect();
    assert_eq!(
        t,
        [
            "Einleitung",
            "Methode",
            "Datengrundlage",
            "Messgröße",
            "Zitierweise",
            "Ergebnisse",
            "Diskussion"
        ]
    );
    let neu = verschieben(&q, 0, None);
    assert_eq!(neu.len(), q.len());
    let ende = neu.rfind("= Einleitung").unwrap();
    assert!(ende < neu.find("#bibliography").unwrap());
    let s = Source::detached(neu);
    assert_eq!(
        titel(s.text())[6].1,
        "Einleitung",
        "Einleitung steht jetzt am Schluss"
    );
}
