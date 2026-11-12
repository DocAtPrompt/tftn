//! `cargo xtask sprachen [DATEI…]`: schreibt `sprachen/en.toml` neu, mit genau den Texten, die im
//! Code stehen (`tr!`, `t`, `n`, `ui/index.html`), gegliedert nach Datei. Vorhandene Übersetzungen
//! bleiben; weitere kommen aus den angegebenen Dateien (Abschnitt `[texte]` oder flach). Neue Texte
//! stehen leer drin, bis jemand sie übersetzt; der Test `englisch_ist_vollstaendig` besteht erst dann.

use std::collections::HashMap;
use std::path::Path;

use tftn_sprachen::quellen::{bestand, sprachdatei};

const KOPF: &str =
    "TFTN – Oberfläche auf Englisch. Links steht der deutsche Text aus dem Programm, rechts die
Übersetzung. {name} ist ein Platzhalter und bleibt stehen. Fehlt ein Eintrag oder ist er leer,
erscheint der deutsche Text.

Eine weitere Sprache: diese Datei kopieren und als <kennung>.toml (etwa fr.toml) in den Ordner
„sprachen“ im Einstellungsordner legen, den Namen unter [sprache] und die rechten Seiten
übersetzen. Einstellungen → Sprache der Oberfläche bietet sie dann an.

Für Entwickler: Nach Änderungen an Texten `cargo xtask sprachen` ausführen und die leeren
Einträge übersetzen.";

fn texte_lesen(pfad: &Path) -> Result<HashMap<String, String>, String> {
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    let t: toml::Table = toml::from_str(&text).map_err(|e| format!("{}: {e}", pfad.display()))?;
    let tabelle = t.get("texte").and_then(|x| x.as_table()).unwrap_or(&t);
    Ok(tabelle
        .iter()
        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
        .filter(|(_, v)| !v.trim().is_empty())
        .collect())
}

pub fn sprachen(dazu: &[String]) -> Result<(), String> {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let ziel = wurzel.join("sprachen/en.toml");
    let mut alt = if ziel.exists() {
        texte_lesen(&ziel)?
    } else {
        HashMap::new()
    };
    for d in dazu {
        for (k, v) in texte_lesen(Path::new(d))? {
            alt.entry(k).or_insert(v);
        }
    }
    let b = bestand(&wurzel);
    let text = sprachdatei(KOPF, "English", &b, &alt);
    std::fs::create_dir_all(wurzel.join("sprachen")).map_err(|e| e.to_string())?;
    std::fs::write(&ziel, text).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let leer = b.texte.keys().filter(|k| !alt.contains_key(*k)).count();
    println!(
        "sprachen/en.toml: {} Texte, davon {leer} noch nicht übersetzt.",
        b.texte.len()
    );
    for o in &b.offen {
        println!("  t(…) ohne festen Text: {o}");
    }
    Ok(())
}
