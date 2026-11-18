//! Inkognito: Personendaten für KI-Assistenten (MCP) durch Ersatztexte ersetzen und beim
//! Zurückschreiben wieder einsetzen.
//!
//! Inkognito sind Formularfelder, die das Schema als sensibel kennzeichnet (`x-tftn-sensibel`) oder
//! die der Benutzer im Projekt ankreuzt (`[inkognito]` in `tftn.toml`). Ihre Werte stammen aus der
//! Datendatei und den Datensätzen einer Serie im Projekt. Aus jedem Wert werden Suchmuster: der
//! ganze Wert, bei Namen auch die Teile, bei Datumswerten die üblichen Schreibweisen, bei Nummern
//! auch mit anderen Trennzeichen. Groß- und Kleinschreibung und Zwischenraum sind egal.
//!
//! Jede gefundene Schreibweise bekommt einen eigenen Platzhalter (`[Patientin]`,
//! `[Patientin, Teil 2]`, `[Patientin · Schreibweise 2]`). So ist das Zurücksetzen genau: Was die KI
//! nicht ändert, steht danach wieder Zeichen für Zeichen wie vorher da. Die Zuordnung bleibt
//! bestehen, solange das Dokument offen ist (`Tabelle`).
//!
//! Was nicht erkannt wird: andere Beugungen als das Genitiv-s, Spitznamen, Tippfehler und Bilder.

use std::collections::{BTreeMap, HashMap};

use regex::Regex;
use serde::Serialize;
use serde_json::Value as Json;
use toml_edit::{DocumentMut, Item, Value};

use crate::schema::{Art, Ersatz, Feld, Formular};
use tftn_sprachen::tr;

// --- Markierungen im Projekt (`[inkognito]` in tftn.toml) -----------------------------------------

/// Was der Benutzer im Projekt für ein Feld festgelegt hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Markierung {
    /// inkognito mit dem Ersatz der Vorlage
    An,
    /// nicht inkognito, auch wenn das Schema das Feld als sensibel kennzeichnet
    Aus,
    /// inkognito mit eigenem Ersatztext
    Text(String),
}

/// Feldpfad (`patient.name`, Einträge ohne Nummer: `autoren.name`) → Markierung.
pub type Markierungen = BTreeMap<String, Markierung>;

/// Liest `[inkognito]` aus dem Text von `tftn.toml`; Unlesbares zählt als nicht festgelegt.
pub fn markierungen_lesen(tftn_toml: &str) -> Markierungen {
    let Ok(doc) = tftn_toml.parse::<DocumentMut>() else {
        return Markierungen::new();
    };
    let Some(t) = doc.get("inkognito").and_then(Item::as_table_like) else {
        return Markierungen::new();
    };
    t.iter()
        .filter_map(|(k, v)| {
            let m = match v.as_value()? {
                Value::Boolean(b) if *b.value() => Markierung::An,
                Value::Boolean(_) => Markierung::Aus,
                Value::String(s) => match ersatz_bereinigen(s.value()) {
                    Some(t) => Markierung::Text(t),
                    None => Markierung::An,
                },
                _ => return None,
            };
            Some((k.to_string(), m))
        })
        .collect()
}

/// Setzt oder entfernt (`None`) die Markierung eines Felds in `tftn.toml`; der Rest der Datei
/// bleibt, wie er ist.
pub fn markierung_schreiben(
    tftn_toml: &str,
    pfad: &str,
    m: Option<&Markierung>,
) -> Result<String, String> {
    let mut doc = tftn_toml
        .parse::<DocumentMut>()
        .map_err(|e| e.message().to_string())?;
    match m {
        None => {
            if let Some(t) = doc.get_mut("inkognito").and_then(Item::as_table_like_mut) {
                t.remove(pfad);
                if t.is_empty() {
                    doc.remove("inkognito");
                }
            }
        }
        Some(m) => {
            if !doc.contains_key("inkognito") {
                doc["inkognito"] = toml_edit::table();
            }
            let t = doc["inkognito"]
                .as_table_like_mut()
                .ok_or_else(|| tr!("[inkognito] in tftn.toml ist keine Tabelle"))?;
            let wert = match m {
                Markierung::An => Value::from(true),
                Markierung::Aus => Value::from(false),
                Markierung::Text(s) => Value::from(
                    ersatz_bereinigen(s).ok_or_else(|| tr!("Der Ersatztext ist leer."))?,
                ),
            };
            t.insert(pfad, Item::Value(wert));
        }
    }
    Ok(doc.to_string())
}

/// Ersatztext ohne Klammern und überflüssigen Zwischenraum; `None`, wenn nichts bleibt.
fn ersatz_bereinigen(s: &str) -> Option<String> {
    let t: String = s.chars().filter(|c| !matches!(c, '[' | ']')).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

// --- Regeln: welche Felder inkognito sind ----------------------------------------------------------

/// Ein Feld, das inkognito ist.
#[derive(Debug, Clone, PartialEq)]
pub struct Regel {
    /// Namen vom Formular bis zum Feld; Einträge ohne Nummer
    pub pfad: Vec<String>,
    pub titel: String,
    pub ersatz: Ersatz,
    pub datum: bool,
}

impl Regel {
    pub fn schluessel(&self) -> String {
        self.pfad.join(".")
    }
}

/// Wie ein Feld im Fach „Dokument“ zur Inkognito-Frage steht.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FeldStand {
    pub an: bool,
    /// eigener Ersatztext des Projekts
    pub eigen: Option<String>,
    /// Ersatz, den die Vorlage vorgibt, in Worten (etwa „Patientin / Patient / Patient/in“)
    pub vorgabe: String,
    /// sensibel laut Schema (von Haus aus an)
    pub sensibel: bool,
}

