//! Fach „Formel“ (PLAN-M2 AP10, Festlegungen 1–3): Stand am Cursor, eigene kleine Übersetzung für die
//! Vorschau, Palette. Die Patches kommen aus `tftn_edit`.

use serde::Serialize;
use tftn_core::vorschau::rastern;
use tftn_core::{ProjektWorld, kompilieren};
use tftn_edit::{MarkupStelle, formel_text, markup_stellen};
use tftn_manifest::Modus;
use tftn_sprachen::tr;
use typst::syntax::Source;

use crate::typen::png_url;

#[derive(Debug, Serialize)]
pub struct FormelBildJs {
    /// PNG als data-URL
    pub bild: Option<String>,
    pub fehler: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct FormelStandJs {
    /// Text der Formel am Cursor; `None`, wenn dort keine steht
    pub text: Option<String>,
    pub block: bool,
    /// Am Cursor darf eine neue Formel eingefügt werden (nur im Text).
    pub einfuegbar: bool,
}

#[derive(Debug, Serialize)]
pub struct AufbauJs {
    pub id: String,
    pub titel: String,
}

#[derive(Debug, Serialize)]
pub struct PaletteJs {
    pub aufbau: Vec<AufbauJs>,
    pub symbole: Vec<SymbolGruppeJs>,
}

#[derive(Debug, Serialize)]
pub struct SymbolJs {
    pub text: String,
    pub zeichen: String,
}

#[derive(Debug, Serialize)]
pub struct SymbolGruppeJs {
    pub name: String,
    pub symbole: Vec<SymbolJs>,
}

/// Quelltext der Vorschau: die Importe der Hauptdatei, eine Seite so groß wie die Formel, die Formel.
pub fn vorschau_quelltext(haupt: &str, text: &str, block: bool) -> String {
    let importe = tftn_katalog::obere_importe(haupt);
    let formel = tftn_edit::formel_quelltext(text, block);
    format!("{importe}#set page(width: auto, height: auto, margin: 4pt)\n{formel}\n")
}

/// Übersetzt die Formel in der World des Projekts; Fehler kommen als Text zurück.
pub fn vorschau(world: &mut ProjektWorld, haupt: &str, text: &str, block: bool) -> FormelBildJs {
    if text.trim().is_empty() {
        return FormelBildJs {
            bild: None,
            fehler: Vec::new(),
        };
    }
    world.zuruecksetzen();
    world.quelle_setzen(Source::new(
        world.haupt(),
        vorschau_quelltext(haupt, text, block),
    ));
    let e = kompilieren(world);
    let fehler: Vec<String> = e
        .meldungen
        .into_iter()
        .filter(|m| m.fehler)
        .map(|m| m.text)
        .collect();
    let bild = e
        .dokument
        .filter(|_| fehler.is_empty())
        .and_then(|d| d.pages().first().map(|s| png_url(&rastern(s, 144.0))));
    FormelBildJs { bild, fehler }
}

pub fn stand(quelle: &Source, byte: usize) -> FormelStandJs {
    let stelle = markup_stellen(quelle, byte)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Formel { .. }));
    FormelStandJs {
        text: stelle.as_ref().and_then(|s| formel_text(quelle, s)),
        block: matches!(stelle, Some(MarkupStelle::Formel { block: true, .. })),
        einfuegbar: stelle.is_none() && tftn_katalog::modus_an(quelle, byte) == Some(Modus::Markup),
    }
}

/// Patch für die Formel bei `byte`. `erwartet` ist der Text, den das Feld geladen hat.
pub fn setzen_patch(
    quelle: &Source,
    byte: usize,
    erwartet: &str,
    text: &str,
) -> Result<tftn_edit::Patch, String> {
    let stelle = markup_stellen(quelle, byte)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Formel { .. }))
        .ok_or_else(|| tr!("Hier steht keine Formel mehr. „Verwerfen“ lädt neu."))?;
    if formel_text(quelle, &stelle).as_deref() != Some(erwartet) {
        return Err(tr!(
            "Die Formel wurde inzwischen geändert. „Verwerfen“ lädt neu."
        ));
    }
    tftn_edit::formel_patch(quelle, &stelle, text).map_err(|e| e.to_string())
}

