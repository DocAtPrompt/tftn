//! Rechtschreibung nach Dokumentsprache: nur Fließtext, Wörterbücher aus `dictionaries/`.

use std::path::Path;

use tftn_edit::rechtschreibung::{
    Pruefer, WOERTERBUECHER, falsche_woerter, woerter, woerterbuch_fuer,
};
use typst_syntax::Source;

fn pruefer(kennung: &str) -> Pruefer {
    let ordner = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dictionaries");
    Pruefer::laden(&ordner, kennung).unwrap()
}

fn texte(quelle: &str) -> Vec<String> {
    let s = Source::detached(quelle);
    woerter(&s)
        .into_iter()
        .map(|r| quelle[r].to_string())
        .collect()
}

fn falsch(p: &Pruefer, quelle: &str) -> Vec<String> {
    let s = Source::detached(quelle);
    falsche_woerter(&s, p)
        .into_iter()
        .map(|r| quelle[r].to_string())
        .collect()
}

#[test]
fn nur_fliesstext_wird_geprueft() {
    let q = "#set text(lang: \"de\")\n= Überschrift\nEin *fetter* Satz mit $x + y$ und `code` \
             // Kommentar\n#figure(image(\"a.png\"), caption: [Bildunterschrift])\n@schlüssel <marke>";
    assert_eq!(
        texte(q),
        [
            "Überschrift",
            "Ein",
            "fetter",
            "Satz",
            "mit",
            "und",
            "Bildunterschrift"
        ]
    );
}

#[test]
fn woerter_mit_bindestrich_und_apostroph_bleiben_ganz() {
    assert_eq!(
        texte("E-Mail-Adresse, dell'anno und don't – aber 3D, 2026 und x."),
        ["E-Mail-Adresse", "dell'anno", "und", "don't", "aber", "und"]
    );
}

#[test]
fn abkuerzungen_in_grossbuchstaben_bleiben_draussen() {
    assert_eq!(
        texte("Die WHO und TFTN, z. B. hier, PDFs und URLs"),
        ["Die", "und", "hier", "und"]
    );
}

#[test]
fn woerterbuch_nach_sprache_und_region() {
    assert_eq!(woerterbuch_fuer("de", Some("AT")), Some("de-AT"));
    assert_eq!(woerterbuch_fuer("de", Some("CH")), Some("de-CH"));
    assert_eq!(woerterbuch_fuer("de", Some("DE")), Some("de"));
    assert_eq!(woerterbuch_fuer("de", None), Some("de"));
    assert_eq!(woerterbuch_fuer("de", Some("LU")), Some("de"));
    assert_eq!(woerterbuch_fuer("en", None), Some("en"));
    assert_eq!(woerterbuch_fuer("en", Some("GB")), Some("en-GB"));
    assert_eq!(woerterbuch_fuer("EN", Some("gb")), Some("en-GB"));
    assert_eq!(woerterbuch_fuer("fr", Some("CA")), Some("fr"));
    assert_eq!(woerterbuch_fuer("it", None), Some("it"));
    assert_eq!(woerterbuch_fuer("es", Some("MX")), Some("es"));
    assert_eq!(woerterbuch_fuer("pt", None), None);
}

#[test]
fn alle_woerterbuecher_liegen_bei_und_laden() {
    let beispiele = [
        ("de", "Das Haus"),
        ("de-AT", "Das Haus"),
        ("de-CH", "Das Haus"),
        ("en", "The house"),
        ("en-GB", "The house"),
        ("fr", "La maison"),
        ("it", "La casa"),
        ("es", "La casa"),
    ];
    assert_eq!(WOERTERBUECHER.len(), beispiele.len());
    for (k, satz) in beispiele {
        assert!(WOERTERBUECHER.contains(&k));
        let p = pruefer(k);
        assert!(falsch(&p, satz).is_empty(), "{k}");
        assert_eq!(falsch(&p, "Xqzvbn"), ["Xqzvbn"], "{k}");
    }
}

#[test]
fn deutsch_mit_zusammensetzungen() {
    let p = pruefer("de-AT");
    assert_eq!(
        falsch(
            &p,
            "Die Rechtschreibprüfung im Jänner findet Fehlr, aber keine Donaudampfschifffahrt. \
             Die E-Mail-Adresse stimmt."
        ),
        ["Fehlr"]
    );
}

#[test]
fn schweiz_ohne_eszett() {
    let p = pruefer("de-CH");
    assert!(falsch(&p, "Die Strasse ist gross, das Wasser nass.").is_empty());
    assert!(p.vorschlaege("Strase").iter().any(|v| v == "Strasse"));
}

#[test]
fn englisch_und_franzoesisch() {
    assert_eq!(
        falsch(&pruefer("en"), "The colour of the color, don't."),
        ["colour"]
    );
    assert_eq!(
        falsch(&pruefer("en-GB"), "The colour of the color."),
        ["color"]
    );
    assert_eq!(
        falsch(
            &pruefer("fr"),
            "L'homme est très content, mais fautif ici: maisonn."
        ),
        ["maisonn"]
    );
}

#[test]
fn eigene_woerter_und_vorschlaege() {
    let mut p = pruefer("de-AT");
    assert_eq!(falsch(&p, "TFTN-Werkzeug Typstdatei"), ["Typstdatei"]);
    p.aufnehmen("Typstdatei");
    assert!(falsch(&p, "Typstdatei").is_empty());
    assert!(p.vorschlaege("Fehlr").iter().any(|v| v == "Fehler"));
}

/// Was im Testartikel als falsch gilt: Namen und Fachwörter, die das Wörterbuch (igerman98) nicht
/// kennt, und „typografisch“, das es nur mit „ph“ führt. Das nimmt man ins eigene Wörterbuch auf.
#[test]
fn testartikel_nur_namen_und_fachwoerter() {
    let pfad =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel/main.typ");
    let text = std::fs::read_to_string(pfad).unwrap();
    let p = pruefer("de-AT");
    assert_eq!(
        falsch(&p, &text),
        [
            "hausärztlichen",
            "Häusler",
            "Typografische",
            "byteweise",
            "Zitierweise",
            "Lüddecke",
            "bytegleich"
        ]
    );
}
