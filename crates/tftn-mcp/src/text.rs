//! Textoperationen der Werkzeuge, ohne App: Stelle suchen und kleinsten geänderten Bereich bilden.

use std::ops::Range;

/// Ersetzt `alt` durch `neu` in `text`: genau ein Vorkommen, oder mit `alle` jedes.
/// Ergebnis: der neue Volltext.
pub fn ersetzen(text: &str, alt: &str, neu: &str, alle: bool) -> Result<String, String> {
    if alt.is_empty() {
        return Err("`alt` ist leer".into());
    }
    let anzahl = text.matches(alt).count();
    match anzahl {
        0 => Err(
            "`alt` kommt im Quelltext nicht vor (wörtlich, mit Leerzeichen und Zeilenumbrüchen)"
                .into(),
        ),
        1 => Ok(text.replacen(alt, neu, 1)),
        _ if alle => Ok(text.replace(alt, neu)),
        n => Err(format!(
            "`alt` kommt {n}-mal vor; mehr umgebenden Text angeben oder `alle: true` setzen"
        )),
    }
}

/// Kleinster Bereich (Bytes in `alt`), der ersetzt werden muss, damit aus `alt` `neu` wird, und der
/// Ersatz dafür. Grenzen liegen immer auf Zeichengrenzen.
pub fn kleinster_bereich(alt: &str, neu: &str) -> (Range<usize>, String) {
    let vorne = alt
        .char_indices()
        .zip(neu.chars())
        .find(|((_, a), b)| a != b)
        .map(|((i, _), _)| i)
        .unwrap_or(alt.len().min(neu.len()));
    // `vorne` liegt in beiden Texten auf einer Zeichengrenze (gleiche Zeichen davor).
    let rest_alt = &alt[vorne..];
    let rest_neu = &neu[vorne..];
    let hinten = rest_alt
        .chars()
        .rev()
        .zip(rest_neu.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    (
        vorne..alt.len() - hinten,
        rest_neu[..rest_neu.len() - hinten].to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ersetzen_eindeutig_mehrfach_und_fehlend() {
        assert_eq!(ersetzen("a b a", "b", "c", false).unwrap(), "a c a");
        assert!(
            ersetzen("a b a", "a", "c", false)
                .unwrap_err()
                .contains("2-mal")
        );
        assert_eq!(ersetzen("a b a", "a", "c", true).unwrap(), "c b c");
        assert!(
            ersetzen("a b a", "x", "c", false)
                .unwrap_err()
                .contains("nicht vor")
        );
        assert!(ersetzen("a", "", "c", true).is_err());
        assert_eq!(ersetzen("= Titel\n", "Titel", "", false).unwrap(), "= \n");
    }

    #[test]
    fn kleinster_bereich_nur_der_unterschied() {
        assert_eq!(
            kleinster_bereich("Hallo Welt", "Hallo Wald"),
            (7..10, "ald".into())
        );
        assert_eq!(kleinster_bereich("abc", "abc"), (3..3, String::new()));
        assert_eq!(kleinster_bereich("abc", "abxc"), (2..2, "x".into()));
        assert_eq!(kleinster_bereich("abxc", "abc"), (2..3, String::new()));
        assert_eq!(kleinster_bereich("", "neu"), (0..0, "neu".into()));
        assert_eq!(kleinster_bereich("alt", ""), (0..3, String::new()));
        // Wiederholung: Bereich darf vorne und hinten nicht überlappen.
        assert_eq!(kleinster_bereich("aa", "aaa"), (2..2, "a".into()));
    }

    #[test]
    fn kleinster_bereich_auf_zeichengrenzen() {
        // ä und ö teilen das erste Byte (0xC3): die Grenze darf nicht dazwischen liegen.
        let (b, t) = kleinster_bereich("Bär", "Bör");
        assert_eq!(b, 1..3);
        assert_eq!(t, "ö");
        let (b, t) = kleinster_bereich("ä", "ö");
        assert_eq!((b, t.as_str()), (0..2, "ö"));
        for (a, n) in [("äöü", "äü"), ("x€y", "x£y"), ("😀a", "😃a"), ("aä", "aäb")] {
            let (b, t) = kleinster_bereich(a, n);
            let mut probe = a.to_string();
            probe.replace_range(b, &t);
            assert_eq!(probe, n);
        }
    }
}