/// Alle Felder mit einem Wert und ihr Stand, nach Feldpfad.
pub fn feld_staende(formular: &Formular, m: &Markierungen) -> BTreeMap<String, FeldStand> {
    let mut aus = BTreeMap::new();
    blaetter(&formular.felder, &mut Vec::new(), &mut |pfad, f| {
        let schluessel = pfad.join(".");
        let markierung = m.get(&schluessel);
        aus.insert(
            schluessel,
            FeldStand {
                an: match markierung {
                    Some(Markierung::Aus) => false,
                    Some(_) => true,
                    None => f.sensibel,
                },
                eigen: match markierung {
                    Some(Markierung::Text(t)) => Some(t.clone()),
                    _ => None,
                },
                vorgabe: match vorlage_ersatz(f) {
                    Ersatz::Text(t) => t,
                    Ersatz::Nach { je, sonst, .. } => {
                        let mut v: Vec<String> = je.into_iter().map(|(_, t)| t).collect();
                        v.push(sonst);
                        v.dedup();
                        v.join(" / ")
                    }
                },
                sensibel: f.sensibel,
            },
        );
    });
    aus
}

/// Die Felder, die inkognito sind.
pub fn regeln(formular: &Formular, m: &Markierungen) -> Vec<Regel> {
    let mut aus = Vec::new();
    blaetter(&formular.felder, &mut Vec::new(), &mut |pfad, f| {
        let ersatz = match m.get(&pfad.join(".")) {
            Some(Markierung::Aus) => return,
            Some(Markierung::Text(t)) => Ersatz::Text(t.clone()),
            Some(Markierung::An) => vorlage_ersatz(f),
            None if f.sensibel => vorlage_ersatz(f),
            None => return,
        };
        aus.push(Regel {
            pfad: pfad.to_vec(),
            titel: f.titel.clone(),
            ersatz,
            datum: matches!(f.art, Art::Datum { .. }),
        });
    });
    aus
}

/// Ersatz der Vorlage in der Sprache der Oberfläche; ohne `x-tftn-ersatz` die Feldbeschriftung.
fn vorlage_ersatz(f: &Feld) -> Ersatz {
    let u = |t: &str| tftn_sprachen::uebersetzen(t);
    match &f.ersatz {
        Some(Ersatz::Text(t)) => Ersatz::Text(u(t)),
        Some(Ersatz::Nach { nach, je, sonst }) => Ersatz::Nach {
            nach: nach.clone(),
            je: je.iter().map(|(k, t)| (k.clone(), u(t))).collect(),
            sonst: u(sonst),
        },
        None => Ersatz::Text(ersatz_bereinigen(&u(&f.titel)).unwrap_or("Wert".into())),
    }
}

/// Ruft `f` für jedes Feld mit einem Wert auf (nicht für Gruppen und Einträge selbst).
fn blaetter(felder: &[Feld], pfad: &mut Vec<String>, f: &mut impl FnMut(&[String], &Feld)) {
    for feld in felder {
        pfad.push(feld.name.clone());
        match &feld.art {
            Art::Gruppe { felder } | Art::Eintraege { felder, .. } => blaetter(felder, pfad, f),
            _ => f(pfad, feld),
        }
        pfad.pop();
    }
}

// --- Werte: was ersetzt wird ----------------------------------------------------------------------

/// Ein Wert eines Felds, das inkognito ist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wert {
    /// so, wie er in den Daten steht
    pub echt: String,
    /// Ersatztext ohne Nummer, etwa `Patientin` oder `geb. 1970`
    pub basis: String,
    pub art: WertArt,
    /// Feldbeschriftung (für Statusleiste und Netzprotokoll)
    pub titel: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WertArt {
    Text,
    Datum { jahr: u32, monat: u32, tag: u32 },
    Nummer,
}

/// Die Datensätze einer TOML-Datei: der ganze Inhalt oder, bei einer Serie in einer Datei, jeder
/// Eintrag von `[[serie]]`.
pub fn datensaetze(toml: &str) -> Result<Vec<Json>, String> {
    let werte = crate::Daten::lesen(toml)?.werte();
    match werte.get("serie").and_then(Json::as_array) {
        Some(a) if a.iter().all(Json::is_object) => Ok(a.clone()),
        _ => Ok(vec![werte]),
    }
}

