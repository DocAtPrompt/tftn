//! Rundgänge (ui/src/rundgang.ts): eine frische Kopie eines Beispieldokuments, in der eine
//! Vorführung tippen darf. Sie entsteht aus der Vorlage eines mitgelieferten Dokumenttyps (wie
//! „Neu → Aus Vorlage“, ohne eingebundene Schriften) in `<temp>/tftn-vorfuehrung-<zeit>/<Name>`, die
//! Platzhalter der Vorlage werden durch erfundene Daten ersetzt. `demo/` liegt nicht im Bündel und
//! bringt 6,8 MB Schriften mit; die Vorlagen liegen dort schon und reichen.
//!
//! Beim nächsten Anlegen werden ältere Vorführ-Ordner gelöscht – nur die eigenen (`PRAEFIX`) und
//! nie der, in dem das offene Dokument liegt. Eine Kopie kommt nicht in „Zuletzt geöffnet“
//! (`ist_vorfuehrung`, geprüft in `oeffnen`).

use std::path::{Path, PathBuf};

use crate::typen::{self, Typquelle};

/// Anfang des Ordnernamens jeder Vorführung im Temp-Ordner.
pub const PRAEFIX: &str = "tftn-vorfuehrung-";

/// Dokumenttyp, wenn die Oberfläche keinen nennt.
pub const STANDARD_TYP: &str = "tftn-fachartikel";

/// Platzhalter der Vorlagen (deutsch und englisch) → erfundene Daten. Längere zuerst, damit
/// „Dr. Vorname Nachname“ (Ärztin, Arzt) einen eigenen Namen bekommt.
const ERSATZ: &[(&str, &str)] = &[
    ("\"Dr. Vorname Nachname\"", "\"Dr. Max Sample\""),
    ("\"Dr First Last\"", "\"Dr Max Sample\""),
    ("\"Vorname Nachname\"", "\"Erika Sample\""),
    ("\"First Last\"", "\"Erika Sample\""),
    ("\"Institution\"", "\"Dr. Sample Practice\""),
    ("\"Titel des Artikels\"", "\"Schlaf und Schichtarbeit\""),
    ("\"Title of the article\"", "\"Sleep and Shift Work\""),
    (
        "\"Titel des Berichts\"",
        "\"Jahresbericht der Sample-Praxis\"",
    ),
    (
        "\"Title of the report\"",
        "\"Annual Report of the Sample Practice\"",
    ),
];

/// Ersetzt die Platzhalter in einer `dokument.toml` (nur ganze, gequotete Werte).
pub fn erfundene_daten(toml: &str) -> String {
    ERSATZ
        .iter()
        .fold(toml.to_string(), |t, (alt, neu)| t.replace(alt, neu))
}

/// Liegt `pfad` in einem Vorführ-Ordner unter `temp`?
pub fn ist_vorfuehrung(pfad: &Path, temp: &Path) -> bool {
    let echt = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let (pfad, temp) = (echt(pfad), echt(temp));
    pfad.strip_prefix(&temp)
        .ok()
        .and_then(|rest| rest.components().next())
        .is_some_and(|c| c.as_os_str().to_string_lossy().starts_with(PRAEFIX))
}

/// Löscht die Vorführ-Ordner unter `temp`, außer dem, in dem `ausser` liegt. Gibt die Zahl zurück.
pub fn aufraeumen(temp: &Path, ausser: Option<&Path>) -> usize {
    let Ok(eintraege) = std::fs::read_dir(temp) else {
        return 0;
    };
    let ausser = ausser.map(|a| a.canonicalize().unwrap_or_else(|_| a.to_path_buf()));
    let mut weg = 0;
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name();
        if !name.to_string_lossy().starts_with(PRAEFIX) || !e.path().is_dir() {
            continue;
        }
        let ordner = e.path().canonicalize().unwrap_or_else(|_| e.path());
        if ausser.as_deref().is_some_and(|a| a.starts_with(&ordner)) {
            continue;
        }
        if std::fs::remove_dir_all(&ordner).is_ok() {
            weg += 1;
        }
    }
    weg
}

