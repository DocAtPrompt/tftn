use tftn_edit::{Zahlen, zaehlen};
use typst_syntax::Source;

fn z(q: &str) -> Zahlen {
    let s = Source::detached(q);
    zaehlen(&s, 0..q.len())
}

#[test]
fn nur_der_geschriebene_text() {
    let q = "#set text(lang: \"de\")\n// Kommentar\n= Einleitung\nHallo *fette* Welt, $a^2 + b$ und `code`.\n";
    // Einleitung Hallo fette Welt, und .
    assert_eq!(z(q).woerter, 5);
}

#[test]
fn unterschriften_und_fussnoten_zaehlen_mit() {
    let q =
        "#figure(image(\"a.png\"), caption: [Zwei Wörter])\nText#footnote[Eine Fußnote hier.]\n";
    assert_eq!(z(q).woerter, 6);
}

#[test]
fn zeichen_mit_und_ohne_leerzeichen() {
    let q = "Ab  cd\n\nef";
    let n = z(q);
    assert_eq!((n.woerter, n.zeichen, n.ohne_leerzeichen), (3, 8, 6));
}

#[test]
fn nur_im_bereich() {
    let q = "eins zwei drei vier";
    let s = Source::detached(q);
    assert_eq!(zaehlen(&s, 5..14).woerter, 2);
    // Ein angeschnittenes Wort zählt mit.
    assert_eq!(zaehlen(&s, 6..9).woerter, 1);
    assert_eq!(zaehlen(&s, 3..3).woerter, 0);
}

#[test]
fn escapes_und_anfuehrungszeichen() {
    let q = "Preis \\#1 \"im\" Angebot -- heute";
    assert_eq!(z(q).woerter, 5);
}

#[test]
fn leeres_dokument() {
    assert_eq!(z(""), Zahlen::default());
}
