//! Typst-Meldungen auf Deutsch, mit einem Tipp, wie man sie behebt. Was nicht übersetzt ist, bleibt
//! englisch; übersetzte tragen den Wortlaut von typst am Ende, damit man danach suchen kann.

/// Übersetzt Meldung samt Hinweisen; `None`, wenn TFTN die Meldung nicht kennt.
pub fn uebersetzen(meldung: &str, hinweise: &[&str]) -> Option<String> {
    let (satz, tipp) = satz(meldung)?;
    let mut aus = satz;
    let mut tipps: Vec<String> = tipp.into_iter().collect();
    for h in hinweise {
        tipps.push(hinweis(h).unwrap_or_else(|| format!("(typst) {h}")));
    }
    for t in tipps {
        aus.push_str("\n  Tipp: ");
        aus.push_str(&t);
    }
    aus.push_str(&format!("\n  (typst: {meldung})"));
    Some(aus)
}

/// Wie `uebersetzen`, sonst Meldung und Hinweise im Wortlaut. Spricht die Oberfläche nicht
/// Deutsch, bleibt die Meldung im Original von typst (englisch).
pub fn meldungstext(meldung: &str, hinweise: &[&str]) -> String {
    let deutsch = tftn_sprachen::ist_deutsch()
        .then(|| uebersetzen(meldung, hinweise))
        .flatten();
    deutsch.unwrap_or_else(|| {
        let mut t = meldung.to_string();
        for h in hinweise {
            t.push_str("\n  ");
            t.push_str(&tftn_sprachen::tr!("Hinweis:"));
            t.push(' ');
            t.push_str(h);
        }
        t
    })
}

fn zwischen<'a>(s: &'a str, vor: &str, nach: &str) -> Option<&'a str> {
    s.strip_prefix(vor)?.strip_suffix(nach)
}

fn satz(m: &str) -> Option<(String, Option<String>)> {
    let ein = |s: &str| Some((s.to_string(), None));
    let mit = |s: String, t: &str| Some((s, Some(t.to_string())));
    match m {
        "unclosed delimiter" => {
            return mit(
                "Klammer nicht geschlossen.".into(),
                "Zu jedem `(`, `[`, `{` und `$` gehört ein schließendes `)`, `]`, `}` bzw. `$`.",
            );
        }
        "unclosed string" => {
            return mit(
                "Anführungszeichen nicht geschlossen.".into(),
                "Text in Code steht zwischen zwei `\"`; das zweite fehlt.",
            );
        }
        "unclosed raw text" => {
            return mit(
                "Code-Abschnitt nicht geschlossen.".into(),
                "Es fehlen die schließenden Backticks (` bzw. ```).",
            );
        }
        "unclosed label" => {
            return mit("Marke nicht geschlossen.".into(), "Nach `<name` fehlt `>`.");
        }
        "unexpected end of block comment" => {
            return ein("`*/` ohne vorheriges `/*`.");
        }
        "unknown image format" => {
            return mit(
                "Unbekanntes Bildformat.".into(),
                "Typst setzt png, jpg, gif, svg, webp und pdf.",
            );
        }
        "unknown paper size" => {
            return mit(
                "Unbekanntes Papierformat.".into(),
                "Im Fach „Seite“ ein Format wählen, etwa a4 oder us-letter.",
            );
        }
        "unknown bibliography format (must be .yaml/.yml or .bib)" => {
            return ein("Literaturdateien müssen .bib oder .yaml sein.");
        }
        "failed to load file (access denied)" => {
            return ein("Die Datei lässt sich nicht lesen: keine Berechtigung.");
        }
        "failed to load file (is a directory)" => {
            return ein("Das ist ein Ordner, keine Datei.");
        }
        "unexpected argument" => {
            return mit(
                "Zu viele Angaben für diese Funktion.".into(),
                "Eine Angabe ohne Namen ist zu viel; vielleicht fehlt ein `name:` davor.",
            );
        }
        _ => {}
    }
    if let Some(name) = m.strip_prefix("unknown variable: ") {
        return mit(
            format!("Unbekannter Name „{name}“."),
            "Tippfehler? Kommt er aus einem Paket, fehlt vielleicht `#import`. Soll `#` als Zeichen erscheinen: `\\#`.",
        );
    }
    if let Some(pfad) = zwischen(m, "file not found (searched at ", ")") {
        return mit(
            format!("Datei nicht gefunden: {pfad}"),
            "Liegt sie im Dokumentordner, und stimmt die Schreibweise (auch groß/klein)? Bilder am einfachsten in den Text ziehen.",
        );
    }
    if let Some(s) = m.strip_prefix("unknown font family: ") {
        return mit(
            format!("Die Schrift „{s}“ ist nicht vorhanden; gesetzt wird mit einer Ersatzschrift."),
            "Im Fach „Seite“ eine vorhandene Schrift wählen.",
        );
    }
    if let Some(l) = zwischen(m, "label `", "` does not exist in the document") {
        return mit(
            format!("Verweis ins Leere: Es gibt keine Marke `{l}`."),
            "Eine Marke steht hinter dem Ziel, etwa `= Einleitung <einleitung>`; verwiesen wird mit `@einleitung`.",
        );
    }
    if let Some(l) = zwischen(m, "label `", "` occurs multiple times in the document") {
        return mit(
            format!("Die Marke `{l}` kommt mehrfach vor."),
            "Jede Marke darf nur einmal stehen; eine umbenennen.",
        );
    }
    if let Some(k) = zwischen(m, "key `", "` does not exist in the bibliography") {
        return mit(
            format!("Die Quelle `{k}` steht in keiner Literaturdatei."),
            "Schreibweise prüfen oder das Zitat über „Zitat einfügen“ aus der Liste wählen.",
        );
    }
    if let Some(a) = m.strip_prefix("missing argument: ") {
        return ein(&format!("Es fehlt die Angabe „{a}“."));
    }
    if let Some(a) = m.strip_prefix("unexpected argument: ") {
        return mit(
            format!("Diese Funktion kennt die Angabe „{a}“ nicht."),
            "Tippfehler im Namen? Die Vervollständigung (Strg+Leertaste) zeigt, was es gibt.",
        );
    }
    if let Some(a) = m.strip_prefix("duplicate argument: ") {
        return ein(&format!("Die Angabe „{a}“ steht doppelt."));
    }
    if let Some(rest) = m.strip_prefix("expected ") {
        let (erwartet, gefunden) = match rest.split_once(", found ") {
            Some((e, g)) => (e, Some(g)),
            None => (rest, None),
        };
        let erwartet = liste(erwartet)?;
        return match gefunden {
            Some(g) => ein(&format!(
                "Hier gehört {erwartet} hin, nicht {}.",
                begriff(g)?
            )),
            None => ein(&format!("Hier fehlt {erwartet}.")),
        };
    }
    if let Some(was) = m.strip_prefix("unexpected ") {
        return mit(
            format!("Hier steht {} zu viel.", begriff(was)?),
            "Oft fehlt davor das Gegenstück, oder ein Zeichen ist doppelt.",
        );
    }
    None
}