/// Die Werte aller Regeln in allen Datensätzen, ohne Doppelte.
pub fn werte(regeln: &[Regel], saetze: &[Json]) -> Vec<Wert> {
    let mut aus: Vec<Wert> = Vec::new();
    for satz in saetze {
        for r in regeln {
            let mut funde = Vec::new();
            sammeln(satz, &r.pfad, &mut Vec::new(), &mut funde);
            for (wert, stellen, zeile) in funde {
                let echt = match &wert {
                    Json::String(s) => s.clone(),
                    Json::Number(n) => n.to_string(),
                    _ => continue,
                };
                if echt.trim().is_empty() {
                    continue;
                }
                let art = match datum_lesen(&echt) {
                    Some((jahr, monat, tag)) if r.datum => WertArt::Datum { jahr, monat, tag },
                    _ if ist_nummer(&echt) => WertArt::Nummer,
                    _ => WertArt::Text,
                };
                let mut basis = match &r.ersatz {
                    Ersatz::Text(t) => t.clone(),
                    Ersatz::Nach { nach, je, sonst } => {
                        let wert = nachschlagen(satz, nach, &stellen);
                        je.iter()
                            .find(|(k, _)| Some(k.as_str()) == wert.as_deref())
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| sonst.clone())
                    }
                };
                let (jahr, monat) = match art {
                    WertArt::Datum { jahr, monat, .. } => {
                        (jahr.to_string(), format!("{monat:02}/{jahr}"))
                    }
                    _ => (String::new(), String::new()),
                };
                basis = basis.replace("{jahr}", &jahr).replace("{monat}", &monat);
                let basis = ersatz_bereinigen(&basis).unwrap_or_else(|| r.titel.clone());
                let basis = match zeile {
                    Some(i) => tr!("{basis}, Zeile {nummer}", basis, nummer = i + 1),
                    None => basis,
                };
                if !aus.iter().any(|w| w.echt == echt && w.basis == basis) {
                    aus.push(Wert {
                        echt,
                        basis,
                        art,
                        titel: r.titel.clone(),
                    });
                }
            }
        }
    }
    aus
}

/// Werte unter `pfad`; Listen von Einträgen werden durchlaufen (Nummern in `stellen`), eine Liste
/// von Texten am Ende ergibt je Zeile einen Wert (mit Zeilennummer, wenn es mehrere sind).
fn sammeln(
    knoten: &Json,
    pfad: &[String],
    stellen: &mut Vec<usize>,
    aus: &mut Vec<(Json, Vec<usize>, Option<usize>)>,
) {
    match (pfad.split_first(), knoten) {
        (None, Json::Array(a)) => {
            let mehrere = a.len() > 1;
            for (i, w) in a.iter().enumerate() {
                aus.push((w.clone(), stellen.clone(), mehrere.then_some(i)));
            }
        }
        (None, w) => aus.push((w.clone(), stellen.clone(), None)),
        (Some(_), Json::Array(a)) => {
            for (i, w) in a.iter().enumerate() {
                stellen.push(i);
                sammeln(w, pfad, stellen, aus);
                stellen.pop();
            }
        }
        (Some((kopf, rest)), Json::Object(o)) => {
            if let Some(w) = o.get(kopf) {
                sammeln(w, rest, stellen, aus);
            }
        }
        _ => {}
    }
}

