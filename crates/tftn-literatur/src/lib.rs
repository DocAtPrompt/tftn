//! Literatur: `.bib` lesen, durchsuchen, Zitate einfügen (KONZEPT 4.6, PLAN-M1 AP7).
//!
//! Die Einträge kommen über Hayagriva (dieselbe Bibliothek, die Typst nutzt), damit die Suche genau
//! die Schlüssel kennt, die auch `bibliography()` kennt. Die Suche ist bewusst einfach: Jedes Wort
//! der Anfrage muss in Schlüssel, Namen, Jahr, Titel, Quelle oder DOI vorkommen; Umlaute werden
//! als `ä`, `a` und `ae` gefunden. Gewichtet wird nach Art des Treffers.

use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

pub mod doi;

use serde::Serialize;
use typst_syntax::{LinkedNode, Side, Source, SyntaxMode};

/// Ein Eintrag mit vorbereitetem Suchtext.
#[derive(Debug, Clone)]
struct Eintrag {
    treffer: Treffer,
    schluessel: String,
    namen: Vec<String>,
    titelwoerter: Vec<String>,
    /// Alles, gefaltet, für Teilwort-Suche.
    heu: String,
}

/// Was die Oberfläche zu einem Eintrag zeigt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Treffer {
    pub schluessel: String,
    pub autoren: String,
    pub jahr: Option<i32>,
    pub titel: String,
    pub quelle: String,
    pub doi: Option<String>,
}

#[derive(Debug, Default)]
pub struct Literatur {
    eintraege: Vec<Eintrag>,
    pub dateien: Vec<PathBuf>,
}

/// Kleinbuchstaben; Umlaute zweifach (ä → a und ae), damit „lued“ und „lud“ beide Lüddecke finden.
fn falten(text: &str) -> String {
    let klein = text.to_lowercase();
    let einfach: String = klein
        .chars()
        .flat_map(|c| match c {
            'ä' | 'á' | 'à' | 'â' => vec!['a'],
            'ö' | 'ó' | 'ò' | 'ô' => vec!['o'],
            'ü' | 'ú' | 'ù' | 'û' => vec!['u'],
            'é' | 'è' | 'ê' => vec!['e'],
            'ß' => vec!['s', 's'],
            c => vec![c],
        })
        .collect();
    let doppelt = klein
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss");
    format!("{klein} {einfach} {doppelt}")
}

/// Anfrage-Wort: Kleinbuchstaben, Umlaute einfach.
fn wort_falten(w: &str) -> String {
    falten(w).split(' ').nth(1).unwrap_or_default().to_string()
}

fn personen(p: &[hayagriva::types::Person]) -> (String, Vec<String>) {
    let namen: Vec<String> = p.iter().map(|p| p.name.clone()).collect();
    let anzeige = match namen.as_slice() {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a}, {b}"),
        [a, ..] => tr!("{a} u. a.", a),
    };
    let mut alle = namen;
    alle.extend(p.iter().filter_map(|p| p.given_name.clone()));
    (anzeige, alle)
}

impl Literatur {
    /// Liest alle Dateien; Fehler einer Datei werden gemeldet, die übrigen trotzdem gelesen.
    pub fn laden(dateien: &[PathBuf]) -> (Self, Vec<String>) {
        let mut lit = Literatur {
            eintraege: Vec::new(),
            dateien: dateien.to_vec(),
        };
        let mut fehler = Vec::new();
        for d in dateien {
            match std::fs::read_to_string(d) {
                Ok(text) => match hayagriva::io::from_biblatex_str(&text) {
                    Ok(bib) => lit.eintraege.extend(bib.iter().map(eintrag)),
                    Err(e) => fehler.push(tr!(
                        "{datei}: {anzahl} Fehler, z. B. {beispiel}",
                        datei = d.display(),
                        anzahl = e.len(),
                        beispiel = format!("{:?}", e.first())
                    )),
                },
                Err(e) => fehler.push(format!("{}: {e}", d.display())),
            }
        }
        (lit, fehler)
    }