/// „length, ratio, or auto“ → „eine Länge, ein Prozentwert oder `auto`“.
fn liste(s: &str) -> Option<String> {
    let teile: Vec<&str> = s
        .split(", ")
        .flat_map(|t| t.split(" or "))
        .map(|t| t.trim_start_matches("or ").trim())
        .filter(|t| !t.is_empty())
        .collect();
    let deutsch: Vec<String> = teile.iter().map(|t| begriff(t)).collect::<Option<_>>()?;
    Some(match deutsch.as_slice() {
        [] => return None,
        [a] => a.clone(),
        [vor @ .., letzt] => format!("{} oder {letzt}", vor.join(", ")),
    })
}

fn begriff(s: &str) -> Option<String> {
    let b = match s {
        "closing paren" => "ein `)`",
        "closing bracket" => "ein `]`",
        "closing brace" => "ein `}`",
        "opening paren" => "ein `(`",
        "opening bracket" => "ein `[`",
        "opening brace" => "ein `{`",
        "comma" => "ein Komma",
        "colon" => "ein Doppelpunkt",
        "semicolon" => "ein Strichpunkt",
        "semicolon or line break" => "ein Strichpunkt oder Zeilenumbruch",
        "comma or semicolon" => "ein Komma oder Strichpunkt",
        "hash" => "ein `#`",
        "dollar sign" => "ein `$`",
        "star" => "ein `*`",
        "underscore" => "ein `_`",
        "equals sign" => "ein `=`",
        "dots" => "ein `..`",
        "expression" => "ein Ausdruck (Wert, Name oder Aufruf)",
        "identifier" => "ein Name",
        "pattern" => "ein Name",
        "block" => "ein Block in `{…}` oder `[…]`",
        "argument list" => "eine Angabe in runden Klammern",
        "keyword `in`" => "das Wort `in`",
        "string" | "str" => "ein Text in Anführungszeichen",
        "content" => "ein Inhalt in eckigen Klammern",
        "integer" | "int" => "eine ganze Zahl",
        "float" => "eine Kommazahl",
        "numeric value" => "eine Zahl",
        "length" => "eine Länge (etwa `2cm`)",
        "relative length" => "eine Länge oder ein Prozentwert",
        "ratio" => "ein Prozentwert (etwa `50%`)",
        "fraction" => "ein Anteil (etwa `1fr`)",
        "angle" => "ein Winkel (etwa `90deg`)",
        "color" => "eine Farbe (etwa `blue` oder `rgb(\"#264653\")`)",
        "boolean" | "bool" => "`true` oder `false`",
        "none" | "`none`" => "`none`",
        "auto" | "`auto`" => "`auto`",
        "array" => "eine Liste in runden Klammern",
        "dictionary" => "eine Zuordnung `(name: wert)`",
        "function" => "eine Funktion",
        "label" => "eine Marke `<name>`",
        "end of tokens" => "das Ende",
        _ => return None,
    };
    Some(b.to_string())
}

