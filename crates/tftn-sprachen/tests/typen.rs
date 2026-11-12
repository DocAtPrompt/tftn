//! Die sichtbaren Texte der mitgelieferten Dokumenttypen (`typen/`) und Paketbeschreibungen
//! (`pakete/`) gehören zum Bestand, damit `sprachen/en.toml` sie übersetzt.

use std::path::Path;

use tftn_sprachen::quellen::bestand;

#[test]
fn typen_und_pakete_im_bestand() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let b = bestand(&wurzel);
    for text in [
        // typen/*/typst.toml
        "Fachartikel",
        "Fachartikel (TFTN)",
        "Hinweiskasten",
        "Randtext",
        "Bilder: lege sie in den Dokumentordner, etwa in einen Unterordner bilder.",
        // typen/*/tftn/formular.schema.json
        "Zusammenfassung",
        "Autorinnen und Autoren",
        "Englisch",
        "BibLaTeX (.bib) oder Hayagriva (.yaml), relativ zum Projektordner",
        // pakete/*.toml
        "Freie Zeichnungen",
        "Zeichnung",
        "So lang ist eine Koordinateneinheit.",
    ] {
        assert!(b.texte.contains_key(text), "{text} fehlt im Bestand");
    }
    assert!(
        b.dateien
            .iter()
            .any(|d| d == "typen/tftn-fachartikel/typst.toml"),
        "{:?}",
        b.dateien
    );
    // Werte, die Typst liest, sind keine Texte der Oberfläche.
    for kein_text in ["info", "luma(230)", "1cm", "de", "vorlage-en"] {
        assert!(
            !b.texte.contains_key(kein_text),
            "{kein_text} ist kein Text"
        );
    }
}
