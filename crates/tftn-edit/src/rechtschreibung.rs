//! Rechtschreibung nach Dokumentsprache: geprüft wird nur geschriebener Text (auch Überschriften,
//! Unterschriften, Fußnoten), nie Befehle, Formeln, Code, Kommentare, Marken oder Verweise.
//! Die Hunspell-Wörterbücher liegen unter `woerterbuecher/<kennung>/index.{aff,dic}`.

use std::ops::Range;
use std::path::Path;
use tftn_sprachen::tr;

use typst_syntax::{LinkedNode, Source, SyntaxKind};

/// Mitgelieferte Wörterbücher (Ordnernamen unter `woerterbuecher/`).
pub const WOERTERBUECHER: [&str; 8] = ["de", "de-AT", "de-CH", "en", "en-GB", "fr", "it", "es"];

/// Wörterbuch für Typsts `lang` und `region`; ohne passende Region das allgemeine der Sprache.
pub fn woerterbuch_fuer(lang: &str, region: Option<&str>) -> Option<&'static str> {
    let lang = lang.to_lowercase();
    if let Some(r) = region {
        let genau = format!("{lang}-{}", r.to_uppercase());
        if let Some(k) = WOERTERBUECHER.iter().find(|k| **k == genau) {
            return Some(k);
        }
    }
    WOERTERBUECHER.iter().find(|k| **k == lang).copied()
}

pub struct Pruefer {
    wb: spellbook::Dictionary,
    /// Schweiz: Das Wörterbuch kennt nur „ß“, geschrieben wird „ss“.
    ohne_eszett: bool,
}

impl Pruefer {
    pub fn aus_text(aff: &str, dic: &str) -> Result<Self, String> {
        let wb = spellbook::Dictionary::new(aff, dic).map_err(|e| e.to_string())?;
        Ok(Pruefer {
            wb,
            ohne_eszett: false,
        })
    }

    /// `<ordner>/<kennung>/index.aff` und `index.dic`.
    pub fn laden(ordner: &Path, kennung: &str) -> Result<Self, String> {
        let lesen = |datei: &str| {
            let p = ordner.join(kennung).join(datei);
            std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
        };
        let mut p = Self::aus_text(&lesen("index.aff")?, &lesen("index.dic")?)
            .map_err(|e| tr!("Wörterbuch {kennung}: {fehler}", kennung, fehler = e))?;
        p.ohne_eszett = kennung == "de-CH";
        Ok(p)
    }

    /// Ein eigenes Wort gilt ab jetzt als richtig (auch großgeschrieben am Satzanfang).
    pub fn aufnehmen(&mut self, wort: &str) {
        let _ = self.wb.add(wort);
    }

    fn bekannt(&self, w: &str) -> bool {
        self.wb.check(w)
            || (w.contains('’') && self.wb.check(&w.replace('’', "'")))
            || (w.contains('\'') && self.wb.check(&w.replace('\'', "’")))
            || (self.ohne_eszett && w.contains("ss") && self.wb.check(&w.replace("ss", "ß")))
    }

    /// Richtig ist ein Wort, das das Wörterbuch kennt; sonst gelten Teile mit Bindestrich bzw.
    /// Apostroph (`E-Mail-Adresse`, `dell'anno`) je für sich.
    pub fn richtig(&self, wort: &str) -> bool {
        if uebergehen(wort) || self.bekannt(wort) {
            return true;
        }
        if let Some(i) = wort.find(['\'', '’']) {
            let links = &wort[..i];
            let ap = wort[i..].chars().next().map_or(1, char::len_utf8);
            let rest = &wort[i + ap..];
            let links_gut = self.bekannt(&wort[..i + ap]) || self.richtig(links);
            if links_gut && !rest.is_empty() && self.richtig(rest) {
                return true;
            }
        }
        if wort.contains('-') {
            return wort.split('-').all(|t| t.is_empty() || self.richtig(t));
        }
        false
    }

    /// Höchstens sechs Vorschläge.
    pub fn vorschlaege(&self, wort: &str) -> Vec<String> {
        let mut aus = Vec::new();
        self.wb.suggest(wort, &mut aus);
        if self.ohne_eszett {
            for v in &mut aus {
                *v = v.replace('ß', "ss");
            }
            let mut gesehen = std::collections::HashSet::new();
            aus.retain(|v| gesehen.insert(v.clone()));
        }
        aus.truncate(6);
        aus
    }
}

/// Zu kurz, mit Ziffern oder ganz in Großbuchstaben (Abkürzung, auch `PDFs`): wird nicht geprüft.
fn uebergehen(w: &str) -> bool {
    let buchstaben: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
    let gross = |b: &[char]| b.iter().all(|c| !c.is_lowercase());
    buchstaben.len() < 2
        || w.chars().any(|c| c.is_numeric())
        || gross(&buchstaben)
        || (buchstaben.len() > 2
            && buchstaben.ends_with(&['s'])
            && gross(&buchstaben[..buchstaben.len() - 1]))
}

fn verbinder(c: char) -> bool {
    matches!(c, '\'' | '’' | '-' | '‑')
}

/// Byte-Bereiche zusammenhängender Textstücke (Text und typografische Anführungszeichen).
fn laeufe(k: &LinkedNode, aus: &mut Vec<Range<usize>>) {
    match k.kind() {
        SyntaxKind::Text | SyntaxKind::SmartQuote => {
            let r = k.range();
            match aus.last_mut() {
                Some(l) if l.end == r.start => l.end = r.end,
                _ => aus.push(r),
            }
        }
        SyntaxKind::Equation | SyntaxKind::Raw => {}
        _ => {
            for kind in k.children() {
                laeufe(&kind, aus);
            }
        }
    }
}

/// Die prüfbaren Wörter des Fließtexts als Byte-Bereiche in der Quelle.
pub fn woerter(quelle: &Source) -> Vec<Range<usize>> {
    let mut stuecke = Vec::new();
    laeufe(&LinkedNode::new(quelle.root()), &mut stuecke);
    let text = quelle.text();
    let mut aus = Vec::new();
    for lauf in stuecke {
        let mut beginn: Option<usize> = None;
        let mut ende = lauf.start;
        let abschliessen = |b: Option<usize>, e: usize, aus: &mut Vec<Range<usize>>| {
            let Some(b) = b else { return };
            let w = &text[b..e];
            let vorne = w.len() - w.trim_start_matches(verbinder).len();
            let hinten = w.trim_end_matches(verbinder).len();
            if vorne < hinten && !uebergehen(&w[vorne..hinten]) {
                aus.push(b + vorne..b + hinten);
            }
        };
        for (i, c) in text[lauf.clone()].char_indices() {
            let i = lauf.start + i;
            if c.is_alphanumeric() || verbinder(c) {
                beginn.get_or_insert(i);
                ende = i + c.len_utf8();
            } else {
                abschliessen(beginn.take(), ende, &mut aus);
            }
        }
        abschliessen(beginn.take(), ende, &mut aus);
    }
    aus
}

/// Wörter, die der Prüfer nicht kennt.
pub fn falsche_woerter(quelle: &Source, p: &Pruefer) -> Vec<Range<usize>> {
    let text = quelle.text();
    woerter(quelle)
        .into_iter()
        .filter(|r| !p.richtig(&text[r.clone()]))
        .collect()
}
