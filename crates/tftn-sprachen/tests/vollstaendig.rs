//! `languages/en.toml` übersetzt jeden Text des Projekts, mit denselben Platzhaltern, und enthält
//! nichts, was im Code nicht mehr vorkommt. Nachziehen: `cargo xtask sprachen`, dann übersetzen.

use std::path::Path;

use tftn_sprachen::quellen::{bestand, platzhalter};

#[test]
fn englisch_ist_vollstaendig() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let b = bestand(&wurzel);
    assert!(
        b.offen.is_empty(),
        "t(…) braucht einen festen Text (für Werte aus Tabellen: n(\"…\") und tn(wert)): {:?}",
        b.offen
    );
    let datei =
        std::fs::read_to_string(wurzel.join("languages/en.toml")).expect("languages/en.toml");
    let tabelle: toml::Table = toml::from_str(&datei).expect("en.toml ist gültiges TOML");
    let texte = tabelle
        .get("texte")
        .and_then(|t| t.as_table())
        .cloned()
        .unwrap_or_default();

    let mut fehlt = Vec::new();
    let mut leer = Vec::new();
    let mut platz = Vec::new();
    for quelle in b.texte.keys() {
        match texte.get(quelle).and_then(|z| z.as_str()) {
            None => fehlt.push(quelle.clone()),
            Some(z) if z.trim().is_empty() => leer.push(quelle.clone()),
            Some(z) if platzhalter(z) != platzhalter(quelle) => platz.push(quelle.clone()),
            Some(_) => {}
        }
    }
    let alt: Vec<_> = texte.keys().filter(|k| !b.texte.contains_key(*k)).collect();
    assert!(
        fehlt.is_empty() && leer.is_empty() && alt.is_empty() && platz.is_empty(),
        "languages/en.toml passt nicht zum Code (cargo xtask sprachen):\n\
         fehlt: {fehlt:#?}\nleer: {leer:#?}\nnicht mehr im Code: {alt:#?}\nandere Platzhalter: {platz:#?}"
    );
}