/// Wert eines anderen Felds (`patient.geschlecht`); in Listen von Einträgen derselbe Eintrag.
fn nachschlagen(satz: &Json, pfad: &str, stellen: &[usize]) -> Option<String> {
    let mut knoten = satz;
    let mut stellen = stellen.iter();
    for teil in pfad.split('.') {
        while let Json::Array(a) = knoten {
            knoten = a.get(*stellen.next().unwrap_or(&0))?;
        }
        knoten = knoten.get(teil)?;
    }
    match knoten {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(n.to_string()),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn datum_lesen(s: &str) -> Option<(u32, u32, u32)> {
    let mut t = s.trim().splitn(3, '-');
    let (j, m, d) = (t.next()?, t.next()?, t.next()?);
    if j.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (j, m, d) = (j.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some((j, m, d))
}

/// Mindestens fünf Ziffern und sonst nur Trennzeichen (SV-Nummer, Telefon). Mit Buchstaben (IBAN,
/// Aktenzeichen) ist es ein Text: Die Buchstaben gehören zum Muster.
fn ist_nummer(s: &str) -> bool {
    let ziffern = s.chars().filter(char::is_ascii_digit).count();
    ziffern >= 5
        && s.chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || "./-()+".contains(c))
}

// --- Suchmuster ------------------------------------------------------------------------------------

/// Zwischenraum: Leerzeichen, Zeilenumbruch (auch mit der Zeilennummer, die `quelltext_lesen`
/// davorsetzt) und geschütztes Leerzeichen von Typst (`~`).
const RAUM: &str = r"(?:\n\d+\t|\s|~)+";

/// Zwischenraum, der auch fehlen darf.
fn raum0() -> String {
    format!("(?:{RAUM})?")
}

/// Buchstaben, die zusammengesetzt (NFD, etwa in macOS-Dateinamen) anders aussehen: Grundbuchstabe
/// und kombinierendes Zeichen.
const ZUSAMMENGESETZT: [(char, char, char); 30] = [
    ('ä', 'a', '\u{308}'),
    ('ö', 'o', '\u{308}'),
    ('ü', 'u', '\u{308}'),
    ('ë', 'e', '\u{308}'),
    ('ï', 'i', '\u{308}'),
    ('ÿ', 'y', '\u{308}'),
    ('á', 'a', '\u{301}'),
    ('é', 'e', '\u{301}'),
    ('í', 'i', '\u{301}'),
    ('ó', 'o', '\u{301}'),
    ('ú', 'u', '\u{301}'),
    ('ć', 'c', '\u{301}'),
    ('ń', 'n', '\u{301}'),
    ('ś', 's', '\u{301}'),
    ('ź', 'z', '\u{301}'),
    ('à', 'a', '\u{300}'),
    ('è', 'e', '\u{300}'),
    ('ì', 'i', '\u{300}'),
    ('ò', 'o', '\u{300}'),
    ('ù', 'u', '\u{300}'),
    ('â', 'a', '\u{302}'),
    ('ê', 'e', '\u{302}'),
    ('î', 'i', '\u{302}'),
    ('ô', 'o', '\u{302}'),
    ('û', 'u', '\u{302}'),
    ('ñ', 'n', '\u{303}'),
    ('ç', 'c', '\u{327}'),
    ('č', 'c', '\u{30c}'),
    ('š', 's', '\u{30c}'),
    ('ž', 'z', '\u{30c}'),
];

/// Ein Zeichen als Muster: wörtlich oder, bei Buchstaben mit Akzent, auch zusammengesetzt.
fn zeichen_muster(c: char) -> String {
    let klein = c.to_lowercase().next().unwrap_or(c);
    match ZUSAMMENGESETZT.iter().find(|(z, _, _)| *z == klein) {
        Some((_, basis, zeichen)) => format!(
            "(?:{}|{}{})",
            regex::escape(&c.to_string()),
            regex::escape(&basis.to_string()),
            regex::escape(&zeichen.to_string())
        ),
        None => regex::escape(&c.to_string()),
    }
}

/// Wörter, die allein nie ersetzt werden (Anreden, Titel, Füllwörter).
const STOPP: [&str; 32] = [
    "frau", "herr", "herrn", "familie", "fam", "dr", "mag", "prof", "dipl", "ing", "med", "univ",
    "und", "von", "van", "der", "die", "das", "den", "dem", "de", "zu", "zur", "zum", "mr", "mrs",
    "ms", "miss", "the", "and", "of", "patient",
];

const MONATE_DE: [&str; 12] = [
    "Januar|Jänner|Jan\\.?",
    "Februar|Feber|Feb\\.?",
    "März|Mär\\.?|Mrz\\.?",
    "April|Apr\\.?",
    "Mai",
    "Juni|Jun\\.?",
    "Juli|Jul\\.?",
    "August|Aug\\.?",
    "September|Sept?\\.?",
    "Oktober|Okt\\.?",
    "November|Nov\\.?",
    "Dezember|Dez\\.?",
];

const MONATE_EN: [&str; 12] = [
    "January|Jan\\.?",
    "February|Feb\\.?",
    "March|Mar\\.?",
    "April|Apr\\.?",
    "May",
    "June|Jun\\.?",
    "July|Jul\\.?",
    "August|Aug\\.?",
    "September|Sept?\\.?",
    "October|Oct\\.?",
    "November|Nov\\.?",
    "December|Dec\\.?",
];

#[derive(Debug)]
struct Muster {
    re: Regex,
    /// Ersatz mit Nummer, ohne Klammern: `Patientin 2`
    name: String,
    /// `""` oder `, Teil 2`
    form: String,
}

/// Regulärer Ausdruck für einen Text: Wörter wörtlich (Akzente auch zusammengesetzt),
/// Zwischenraum beliebig.
fn text_muster(s: &str) -> String {
    s.split_whitespace()
        .map(|w| w.chars().map(zeichen_muster).collect::<String>())
        .collect::<Vec<_>>()
        .join(RAUM)
}

/// Teile eines Namens: Wörter (und Teile mit Bindestrich) mit mindestens drei Buchstaben, ohne
/// Abkürzungen und Stoppwörter; Werte mit Ziffern werden nicht zerlegt.
fn teile(s: &str) -> Vec<String> {
    let mut aus: Vec<String> = Vec::new();
    let woerter: Vec<&str> = s.split_whitespace().collect();
    // Anschriften („1010 Wien“) nicht zerlegen: „Wien“ allein ist kein Personenbezug.
    if woerter.len() < 2 && !s.contains('-') || s.chars().any(|c| c.is_ascii_digit()) {
        return aus;
    }
    for wort in woerter {
        let wort = wort.trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '(' | ')' | '"'));
        let mut kandidaten = vec![wort];
        if wort.contains('-') {
            kandidaten.extend(wort.split('-'));
        }
        for k in kandidaten {
            let buchstaben = k.chars().filter(|c| c.is_alphabetic()).count();
            if buchstaben < 3
                || k.ends_with('.')
                || k.chars().any(|c| c.is_ascii_digit())
                || STOPP.contains(&k.to_lowercase().as_str())
                || k == s
                || aus.iter().any(|a| a == k)
            {
                continue;
            }
            aus.push(k.to_string());
        }
    }
    aus
}

fn datum_muster(jahr: u32, monat: u32, tag: u32) -> Vec<String> {
    let (j, m, d) = (jahr, monat, tag);
    let jj = format!("{:02}", j % 100);
    let tz = format!(r"0?{d}");
    let mz = format!(r"0?{m}");
    let de = MONATE_DE[(m - 1) as usize];
    let en = MONATE_EN[(m - 1) as usize];
    let (r0, r1) = (raum0(), RAUM);
    vec![
        format!(r"{j}-{m:02}-{d:02}"),
        format!(r"{tz}\.{r0}{mz}\.{r0}{j}"),
        format!(r"{tz}\.{r0}{mz}\.{r0}{jj}"),
        format!(r"{tz}\.?{r0}(?:{de}){r0}{j}"),
        format!(r"{tz}(?:st|nd|rd|th)?{r1}(?:{en}){r1}{j}"),
        format!(r"(?:{en}){r1}{tz}(?:st|nd|rd|th)?,?{r1}{j}"),
        format!(r"{tz}/{mz}/{j}"),
        format!(r"{mz}/{tz}/{j}"),
        // So zeigt Typst ein Datum in Meldungen (`repr`).
        format!(r"datetime\({r0}year:{r0}{j},{r0}month:{r0}{m},{r0}day:{r0}{d}{r0},?{r0}\)"),
    ]
}