    pub fn len(&self) -> usize {
        self.eintraege.len()
    }

    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty()
    }

    pub fn enthaelt(&self, schluessel: &str) -> bool {
        self.eintraege
            .iter()
            .any(|e| e.treffer.schluessel == schluessel)
    }

    /// Bis zu `max` Treffer, bester zuerst. Leere Anfrage: die ersten Einträge in Dateireihenfolge.
    pub fn suchen(&self, anfrage: &str, max: usize) -> Vec<Treffer> {
        let woerter: Vec<String> = anfrage
            .split_whitespace()
            .map(wort_falten)
            .filter(|w| !w.is_empty())
            .collect();
        if woerter.is_empty() {
            return self
                .eintraege
                .iter()
                .take(max)
                .map(|e| e.treffer.clone())
                .collect();
        }
        let mut bewertet: Vec<(u32, &Eintrag)> = self
            .eintraege
            .iter()
            .filter_map(|e| {
                let mut summe = 0;
                for w in &woerter {
                    summe += bewerten(e, w)?;
                }
                Some((summe, e))
            })
            .collect();
        bewertet.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(b.1.treffer.jahr.cmp(&a.1.treffer.jahr))
                .then(a.1.treffer.schluessel.cmp(&b.1.treffer.schluessel))
        });
        bewertet
            .into_iter()
            .take(max)
            .map(|(_, e)| e.treffer.clone())
            .collect()
    }
}

/// Punkte für ein Anfrage-Wort; `None`, wenn es nirgends vorkommt.
fn bewerten(e: &Eintrag, w: &str) -> Option<u32> {
    if e.schluessel.starts_with(w) {
        return Some(100);
    }
    if e.namen
        .iter()
        .any(|n| n.split(' ').any(|t| t.starts_with(w)))
    {
        return Some(80);
    }
    if e.treffer.jahr.is_some_and(|j| j.to_string() == w) {
        return Some(60);
    }
    if e.titelwoerter.iter().any(|t| t.starts_with(w)) {
        return Some(40);
    }
    if e.heu.contains(w) {
        return Some(20);
    }
    // Buchstabenfolge mit Lücken („lddk“ → Lüddecke), nur ab drei Zeichen
    if w.chars().count() >= 3 && teilfolge(&e.schluessel, w) {
        return Some(5);
    }
    None
}

fn teilfolge(heu: &str, nadel: &str) -> bool {
    let mut zeichen = heu.chars();
    nadel.chars().all(|n| zeichen.any(|h| h == n))
}

fn eintrag(e: &hayagriva::Entry) -> Eintrag {
    let titel = e.title().map(|t| t.to_string()).unwrap_or_default();
    let quelle = e
        .parents()
        .first()
        .and_then(|p| p.title())
        .map(|t| t.to_string())
        .unwrap_or_default();
    let (autoren, mut namen) = match e.authors() {
        Some(a) if !a.is_empty() => personen(a),
        _ => match e.editors() {
            Some(ed) => personen(ed),
            None => (String::new(), Vec::new()),
        },
    };
    if let Some(ed) = e.editors() {
        namen.extend(personen(ed).1);
    }
    let jahr = e.date_any().map(|d| d.year);
    let doi = e.doi().map(str::to_string);
    let namen: Vec<String> = namen.iter().map(|n| falten(n)).collect();
    let titelwoerter: Vec<String> = falten(&format!("{titel} {quelle}"))
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect();
    let heu = falten(&format!(
        "{} {} {} {titel} {quelle} {}",
        e.key(),
        namen.join(" "),
        jahr.map(|j| j.to_string()).unwrap_or_default(),
        doi.as_deref().unwrap_or_default()
    ));
    Eintrag {
        schluessel: e.key().to_lowercase(),
        namen,
        titelwoerter,
        heu,
        treffer: Treffer {
            schluessel: e.key().to_string(),
            autoren,
            jahr,
            titel,
            quelle,
            doi,
        },
    }
}

/// Fehler beim Einfügen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZitatFehler {
    /// In Kommentaren und Raw-Text gibt es keine Zitate.
    KeinZitatOrt,
}

impl std::fmt::Display for ZitatFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&tr!(
            "Hier kann kein Zitat stehen (Kommentar oder Code-Text)."
        ))
    }
}

fn label_zeichen(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')
}

/// Seitenangabe: reine Zahl → „S. 12“, sonst unverändert („Kap. 9“).
pub fn ergaenzung(seite: Option<&str>) -> Option<String> {
    let s = seite?.trim();
    if s.is_empty() {
        None
    } else if s.starts_with(|c: char| c.is_ascii_digit()) {
        Some(format!("S. {s}"))
    } else {
        Some(s.to_string())
    }
}

