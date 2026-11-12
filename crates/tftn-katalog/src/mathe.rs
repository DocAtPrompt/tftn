//! Symbole der Formel-Palette (PLAN-M2 AP10, Festlegung 3). Sie sind keine Bausteine, damit Menü und
//! Kürzel-Tabelle überschaubar bleiben.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Symbol {
    /// wird eingefügt, z. B. `alpha`
    pub text: String,
    /// steht auf dem Knopf, z. B. `α`
    pub zeichen: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SymbolGruppe {
    pub name: String,
    #[serde(rename = "symbol")]
    pub symbole: Vec<Symbol>,
}

#[derive(Deserialize)]
struct Datei {
    gruppe: Vec<SymbolGruppe>,
}

/// Die mitgelieferten Symbolgruppen (`mathe.toml`).
pub fn symbole() -> Vec<SymbolGruppe> {
    let d: Datei =
        toml::from_str(include_str!("../mathe.toml")).expect("mathe.toml ist fehlerhaft");
    d.gruppe
}