fn nummer_muster(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_digit)
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(r"(?:[./()+-]|\s|~|\n\d+\t){0,3}")
}

/// Steht ein Fund für sich? Davor kein Buchstabe und keine Ziffer (oder gleich der vorige Fund),
/// damit „Erika“ nicht in „Amerika“ trifft. Danach höchstens eine kurze Endung (s, n, en, es, ns:
/// „Musterfraus“, „Musterfrauen“), damit „Ben“ nicht in „Benutzer“ trifft. `_` zählt nicht als
/// Buchstabe: `_Musterfrau_` ist Typst-Kursiv. Grenze: ein Name, der links an einem Wort klebt, wird
/// nicht erkannt.
fn frei(text: &str, von: usize, bis: usize, voriges_ende: Option<usize>) -> bool {
    let davor = voriges_ende == Some(von)
        || !text[..von]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric);
    if !davor {
        return false;
    }
    let endung: String = text[bis..]
        .chars()
        .take_while(|c| c.is_alphanumeric())
        .take(3)
        .collect();
    matches!(
        endung.to_lowercase().as_str(),
        "" | "s" | "n" | "en" | "es" | "ns"
    )
}

// --- Die Tabelle ----------------------------------------------------------------------------------

/// Zuordnung Platzhalter ↔ echte Schreibweise für ein offenes Dokument, dazu die Suchmuster der
/// aktuellen Werte.
#[derive(Debug, Default)]
pub struct Tabelle {
    /// (Platzhalter mit Klammern, echte Schreibweise), in der Reihenfolge der Vergabe
    paare: Vec<(String, String)>,
    nach_echt: HashMap<String, usize>,
    nach_platzhalter: HashMap<String, usize>,
    /// Ersatz ohne Nummer → Werte (klein geschrieben) in der Reihenfolge ihrer Nummer
    nummern: HashMap<String, Vec<String>>,
    muster: Vec<Muster>,
    /// Feldbeschriftungen der aktuellen Werte
    titel: Vec<String>,
    /// Anzahl der aktuellen Werte
    anzahl: usize,
    /// Text in eckigen Klammern, der wie ein Platzhalter aussieht, aber so im echten Text steht
    woertlich: std::collections::HashSet<String>,
    /// Alle Ersatztexte, die die Regeln ergeben können (vereinfacht, siehe `vereinfacht`)
    basen: Vec<String>,
}

/// Für den Vergleich mit möglichen Platzhaltern: klein, jede Ziffer als 0, `{jahr}` als 0000.
fn vereinfacht(s: &str) -> String {
    s.to_lowercase()
        .replace("{jahr}", "0000")
        .replace("{monat}", "00/0000")
        .chars()
        .map(|c| if c.is_ascii_digit() { '0' } else { c })
        .collect()
}

impl Tabelle {
    /// Gibt es etwas zu ersetzen?
    pub fn aktiv(&self) -> bool {
        !self.muster.is_empty()
    }

    /// Feldbeschriftungen der aktuellen Werte, ohne Doppelte.
    pub fn felder(&self) -> &[String] {
        &self.titel
    }

    pub fn anzahl(&self) -> usize {
        self.anzahl
    }

    /// Alle bisher vergebenen Platzhalter mit der echten Schreibweise.
    pub fn paare(&self) -> &[(String, String)] {
        &self.paare
    }

    /// Merkt alle Ersatztexte, die die Regeln ergeben können (auch für Werte, die es noch nicht
    /// gibt): Ein Platzhalter, der so aussieht, aber nicht vergeben ist, wird abgelehnt.
    pub fn regeln_merken(&mut self, regeln: &[Regel]) {
        for r in regeln {
            let texte: Vec<&String> = match &r.ersatz {
                Ersatz::Text(t) => vec![t],
                Ersatz::Nach { je, sonst, .. } => {
                    je.iter().map(|(_, t)| t).chain([sonst]).collect()
                }
            };
            for t in texte {
                let v = vereinfacht(t);
                if !self.basen.contains(&v) {
                    self.basen.push(v);
                }
            }
        }
    }

    /// Einige vergebene Platzhalter, als Beispiel für den Assistenten.
    pub fn beispiele(&self, n: usize) -> Vec<String> {
        self.paare.iter().take(n).map(|(p, _)| p.clone()).collect()
    }

