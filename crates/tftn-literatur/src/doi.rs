//! Literatur aus DOI: Das BibTeX, das der Holer bei Crossref bzw. DataCite holt, wird ein lesbarer
//! Eintrag mit Schlüssel `<name><jahr>` (wie `lecun2015`) und landet am Ende der `.bib`.

use std::path::Path;
use tftn_sprachen::tr;

use biblatex::{Bibliography, ChunksExt, Person};

use crate::Literatur;

/// Ergebnis: der Schlüssel zum Zitieren und der neue Eintrag (`None`: die DOI ist schon da).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoiEintrag {
    pub schluessel: String,
    pub text: Option<String>,
}

/// Wörter, die kein Name sind (Artikel, Namenszusätze).
const KEIN_NAME: [&str; 14] = [
    "the", "der", "die", "das", "von", "van", "de", "del", "della", "la", "le", "du", "di", "da",
];

fn falten(c: char) -> &'static str {
    match c {
        'ä' => "ae",
        'ö' => "oe",
        'ü' => "ue",
        'ß' => "ss",
        'á' | 'à' | 'â' | 'ã' | 'å' | 'ā' | 'ą' => "a",
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ě' | 'ę' => "e",
        'í' | 'ì' | 'î' | 'ï' => "i",
        'ó' | 'ò' | 'ô' | 'õ' | 'ø' | 'ő' => "o",
        'ú' | 'ù' | 'û' | 'ů' | 'ű' => "u",
        'ç' | 'č' | 'ć' => "c",
        'š' | 'ś' => "s",
        'ž' | 'ź' | 'ż' => "z",
        'ñ' | 'ń' | 'ň' => "n",
        'ř' => "r",
        'ł' => "l",
        'ý' | 'ÿ' => "y",
        'ď' | 'đ' => "d",
        'ť' => "t",
        _ => "",
    }
}

fn ascii(wort: &str) -> String {
    wort.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() {
                c.to_string()
            } else {
                falten(c).to_string()
            }
        })
        .collect()
}

/// Stamm des Schlüssels aus dem Feld `author`: Nachname der ersten Person, ohne Zusätze, in
/// Kleinbuchstaben ohne Umlaute; ohne brauchbaren Namen `quelle`.
pub fn schluessel_stamm(autoren: &str) -> String {
    let bib = Bibliography::parse(&format!("@misc{{x, author = {{{autoren}}}}}")).ok();
    let name = bib
        .as_ref()
        .and_then(|b| b.get("x"))
        .and_then(|e| e.get_as::<Vec<Person>>("author").ok())
        .and_then(|p| p.into_iter().next())
        .map(|p| p.name)
        .unwrap_or_default();
    name.split(|c: char| c.is_whitespace() || c == '-')
        .map(ascii)
        .find(|w| w.len() >= 2 && !KEIN_NAME.contains(&w.as_str()))
        .unwrap_or_else(|| "quelle".into())
}

fn jahr(e: &biblatex::Entry) -> String {
    ["year", "date"]
        .iter()
        .filter_map(|f| e.fields.get(*f))
        .map(|c| c.format_verbatim())
        .find_map(|t| {
            let z: String = t.chars().take_while(char::is_ascii_digit).collect();
            (z.len() == 4).then_some(z)
        })
        .unwrap_or_default()
}

/// Eintrag zu `doi` aus dem geholten BibTeX. Gibt es die DOI in der Literatur schon (ohne Rücksicht
/// auf Groß-/Kleinschreibung), kommt deren Schlüssel zurück und nichts Neues.
pub fn doi_eintrag(bibtex: &str, literatur: &Literatur, doi: &str) -> Result<DoiEintrag, String> {
    if let Some(e) = literatur.eintraege.iter().find(|e| {
        e.treffer
            .doi
            .as_deref()
            .is_some_and(|d| d.eq_ignore_ascii_case(doi))
    }) {
        return Ok(DoiEintrag {
            schluessel: e.treffer.schluessel.clone(),
            text: None,
        });
    }
    let bib = Bibliography::parse(bibtex).map_err(|f| {
        tr!(
            "Die Antwort zu {doi} ist kein lesbares BibTeX ({fehler}).",
            doi,
            fehler = format!("{f:?}")
        )
    })?;
    let mut eintraege = bib.into_iter();
    let (Some(mut e), None) = (eintraege.next(), eintraege.next()) else {
        return Err(tr!(
            "Die Antwort zu {doi} enthält nicht genau einen Eintrag.",
            doi
        ));
    };
    let autoren = e
        .fields
        .get("author")
        .or_else(|| e.fields.get("editor"))
        .map(|c| c.to_biblatex_string(false))
        .unwrap_or_default();
    let stamm = format!("{}{}", schluessel_stamm(&autoren), jahr(&e));
    let schluessel = std::iter::once(stamm.clone())
        .chain(('a'..='z').map(|z| format!("{stamm}{z}")))
        .find(|k| !literatur.enthaelt(k))
        .ok_or_else(|| tr!("Kein freier Schlüssel für {stamm}.", stamm))?;
    e.key = schluessel.clone();
    // Felder eingerückt, wie man es in einer .bib erwartet.
    let text: String = e
        .to_biblatex_string()
        .lines()
        .enumerate()
        .map(|(i, z)| {
            if i == 0 || z == "}" {
                format!("{z}\n")
            } else {
                format!("  {z}\n")
            }
        })
        .collect();
    let text = text.trim_end().to_string();
    // Gegenprobe: Hayagriva (wie Typst) muss den Eintrag lesen können.
    match hayagriva::io::from_biblatex_str(&text) {
        Ok(b) if b.len() == 1 => {}
        _ => {
            return Err(tr!(
                "Der Eintrag zu {doi} ließ sich nicht sauber schreiben.",
                doi
            ));
        }
    }
    Ok(DoiEintrag {
        schluessel,
        text: Some(format!("{text}\n")),
    })
}

/// Hängt `eintrag` an `datei` an (legt sie bei Bedarf an), mit einer Leerzeile Abstand.
pub fn anhaengen(datei: &Path, eintrag: &str) -> Result<(), String> {
    let mut inhalt = match std::fs::read_to_string(datei) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", datei.display())),
    };
    if !inhalt.trim().is_empty() {
        while !inhalt.ends_with("\n\n") {
            inhalt.push('\n');
        }
    } else {
        inhalt.clear();
    }
    inhalt.push_str(eintrag);
    if !inhalt.ends_with('\n') {
        inhalt.push('\n');
    }
    std::fs::write(datei, inhalt).map_err(|e| format!("{}: {e}", datei.display()))
}
