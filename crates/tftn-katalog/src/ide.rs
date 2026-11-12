//! Vervollständigung mit Strg+Leertaste (PLAN-M2 AP2, Entwurf 3): `typst_ide::autocomplete`, die
//! Schnipsel-Syntax der IDE als Felder wie bei den Bausteinen.

use typst_ide::{CompletionKind, IdeWorld};
use typst_layout::PagedDocument;
use typst_syntax::Source;

use crate::{Eingesetzt, Schnipsel};

/// Ein Vorschlag mit eingesetztem Text (Bereiche in Bytes von `eingesetzt.text`).
#[derive(Debug, Clone)]
pub struct Vorschlag {
    pub titel: String,
    pub detail: Option<String>,
    /// CodeMirror-Typ für das Symbol in der Liste
    pub art: &'static str,
    pub eingesetzt: Eingesetzt,
}

/// Vorschläge für `byte` in `quelle`; das `usize` ist das Startbyte des zu ersetzenden Bereichs.
/// `dokument` (der letzte gute Stand) liefert Labels und Literaturschlüssel.
pub fn vervollstaendigen(
    world: &dyn IdeWorld,
    dokument: Option<&PagedDocument>,
    quelle: &Source,
    byte: usize,
) -> Option<(usize, Vec<Vorschlag>)> {
    let (von, liste) = typst_ide::autocomplete(world, dokument, quelle, byte, true)?;
    let vorschlaege = liste
        .into_iter()
        .map(|c| Vorschlag {
            eingesetzt: Schnipsel::aus_ide(c.apply.as_deref().unwrap_or(&c.label)).einsetzen(""),
            titel: c.label.to_string(),
            detail: c.detail.map(|d| d.to_string()),
            art: art(&c.kind),
        })
        .collect();
    Some((von, vorschlaege))
}

fn art(kind: &CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Syntax => "keyword",
        CompletionKind::Func => "function",
        CompletionKind::Type => "type",
        CompletionKind::Param => "property",
        CompletionKind::Constant | CompletionKind::Symbol(_) => "constant",
        CompletionKind::Path | CompletionKind::Font => "text",
        CompletionKind::Package => "namespace",
        CompletionKind::Label => "variable",
    }
}
