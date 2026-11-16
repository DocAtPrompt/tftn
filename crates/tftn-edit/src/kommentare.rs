//! Kommentare im Text (`// …` und `/* … */`) für die Kommentar-Übersicht neben dem Editor.
//!
//! Zeilenkommentare in aufeinanderfolgenden Zeilen ergeben einen Eintrag; eine Leerzeile oder Text
//! dazwischen trennt. Zierlinien ohne Buchstaben oder Ziffern (`// -----`) zählen nicht.

use typst_syntax::{LinkedNode, Source, SyntaxKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Kommentar {
    /// Ohne `//` bzw. `/* */`, Zeilen getrennt durch `\n`
    pub text: String,
    /// Bytes: Anfang des ersten Kommentarzeichens
    pub von: usize,
    /// Bytes: Ende des letzten Kommentarzeichens
    pub bis: usize,
}

fn zeile(z: &str) -> &str {
    z.trim().trim_start_matches('*').trim()
}

fn blocktext(roh: &str) -> String {
    let innen = roh.trim_start_matches("/*").trim_end_matches("*/");
    innen
        .lines()
        .map(zeile)
        .filter(|z| !z.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn sammeln(k: &LinkedNode, aus: &mut Vec<(SyntaxKind, usize, usize)>) {
    match k.kind() {
        SyntaxKind::LineComment | SyntaxKind::BlockComment => {
            aus.push((k.kind(), k.offset(), k.offset() + k.len()))
        }
        _ => {
            for kind in k.children() {
                sammeln(&kind, aus);
            }
        }
    }
}

/// Alle Kommentare in Textreihenfolge.
pub fn kommentare(quelle: &Source) -> Vec<Kommentar> {
    let text = quelle.text();
    let mut roh = Vec::new();
    sammeln(&LinkedNode::new(quelle.root()), &mut roh);

    let mut liste: Vec<(Kommentar, bool)> = Vec::new(); // (Eintrag, endet mit Zeilenkommentar)
    for (art, von, bis) in roh {
        let quelltext = &text[von..bis];
        if art == SyntaxKind::LineComment {
            let z = quelltext.trim_start_matches('/').trim().to_string();
            if let Some((letzter, true)) = liste.last_mut() {
                let dazwischen = &text[letzter.bis..von];
                if dazwischen.trim().is_empty() && dazwischen.matches('\n').count() == 1 {
                    letzter.text.push('\n');
                    letzter.text.push_str(&z);
                    letzter.bis = bis;
                    continue;
                }
            }
            liste.push((Kommentar { text: z, von, bis }, true));
        } else {
            liste.push((
                Kommentar {
                    text: blocktext(quelltext),
                    von,
                    bis,
                },
                false,
            ));
        }
    }
    liste
        .into_iter()
        .map(|(mut k, _)| {
            k.text = k
                .text
                .lines()
                .map(str::trim)
                .filter(|z| !z.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            k
        })
        .filter(|k| k.text.chars().any(char::is_alphanumeric))
        .collect()
}