/// Was an `byte` in `quelle` eingefügt wird: im Text `@key` bzw. `@key[S. 12]`, in Mathe `#cite(…)`,
/// im Code `cite(…)`. Schlüssel, die als `@`-Verweis nicht eindeutig wären, bekommen die `cite`-Form.
pub fn einfuegetext(
    quelle: &Source,
    byte: usize,
    schluessel: &str,
    seite: Option<&str>,
) -> Result<String, ZitatFehler> {
    let wurzel = LinkedNode::new(quelle.root());
    let modus = match wurzel.leaf_at(byte, Side::Before) {
        Some(blatt) => blatt.mode_after().ok_or(ZitatFehler::KeinZitatOrt)?,
        None => SyntaxMode::Markup,
    };
    let zusatz = ergaenzung(seite);
    let label = if schluessel.chars().all(label_zeichen) {
        format!("<{schluessel}>")
    } else {
        format!(
            "label(\"{}\")",
            schluessel.replace('\\', "\\\\").replace('"', "\\\"")
        )
    };
    let cite = || match &zusatz {
        Some(z) => format!("cite({label}, supplement: [{z}])"),
        None => format!("cite({label})"),
    };
    let danach = quelle.text()[byte..].chars().next();
    // Direkt nach einem Buchstaben wäre `Text@key` kein Verweis: ein Leerzeichen davor.
    let abstand = if quelle.text()[..byte]
        .chars()
        .next_back()
        .is_some_and(char::is_alphanumeric)
    {
        " "
    } else {
        ""
    };
    let als_verweis = schluessel.chars().all(label_zeichen)
        && !schluessel.ends_with(['.', ':'])
        && !danach.is_some_and(label_zeichen);
    Ok(match modus {
        SyntaxMode::Markup if als_verweis => match &zusatz {
            Some(z) => format!("{abstand}@{schluessel}[{z}]"),
            None => format!("{abstand}@{schluessel}"),
        },
        SyntaxMode::Markup | SyntaxMode::Math => format!("#{}", cite()),
        SyntaxMode::Code => cite(),
    })
}

/// Namen der eingebauten Zitierstile von Typst (samt Aliasen), etwa `ieee` (PLAN-M2 AP7).
pub fn eingebaute_stile() -> Vec<&'static str> {
    hayagriva::archive::ArchivedStyle::all()
        .iter()
        .flat_map(|s| s.names().iter().copied())
        .collect()
}

/// Dateien der Literatur eines Projekts (Manifest `literatur.dateien`, sonst `literatur.bib`).
pub fn dateien_im_projekt(wurzel: &Path, aus_manifest: &[String]) -> Vec<PathBuf> {
    if aus_manifest.is_empty() {
        vec![wurzel.join("literatur.bib")]
    } else {
        aus_manifest.iter().map(|d| wurzel.join(d)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebaute_stile_enthaelt_apa() {
        let s = eingebaute_stile();
        assert!(s.contains(&"american-psychological-association"));
        assert!(s.contains(&"ieee"));
    }

    fn text(t: &str, cursor: &str, key: &str, seite: Option<&str>) -> Result<String, ZitatFehler> {
        // Die Markierung zeigt nur die Stelle und wird vorher entfernt.
        let pos = t.find(cursor).unwrap();
        let q = Source::detached(t.replacen(cursor, "", 1));
        einfuegetext(&q, pos, key, seite)
    }

    #[test]
    fn modus_am_cursor() {
        assert_eq!(
            text("Text | mehr", "|", "smith2023", None).unwrap(),
            "@smith2023"
        );
        assert_eq!(
            text("Text | mehr", "|", "smith2023", Some("12")).unwrap(),
            "@smith2023[S. 12]"
        );
        assert_eq!(
            text("Text | mehr", "|", "smith2023", Some("Kap. 9")).unwrap(),
            "@smith2023[Kap. 9]"
        );
        assert_eq!(text("$ x | y $", "|", "a1", None).unwrap(), "#cite(<a1>)");
        assert_eq!(
            text("#let x = (|)", "|", "a1", Some("3")).unwrap(),
            "cite(<a1>, supplement: [S. 3])"
        );
        assert_eq!(
            text("// Kommentar | hier", "|", "a1", None),
            Err(ZitatFehler::KeinZitatOrt)
        );
    }

    #[test]
    fn ungewoehnliche_schluessel_und_folgezeichen() {
        assert_eq!(
            text("Text |", "|", "doi/10.1/x", None).unwrap(),
            "#cite(label(\"doi/10.1/x\"))"
        );
        assert_eq!(text("Text |", "|", "a1.", None).unwrap(), "#cite(<a1.>)");
        // direkt vor einem Buchstaben würde @key ihn verschlucken
        assert_eq!(text("Text |x", "|", "a1", None).unwrap(), "#cite(<a1>)");
        assert_eq!(text("Wort| weiter", "|", "a1", None).unwrap(), " @a1");
    }

    #[test]
    fn falten_findet_umlaute_dreifach() {
        let f = falten("Lüddecke");
        assert!(f.contains("lüddecke") && f.contains("luddecke") && f.contains("lueddecke"));
    }
}
