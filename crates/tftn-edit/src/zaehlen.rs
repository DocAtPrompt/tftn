//! Wörter und Zeichen zählen: nur geschriebener Text (auch in Unterschriften und Fußnoten), ohne
//! Befehle, Formeln, Code und Kommentare.

use std::ops::Range;

use serde::Serialize;
use typst_syntax::{LinkedNode, Source, SyntaxKind};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Zahlen {
    pub woerter: usize,
    /// mit Leerzeichen, Zwischenraum zu einem Leerzeichen zusammengefasst
    pub zeichen: usize,
    pub ohne_leerzeichen: usize,
}

fn sammeln(k: &LinkedNode, bereich: &Range<usize>, aus: &mut String) {
    let r = k.range();
    if r.end <= bereich.start || r.start >= bereich.end {
        return;
    }
    match k.kind() {
        SyntaxKind::Equation | SyntaxKind::Raw => {
            aus.push(' ');
            return;
        }
        SyntaxKind::Space | SyntaxKind::Parbreak | SyntaxKind::Linebreak => {
            aus.push(' ');
            return;
        }
        SyntaxKind::Text | SyntaxKind::SmartQuote | SyntaxKind::Shorthand | SyntaxKind::Escape => {
            let von = bereich.start.max(r.start) - r.start;
            let bis = bereich.end.min(r.end) - r.start;
            let t = k.get().leaf_text();
            if let Some(stueck) = t.get(von..bis) {
                aus.push_str(if k.kind() == SyntaxKind::Escape {
                    stueck.trim_start_matches('\\')
                } else {
                    stueck
                });
            }
            return;
        }
        _ => {}
    }
    let block = k.kind() == SyntaxKind::ContentBlock;
    if block {
        aus.push(' ');
    }
    for kind in k.children() {
        sammeln(&kind, bereich, aus);
    }
    if block {
        aus.push(' ');
    }
}

/// Zahlen für den Text im Byte-Bereich `bereich`; ein angeschnittenes Wort zählt mit.
pub fn zaehlen(quelle: &Source, bereich: Range<usize>) -> Zahlen {
    if bereich.is_empty() {
        return Zahlen::default();
    }
    let mut roh = String::new();
    sammeln(&LinkedNode::new(quelle.root()), &bereich, &mut roh);
    let woerter: Vec<&str> = roh.split_whitespace().collect();
    let ohne_leerzeichen: usize = woerter.iter().map(|w| w.chars().count()).sum();
    Zahlen {
        woerter: woerter
            .iter()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count(),
        zeichen: ohne_leerzeichen + woerter.len().saturating_sub(1),
        ohne_leerzeichen,
    }
}
