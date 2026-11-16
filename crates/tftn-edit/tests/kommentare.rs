use tftn_edit::kommentare;
use typst_syntax::Source;

fn texte(q: &str) -> Vec<String> {
    kommentare(&Source::detached(q))
        .into_iter()
        .map(|k| k.text)
        .collect()
}

#[test]
fn zeilenkommentar_ohne_schraegstriche() {
    assert_eq!(texte("Text // nachsehen\nmehr"), ["nachsehen"]);
}

#[test]
fn aufeinanderfolgende_zeilen_sind_ein_kommentar() {
    let q = "// Zeile eins\n// Zeile zwei\nText\n\n// allein\n";
    assert_eq!(texte(q), ["Zeile eins\nZeile zwei", "allein"]);
}

#[test]
fn leerzeile_trennt() {
    assert_eq!(texte("// a\n\n// b\n"), ["a", "b"]);
}

#[test]
fn blockkommentar_ohne_rahmen_und_sterne() {
    let q = "Text /* kurz */ Text\n/*\n * Punkt eins\n * Punkt zwei\n */\n";
    assert_eq!(texte(q), ["kurz", "Punkt eins\nPunkt zwei"]);
}

#[test]
fn zierlinien_und_leere_zaehlen_nicht() {
    assert_eq!(
        texte("// ------\n\n//\n\n/* */\n\n// === Teil 2 ===\n"),
        ["=== Teil 2 ==="]
    );
}

#[test]
fn auch_in_code_und_formeln() {
    let q = "#let x = 1 // Breite\n$ a + b // Summe\n$\n";
    assert_eq!(texte(q), ["Breite", "Summe"]);
}

#[test]
fn bereich_deckt_den_ganzen_kommentar() {
    let q = "A\n// eins\n// zwei\nB";
    let k = kommentare(&Source::detached(q));
    assert_eq!(k.len(), 1);
    assert_eq!(&q[k[0].von..k[0].bis], "// eins\n// zwei");
}

#[test]
fn kein_kommentar_in_zeichenketten_und_raw() {
    assert!(texte("#let u = \"http://x\"\n`// kein` und ```\n// auch nicht\n```\n").is_empty());
}
