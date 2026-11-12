//! Rechtschreibung nach Dokumentsprache: Wörterbuch wählen und halten, eigene Wörter in
//! `<Einstellungsordner>/woerter.txt` (ein Wort je Zeile, synchronisiert wie `tasten.toml`).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tftn_edit::rechtschreibung::{Pruefer, woerterbuch_fuer};
use tftn_sprachen::tr;

pub const EIGENE: &str = "woerter.txt";

/// Mitgelieferte Wörterbücher: im Entwicklungsbau `woerterbuecher/` im Repo, sonst im App-Bündel.
pub fn ordner(resource_dir: Option<PathBuf>) -> PathBuf {
    match resource_dir {
        Some(r) if !cfg!(debug_assertions) => r.join("woerterbuecher"),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../woerterbuecher"),
    }
}

/// Anzeige für die Statuszeile.
pub fn bezeichnung(lang: &str, region: Option<&str>) -> String {
    match woerterbuch_fuer(lang, region) {
        Some("de") => tr!("Deutsch"),
        Some("de-AT") => tr!("Deutsch (Österreich)"),
        Some("de-CH") => tr!("Deutsch (Schweiz)"),
        Some("en") => tr!("Englisch (USA)"),
        Some("en-GB") => tr!("Englisch (Großbritannien)"),
        Some("fr") => tr!("Französisch"),
        Some("it") => tr!("Italienisch"),
        Some("es") => tr!("Spanisch"),
        _ => tr!("„{lang}“: kein Wörterbuch", lang),
    }
}

/// Eigene Wörter; ohne Datei keine. Leere Zeilen und `#`-Zeilen zählen nicht.
pub fn eigene_lesen(einstellungen: &Path) -> Vec<String> {
    std::fs::read_to_string(einstellungen.join(EIGENE))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|z| !z.is_empty() && !z.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Änderungszeit und Größe von `woerter.txt` (Größe für Dateisysteme mit grober Zeit).
fn zeitpunkt(einstellungen: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(einstellungen.join(EIGENE)).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// Hängt `wort` an, falls es noch nicht drinsteht.
pub fn eigenes_schreiben(einstellungen: &Path, wort: &str) -> Result<(), String> {
    let wort = wort.trim();
    if wort.is_empty() || wort.contains(char::is_whitespace) {
        return Err(tr!("„{wort}“ ist kein einzelnes Wort.", wort));
    }
    if eigene_lesen(einstellungen).iter().any(|w| w == wort) {
        return Ok(());
    }
    let pfad = einstellungen.join(EIGENE);
    let mut text = std::fs::read_to_string(&pfad).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(wort);
    text.push('\n');
    std::fs::create_dir_all(einstellungen).map_err(|e| e.to_string())?;
    std::fs::write(&pfad, text).map_err(|e| format!("{}: {e}", pfad.display()))
}

/// Das zuletzt gebrauchte Wörterbuch samt eigener Wörter.
pub struct Stand {
    kennung: &'static str,
    einstellungen: PathBuf,
    eigene_zeit: Option<(SystemTime, u64)>,
    pub pruefer: Pruefer,
}

impl Stand {
    /// Wiederverwenden, solange Sprache, Einstellungsordner und `woerter.txt` gleich sind; sonst neu laden.
    pub fn holen(
        alt: &mut Option<Stand>,
        woerterbuecher: &Path,
        einstellungen: &Path,
        kennung: &'static str,
    ) -> Result<(), String> {
        let zeit = zeitpunkt(einstellungen);
        if let Some(s) = alt
            && s.kennung == kennung
            && s.einstellungen == einstellungen
            && s.eigene_zeit == zeit
        {
            return Ok(());
        }
        let mut pruefer = Pruefer::laden(woerterbuecher, kennung)?;
        for w in eigene_lesen(einstellungen) {
            pruefer.aufnehmen(&w);
        }
        *alt = Some(Stand {
            kennung,
            einstellungen: einstellungen.to_path_buf(),
            eigene_zeit: zeit,
            pruefer,
        });
        Ok(())
    }

    /// Wort aufnehmen: in `woerter.txt` und sofort in den Prüfer.
    pub fn aufnehmen(&mut self, wort: &str) -> Result<(), String> {
        eigenes_schreiben(&self.einstellungen, wort)?;
        self.pruefer.aufnehmen(wort.trim());
        self.eigene_zeit = zeitpunkt(&self.einstellungen);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordner_fuer(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-rs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        o
    }

    #[test]
    fn eigene_woerter_anhaengen_ohne_doppelte() {
        let o = ordner_fuer("anhaengen");
        assert!(eigene_lesen(&o).is_empty());
        eigenes_schreiben(&o, "Typstdatei").unwrap();
        eigenes_schreiben(&o, " Typstdatei ").unwrap();
        eigenes_schreiben(&o, "Lüddecke").unwrap();
        assert_eq!(eigene_lesen(&o), ["Typstdatei", "Lüddecke"]);
        assert!(eigenes_schreiben(&o, "zwei Wörter").is_err());
        assert!(eigenes_schreiben(&o, "").is_err());
        std::fs::write(o.join(EIGENE), "# Kommentar\nohne-Zeilenende").unwrap();
        eigenes_schreiben(&o, "neu").unwrap();
        assert_eq!(eigene_lesen(&o), ["ohne-Zeilenende", "neu"]);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn stand_laedt_neu_wenn_woerter_txt_sich_aendert() {
        let o = ordner_fuer("stand");
        let wb = ordner(None);
        let mut s = None;
        Stand::holen(&mut s, &wb, &o, "de-AT").unwrap();
        assert!(!s.as_ref().unwrap().pruefer.richtig("Lüddecke"));
        s.as_mut().unwrap().aufnehmen("Lüddecke").unwrap();
        assert!(s.as_ref().unwrap().pruefer.richtig("Lüddecke"));
        // von außen (Sync) geschrieben
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(o.join(EIGENE), "Häusler\n").unwrap();
        Stand::holen(&mut s, &wb, &o, "de-AT").unwrap();
        let p = &s.as_ref().unwrap().pruefer;
        assert!(p.richtig("Häusler"));
        assert!(!p.richtig("Lüddecke"));
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn bezeichnungen() {
        assert_eq!(bezeichnung("de", Some("AT")), "Deutsch (Österreich)");
        assert_eq!(bezeichnung("en", Some("GB")), "Englisch (Großbritannien)");
        assert_eq!(bezeichnung("pt", None), "„pt“: kein Wörterbuch");
    }
}
