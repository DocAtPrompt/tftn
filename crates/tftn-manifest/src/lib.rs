//! Dokumenttyp-Pakete: `typst.toml` mit `[tool.tftn]` lesen (KONZEPT 5) und Projekte anlegen.
//!
//! M1 liest nur die Schlüssel, die das Anlegen braucht. Unbekannte Schlüssel werden
//! hingenommen (KONZEPT 5.1); die vollständige Prüfung kommt mit `tftn typ-pruefen` (M4).

pub mod aktualisieren;
pub mod anlegen;
pub mod typen;

use std::fmt;
use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};

/// `typst.toml` eines Dokumenttyp-Pakets.
#[derive(Debug, Clone, Deserialize)]
pub struct TypstToml {
    pub package: Paket,
    pub template: Option<Vorlage>,
    #[serde(default)]
    pub tool: Werkzeuge,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Paket {
    pub name: String,
    pub version: String,
    pub entrypoint: String,
    /// Kurzbeschreibung (Dialog „Neues Projekt“, PLAN-M2 AP8)
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vorlage {
    pub path: String,
    pub entrypoint: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Werkzeuge {
    pub tftn: Option<Manifest>,
}

/// `[tool.tftn]`, Version 1 (Auszug für M1).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub schema_version: u32,
    pub titel: String,
    pub achse: Achse,
    pub formular: Option<String>,
    pub daten: Option<String>,
    #[serde(default)]
    pub schriften: Vec<String>,
    /// Was man für diesen Typ mitbringen oder ausfüllen muss, in einfachen Worten.
    #[serde(default)]
    pub braucht: Vec<String>,
    #[serde(default)]
    pub baustein: Vec<Baustein>,
    pub literatur: Option<LiteraturManifest>,
    #[serde(default)]
    pub export: std::collections::BTreeMap<String, ExportManifest>,
}

/// `[tool.tftn.export.<id>]` (KONZEPT 5.6).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExportManifest {
    pub art: String,
    #[serde(default)]
    pub pdf_standards: Vec<String>,
    pub seiten: Option<String>,
    pub getaggt: Option<bool>,
    pub dateiname: Option<String>,
}

/// `[tool.tftn.literatur]` (KONZEPT 5.5).
#[derive(Debug, Clone, Deserialize)]
pub struct LiteraturManifest {
    #[serde(default)]
    pub dateien: Vec<String>,
    pub stil: Option<String>,
    pub quelle: Option<String>,
}

/// `[[tool.tftn.baustein]]` (KONZEPT 5.4).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Baustein {
    pub id: String,
    pub funktion: String,
    pub titel: String,
    pub einfuegen: String,
    #[serde(default)]
    pub parameter: std::collections::BTreeMap<String, Parameter>,
    /// Vorgeschlagene Tastenbelegung, z. B. `Mod-Alt-h` (PLAN-M2 AP3).
    pub kuerzel: Option<String>,
    /// Überschrift der Gruppe im Bausteinmenü.
    pub gruppe: Option<String>,
    /// Wo der Baustein eingefügt werden darf; ohne Angabe nur im Markup.
    #[serde(default = "nur_markup")]
    pub modus: Vec<Modus>,
}

/// Syntaxmodus an der Einfügestelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modus {
    Markup,
    Code,
    Mathe,
}

fn nur_markup() -> Vec<Modus> {
    vec![Modus::Markup]
}

/// Widget eines benannten Arguments (KONZEPT 5.4, Tabelle der Widget-Arten).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Parameter {
    pub widget: String,
    pub titel: Option<String>,
    pub hilfe: Option<String>,
    pub standard: Option<String>,
    pub raum: Option<String>,
    #[serde(default)]
    pub werte: Vec<String>,
    /// `text`: mehrzeiliges Eingabefeld
    #[serde(default)]
    pub mehrzeilig: bool,
    /// `zahl`, `prozent`
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub schritt: Option<f64>,
    /// `zahl`: nur ganze Zahlen
    #[serde(default)]
    pub ganzzahl: bool,
    /// `laenge`: erlaubte Einheiten (leer: pt, mm, cm, in, em)
    #[serde(default)]
    pub einheiten: Vec<String>,
    /// `wahl`: Anzeigenamen zu `werte`
    #[serde(default)]
    pub beschriftungen: Vec<String>,
    /// `label`: `zitat` oder `verweis`
    pub art: Option<String>,
    /// `datei`: erlaubte Endungen
    #[serde(default)]
    pub endungen: Vec<String>,
    /// `liste`: Widget-Art der Elemente (Standard `text`; schema-version 2)
    pub element: Option<String>,
    /// `inhalt`: der Rumpf `[…]` hinter der Argumentliste statt eines benannten Arguments
    #[serde(default)]
    pub rumpf: bool,
    /// Positionaler Parameter: Index unter den positionalen Argumenten (PLAN-M2 AP6)
    pub position: Option<usize>,
}

