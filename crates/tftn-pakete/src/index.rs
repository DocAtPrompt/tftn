//! Paketindex des Universe (`preview/index.json`) lesen und durchsuchen (Festlegungen 5 und 6).
//!
//! Das Format ist nicht festgeschrieben; gelesen wird nachsichtig. Ein Eintrag, der sich nicht
//! lesen lässt oder einen ungültigen Namen hat, wird übersprungen und gezählt.

use std::collections::{BTreeMap, BTreeSet};
use tftn_sprachen::tr;

use serde::Deserialize;

use crate::{PaketFehler, PaketSpec};

#[derive(Debug, Clone)]
pub struct IndexEintrag {
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub compiler: Option<semver::Version>,
}

#[derive(Deserialize)]
struct Roh {
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    compiler: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Index {
    pub eintraege: Vec<IndexEintrag>,
    /// Einträge, die sich nicht lesen ließen
    pub uebersprungen: usize,
}

impl Index {
    /// `Err` nur, wenn es kein JSON-Array ist. Doppelte Einträge (Name und Version) zählen einmal.
    pub fn lesen(json: &[u8]) -> Result<Index, PaketFehler> {
        let werte: Vec<serde_json::Value> = serde_json::from_slice(json).map_err(|e| {
            PaketFehler::Index(tr!("Paketindex nicht lesbar: {fehler}", fehler = e))
        })?;
        let mut index = Index::default();
        let mut gesehen = BTreeSet::new();
        for wert in werte {
            let Some(e) = Roh::deserialize(wert).ok().and_then(|r| eintrag(r).ok()) else {
                index.uebersprungen += 1;
                continue;
            };
            if gesehen.insert((e.name.clone(), e.version.clone())) {
                index.eintraege.push(e);
            }
        }
        Ok(index)
    }

    /// Je Name die höchste Version, deren `compiler` höchstens `typst` ist (fehlt er, zählt sie);
    /// alle Wörter der Anfrage müssen vorkommen. Leere Anfrage: keine Treffer.
    /// Die höchste Version von `name`, die `typst` übersetzen kann.
    pub fn neueste(&self, name: &str, typst: &semver::Version) -> Option<&IndexEintrag> {
        self.eintraege
            .iter()
            .filter(|e| e.name == name && e.compiler.as_ref().is_none_or(|c| c <= typst))
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    pub fn suchen(&self, anfrage: &str, typst: &semver::Version, max: usize) -> Vec<IndexEintrag> {
        let woerter: Vec<String> = anfrage.split_whitespace().map(str::to_lowercase).collect();
        if woerter.is_empty() {
            return Vec::new();
        }
        let ganz = woerter.join("-");
        let mut beste: BTreeMap<&str, &IndexEintrag> = BTreeMap::new();
        for e in &self.eintraege {
            if e.compiler.as_ref().is_some_and(|c| c > typst) {
                continue;
            }
            let b = beste.entry(&e.name).or_insert(e);
            if e.version > b.version {
                *b = e;
            }
        }
        let mut treffer: Vec<(u8, &IndexEintrag)> = beste
            .into_values()
            .filter_map(|e| {
                let name = e.name.to_lowercase();
                let beschreibung = e.description.as_deref().unwrap_or("").to_lowercase();
                let marken: Vec<String> = e
                    .keywords
                    .iter()
                    .chain(&e.categories)
                    .map(|k| k.to_lowercase())
                    .collect();
                let in_marken = |w: &str| marken.iter().any(|k| k.contains(w));
                let alle = woerter.iter().all(|w| {
                    name.contains(w.as_str()) || beschreibung.contains(w.as_str()) || in_marken(w)
                });
                if !alle {
                    return None;
                }
                let rang = if name == ganz {
                    0
                } else if name.starts_with(&ganz) {
                    1
                } else if name.contains(&ganz) {
                    2
                } else if woerter
                    .iter()
                    .any(|w| in_marken(w) || name.contains(w.as_str()))
                {
                    3
                } else {
                    4
                };
                Some((rang, e))
            })
            .collect();
        treffer.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
        treffer
            .into_iter()
            .take(max)
            .map(|(_, e)| e.clone())
            .collect()
    }
}

fn eintrag(r: Roh) -> Result<IndexEintrag, PaketFehler> {
    let spec = PaketSpec::neu(&r.name, &r.version)?;
    Ok(IndexEintrag {
        name: spec.name,
        version: spec.version,
        description: r.description,
        license: r.license,
        keywords: r.keywords,
        categories: r.categories,
        compiler: r.compiler.and_then(|c| semver::Version::parse(&c).ok()),
    })
}
