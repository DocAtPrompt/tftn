//! Sprache der Oberfläche: mitgelieferte Dateien (`sprachen/` im Bündel) und eigene im Ordner
//! `sprachen` des Einstellungsordners; die eigenen haben Vorrang.

use std::path::{Path, PathBuf};

use tftn_sprachen::{Katalog, Sprache, VORLAGE};

/// Mitgelieferte Sprachdateien; im Entwicklungsbau direkt aus dem Quellbaum.
pub fn mitgeliefert(resource_dir: Option<PathBuf>) -> PathBuf {
    match resource_dir {
        Some(r) if !cfg!(debug_assertions) => r.join("sprachen"),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../sprachen"),
    }
}

/// Ordner für eigene Sprachdateien.
pub fn eigene(einstellungsordner: &Path) -> PathBuf {
    einstellungsordner.join("sprachen")
}

pub fn verfuegbar(mitgeliefert: &Path, eigene: &Path) -> Vec<Sprache> {
    tftn_sprachen::verfuegbar(&[mitgeliefert.to_path_buf(), eigene.to_path_buf()])
}

/// Der Katalog, der gilt (`None` = Deutsch). Eine unlesbare Datei heißt Deutsch.
pub fn laden(da: &[Sprache], wunsch: Option<&str>, system: &str) -> Option<Katalog> {
    let s = tftn_sprachen::waehlen(wunsch, system, da)?;
    Katalog::laden(&s.pfad).ok()
}

/// Legt `vorlage.toml` (Kopie der englischen Datei) im eigenen Ordner an, wenn es sie noch nicht
/// gibt, und gibt ihren Pfad zurück.
pub fn vorlage_anlegen(mitgeliefert: &Path, eigene: &Path) -> Result<PathBuf, String> {
    let ziel = eigene.join(format!("{VORLAGE}.toml"));
    if ziel.exists() {
        return Ok(ziel);
    }
    std::fs::create_dir_all(eigene).map_err(|e| format!("{}: {e}", eigene.display()))?;
    let text = std::fs::read_to_string(mitgeliefert.join("en.toml"))
        .map_err(|e| format!("{}: {e}", mitgeliefert.join("en.toml").display()))?;
    let text = text.replacen("name = \"English\"", "name = \"Neue Sprache\"", 1);
    std::fs::write(&ziel, text).map_err(|e| format!("{}: {e}", ziel.display()))?;
    Ok(ziel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mitgeliefert_ist_englisch_und_vorlage_wird_angelegt() {
        let mit = mitgeliefert(None);
        let t = std::env::temp_dir().join(format!("tftn-app-sprachen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        let eig = eigene(&t);
        let da = verfuegbar(&mit, &eig);
        assert!(
            da.iter().any(|s| s.kennung == "en" && s.name == "English"),
            "{da:?}"
        );
        assert_eq!(laden(&da, None, "de-AT"), None);
        assert_eq!(laden(&da, None, "en-GB").unwrap().kennung, "en");

        let v = vorlage_anlegen(&mit, &eig).unwrap();
        assert_eq!(v, eig.join("vorlage.toml"));
        let k = Katalog::laden(&v).unwrap();
        assert_eq!(k.name, "Neue Sprache");
        assert_eq!(
            verfuegbar(&mit, &eig).len(),
            da.len(),
            "die Vorlage ist keine Sprache"
        );
        std::fs::write(&v, "bearbeitet").unwrap();
        vorlage_anlegen(&mit, &eig).unwrap();
        assert_eq!(
            std::fs::read_to_string(&v).unwrap(),
            "bearbeitet",
            "nicht überschrieben"
        );
        let _ = std::fs::remove_dir_all(&t);
    }
}