/// Widget-Arten, die erst mit `schema-version = 2` erlaubt sind (PLAN-M2 AP4).
pub const SCHEMA_V2_ARTEN: [&str; 4] = ["inhalt", "liste", "punkt", "tabelle"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Achse {
    Publikation,
    Formular,
}

/// Ein Paket aus dem Ordner `typen/`.
#[derive(Debug, Clone)]
pub struct Typpaket {
    pub ordner: PathBuf,
    pub toml: TypstToml,
}

#[derive(Debug)]
pub enum ManifestFehler {
    Lesen(PathBuf, std::io::Error),
    Format(PathBuf, String),
    Inhalt(String),
}

impl fmt::Display for ManifestFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lesen(p, e) => write!(f, "{}: {e}", p.display()),
            Self::Format(p, e) => f.write_str(&tr!(
                "{datei} ist fehlerhaft: {fehler}",
                datei = p.display(),
                fehler = e
            )),
            Self::Inhalt(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for ManifestFehler {}

impl Typpaket {
    /// Liest `<ordner>/typst.toml`.
    pub fn lesen(ordner: &Path) -> Result<Self, ManifestFehler> {
        let pfad = ordner.join("typst.toml");
        let text =
            std::fs::read_to_string(&pfad).map_err(|e| ManifestFehler::Lesen(pfad.clone(), e))?;
        let toml: TypstToml = toml::from_str(&text)
            .map_err(|e| ManifestFehler::Format(pfad.clone(), e.to_string()))?;
        if let Some(m) = &toml.tool.tftn {
            if !(1..=2).contains(&m.schema_version) {
                return Err(ManifestFehler::Format(
                    pfad,
                    tr!(
                        "schema-version {version} wird nicht unterstützt (erwartet 1 oder 2)",
                        version = m.schema_version
                    ),
                ));
            }
            if m.schema_version < 2 {
                for b in &m.baustein {
                    for (name, p) in &b.parameter {
                        if SCHEMA_V2_ARTEN.contains(&p.widget.as_str()) {
                            return Err(ManifestFehler::Format(
                                pfad,
                                tr!(
                                    "{baustein}.{name}: Widget-Art {widget} braucht schema-version 2",
                                    baustein = b.id,
                                    name,
                                    widget = p.widget
                                ),
                            ));
                        }
                    }
                }
            }
        }
        Ok(Self {
            ordner: ordner.to_path_buf(),
            toml,
        })
    }

    /// Sucht das Paket `name` im Ordner der Dokumenttypen; die Version muss passen, falls angegeben.
    pub fn finden(typen: &Path, name: &str, version: Option<&str>) -> Result<Self, ManifestFehler> {
        let paket = Self::lesen(&typen.join(name))?;
        if let Some(v) = version
            && paket.toml.package.version != v
        {
            return Err(ManifestFehler::Inhalt(tr!(
                "{name} liegt in Version {gefunden}, verlangt ist {v}",
                name,
                gefunden = paket.toml.package.version,
                v
            )));
        }
        Ok(paket)
    }

    /// `@local/<name>:<version>`
    pub fn spec(&self) -> String {
        format!(
            "@local/{}:{}",
            self.toml.package.name, self.toml.package.version
        )
    }

    /// Liest den Dokumenttyp eines Projekts aus seinem vendorten Paket (`tftn.toml` → `typ`).
    pub fn vom_projekt(projekt: &tftn_core::Projekt) -> Result<Self, ManifestFehler> {
        let typ = projekt.konfig.typ.as_deref().ok_or_else(|| {
            ManifestFehler::Inhalt(tr!("tftn.toml nennt keinen Dokumenttyp (typ = …)"))
        })?;
        let rest = typ.strip_prefix("@local/").ok_or_else(|| {
            ManifestFehler::Inhalt(tr!(
                "typ = {typ}: nur @local-Pakete werden unterstützt",
                typ
            ))
        })?;
        let (name, version) = rest
            .split_once(':')
            .ok_or_else(|| ManifestFehler::Inhalt(tr!("typ = {typ}: Version fehlt", typ)))?;
        Self::lesen(&projekt.pakete().join("local").join(name).join(version))
    }

    pub fn manifest(&self) -> Result<&Manifest, ManifestFehler> {
        self.toml.tool.tftn.as_ref().ok_or_else(|| {
            ManifestFehler::Inhalt(tr!(
                "{paket} hat keinen [tool.tftn]-Abschnitt",
                paket = self.spec()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baustein_ohne_neue_schluessel_ist_markup() {
        let b: Baustein = toml::from_str(
            r##"id = "x"
funktion = "x"
titel = "X"
einfuegen = "#x[${auswahl}]""##,
        )
        .unwrap();
        assert_eq!(b.modus, vec![Modus::Markup]);
        assert_eq!(b.kuerzel, None);
        assert_eq!(b.gruppe, None);
    }

    #[test]
    fn baustein_mit_neuen_schluesseln() {
        let b: Baustein = toml::from_str(
            r#"id = "x"
funktion = "x"
titel = "X"
einfuegen = "x"
kuerzel = "Mod-Alt-h"
gruppe = "Kästen"
modus = ["markup", "code"]"#,
        )
        .unwrap();
        assert_eq!(b.modus, vec![Modus::Markup, Modus::Code]);
        assert_eq!(b.kuerzel.as_deref(), Some("Mod-Alt-h"));
        assert_eq!(b.gruppe.as_deref(), Some("Kästen"));
    }

    #[test]
    fn fachartikel_liest_weiter() {
        let p = Typpaket::lesen(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typen/tftn-fachartikel"),
        )
        .unwrap();
        assert_eq!(p.manifest().unwrap().baustein[0].modus, vec![Modus::Markup]);
    }

    /// Legt `typst.toml` mit `[tool.tftn]` (Version `v`) und einem Baustein mit `parameter` an.
    fn paket(name: &str, v: u32, parameter: &str) -> Result<Typpaket, ManifestFehler> {
        let ordner = std::env::temp_dir()
            .join(format!("tftn-manifest-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&ordner);
        std::fs::create_dir_all(&ordner).unwrap();
        std::fs::write(
            ordner.join("typst.toml"),
            format!(
                r##"[package]
name = "{name}"
version = "0.1.0"
entrypoint = "lib.typ"

[tool.tftn]
schema-version = {v}
titel = "T"
achse = "publikation"

[[tool.tftn.baustein]]
id = "kasten"
funktion = "kasten"
titel = "Kasten"
einfuegen = "#kasten[]"
{parameter}
"##
            ),
        )
        .unwrap();
        Typpaket::lesen(&ordner)
    }

    #[test]
    fn schema_2_mit_neuen_arten() {
        let p = paket(
            "v2",
            2,
            "  [tool.tftn.baustein.parameter.body]\n  widget = \"inhalt\"\n  rumpf = true\n  [tool.tftn.baustein.parameter.worte]\n  widget = \"liste\"\n  element = \"text\"",
        )
        .unwrap();
        let b = &p.manifest().unwrap().baustein[0];
        assert!(b.parameter["body"].rumpf);
        assert_eq!(b.parameter["worte"].element.as_deref(), Some("text"));
    }

    #[test]
    fn schema_1_mit_neuer_art_ist_fehler() {
        let f = paket(
            "v1-inhalt",
            1,
            "  [tool.tftn.baustein.parameter.body]\n  widget = \"inhalt\"",
        )
        .unwrap_err();
        assert!(
            f.to_string()
                .contains("kasten.body: Widget-Art inhalt braucht schema-version 2"),
            "{f}"
        );
    }

    #[test]
    fn schema_3_ist_fehler() {
        let f = paket("v3", 3, "").unwrap_err();
        assert!(f.to_string().contains("erwartet 1 oder 2"), "{f}");
    }

    #[test]
    fn zusatzschluessel_zahl_laenge() {
        let p = paket(
            "zusatz",
            1,
            "  [tool.tftn.baustein.parameter.n]\n  widget = \"zahl\"\n  min = 0\n  max = 10.5\n  ganzzahl = true\n  [tool.tftn.baustein.parameter.b]\n  widget = \"laenge\"\n  einheiten = [\"pt\", \"mm\"]\n  [tool.tftn.baustein.parameter.d]\n  widget = \"datei\"\n  endungen = [\"png\"]",
        )
        .unwrap();
        let par = &p.manifest().unwrap().baustein[0].parameter;
        assert_eq!(
            (par["n"].min, par["n"].max, par["n"].ganzzahl),
            (Some(0.0), Some(10.5), true)
        );
        assert_eq!(par["b"].einheiten, ["pt", "mm"]);
        assert_eq!(par["d"].endungen, ["png"]);
    }
}
