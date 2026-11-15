//! Die eingebauten Bausteine (`tftn-katalog/eingebaut.toml`) und die Gruppen der Formelpalette
//! (`mathe.toml`) sind sichtbar und gehören deshalb in die Sprachdatei.

use std::path::Path;

use tftn_sprachen::quellen::bestand;

#[test]
fn eingebaute_bausteine_und_formelgruppen_im_bestand() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let b = bestand(&wurzel);
    for text in [
        "Abbildungen",
        "Bild",
        "Breite",
        "Unterschrift",
        "Griechisch",
        "Pfeile",
    ] {
        assert!(b.texte.contains_key(text), "{text} fehlt im Bestand");
    }
}
