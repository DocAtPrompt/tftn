//! Literaturverwaltung: Einträge einer `.bib` lesen, ändern, anlegen und löschen.
//!
//! Gelesen wird mit dem Rohparser von `biblatex` (mit Bytebereichen), geändert wird nur der Bereich
//! des einen Eintrags; Kommentare, `@string` und die Schreibweise der übrigen Einträge bleiben.
//! Ein Feldwert in `{…}` oder `"…"` kommt ohne die Hülle; alles andere (Zahlen ohne Klammern,
//! Abkürzungen wie `nat`, Verkettungen mit `#`) bleibt „roh“ und wird so zurückgeschrieben.

use std::ops::Range;

use biblatex::RawBibliography;
use serde::{Deserialize, Serialize};
use tftn_sprachen::tr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feld {
    pub name: String,
    pub wert: String,
    /// `wert` ist BibTeX-Quelltext und wird unverändert geschrieben (sonst in `{…}`).
    #[serde(default)]
    pub roh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibEintrag {
    /// `article`, `book`, … (klein geschrieben)
    pub art: String,
    pub schluessel: String,
    pub felder: Vec<Feld>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gelesen {
    pub eintrag: BibEintrag,
    /// Bytes: von `@` bis zur schließenden Klammer
    pub bereich: Range<usize>,
}

/// Alle Einträge der Datei in ihrer Reihenfolge.
pub fn lesen(text: &str) -> Result<Vec<Gelesen>, String> {
    let roh = RawBibliography::parse(text).map_err(|f| {
        let zeile = text[..f.span.start.min(text.len())].matches('\n').count() + 1;
        tr!(
            "Die Literaturdatei ist in Zeile {zeile} nicht lesbar.",
            zeile
        )
    })?;
    Ok(roh
        .entries
        .iter()
        .map(|e| {
            let felder =
                e.v.fields
                    .iter()
                    .map(|p| {
                        let quelle = &text[p.value.span.clone()];
                        let huelle = p.value.v.len() == 1
                            && ((quelle.starts_with('{') && quelle.ends_with('}'))
                                || (quelle.starts_with('"') && quelle.ends_with('"')))
                            && quelle.len() >= 2;
                        Feld {
                            name: p.key.v.to_lowercase(),
                            wert: if huelle {
                                quelle[1..quelle.len() - 1].to_string()
                            } else {
                                quelle.to_string()
                            },
                            roh: !huelle,
                        }
                    })
                    .collect();
            Gelesen {
                eintrag: BibEintrag {
                    art: e.v.kind.v.to_lowercase(),
                    schluessel: e.v.key.v.to_string(),
                    felder,
                },
                bereich: eintrag_bereich(text, e.span.clone()),
            }
        })
        .collect())
}

/// Der Bereich des Rohparsers (ab `@`, ohne die schließende Klammer) samt `}` bzw. `)`.
fn eintrag_bereich(text: &str, b: Range<usize>) -> Range<usize> {
    let rest = &text[b.end..];
    let bis = match rest.trim_start().chars().next() {
        Some('}' | ')') => b.end + (rest.len() - rest.trim_start().len()) + 1,
        _ => b.end,
    };
    b.start..bis
}

fn klammern_ok(wert: &str) -> bool {
    let mut tiefe = 0i32;
    let mut zeichen = wert.chars().peekable();
    while let Some(c) = zeichen.next() {
        match c {
            '\\' => {
                zeichen.next();
            }
            '{' => tiefe += 1,
            '}' => {
                tiefe -= 1;
                if tiefe < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    tiefe == 0
}

fn pruefen(e: &BibEintrag) -> Result<(), String> {
    if e.schluessel.is_empty()
        || !e
            .schluessel
            .chars()
            .all(|c| schluessel_zeichen(c) || c == '.')
    {
        return Err(tr!(
            "Der Schlüssel „{schluessel}“ geht nicht: nur Buchstaben, Ziffern und - _ : . ohne Leerzeichen.",
            schluessel = e.schluessel
        ));
    }
    if e.art.is_empty() || !e.art.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(tr!("Die Art „{art}“ geht nicht.", art = e.art));
    }
    for f in &e.felder {
        if f.name.is_empty()
            || !f
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(tr!("Der Feldname „{name}“ geht nicht.", name = f.name));
        }
        if !klammern_ok(&f.wert) {
            return Err(tr!(
                "Im Feld „{name}“ passen die geschweiften Klammern nicht zusammen.",
                name = f.name
            ));
        }
    }
    Ok(())
}

/// Der Eintrag als BibTeX, Felder eingerückt, leere Felder weggelassen; ohne Zeilenende.
pub fn bibtex(e: &BibEintrag) -> Result<String, String> {
    pruefen(e)?;
    let mut s = format!("@{}{{{},\n", e.art, e.schluessel);
    for f in &e.felder {
        let wert = f.wert.trim();
        if wert.is_empty() {
            continue;
        }
        if f.roh {
            s.push_str(&format!("  {} = {},\n", f.name, wert));
        } else {
            s.push_str(&format!("  {} = {{{}}},\n", f.name, wert));
        }
    }
    s.push('}');
    Ok(s)
}

fn finden(eintraege: &[Gelesen], schluessel: &str) -> Result<usize, String> {
    eintraege
        .iter()
        .position(|g| g.eintrag.schluessel == schluessel)
        .ok_or_else(|| {
            tr!(
                "Den Eintrag „{schluessel}“ gibt es nicht (mehr).",
                schluessel
            )
        })
}

fn frei(eintraege: &[Gelesen], schluessel: &str) -> Result<(), String> {
    if eintraege.iter().any(|g| g.eintrag.schluessel == schluessel) {
        return Err(tr!(
            "Den Schlüssel „{schluessel}“ gibt es schon.",
            schluessel
        ));
    }
    Ok(())
}

/// Gegenprobe: Der neue Text muss sich lesen lassen, mit der erwarteten Zahl an Einträgen; las
/// Hayagriva (wie Typst) den alten Text, muss es auch den neuen lesen.
fn gegenprobe(alt: &str, neu: String, anzahl: usize) -> Result<String, String> {
    let ok = lesen(&neu).is_ok_and(|e| e.len() == anzahl)
        && (hayagriva::io::from_biblatex_str(alt).is_err()
            || hayagriva::io::from_biblatex_str(&neu).is_ok());
    if ok {
        Ok(neu)
    } else {
        Err(tr!(
            "Der Eintrag ließ sich nicht sauber schreiben; die Datei bleibt, wie sie ist."
        ))
    }
}

/// Ersetzt den Eintrag `schluessel` durch `neu` (auch mit neuem Schlüssel).
pub fn aendern(text: &str, schluessel: &str, neu: &BibEintrag) -> Result<String, String> {
    let alle = lesen(text)?;
    let i = finden(&alle, schluessel)?;
    if neu.schluessel != schluessel {
        frei(&alle, &neu.schluessel)?;
    }
    let b = alle[i].bereich.clone();
    let ergebnis = format!("{}{}{}", &text[..b.start], bibtex(neu)?, &text[b.end..]);
    gegenprobe(text, ergebnis, alle.len())
}

/// Hängt `neu` mit einer Leerzeile Abstand ans Ende.
pub fn anlegen(text: &str, neu: &BibEintrag) -> Result<String, String> {
    let alle = lesen(text)?;
    frei(&alle, &neu.schluessel)?;
    let vorne = text.trim_end();
    let ergebnis = if vorne.is_empty() {
        format!("{}\n", bibtex(neu)?)
    } else {
        format!("{vorne}\n\n{}\n", bibtex(neu)?)
    };
    gegenprobe(text, ergebnis, alle.len() + 1)
}

/// Entfernt den Eintrag samt dem Leerraum dahinter.
pub fn loeschen(text: &str, schluessel: &str) -> Result<String, String> {
    let alle = lesen(text)?;
    let b = alle[finden(&alle, schluessel)?].bereich.clone();
    let rest = &text[b.end..];
    let danach = rest.trim_start();
    let ergebnis = if danach.is_empty() {
        let vorne = text[..b.start].trim_end();
        if vorne.is_empty() {
            String::new()
        } else {
            format!("{vorne}\n")
        }
    } else {
        format!("{}{danach}", &text[..b.start])
    };
    gegenprobe(text, ergebnis, alle.len() - 1)
}

fn schluessel_zeichen(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | ':')
}

/// Kommt `schluessel` im Typst-Text als Zitat vor (`@schluessel` oder `<schluessel>`)?
pub fn zitiert(quelltext: &str, schluessel: &str) -> bool {
    let at = format!("@{schluessel}");
    let mut ab = 0;
    while let Some(i) = quelltext[ab..].find(&at) {
        let von = ab + i;
        let bis = von + at.len();
        let davor_frei = !quelltext[..von]
            .chars()
            .next_back()
            .is_some_and(schluessel_zeichen);
        let mut danach = quelltext[bis..].chars();
        let danach_frei = match danach.next() {
            None => true,
            Some('.') => !danach.next().is_some_and(schluessel_zeichen),
            Some(c) => !schluessel_zeichen(c),
        };
        if davor_frei && danach_frei {
            return true;
        }
        ab = bis;
    }
    quelltext.contains(&format!("<{schluessel}>"))
}