/// Räumt alte Vorführungen weg und legt `<temp>/tftn-vorfuehrung-<zeit>/<name>` aus der Vorlage von
/// `typ` an (Sprache wie die Oberfläche), mit erfundenen Daten. Liefert den Dokumentordner.
pub fn anlegen(
    quellen: &[Typquelle],
    typ: &str,
    temp: &Path,
    zeit: u128,
    name: &str,
    sprache: Option<&str>,
    offen: Option<&Path>,
) -> Result<PathBuf, String> {
    aufraeumen(temp, offen);
    let eltern = temp.join(format!("{PRAEFIX}{zeit}"));
    let ziel = typen::anlegen(quellen, typ, &eltern, name, false, sprache).inspect_err(|_| {
        let _ = std::fs::remove_dir_all(&eltern);
    })?;
    let daten = ziel.join("dokument.toml");
    if let Ok(text) = std::fs::read_to_string(&daten) {
        std::fs::write(&daten, erfundene_daten(&text))
            .map_err(|e| format!("{}: {e}", daten.display()))?;
    }
    Ok(ziel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typen_ordner() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../types")
    }

    /// Eigener Test-Temp-Ordner: nie die echten Vorführungen der laufenden App anfassen.
    fn temp(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!(
            "tftn-app-vorfuehrung-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        o
    }

    #[test]
    fn alle_vorlagen_bekommen_erfundene_namen_und_bleiben_gueltig() {
        let mut gesehen = 0;
        for typ in std::fs::read_dir(typen_ordner())
            .unwrap()
            .filter_map(Result::ok)
        {
            for vorlage in ["vorlage", "vorlage-en"] {
                let Ok(text) =
                    std::fs::read_to_string(typ.path().join(vorlage).join("dokument.toml"))
                else {
                    continue;
                };
                gesehen += 1;
                let neu = erfundene_daten(&text);
                assert!(
                    toml::from_str::<toml::Table>(&neu).is_ok(),
                    "{:?}/{vorlage}",
                    typ.file_name()
                );
                for platzhalter in ["Vorname Nachname", "First Last", "\"Institution\""] {
                    assert!(
                        !neu.contains(platzhalter),
                        "{:?}: {platzhalter}",
                        typ.file_name()
                    );
                }
                assert!(!neu.contains("Dr. Dr."));
            }
        }
        assert!(gesehen >= 10, "nur {gesehen} Vorlagen gefunden");
    }

    #[test]
    fn erkennt_nur_eigene_ordner_im_temp() {
        let t = temp("erkennen");
        let drin = t.join(format!("{PRAEFIX}1")).join("Beispiel");
        std::fs::create_dir_all(&drin).unwrap();
        assert!(ist_vorfuehrung(&drin, &t));
        assert!(ist_vorfuehrung(&t.join(format!("{PRAEFIX}1")), &t));
        let fremd = t.join("tftn-anderes").join("Beispiel");
        std::fs::create_dir_all(&fremd).unwrap();
        assert!(!ist_vorfuehrung(&fremd, &t));
        assert!(!ist_vorfuehrung(Path::new("/Users/x/Dokumente/Brief"), &t));
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn legt_kopie_an_und_raeumt_alte_auf_ausser_der_offenen() {
        let t = temp("anlegen");
        let q = typen::quellen(&typen_ordner(), &t.join("keine-einstellungen"));
        // Zwei alte Vorführungen, in der zweiten ist gerade ein Dokument offen; dazu ein fremder Ordner.
        let alt = t.join(format!("{PRAEFIX}1"));
        let offen = t.join(format!("{PRAEFIX}2")).join("Beispiel");
        let fremd = t.join("tftn-etwas-anderes");
        for o in [&alt, &offen, &fremd] {
            std::fs::create_dir_all(o).unwrap();
        }
        let ziel = anlegen(
            &q,
            STANDARD_TYP,
            &t,
            3,
            "Beispiel",
            Some("de"),
            Some(&offen),
        )
        .unwrap();
        assert!(!alt.exists(), "alte Vorführung gelöscht");
        assert!(offen.exists(), "die offene bleibt");
        assert!(fremd.exists(), "fremde Ordner bleiben");
        assert!(ziel.join("tftn.toml").is_file() && ziel.join("main.typ").is_file());
        assert!(ist_vorfuehrung(&ziel, &t));
        assert!(!ziel.join("fonts").exists(), "ohne eingebundene Schriften");
        let daten = std::fs::read_to_string(ziel.join("dokument.toml")).unwrap();
        assert!(
            daten.contains("Erika Sample") && daten.contains("Sample Practice"),
            "{daten}"
        );
        // Nächstes Anlegen räumt die vorige weg.
        let zweite = anlegen(&q, STANDARD_TYP, &t, 4, "Beispiel", Some("en"), None).unwrap();
        assert!(!ziel.exists() && !offen.exists());
        let en = std::fs::read_to_string(zweite.join("dokument.toml")).unwrap();
        assert!(en.contains("Sleep and Shift Work"), "{en}");
        // Unbekannter Typ: Fehler, kein halber Ordner.
        assert!(anlegen(&q, "gibt-es-nicht", &t, 5, "Beispiel", None, Some(&zweite)).is_err());
        assert!(!t.join(format!("{PRAEFIX}5")).exists());
        let _ = std::fs::remove_dir_all(&t);
    }
}
