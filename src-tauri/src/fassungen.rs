//! Frühere Fassungen: Beim Speichern legt TFTN höchstens alle zehn Minuten eine Kopie des Textes
//! in `<projekt>/.fassungen/<Unix-Sekunden>.typ` ab. Vom letzten Tag bleiben alle, davor eine je
//! Tag, nach 60 Tagen keine mehr. Zurückholen ersetzt den Text im Editor (ein Undo-Schritt).

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tftn_sprachen::tr;

pub const ORDNER: &str = ".fassungen";
const ABSTAND: u64 = 10 * 60;
const TAG: u64 = 24 * 60 * 60;
const TAGE: u64 = 60;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FassungJs {
    pub name: String,
    /// Unix-Sekunden
    pub zeit: u64,
    pub zeichen: usize,
    /// Gleicht dem Text jetzt.
    pub wie_jetzt: bool,
}

pub fn jetzt() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn pfad(ordner: &Path, zeit: u64) -> PathBuf {
    ordner.join(format!("{zeit}.typ"))
}

fn zeit_aus(name: &str) -> Option<u64> {
    let zahl = name.strip_suffix(".typ")?;
    let z: u64 = zahl.parse().ok()?;
    (z.to_string() == zahl).then_some(z)
}

/// Zeiten der vorhandenen Fassungen, aufsteigend.
fn zeiten(ordner: &Path) -> Vec<u64> {
    let mut z: Vec<u64> = std::fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| zeit_aus(&e.file_name().to_string_lossy()))
        .collect();
    z.sort_unstable();
    z
}

/// Welche Fassungen beim Ausdünnen wegfallen.
pub fn ausduennen(zeiten: &[u64], jetzt: u64) -> Vec<u64> {
    let mut tage = HashSet::new();
    let mut weg = Vec::new();
    for &z in zeiten.iter().rev() {
        let alter = jetzt.saturating_sub(z);
        if alter < TAG {
            continue;
        }
        if alter >= TAGE * TAG || !tage.insert(alter / TAG) {
            weg.push(z);
        }
    }
    weg
}

/// Legt `text` als Fassung ab, wenn er sich von der letzten unterscheidet und diese mindestens
/// zehn Minuten alt ist (`erzwingen`: auch früher). Liefert, ob eine Fassung entstand.
pub fn ablegen(ordner: &Path, text: &str, jetzt: u64, erzwingen: bool) -> io::Result<bool> {
    let vorhanden = zeiten(ordner);
    match vorhanden.last() {
        Some(&letzte) => {
            if std::fs::read_to_string(pfad(ordner, letzte)).is_ok_and(|t| t == text) {
                return Ok(false);
            }
            if !erzwingen && jetzt.saturating_sub(letzte) < ABSTAND {
                return Ok(false);
            }
        }
        None if text.trim().is_empty() => return Ok(false),
        None => {}
    }
    std::fs::create_dir_all(ordner)?;
    let mut zeit = jetzt.max(vorhanden.last().map_or(0, |l| l + 1));
    while pfad(ordner, zeit).exists() {
        zeit += 1;
    }
    std::fs::write(pfad(ordner, zeit), text)?;
    let mut alle = vorhanden;
    alle.push(zeit);
    for z in ausduennen(&alle, jetzt) {
        let _ = std::fs::remove_file(pfad(ordner, z));
    }
    Ok(true)
}

/// Alle Fassungen, die neueste zuerst.
pub fn liste(ordner: &Path, jetzt_text: &str) -> Vec<FassungJs> {
    zeiten(ordner)
        .into_iter()
        .rev()
        .filter_map(|z| {
            let text = std::fs::read_to_string(pfad(ordner, z)).ok()?;
            Some(FassungJs {
                name: format!("{z}.typ"),
                zeit: z,
                zeichen: text.chars().count(),
                wie_jetzt: text == jetzt_text,
            })
        })
        .collect()
}

/// Text einer Fassung; nur Namen, die TFTN selbst vergibt.
pub fn lesen(ordner: &Path, name: &str) -> Result<String, String> {
    let z = zeit_aus(name).ok_or_else(|| tr!("unbekannte Fassung"))?;
    std::fs::read_to_string(pfad(ordner, z))
        .map_err(|e| tr!("Fassung nicht lesbar: {fehler}", fehler = e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordner(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-fassungen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        o
    }

    const T0: u64 = 1_800_000_000;

    #[test]
    fn hoechstens_alle_zehn_minuten() {
        let o = ordner("abstand");
        assert!(ablegen(&o, "a", T0, false).unwrap());
        assert!(!ablegen(&o, "ab", T0 + 60, false).unwrap());
        assert!(ablegen(&o, "ab", T0 + ABSTAND, false).unwrap());
        assert_eq!(zeiten(&o), vec![T0, T0 + ABSTAND]);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn gleicher_text_keine_neue_fassung() {
        let o = ordner("gleich");
        assert!(ablegen(&o, "a", T0, false).unwrap());
        assert!(!ablegen(&o, "a", T0 + 2 * ABSTAND, true).unwrap());
        assert_eq!(zeiten(&o).len(), 1);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn erzwungen_auch_in_derselben_sekunde() {
        let o = ordner("erzwungen");
        assert!(ablegen(&o, "a", T0, false).unwrap());
        assert!(ablegen(&o, "b", T0, true).unwrap());
        assert_eq!(zeiten(&o), vec![T0, T0 + 1]);
        assert_eq!(lesen(&o, &format!("{}.typ", T0 + 1)).unwrap(), "b");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn leeres_dokument_ohne_fassung() {
        let o = ordner("leer");
        assert!(!ablegen(&o, "  \n", T0, false).unwrap());
        assert!(!o.exists());
    }

    #[test]
    fn ausduennen_letzter_tag_ganz_dann_eine_je_tag() {
        let jetzt = T0 + 100 * TAG;
        let heute = [jetzt - 60, jetzt - 3600, jetzt - TAG + 1];
        let gestern = [jetzt - TAG - 10, jetzt - TAG - 3600];
        let uralt = [jetzt - TAGE * TAG, jetzt - 90 * TAG];
        let mut alle: Vec<u64> = heute
            .iter()
            .chain(&gestern)
            .chain(&uralt)
            .copied()
            .collect();
        alle.sort_unstable();
        let mut weg = ausduennen(&alle, jetzt);
        weg.sort_unstable();
        // Von gestern bleibt die neuere, die ab 60 Tagen fallen weg.
        let mut erwartet = vec![jetzt - TAG - 3600, jetzt - TAGE * TAG, jetzt - 90 * TAG];
        erwartet.sort_unstable();
        assert_eq!(weg, erwartet);
    }

    #[test]
    fn liste_neueste_zuerst_mit_vergleich() {
        let o = ordner("liste");
        ablegen(&o, "alt", T0, false).unwrap();
        ablegen(&o, "neu", T0 + ABSTAND, false).unwrap();
        let l = liste(&o, "neu");
        assert_eq!(l.len(), 2);
        assert_eq!(
            (l[0].zeit, l[0].wie_jetzt, l[0].zeichen),
            (T0 + ABSTAND, true, 3)
        );
        assert!(!l[1].wie_jetzt);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn fremde_namen_abgelehnt() {
        let o = ordner("namen");
        ablegen(&o, "a", T0, false).unwrap();
        for name in ["../main.typ", "01800000000.typ", "x.typ", "1800000000"] {
            assert!(lesen(&o, name).is_err(), "{name}");
        }
        let _ = std::fs::remove_dir_all(&o);
    }
}