pub fn palette() -> PaletteJs {
    PaletteJs {
        aufbau: tftn_katalog::eingebaut()
            .into_iter()
            .filter(|e| e.modus.contains(&Modus::Mathe))
            .map(|e| AufbauJs {
                id: e.id,
                titel: e.titel,
            })
            .collect(),
        symbole: tftn_katalog::symbole()
            .into_iter()
            .map(|g| SymbolGruppeJs {
                name: g.name,
                symbole: g
                    .symbole
                    .into_iter()
                    .map(|s| SymbolJs {
                        text: s.text,
                        zeichen: s.zeichen,
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tftn_core::Projekt;

    fn world() -> ProjektWorld {
        let w = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        ProjektWorld::neu(&Projekt::oeffnen(&w).unwrap()).unwrap()
    }

    #[test]
    fn quelltext_nimmt_importe() {
        let t = "#import \"@local/a:0.1.0\": *\n#show: x\n= Kopf\n#import \"b.typ\"\nText";
        let q = vorschau_quelltext(t, "x^2", true);
        assert!(
            q.starts_with("#import \"@local/a:0.1.0\": *\n#import \"b.typ\"\n"),
            "{q}"
        );
        assert!(!q.contains("#show") && !q.contains("Kopf"), "{q}");
        assert!(q.contains("$ x^2 $"));
        assert!(vorschau_quelltext("", "x", false).contains("\n$x$\n"));
    }

    #[test]
    fn vorschau_liefert_bild() {
        let b = vorschau(&mut world(), "", "frac(a, b)", true);
        assert_eq!(b.fehler, Vec::<String>::new());
        assert!(b.bild.unwrap().starts_with("data:image/png"));
    }

    /// Abschlussprüfung AP10: Endet die Formel mit einem `//`-Kommentar, ist das kein Syntaxfehler.
    #[test]
    fn kommentar_am_ende() {
        let mut w = world();
        for block in [true, false] {
            let b = vorschau(&mut w, "", "x^2 // Kommentar", block);
            assert_eq!(b.fehler, Vec::<String>::new(), "block {block}");
            assert!(b.bild.is_some());
        }
    }

    #[test]
    fn vorschau_meldet_fehler() {
        let mut w = world();
        let b = vorschau(&mut w, "", "frac(a", true);
        assert!(b.bild.is_none() && !b.fehler.is_empty());
        let b = vorschau(&mut w, "", "unbekannt", false);
        assert!(b.bild.is_none() && !b.fehler.is_empty());
        // Danach geht es weiter, und Leeres ist kein Fehler.
        assert!(vorschau(&mut w, "", "x", false).bild.is_some());
        let b = vorschau(&mut w, "", "  ", false);
        assert!(b.bild.is_none() && b.fehler.is_empty());
    }

    #[test]
    fn stand_in_und_ausser_formel() {
        let t = "Text $ x $ mehr #{ 1 }";
        let q = Source::detached(t);
        assert_eq!(
            stand(&q, t.find("x $").unwrap()),
            FormelStandJs {
                text: Some("x".into()),
                block: true,
                einfuegbar: false
            }
        );
        assert_eq!(
            stand(&q, 2),
            FormelStandJs {
                text: None,
                block: false,
                einfuegbar: true
            }
        );
        assert!(!stand(&q, t.find('1').unwrap()).einfuegbar);
    }

    /// Abschlussprüfung AP10: Steht an der Stelle eine andere Formel, wird nichts geschrieben.
    #[test]
    fn setzen_lehnt_fremde_formel_ab() {
        let t = "$a$ und $b$ und $c$";
        let q = Source::detached(t);
        let p = setzen_patch(&q, t.find('b').unwrap(), "b", "y").unwrap();
        assert_eq!(tftn_edit::anwenden(t, &p), "$a$ und $y$ und $c$");
        let f = setzen_patch(&q, t.find('c').unwrap(), "b", "y").unwrap_err();
        assert!(f.contains("geändert"), "{f}");
        assert!(setzen_patch(&q, 4, "b", "y").is_err());
    }

    #[test]
    fn palette_vollstaendig() {
        let p = palette();
        assert_eq!(p.aufbau.len(), 18);
        assert_eq!(p.aufbau[0].id, "typst/mathe-bruch");
        assert_eq!(p.aufbau[17].id, "typst/mathe-zeile");
        assert!(p.symbole.iter().all(|g| !g.symbole.is_empty()));
    }
}
