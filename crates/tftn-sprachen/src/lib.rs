//! Sprache der Oberfläche.
//!
//! Deutsch ist die Quelle: Jeder Text steht deutsch im Code, in Rust als `tr!("…")`, in der
//! Oberfläche als `t("…")`. Jede andere Sprache ist eine Datei `<kennung>.toml`:
//!
//! ```toml
//! [sprache]
//! name = "English"
//!
//! [texte]
//! "Die DOI {doi} kennen weder Crossref noch DataCite." = "Neither Crossref nor DataCite know the DOI {doi}."
//! ```
//!
//! `{name}` ist ein Platzhalter und bleibt in der Übersetzung stehen. Fehlt ein Text oder ist er
//! leer, bleibt er deutsch. Mitgeliefert wird `sprachen/en.toml`; eigene Dateien im Ordner
//! `sprachen` des Einstellungsordners kommen dazu und haben Vorrang.

pub mod quellen;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Dateiname, den der Lader übergeht: die Vorlage für eine neue Sprache.
pub const VORLAGE: &str = "vorlage";

/// Eine geladene Sprache.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Katalog {
    pub kennung: String,
    pub name: String,
    pub texte: HashMap<String, String>,
    /// Woher er kommt (für den Holer, der ihn selbst liest).
    pub pfad: Option<PathBuf>,
}

impl Katalog {
    /// Liest eine Sprachdatei. Leere Übersetzungen fallen weg (bleiben also deutsch).
    pub fn lesen(kennung: &str, text: &str) -> Result<Katalog, String> {
        let wert: toml::Table = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let name = wert
            .get("sprache")
            .and_then(|s| s.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or(kennung)
            .to_string();
        let mut texte = HashMap::new();
        if let Some(t) = wert.get("texte").and_then(|t| t.as_table()) {
            for (quelle, ziel) in t {
                match ziel.as_str() {
                    Some(z) if !z.trim().is_empty() => {
                        texte.insert(quelle.clone(), z.to_string());
                    }
                    _ => {}
                }
            }
        }
        Ok(Katalog {
            kennung: kennung.to_string(),
            name,
            texte,
            pfad: None,
        })
    }

    pub fn laden(pfad: &Path) -> Result<Katalog, String> {
        let kennung = pfad
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
        let mut k =
            Katalog::lesen(kennung, &text).map_err(|e| format!("{}: {e}", pfad.display()))?;
        k.pfad = Some(pfad.to_path_buf());
        Ok(k)
    }
}

/// Eine Sprache, die man wählen kann.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprache {
    pub kennung: String,
    pub name: String,
    pub pfad: PathBuf,
}

/// Die Sprachdateien in `ordner` (spätere haben Vorrang), nach Kennung sortiert. Deutsch ist immer
/// da und hat keine Datei; `de.toml` und die Vorlage werden übergangen, unlesbare Dateien auch.
pub fn verfuegbar(ordner: &[PathBuf]) -> Vec<Sprache> {
    let mut nach_kennung = std::collections::BTreeMap::new();
    for o in ordner {
        let Ok(eintraege) = std::fs::read_dir(o) else {
            continue;
        };
        let mut pfade: Vec<PathBuf> = eintraege.flatten().map(|e| e.path()).collect();
        pfade.sort();
        for p in pfade {
            if p.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(kennung) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if kennung == "de" || kennung == VORLAGE {
                continue;
            }
            if let Ok(k) = Katalog::laden(&p) {
                nach_kennung.insert(
                    kennung.to_string(),
                    Sprache {
                        kennung: kennung.to_string(),
                        name: k.name,
                        pfad: p,
                    },
                );
            }
        }
    }
    nach_kennung.into_values().collect()
}

/// Welche Sprache gilt: `wunsch` aus den Einstellungen (`None` = wie das System), sonst die des
/// Systems (`de-AT` → `de`). Deutsch heißt `None`. Spricht das System eine Sprache, für die es
/// keine Datei gibt, gilt Englisch, wenn es das gibt.
pub fn waehlen<'a>(wunsch: Option<&str>, system: &str, da: &'a [Sprache]) -> Option<&'a Sprache> {
    let finden = |k: &str| {
        da.iter()
            .find(|s| s.kennung.eq_ignore_ascii_case(k))
            .or_else(|| {
                let haupt = k.split(['-', '_']).next().unwrap_or(k);
                da.iter().find(|s| s.kennung.eq_ignore_ascii_case(haupt))
            })
    };
    if let Some(w) = wunsch.filter(|w| !w.is_empty()) {
        if w == "de" {
            return None;
        }
        if let Some(s) = finden(w) {
            return Some(s);
        }
    }
    let haupt = system.split(['-', '_']).next().unwrap_or("").to_lowercase();
    if haupt == "de" {
        return None;
    }
    finden(system).or_else(|| finden("en"))
}

static AKTUELL: RwLock<Option<Arc<Katalog>>> = RwLock::new(None);

/// Setzt die geltende Sprache für diesen Prozess (`None` = Deutsch).
pub fn setzen(katalog: Option<Katalog>) {
    *AKTUELL.write().unwrap_or_else(|e| e.into_inner()) = katalog.map(Arc::new);
}

/// Die geltende Sprache (`None` = Deutsch).
pub fn aktuell() -> Option<Arc<Katalog>> {
    AKTUELL.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Kennung der geltenden Sprache, `de` ohne Katalog.
pub fn kennung() -> String {
    aktuell().map_or_else(|| "de".into(), |k| k.kennung.clone())
}

pub fn ist_deutsch() -> bool {
    aktuell().is_none()
}

/// Der Text in der geltenden Sprache; ohne Übersetzung bleibt er deutsch.
pub fn uebersetzen(quelle: &str) -> String {
    aktuell()
        .and_then(|k| k.texte.get(quelle).cloned())
        .unwrap_or_else(|| quelle.to_string())
}

/// Setzt die Werte für `{name}` ein. Unbekannte Platzhalter bleiben stehen.
pub fn einsetzen(vorlage: &str, werte: &[(&str, String)]) -> String {
    let mut aus = String::with_capacity(vorlage.len());
    let mut rest = vorlage;
    while let Some(auf) = rest.find('{') {
        aus.push_str(&rest[..auf]);
        let nach = &rest[auf + 1..];
        match nach.find('}') {
            Some(zu) if quellen::ist_name(&nach[..zu]) => {
                let name = &nach[..zu];
                match werte.iter().find(|(n, _)| *n == name) {
                    Some((_, w)) => aus.push_str(w),
                    None => {
                        aus.push('{');
                        aus.push_str(name);
                        aus.push('}');
                    }
                }
                rest = &nach[zu + 1..];
            }
            _ => {
                aus.push('{');
                rest = nach;
            }
        }
    }
    aus.push_str(rest);
    aus
}

/// Text in der geltenden Sprache: `tr!("Datei {name} fehlt.", name = pfad.display())`;
/// `tr!("… {doi} …", doi)` ist kurz für `doi = doi`. Werte brauchen `Display`.
#[macro_export]
macro_rules! tr {
    ($quelle:literal $(,)?) => {
        $crate::uebersetzen($quelle)
    };
    ($quelle:literal, $($name:ident $(= $wert:expr)?),+ $(,)?) => {
        $crate::einsetzen(
            &$crate::uebersetzen($quelle),
            &[$((stringify!($name), $crate::tr!(@wert $name $(= $wert)?))),+],
        )
    };
    (@wert $name:ident) => {
        ($name).to_string()
    };
    (@wert $name:ident = $wert:expr) => {
        ($wert).to_string()
    };
}