    /// Setzt die Suchmuster nach den Werten; vergebene Platzhalter bleiben. Jeder Wert bekommt
    /// gleich seinen Platzhalter, damit der Assistent ihn verwenden kann, bevor er etwas gelesen hat.
    /// `Err`, wenn sich ein Muster nicht bauen lässt (dann ist der Wert nicht sicher zu ersetzen).
    pub fn werte_setzen(&mut self, werte: &[Wert]) -> Result<(), String> {
        self.muster.clear();
        self.titel.clear();
        self.anzahl = werte.len();
        for w in werte {
            if !self.titel.contains(&w.titel) {
                self.titel.push(w.titel.clone());
            }
            let liste = self.nummern.entry(w.basis.clone()).or_default();
            let schluessel = w.echt.to_lowercase();
            let nr = match liste.iter().position(|x| *x == schluessel) {
                Some(i) => i,
                None => {
                    liste.push(schluessel);
                    liste.len() - 1
                }
            };
            let name = if nr == 0 {
                w.basis.clone()
            } else {
                format!("{} {}", w.basis, nr + 1)
            };
            let mut fehler = None;
            let mut neu = |muster: String, form: String| match Regex::new(&format!("(?i){muster}"))
            {
                Ok(re) => self.muster.push(Muster {
                    re,
                    name: name.clone(),
                    form,
                }),
                Err(e) => fehler = Some(e.to_string()),
            };
            match w.art {
                WertArt::Datum { jahr, monat, tag } => {
                    for m in datum_muster(jahr, monat, tag) {
                        neu(m, String::new());
                    }
                }
                WertArt::Nummer => neu(nummer_muster(&w.echt), String::new()),
                WertArt::Text => {
                    if w.echt.trim().chars().count() < 2
                        || STOPP.contains(&w.echt.trim().to_lowercase().as_str())
                    {
                        continue;
                    }
                    neu(text_muster(&w.echt), String::new());
                    for (i, t) in teile(&w.echt).into_iter().enumerate() {
                        neu(text_muster(&t), tr!(", Teil {nummer}", nummer = i + 1));
                    }
                }
            }
            if let Some(e) = fehler {
                return Err(tr!(
                    "Ein Wert des Felds {feld} lässt sich nicht als Suchmuster fassen ({fehler}).",
                    feld = w.titel,
                    fehler = e
                ));
            }
            if !matches!(w.art, WertArt::Text)
                || (w.echt.trim().chars().count() >= 2
                    && !STOPP.contains(&w.echt.trim().to_lowercase().as_str()))
            {
                self.platzhalter(&name, "", &w.echt, "");
            }
        }
        Ok(())
    }

    /// Platzhalter für eine Schreibweise; neu vergeben, wenn es sie noch nicht gibt. Ein Name, der
    /// schon vergeben ist oder im Text vorkommt, bekommt den Zusatz „Schreibweise n“.
    fn platzhalter(&mut self, name: &str, form: &str, echt: &str, text: &str) -> String {
        if let Some(&i) = self.nach_echt.get(echt) {
            return self.paare[i].0.clone();
        }
        let mut p = format!("[{name}{form}]");
        let mut n = 1;
        while self.nach_platzhalter.contains_key(&p) || text.contains(&p) {
            n += 1;
            p = format!(
                "[{name}{form} · {}]",
                tr!("Schreibweise {nummer}", nummer = n)
            );
        }
        let i = self.paare.len();
        self.paare.push((p.clone(), echt.to_string()));
        self.nach_echt.insert(echt.to_string(), i);
        self.nach_platzhalter.insert(p.clone(), i);
        p
    }

    /// Ersetzt jeden Fund durch seinen Platzhalter. Überlappen Funde, gewinnt der frühere, bei
    /// gleichem Anfang der längere.
    pub fn verbergen(&mut self, text: &str) -> String {
        self.verbergen_mit(text, true)
    }

    /// `schuetzen`: vergebene Platzhalter, die wörtlich im Text stehen, als „wörtlich“ markieren
    /// (nicht nötig, wo die Platzhalter von TFTN selbst stammen, etwa nach `toml_verbergen`).
    pub fn verbergen_mit(&mut self, text: &str, schuetzen: bool) -> String {
        if self.muster.is_empty() {
            return text.to_string();
        }
        self.woertliche_merken(text);
        // Ein Durchgang von links: an jeder Stelle der früheste freie Fund, bei gleichem Anfang
        // der längste. `naechste[i]`: nächster Fund des Musters i ab der Stelle, ungeprüft.
        let mut naechste: Vec<Option<(usize, usize)>> = vec![None; self.muster.len()];
        let mut aus = String::with_capacity(text.len());
        let mut pos = 0;
        let mut voriges: Option<usize> = None;
        loop {
            let mut bester: Option<(usize, usize, usize)> = None;
            for (i, m) in self.muster.iter().enumerate() {
                let mut ab = pos;
                loop {
                    let f = match naechste[i] {
                        Some(f) if f.0 >= ab => Some(f),
                        _ => {
                            m.re.find_at(text, ab)
                                .filter(|f| f.start() < f.end())
                                .map(|f| (f.start(), f.end()))
                        }
                    };
                    naechste[i] = f;
                    let Some((von, bis)) = f else { break };
                    if frei(text, von, bis, voriges) {
                        if bester.is_none_or(|b| von < b.0 || von == b.0 && bis > b.1) {
                            bester = Some((von, bis, i));
                        }
                        break;
                    }
                    ab = von + text[von..].chars().next().map_or(1, char::len_utf8);
                }
            }
            let Some((von, bis, i)) = bester else { break };
            let stueck = if schuetzen {
                self.woertlich_schuetzen(&text[pos..von], text)
            } else {
                text[pos..von].to_string()
            };
            aus.push_str(&stueck);
            let (name, form) = (self.muster[i].name.clone(), self.muster[i].form.clone());
            let p = self.platzhalter(&name, &form, &text[von..bis], text);
            aus.push_str(&p);
            pos = bis;
            voriges = Some(bis);
        }
        let stueck = if schuetzen {
            self.woertlich_schuetzen(&text[pos..], text)
        } else {
            text[pos..].to_string()
        };
        aus.push_str(&stueck);
        aus
    }

