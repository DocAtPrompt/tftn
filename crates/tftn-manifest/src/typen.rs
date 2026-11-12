//! Woher die App Dokumenttypen kennt (Spec 9, PLAN-M2 AP8): aus dem eigenen Ordner im Tresor und aus
//! dem App-Bündel, in dieser Reihenfolge. Bei gleichem Namen gewinnt die frühere Quelle.

use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::Serialize;

use crate::{ManifestFehler, Typpaket};

/// Woher ein Typ kommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Herkunft {
    /// `<Einstellungsordner>/typen/` im Tresor
    Eigen,
    /// mit TFTN geliefert (App-Bündel bzw. `typen/` im Repo)
    Tftn,
}

impl Herkunft {
    fn ort(self) -> &'static str {
        match self {
            Herkunft::Eigen => "im eigenen Ordner",
            Herkunft::Tftn => "aus TFTN",
        }
    }
}

/// Ein Ordner mit Dokumenttypen (je Unterordner ein Paket).
#[derive(Debug, Clone)]
pub struct Typquelle {
    pub ordner: PathBuf,
    pub herkunft: Herkunft,
}

/// Ein wählbarer Dokumenttyp (Paket mit `[tool.tftn]` und `[template]`).
#[derive(Debug, Clone)]
pub struct TypEintrag {
    pub name: String,
    pub version: semver::Version,
    pub titel: String,
    pub beschreibung: Option<String>,
    pub braucht: Vec<String>,
    pub herkunft: Herkunft,
    pub ordner: PathBuf,
}

/// Unterordner von `ordner` mit einer `typst.toml`, sortiert.
fn pakete(ordner: &Path) -> Vec<PathBuf> {
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return Vec::new();
    };
    let mut aus: Vec<PathBuf> = eintraege
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("typst.toml").is_file())
        .collect();
    aus.sort();
    aus
}

/// Alle wählbaren Typen; bei gleichem Namen gewinnt die frühere Quelle. Warnungen nennen unlesbare
/// Typen und Überdeckungen.
pub fn verfuegbar(quellen: &[Typquelle]) -> (Vec<TypEintrag>, Vec<String>) {
    let mut liste: Vec<TypEintrag> = Vec::new();
    let mut warnungen = Vec::new();
    for q in quellen {
        for ordner in pakete(&q.ordner) {
            let ordnername = ordner
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let p = match Typpaket::lesen(&ordner) {
                Ok(p) => p,
                Err(e) => {
                    warnungen.push(tr!(
                        "Dokumenttyp {ordnername} {ort} nicht lesbar: {fehler}",
                        ordnername,
                        ort = q.herkunft.ort(),
                        fehler = e
                    ));
                    continue;
                }
            };
            let (Some(m), Some(_)) = (p.toml.tool.tftn.as_ref(), p.toml.template.as_ref()) else {
                continue; // Hilfspaket wie tftn-kern: kein wählbarer Typ
            };
            let name = p.toml.package.name.clone();
            let version = match semver::Version::parse(&p.toml.package.version) {
                Ok(v) => v,
                Err(e) => {
                    warnungen.push(tr!(
                        "Dokumenttyp {name} {ort}: Version ungültig ({fehler})",
                        name,
                        ort = q.herkunft.ort(),
                        fehler = e
                    ));
                    continue;
                }
            };
            if let Some(vorher) = liste.iter().find(|t| t.name == name) {
                warnungen.push(tr!(
                    "{name} {ort1} überdeckt {name} {version} {ort2}",
                    name,
                    ort1 = vorher.herkunft.ort(),
                    version,
                    ort2 = q.herkunft.ort()
                ));
                continue;
            }
            liste.push(TypEintrag {
                name,
                version,
                titel: m.titel.clone(),
                beschreibung: p.toml.package.description.clone(),
                braucht: m.braucht.clone(),
                herkunft: q.herkunft,
                ordner,
            });
        }
    }
    (liste, warnungen)
}

/// Wie `Typpaket::finden`, aber über alle Quellen: ohne Version die erste Quelle, die das Paket hat;
/// mit Version die erste, die genau diese Version hat.
pub fn finden_in(
    quellen: &[Typquelle],
    name: &str,
    version: Option<&str>,
) -> Result<Typpaket, ManifestFehler> {
    let mut letzter = None;
    for q in quellen {
        // Der Ordner `<name>` zuerst, dann jeder andere: Maßgeblich ist der Name in typst.toml, so
        // wie in `verfuegbar` (ein eigener Typ darf anders heißen als sein Ordner).
        let erst = q.ordner.join(name);
        let mut ordner: Vec<PathBuf> = pakete(&q.ordner)
            .into_iter()
            .filter(|o| *o != erst)
            .collect();
        if erst.join("typst.toml").is_file() {
            ordner.insert(0, erst);
        }
        for o in ordner {
            match Typpaket::lesen(&o) {
                Ok(p) if p.toml.package.name != name => {}
                Ok(p) if version.is_none_or(|v| p.toml.package.version == v) => return Ok(p),
                Ok(_) => {}
                Err(e) if o.file_name().is_some_and(|f| f == name) => letzter = Some(e),
                Err(_) => {}
            }
        }
    }
    Err(letzter.unwrap_or_else(|| {
        ManifestFehler::Inhalt(match version {
            Some(v) => tr!(
                "Paket {name} {v} in keiner Quelle der Dokumenttypen gefunden",
                name,
                v
            ),
            None => tr!(
                "Paket {name} in keiner Quelle der Dokumenttypen gefunden",
                name
            ),
        })
    }))
}