fn hinweis(h: &str) -> Option<String> {
    if let Some(r) = h.strip_prefix(
        "if you meant to display multiple letters as is, try adding spaces between each letter: ",
    ) {
        return Some(format!(
            "In Formeln gelten mehrere Buchstaben ohne Abstand als ein Name. Einzelne Buchstaben: {r}"
        ));
    }
    if let Some(r) =
        h.strip_prefix("or if you meant to display this as text, try placing it in quotes: ")
    {
        return Some(format!("Als Wort in der Formel: {r}"));
    }
    if let Some((_, r)) = h
        .strip_prefix("if you meant to use subtraction, try adding spaces around the minus sign")
        .and_then(|r| r.split_once(": "))
    {
        return Some(format!("Für ein Minus Leerzeichen drumherum: {r}"));
    }
    if let Some(r) = h.strip_prefix("if you meant to use a literal, try adding a hash before it: ")
    {
        return Some(format!("Als Wert mit `#` davor: {r}"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn klammer_mit_tipp_und_wortlaut() {
        let t = uebersetzen("unclosed delimiter", &[]).unwrap();
        assert!(t.starts_with("Klammer nicht geschlossen.\n  Tipp: "), "{t}");
        assert!(t.ends_with("(typst: unclosed delimiter)"), "{t}");
    }

    #[test]
    fn erwartet_und_gefunden() {
        assert_eq!(
            satz("expected length, ratio, or auto, found string")
                .unwrap()
                .0,
            "Hier gehört eine Länge (etwa `2cm`), ein Prozentwert (etwa `50%`) oder `auto` hin, nicht ein Text in Anführungszeichen."
        );
        assert_eq!(satz("expected comma").unwrap().0, "Hier fehlt ein Komma.");
        assert_eq!(
            satz("unexpected closing bracket").unwrap().0,
            "Hier steht ein `]` zu viel."
        );
    }

    #[test]
    fn unbekannter_name_in_formel_mit_deutschen_tipps() {
        let t = uebersetzen(
            "unknown variable: ab",
            &[
                "if you meant to display multiple letters as is, try adding spaces between each letter: `a b`",
                "or if you meant to display this as text, try placing it in quotes: `\"ab\"`",
            ],
        )
        .unwrap();
        assert!(t.starts_with("Unbekannter Name „ab“."), "{t}");
        assert!(t.contains("Einzelne Buchstaben: `a b`"), "{t}");
        assert!(t.contains("Als Wort in der Formel: `\"ab\"`"), "{t}");
    }

    #[test]
    fn datei_marke_quelle_schrift() {
        assert!(
            satz("file not found (searched at /x/bilder/a.png)")
                .unwrap()
                .0
                .contains("/x/bilder/a.png")
        );
        assert_eq!(
            satz("label `<einl>` does not exist in the document")
                .unwrap()
                .0,
            "Verweis ins Leere: Es gibt keine Marke `<einl>`."
        );
        assert_eq!(
            satz("key `mueller2020` does not exist in the bibliography")
                .unwrap()
                .0,
            "Die Quelle `mueller2020` steht in keiner Literaturdatei."
        );
        assert!(
            satz("unknown font family: comic")
                .unwrap()
                .0
                .contains("„comic“")
        );
    }

    #[test]
    fn unbekanntes_bleibt_im_wortlaut() {
        assert_eq!(uebersetzen("cannot add integer and content", &[]), None);
        assert_eq!(satz("expected wombat"), None);
        assert_eq!(
            meldungstext("cannot add integer and content", &["try x"]),
            "cannot add integer and content\n  Hinweis: try x"
        );
    }

    #[test]
    fn fremder_hinweis_bleibt_englisch() {
        let t = uebersetzen("unknown variable: x", &["something new"]).unwrap();
        assert!(t.contains("Tipp: (typst) something new"), "{t}");
    }
}