    /// Steht ein vergebener Platzhalter wörtlich im echten Text (etwa „[Patientin]“ in einer
    /// Vorlage), bekommt er selbst einen Platzhalter („[wörtlich: Patientin]“): Sonst würde er beim
    /// Zurücksetzen zum echten Wert.
    fn woertlich_schuetzen(&mut self, stueck: &str, text: &str) -> String {
        if !stueck.contains('[') || self.nach_platzhalter.is_empty() {
            return stueck.to_string();
        }
        let mut aus = String::with_capacity(stueck.len());
        let mut rest = stueck;
        while let Some(i) = rest.find('[') {
            aus.push_str(&rest[..i]);
            let ab = &rest[i..];
            match ab.find(']').map(|j| &ab[..=j]) {
                Some(k) if self.nach_platzhalter.contains_key(k) => {
                    let p = if let Some(&n) = self.nach_echt.get(k) {
                        self.paare[n].0.clone()
                    } else {
                        let innen = &k[1..k.len() - 1];
                        let mut p = format!("[{} {innen}]", tr!("wörtlich:"));
                        while self.nach_platzhalter.contains_key(&p) || text.contains(&p) {
                            p = format!("[{} {}]", tr!("wörtlich:"), &p[1..p.len() - 1]);
                        }
                        let n = self.paare.len();
                        self.paare.push((p.clone(), k.to_string()));
                        self.nach_echt.insert(k.to_string(), n);
                        self.nach_platzhalter.insert(p.clone(), n);
                        p
                    };
                    aus.push_str(&p);
                    rest = &ab[k.len()..];
                }
                _ => {
                    aus.push('[');
                    rest = &ab[1..];
                }
            }
        }
        aus.push_str(rest);
        aus
    }

    /// Merkt Text wie `[Patientin]`, der schon im echten Text steht und kein Platzhalter ist: Beim
    /// Zurücksetzen ist er dann kein Fehler.
    fn woertliche_merken(&mut self, text: &str) {
        let mut rest = text;
        while let Some(i) = rest.find('[') {
            let ab = &rest[i..];
            if let Some(j) = ab.find(']') {
                let k = &ab[..=j];
                if !self.nach_platzhalter.contains_key(k) && self.sieht_aus_wie_platzhalter(k) {
                    self.woertlich.insert(k.to_string());
                }
            }
            rest = &ab[1..];
        }
    }

    /// Ersetzt JSON-Werte in einem Baum (alle Texte).
    pub fn verbergen_json(&mut self, wert: &mut Json) {
        match wert {
            Json::String(s) => *s = self.verbergen(s),
            Json::Array(a) => a.iter_mut().for_each(|w| self.verbergen_json(w)),
            Json::Object(o) => o.values_mut().for_each(|w| self.verbergen_json(w)),
            _ => {}
        }
    }

    /// Setzt die echten Schreibweisen wieder ein. Ein Platzhalter, den es nicht gibt (etwa
    /// `[Patientin 7]`), ist ein Fehler: Er soll nie als Text im Dokument landen.
    pub fn aufdecken(&self, text: &str) -> Result<String, String> {
        let mut aus = String::with_capacity(text.len());
        let mut unbekannt: Vec<String> = Vec::new();
        let mut rest = text;
        while let Some(i) = rest.find('[') {
            aus.push_str(&rest[..i]);
            let ab = &rest[i..];
            let kandidat = ab.find(']').map(|j| &ab[..=j]);
            match kandidat.and_then(|k| self.nach_platzhalter.get(k).map(|&n| (k, n))) {
                Some((k, n)) => {
                    aus.push_str(&self.paare[n].1);
                    rest = &ab[k.len()..];
                }
                None => {
                    if let Some(k) = kandidat
                        && self.sieht_aus_wie_platzhalter(k)
                        && !self.woertlich.contains(k)
                        && !unbekannt.iter().any(|u| u == k)
                    {
                        unbekannt.push(k.to_string());
                    }
                    aus.push('[');
                    rest = &ab[1..];
                }
            }
        }
        aus.push_str(rest);
        if unbekannt.is_empty() {
            Ok(aus)
        } else {
            // Ohne eckige Klammern, damit die Meldung selbst nicht als „wörtlich“ gilt.
            let namen: Vec<String> = unbekannt
                .iter()
                .map(|u| format!("„{}“", &u[1..u.len() - 1]))
                .collect();
            Err(format!(
                "Inkognito: Diese Platzhalter hat TFTN nicht vergeben: {}. Bitte nur Platzhalter \
                 verwenden, die TFTN geliefert hat (genau so geschrieben); nichts geändert.",
                namen.join(", ")
            ))
        }
    }

    /// Beginnt `k` (mit Klammern) wie ein Platzhalter dieser Tabelle?
    /// Groß- und Kleinschreibung und Ziffern zählen dabei nicht („[patientin]“, „[geb. 1971]“), und
    /// auch der Anfang eines möglichen Ersatztexts zählt („[Patient]“ zu „Patient/in“).
    fn sieht_aus_wie_platzhalter(&self, k: &str) -> bool {
        let innen = vereinfacht(&k[1..k.len() - 1]);
        let innen = innen.trim();
        self.nummern
            .keys()
            .map(|b| vereinfacht(b))
            .chain(self.basen.iter().cloned())
            .any(|basis| {
                innen
                    .strip_prefix(basis.as_str())
                    .is_some_and(|r| r.is_empty() || r.starts_with([' ', ',', '·']))
                    || (innen.chars().count() >= 3 && basis.starts_with(innen))
            })
    }

