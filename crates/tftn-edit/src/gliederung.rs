//! Gliederung: Überschriften der obersten Ebene und das Verschieben ganzer Abschnitte.
//!
//! Ein Abschnitt reicht vom Zeilenanfang seiner Überschrift bis zur nächsten Überschrift gleicher
//! oder höherer Ebene. Ein `#bibliography(…)` nach der letzten Überschrift gehört zu keinem
//! Abschnitt, es bleibt beim Verschieben am Ende.

use std::ops::Range;
use tftn_sprachen::tr;

use typst_syntax::ast::{self, AstNode};
use typst_syntax::{LinkedNode, Source, SyntaxKind, SyntaxNode};

use crate::{Patch, PatchFehler, fehler};

#[derive(Debug, Clone, PartialEq)]
pub struct Ueberschrift {
    pub ebene: usize,
    pub titel: String,
    /// Bytes: Anfang der Überschrift
    pub von: usize,
    /// Bytes: ganzer Abschnitt samt Unterabschnitten, ab Zeilenanfang
    pub abschnitt: Range<usize>,
}

fn zeilenanfang(text: &str, pos: usize) -> usize {
    let vor = &text[..pos];
    let ohne = vor.trim_end_matches([' ', '\t']);
    if ohne.is_empty() || ohne.ends_with('\n') {
        ohne.len()
    } else {
        pos
    }
}

fn titel(knoten: &SyntaxNode, aus: &mut String) {
    match knoten.kind() {
        SyntaxKind::Text | SyntaxKind::SmartQuote | SyntaxKind::Shorthand => {
            aus.push_str(knoten.leaf_text())
        }
        SyntaxKind::Space => aus.push(' '),
        SyntaxKind::Escape => aus.push_str(knoten.leaf_text().trim_start_matches('\\')),
        SyntaxKind::Label | SyntaxKind::Str | SyntaxKind::Ident => {}
        _ => {
            for k in knoten.children() {
                titel(k, aus);
            }
        }
    }
}

/// Anfang des Schlussteils (Literaturverzeichnis nach der letzten Überschrift), sonst Textende.
fn schluss(wurzel: &LinkedNode, text: &str, letzte: usize) -> usize {
    wurzel
        .children()
        .filter(|k| k.offset() > letzte)
        .find(|k| {
            k.cast::<ast::FuncCall>().is_some_and(
                |f| matches!(f.callee(), ast::Expr::Ident(i) if i.as_str() == "bibliography"),
            )
        })
        .map_or(text.len(), |k| {
            let o = k.offset();
            zeilenanfang(text, if text[..o].ends_with('#') { o - 1 } else { o })
        })
}

/// Die Überschriften der obersten Ebene in Textreihenfolge.
pub fn gliederung(quelle: &Source) -> Vec<Ueberschrift> {
    let text = quelle.text();
    let wurzel = LinkedNode::new(quelle.root());
    let kopf: Vec<(usize, String, usize)> = wurzel
        .children()
        .filter_map(|k| {
            let h = k.cast::<ast::Heading>()?;
            let mut t = String::new();
            titel(h.body().to_untyped(), &mut t);
            let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
            Some((h.depth().get(), t, k.offset()))
        })
        .collect();
    let ende = kopf
        .last()
        .map_or(text.len(), |l| schluss(&wurzel, text, l.2));
    kopf.iter()
        .enumerate()
        .map(|(i, (ebene, t, von))| {
            let bis = kopf[i + 1..]
                .iter()
                .find(|n| n.0 <= *ebene)
                .map_or(ende, |n| zeilenanfang(text, n.2));
            Ueberschrift {
                ebene: *ebene,
                titel: t.clone(),
                von: *von,
                abschnitt: zeilenanfang(text, *von)..bis.max(*von),
            }
        })
        .collect()
}

fn mit_zeilenende(s: &str) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}

/// Verschiebt Abschnitt `nr` vor Abschnitt `vor` (`None`: ans Ende, vor einen Schlussteil).
pub fn abschnitt_verschieben(
    quelle: &Source,
    nr: usize,
    vor: Option<usize>,
) -> Result<Patch, PatchFehler> {
    let text = quelle.text();
    let g = gliederung(quelle);
    let unbekannt = || PatchFehler::UngueltigerWert(tr!("unbekannter Abschnitt"));
    let a = g.get(nr).ok_or_else(unbekannt)?.abschnitt.clone();
    let ziel = match vor {
        Some(j) => g.get(j).ok_or_else(unbekannt)?.abschnitt.start,
        None => g
            .iter()
            .map(|u| u.abschnitt.end)
            .max()
            .unwrap_or(text.len()),
    };
    if ziel == a.start || ziel == a.end {
        return Ok(Patch {
            bereich: a.start..a.start,
            text: String::new(),
        });
    }
    if a.contains(&ziel) {
        return Err(PatchFehler::Gesperrt(tr!(
            "Ein Abschnitt kann nicht in sich selbst verschoben werden."
        )));
    }
    let teil = mit_zeilenende(&text[a.clone()]);
    let patch = if ziel < a.start {
        Patch {
            bereich: ziel..a.end,
            text: format!("{teil}{}", &text[ziel..a.start]),
        }
    } else {
        Patch {
            bereich: a.start..ziel,
            text: format!("{}{teil}", mit_zeilenende(&text[a.end..ziel])),
        }
    };
    let neu = Source::detached(crate::anwenden(text, &patch));
    if fehler(neu.root()) > fehler(quelle.root()) {
        return Err(PatchFehler::Invariante("kein neuer Fehlerknoten"));
    }
    Ok(patch)
}
