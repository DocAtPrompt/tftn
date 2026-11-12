//! Wo im Dokument eine Stelle liegt (PLAN-M2 AP2): Markup, Code oder Mathe. Bestimmt, welche Bausteine
//! das Menü anbietet und ob „/“ es öffnet.

use tftn_manifest::Modus;
use typst_syntax::{LinkedNode, Side, Source, SyntaxMode};

/// Modus unmittelbar vor `byte`; `None` in Kommentaren und im Text eines Raw-Blocks.
pub fn modus_an(quelle: &Source, byte: usize) -> Option<Modus> {
    let wurzel = LinkedNode::new(quelle.root());
    let mut modus = match wurzel.leaf_at(byte, Side::Before) {
        Some(blatt) => blatt.mode_after()?,
        // Kein Blatt vor der Stelle (Dateianfang): Modus der Wurzel.
        None => SyntaxMode::Markup,
    };
    // Direkt hinter einem eingebetteten Ausdruck (`#footnote[x]|`) nennt typst-syntax Code, weil der
    // Ausdruck weitergehen könnte. Was dort tatsächlich folgt, entscheidet: Leerraum oder Text heißt
    // Markup, `.` oder `(` bleibt Code. Am Dateiende geht es mit dem Modus der Wurzel weiter.
    if modus == SyntaxMode::Code {
        match wurzel.leaf_at(byte, Side::After) {
            Some(danach) if danach.offset() == byte => {
                if let Some(m) = danach.mode_after() {
                    modus = m;
                }
            }
            None if byte == quelle.text().len() => {
                if eingebettet_bis_hier(&wurzel, byte) {
                    modus = SyntaxMode::Markup;
                }
            }
            _ => {}
        }
    }
    Some(match modus {
        SyntaxMode::Markup => Modus::Markup,
        SyntaxMode::Code => Modus::Code,
        SyntaxMode::Math => Modus::Mathe,
    })
}

/// Endet am Dateiende ein Ausdruck, der mit `#` in Markup eingebettet ist und geschlossen ist?
fn eingebettet_bis_hier(wurzel: &LinkedNode, byte: usize) -> bool {
    let Some(mut knoten) = wurzel.leaf_at(byte, Side::Before) else {
        return false;
    };
    while let Some(eltern) = knoten.parent().cloned() {
        if eltern.kind() == typst_syntax::SyntaxKind::Markup
            && knoten
                .prev_sibling_with_trivia()
                .is_some_and(|v| v.kind() == typst_syntax::SyntaxKind::Hash)
        {
            return knoten.get().errors_and_warnings().0.is_empty();
        }
        knoten = eltern;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(text: &str, byte: usize) -> Option<Modus> {
        modus_an(&Source::detached(text), byte)
    }

    #[test]
    fn modus_nach_umgebung() {
        assert_eq!(m("/", 0), Some(Modus::Markup)); // Dateianfang
        assert_eq!(m("Text\n/", 5), Some(Modus::Markup));
        assert_eq!(m("#{\n/", 3), Some(Modus::Code));
        assert_eq!(m("$ a\n/ b $", 4), Some(Modus::Mathe));
        assert_eq!(m("#figure(caption: [\n/])", 19), Some(Modus::Markup));
    }

    #[test]
    fn hinter_eingebettetem_ausdruck_geht_text_weiter() {
        let t = "Satz#footnote[x]";
        assert_eq!(m(t, t.len()), Some(Modus::Markup)); // Dateiende
        assert_eq!(m("Satz#footnote[x] weiter", 16), Some(Modus::Markup));
        assert_eq!(m("Satz#footnote[x]\n", 16), Some(Modus::Markup));
        // Geht der Ausdruck weiter (Feldzugriff, Argumente), bleibt es Code.
        assert_eq!(m("#foo.bar", 4), Some(Modus::Code));
        assert_eq!(m("#foo(1)", 4), Some(Modus::Code));
    }

    #[test]
    fn kein_modus_in_kommentar_und_raw() {
        assert_eq!(m("// Kommentar", 3), None);
        assert_eq!(m("```\n/usr\n```", 4), None);
    }
}