    /// Ersetzt in `echt` so, wie es der Assistent in der Fassung mit Platzhaltern meint: `f` bekommt
    /// den Text mit Platzhaltern, das Ergebnis kommt aufgedeckt zurück. So trifft „alle ersetzen“
    /// genau die Stellen, die der Assistent gesehen hat.
    pub fn im_platzhaltertext(
        &mut self,
        echt: &str,
        f: impl FnOnce(&str) -> Result<String, String>,
    ) -> Result<String, String> {
        let p = self.verbergen(echt);
        let neu = f(&p)?;
        self.aufdecken(&neu)
    }

    /// Würde `text` ersetzt?
    pub fn betrifft(&mut self, text: &str) -> bool {
        self.verbergen(text) != text
    }

    /// Setzt in allen Texten eines JSON-Baums die echten Schreibweisen ein.
    pub fn aufdecken_json(&self, wert: &mut Json) -> Result<(), String> {
        match wert {
            Json::String(s) => *s = self.aufdecken(s)?,
            Json::Array(a) => {
                for w in a {
                    self.aufdecken_json(w)?;
                }
            }
            Json::Object(o) => {
                for w in o.values_mut() {
                    self.aufdecken_json(w)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Eine TOML-Datei (Datendatei, Datensätze) für den Satz eines Seitenbilds: Werte der Regeln
    /// werden ersetzt, Texte durch Platzhalter. Datumswerte mit `datum_als_text` ebenfalls durch
    /// den Platzhalter (als Text; nicht jede Vorlage kann das setzen), sonst durch den 1. Jänner des
    /// Jahres (wenn der Ersatz das Jahr nennt) bzw. den 1. Jänner 1900. Danach der ganze Text wie
    /// jeder andere.
    pub fn toml_verbergen(
        &mut self,
        toml: &str,
        regeln: &[Regel],
        datum_als_text: bool,
    ) -> Result<String, String> {
        let mut doc = toml
            .parse::<DocumentMut>()
            .map_err(|e| e.message().to_string())?;
        let jahr_erlaubt = |r: &Regel| match &r.ersatz {
            Ersatz::Text(t) => t.contains("{jahr}") || t.contains("{monat}"),
            Ersatz::Nach { je, sonst, .. } => {
                je.iter().any(|(_, t)| t.contains("{jahr}")) || sonst.contains("{jahr}")
            }
        };
        let serie = doc
            .get("serie")
            .is_some_and(|s| s.as_array_of_tables().is_some());
        for r in regeln {
            let behalten = jahr_erlaubt(r);
            let mut f = |v: &mut Value| self.wert_ersetzen(v, behalten, datum_als_text);
            if serie {
                if let Some(a) = doc["serie"].as_array_of_tables_mut() {
                    for t in a.iter_mut() {
                        in_tabelle(t, &r.pfad, &mut f);
                    }
                }
            } else {
                in_tabelle(doc.as_table_mut(), &r.pfad, &mut f);
            }
        }
        Ok(self.verbergen_mit(&doc.to_string(), false))
    }

    fn wert_ersetzen(&mut self, v: &mut Value, jahr_behalten: bool, datum_als_text: bool) {
        match v {
            Value::String(s) => {
                let neu = self.verbergen(s.value());
                *v = Value::from(neu);
            }
            Value::Integer(i) => {
                let neu = self.verbergen(&i.value().to_string());
                *v = Value::from(neu);
            }
            Value::Datetime(d) if datum_als_text => {
                let neu = self.verbergen(&d.value().to_string());
                *v = Value::from(neu);
            }
            Value::Datetime(d) => {
                let jahr = d
                    .value()
                    .date
                    .map(|t| t.year)
                    .filter(|_| jahr_behalten)
                    .unwrap_or(1900);
                *v = Value::from(toml_edit::Datetime {
                    date: Some(toml_edit::Date {
                        year: jahr,
                        month: 1,
                        day: 1,
                    }),
                    time: None,
                    offset: None,
                });
            }
            Value::Array(a) => {
                for w in a.iter_mut() {
                    self.wert_ersetzen(w, jahr_behalten, datum_als_text);
                }
            }
            _ => {}
        }
    }
}

// --- Werte in einer TOML-Datei ansteuern ----------------------------------------------------------

fn in_tabelle(t: &mut toml_edit::Table, pfad: &[String], f: &mut impl FnMut(&mut Value)) {
    let Some((kopf, rest)) = pfad.split_first() else {
        return;
    };
    let Some(item) = t.get_mut(kopf) else { return };
    match item {
        Item::Table(t) => in_tabelle(t, rest, f),
        Item::ArrayOfTables(a) => {
            for t in a.iter_mut() {
                in_tabelle(t, rest, f);
            }
        }
        Item::Value(v) => in_wert(v, rest, f),
        Item::None => {}
    }
}

fn in_wert(v: &mut Value, pfad: &[String], f: &mut impl FnMut(&mut Value)) {
    match (pfad.split_first(), v) {
        (None, v) => f(v),
        (Some((kopf, rest)), Value::InlineTable(t)) => {
            if let Some(w) = t.get_mut(kopf) {
                in_wert(w, rest, f);
            }
        }
        (Some(_), Value::Array(a)) => {
            for w in a.iter_mut() {
                in_wert(w, pfad, f);
            }
        }
        _ => {}
    }
}
