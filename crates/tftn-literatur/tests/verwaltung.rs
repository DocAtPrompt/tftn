//! Literaturverwaltung: Einträge einer .bib lesen, ändern, anlegen, löschen. Der Rest der Datei
//! (Kommentare, @string, andere Einträge, deren Schreibweise) bleibt Byte für Byte.

use tftn_literatur::verwaltung::{
    BibEintrag, Feld, aendern, anlegen, bibtex, lesen, loeschen, zitiert,
};

const BIB: &str = "% Meine Literatur
@string{nat = {Nature}}

@article{lecun2015,
  author    = {LeCun, Yann and Bengio, Yoshua},
  title     = {Deep learning},
  journal   = nat,
  year      = 2015,
}

@book{knuth1984, author = \"Knuth, Donald E.\", title = {The {\\TeX}book}, year = {1984}}
";

fn feld(name: &str, wert: &str) -> Feld {
    Feld {
        name: name.into(),
        wert: wert.into(),
        roh: false,
    }
}

fn buch(schluessel: &str) -> BibEintrag {
    BibEintrag {
        art: "book".into(),
        schluessel: schluessel.into(),
        felder: vec![
            feld("author", "Müller, Anna"),
            feld("title", "Über {Typst}"),
            feld("year", "2026"),
        ],
    }
}

#[test]
fn liest_art_schluessel_und_felder() {
    let e = lesen(BIB).unwrap();
    assert_eq!(e.len(), 2);
    let a = &e[0].eintrag;
    assert_eq!(
        (a.art.as_str(), a.schluessel.as_str()),
        ("article", "lecun2015")
    );
    assert_eq!(
        a.felder,
        [
            feld("author", "LeCun, Yann and Bengio, Yoshua"),
            feld("title", "Deep learning"),
            Feld {
                name: "journal".into(),
                wert: "nat".into(),
                roh: true
            },
            Feld {
                name: "year".into(),
                wert: "2015".into(),
                roh: true
            },
        ]
    );
    let b = &e[1].eintrag;
    assert_eq!(b.felder[0], feld("author", "Knuth, Donald E."));
    assert_eq!(b.felder[1], feld("title", "The {\\TeX}book"));
    assert!(BIB[e[1].bereich.clone()].starts_with("@book{knuth1984"));
    assert!(BIB[e[1].bereich.clone()].ends_with("}"));
}

#[test]
fn aendern_ersetzt_nur_den_eintrag() {
    let mut neu = lesen(BIB).unwrap()[1].eintrag.clone();
    neu.felder.push(feld("publisher", "Addison-Wesley"));
    let text = aendern(BIB, "knuth1984", &neu).unwrap();
    let vorne = BIB.find("@book").unwrap();
    assert_eq!(&text[..vorne], &BIB[..vorne]);
    assert!(text.contains("  publisher = {Addison-Wesley},\n"), "{text}");
    assert_eq!(
        text,
        format!("{}{}\n", &BIB[..vorne], bibtex(&neu).unwrap())
    );
    let wieder = lesen(&text).unwrap();
    assert_eq!(wieder[1].eintrag, neu);
    assert_eq!(wieder[0].eintrag, lesen(BIB).unwrap()[0].eintrag);
}

#[test]
fn rohe_werte_bleiben_roh() {
    let alt = lesen(BIB).unwrap()[0].eintrag.clone();
    let text = aendern(BIB, "lecun2015", &alt).unwrap();
    assert!(text.contains("journal = nat,"), "{text}");
    assert_eq!(lesen(&text).unwrap()[0].eintrag, alt);
}

#[test]
fn anlegen_haengt_an() {
    let text = anlegen(BIB, &buch("mueller2026")).unwrap();
    assert!(text.starts_with(BIB.trim_end()));
    assert!(text.ends_with("@book{mueller2026,\n  author = {Müller, Anna},\n  title = {Über {Typst}},\n  year = {2026},\n}\n"), "{text}");
    assert_eq!(lesen(&text).unwrap().len(), 3);
    assert_eq!(
        anlegen("", &buch("x")).unwrap().lines().next(),
        Some("@book{x,")
    );
}

#[test]
fn loeschen_nimmt_den_eintrag_samt_leerzeile() {
    let text = loeschen(BIB, "lecun2015").unwrap();
    assert!(!text.contains("lecun2015"));
    assert!(
        text.contains("@string{nat = {Nature}}\n\n@book{knuth1984"),
        "{text}"
    );
    assert_eq!(lesen(&text).unwrap().len(), 1);
}

#[test]
fn fehler_werden_gemeldet_und_nichts_geschrieben() {
    assert!(aendern(BIB, "gibtsnicht", &buch("gibtsnicht")).is_err());
    assert!(loeschen(BIB, "gibtsnicht").is_err());
    assert!(
        anlegen(BIB, &buch("knuth1984")).is_err(),
        "Schlüssel doppelt"
    );
    assert!(
        aendern(BIB, "lecun2015", &buch("knuth1984")).is_err(),
        "Schlüssel doppelt"
    );
    assert!(anlegen(BIB, &buch("mit leerzeichen")).is_err());
    assert!(anlegen(BIB, &buch("")).is_err());
    let mut kaputt = buch("k");
    kaputt.felder.push(feld("title", "offen {"));
    assert!(anlegen(BIB, &kaputt).is_err(), "Klammern");
    let mut name = buch("k");
    name.felder.push(feld("mein feld", "x"));
    assert!(anlegen(BIB, &name).is_err(), "Feldname");
    let mut art = buch("k");
    art.art = "bu ch".into();
    assert!(anlegen(BIB, &art).is_err(), "Art");
}

#[test]
fn leere_felder_entfallen() {
    let mut e = buch("k");
    e.felder.push(feld("note", "  "));
    let text = anlegen("", &e).unwrap();
    assert!(!text.contains("note"), "{text}");
}

#[test]
fn zitiert_erkennt_at_und_cite() {
    let q = "Wie @lecun2015 zeigt und #cite(<knuth1984>, supplement: [S. 3]). Mail: a@b.at";
    assert!(zitiert(q, "lecun2015"));
    assert!(zitiert(q, "knuth1984"));
    assert!(!zitiert(q, "lecun"), "nur ganze Schlüssel");
    assert!(!zitiert(q, "b"), "Mailadresse zählt nicht (Wortteil davor)");
    assert!(zitiert("@lecun2015[S. 4]", "lecun2015"));
}
