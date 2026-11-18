//! CSV als Quelle einer Serie: Adresslisten aus Excel oder der Ordinationssoftware.
//!
//! Ohne eigene Abhängigkeit: ein Leser nach RFC 4180 (Anführungszeichen, verdoppelte `""`,
//! Zeilenumbrüche in Zellen, CRLF oder LF), das Trennzeichen aus der Kopfzeile (`;` wie das
//! deutsche Excel, `,` oder Tabulator), die Kodierung UTF-8 (mit oder ohne BOM), UTF-16 mit BOM,
//! sonst Windows-1252 (so landen Umlaute aus altem Excel richtig).
//!
//! Die Kopfzeile nennt die Spalten. Eine Spalte gehört zu einem Feld des Formulars, wenn ihr Name
//! dem Feldnamen oder -titel entspricht (Groß/klein, Leerzeichen, Satzzeichen und Umlaut-
//! Schreibweisen egal; verschachtelt `patient.name` oder „Patient Name“) oder wenn das Projekt es
//! festlegt: `[serie-csv."<datei>.csv"]` in `tftn.toml`, Spalte = Feldpfad, `""` = nicht verwenden.
//! Gespeichert werden nur Spalten- und Feldnamen, nie Werte.
//!
//! Jede Zeile wird ein Datensatz wie ein Abschnitt `[[serie]]` (`serie_toml`): Werte nach dem Typ
//! des Felds umgewandelt (Zahlen `1.234,50` und `1234.50`, Datum `31.01.2026` und `2026-01-31`),
//! leere Zellen weggelassen (dann gilt `dokument.toml`). Eine Liste von Texten (Anschriftzeilen)
//! kommt aus mehreren Spalten (`Empfänger 1`, `Empfänger 2` …) oder aus Zeilen in einer Zelle.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value as Json;
use tftn_sprachen::tr;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};

use crate::{Art, Feld, Formular};

/// Abschnitt in `tftn.toml` mit den eigenen Zuordnungen, je CSV-Datei (Dateiname).
pub const ABSCHNITT: &str = "serie-csv";
/// Pfad des Pseudo-Felds für den Dateinamen des PDFs (wie `datei` in `[[serie]]`).
pub const DATEI: &str = "datei";

// --- Lesen ---------------------------------------------------------------------------------------

/// Kodierung der Datei, wie erkannt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kodierung {
    #[serde(rename = "UTF-8")]
    Utf8,
    #[serde(rename = "UTF-16")]
    Utf16,
    #[serde(rename = "Windows-1252")]
    Windows1252,
}

/// Windows-1252 für 0x80–0x9F; die fünf unbelegten Stellen bleiben, was Latin-1 sagt.
const W1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

fn zeichen_1252(b: u8) -> char {
    match b {
        0x80..=0x9f => W1252[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// Bytes → Text: UTF-8 (BOM wird entfernt), UTF-16 mit BOM, sonst Windows-1252.
pub fn dekodieren(bytes: &[u8]) -> (String, Kodierung) {
    if let Some(rest) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return (String::from_utf8_lossy(rest).into_owned(), Kodierung::Utf8);
    }
    let utf16 = |rest: &[u8], le: bool| {
        let einheiten: Vec<u16> = rest
            .chunks_exact(2)
            .map(|p| {
                if le {
                    u16::from_le_bytes([p[0], p[1]])
                } else {
                    u16::from_be_bytes([p[0], p[1]])
                }
            })
            .collect();
        (String::from_utf16_lossy(&einheiten), Kodierung::Utf16)
    };
    if let Some(rest) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xfe, 0xff]) {
        return utf16(rest, false);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), Kodierung::Utf8),
        Err(_) => (
            bytes.iter().map(|&b| zeichen_1252(b)).collect(),
            Kodierung::Windows1252,
        ),
    }
}

/// Trennzeichen aus der ersten Zeile: das häufigste von `;`, Tabulator und `,` außerhalb von
/// Anführungszeichen; bei Gleichstand in dieser Reihenfolge (deutsches Excel zuerst).
pub fn trennzeichen(text: &str) -> char {
    let mut zahl = [0usize; 3];
    let mut in_anf = false;
    for c in text.chars() {
        match c {
            '"' => in_anf = !in_anf,
            '\n' | '\r' if !in_anf => break,
            ';' if !in_anf => zahl[0] += 1,
            '\t' if !in_anf => zahl[1] += 1,
            ',' if !in_anf => zahl[2] += 1,
            _ => {}
        }
    }
    let mut best = 0;
    for i in 1..3 {
        if zahl[i] > zahl[best] {
            best = i;
        }
    }
    [';', '\t', ','][best]
}

/// Zerlegt den Text nach RFC 4180 in Zeilen aus Zellen. Eine Zeile ist ein Datensatz, auch wenn
/// eine Zelle Zeilenumbrüche enthält (so zählt auch Excel). Umbrüche in Zellen werden `\n`.
/// Ein Anführungszeichen mitten in einer Zelle ohne Anführungszeichen gilt wörtlich.
pub fn zerlegen(text: &str, trenn: char) -> Result<Vec<Vec<String>>, String> {
    let mut zeilen = Vec::new();
    let mut zeile = Vec::new();
    let mut zelle = String::new();
    let mut zeichen = text.chars().peekable();
    let mut in_anf = false;
    let mut zellanfang = true;
    let mut offen = false;
    while let Some(c) = zeichen.next() {
        offen = true;
        if in_anf {
            match c {
                '"' if zeichen.peek() == Some(&'"') => {
                    zeichen.next();
                    zelle.push('"');
                }
                '"' => in_anf = false,
                '\r' => {
                    if zeichen.peek() == Some(&'\n') {
                        zeichen.next();
                    }
                    zelle.push('\n');
                }
                _ => zelle.push(c),
            }
            continue;
        }
        match c {
            '"' if zellanfang => {
                in_anf = true;
                zellanfang = false;
            }
            '\r' | '\n' => {
                if c == '\r' && zeichen.peek() == Some(&'\n') {
                    zeichen.next();
                }
                zeile.push(std::mem::take(&mut zelle));
                zeilen.push(std::mem::take(&mut zeile));
                zellanfang = true;
                offen = false;
            }
            c if c == trenn => {
                zeile.push(std::mem::take(&mut zelle));
                zellanfang = true;
            }
            _ => {
                zelle.push(c);
                zellanfang = false;
            }
        }
    }
    if in_anf {
        return Err(tr!(
            "Zeile {zeile}: Ein Anführungszeichen wird bis zum Ende der Datei nicht geschlossen.",
            zeile = zeilen.len() + 1
        ));
    }
    if offen {
        zeile.push(zelle);
        zeilen.push(zeile);
    }
    Ok(zeilen)
}

/// Eine gelesene CSV-Datei.
#[derive(Debug, Clone, PartialEq)]
pub struct Csv {
    pub spalten: Vec<String>,
    /// Zeilennummer (wie in Excel: Kopfzeile 1) und Zellen; leere Zeilen fehlen
    pub zeilen: Vec<(usize, Vec<String>)>,
    pub trennzeichen: char,
    pub kodierung: Kodierung,
}

/// Liest eine CSV-Datei: Kodierung, Trennzeichen, Kopfzeile, Zeilen.
pub fn lesen(bytes: &[u8]) -> Result<Csv, String> {
    let (text, kodierung) = dekodieren(bytes);
    let text = text.trim_start_matches('\u{feff}');
    let trenn = trennzeichen(text);
    let mut alle = zerlegen(text, trenn)?.into_iter();
    let kopf = alle
        .next()
        .filter(|k| k.iter().any(|z| !z.trim().is_empty()))
        .ok_or_else(|| tr!("Die CSV-Datei hat keine Kopfzeile mit Spaltennamen."))?;
    let mut spalten: Vec<String> = Vec::new();
    for (i, s) in kopf.iter().enumerate() {
        let name = s.split_whitespace().collect::<Vec<_>>().join(" ");
        let name = if name.is_empty() {
            tr!("Spalte {nummer}", nummer = i + 1)
        } else {
            name
        };
        if spalten
            .iter()
            .any(|a| a.to_lowercase() == name.to_lowercase())
        {
            return Err(tr!(
                "Die Spalte „{name}“ steht zweimal in der Kopfzeile.",
                name
            ));
        }
        spalten.push(name);
    }
    let zeilen = alle
        .enumerate()
        .map(|(i, z)| (i + 2, z))
        .filter(|(_, z)| z.iter().any(|c| !c.trim().is_empty()))
        .collect();
    Ok(Csv {
        spalten,
        zeilen,
        trennzeichen: trenn,
        kodierung,
    })
}

// --- Spalten und Felder --------------------------------------------------------------------------

/// Vergleichsschlüssel: klein, ä → ae, ß → ss, nur Buchstaben und Ziffern.
pub fn schluessel(s: &str) -> String {
    let mut aus = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' => aus.push_str("ae"),
            'ö' => aus.push_str("oe"),
            'ü' => aus.push_str("ue"),
            'ß' => aus.push_str("ss"),
            // zerlegtes Umlaut (u + ¨), etwa aus macOS-Dateinamen
            '\u{308}' if aus.ends_with(['a', 'o', 'u']) => aus.push('e'),
            c if c.is_alphanumeric() => aus.push(c),
            _ => {}
        }
    }
    aus
}

/// Ein Feld, dem eine Spalte zugeordnet werden kann.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ziel {
    /// Feldpfad mit Punkt, etwa `absender.name`; `datei` für den Dateinamen des PDFs
    pub pfad: String,
    /// Titel mit Gruppen, etwa „Absender › Name“
    pub titel: String,
    /// Liste von Werten: mehrere Spalten oder Zeilen in einer Zelle
    pub liste: bool,
    #[serde(skip)]
    art: Art,
    /// Schlüssel aus Pfad und Titelpfad
    #[serde(skip)]
    stark: Vec<String>,
    /// Schlüssel aus Name und Titel allein (bei verschachtelten Feldern nur, wenn eindeutig)
    #[serde(skip)]
    schwach: Vec<String>,
}

/// Alle Felder, die aus einer Spalte kommen können: Text, Zahl, Datum, Schalter, Auswahl, Liste
/// (auch in Gruppen). Listen von Einträgen (etwa Posten) nicht: Sie kommen aus `dokument.toml`.
pub fn ziele(formular: &Formular) -> Vec<Ziel> {
    fn sammeln(felder: &[Feld], namen: &[String], titel: &[String], aus: &mut Vec<Ziel>) {
        for f in felder {
            let mut n = namen.to_vec();
            n.push(f.name.clone());
            let mut t = titel.to_vec();
            t.push(f.titel.clone());
            match &f.art {
                Art::Gruppe { felder } => sammeln(felder, &n, &t, aus),
                Art::Eintraege { .. } => {}
                art => {
                    let oben = namen.is_empty();
                    // Titel auch in der Sprache der Oberfläche (englisches Excel, englische Titel).
                    let u = |x: &String| tftn_sprachen::uebersetzen(x);
                    let tu: Vec<String> = t.iter().map(u).collect();
                    let mut stark = vec![
                        schluessel(&n.join(".")),
                        schluessel(&t.join(" ")),
                        schluessel(&tu.join(" ")),
                    ];
                    let mut schwach = vec![
                        schluessel(&f.name),
                        schluessel(&f.titel),
                        schluessel(&u(&f.titel)),
                    ];
                    stark.dedup();
                    schwach.dedup();
                    if oben {
                        stark.extend(schwach.iter().cloned());
                    }
                    aus.push(Ziel {
                        pfad: n.join("."),
                        titel: t.join(" › "),
                        liste: matches!(art, Art::Liste { .. }),
                        art: art.clone(),
                        stark,
                        schwach: if oben { Vec::new() } else { schwach },
                    });
                }
            }
        }
    }
    let mut aus = Vec::new();
    sammeln(&formular.felder, &[], &[], &mut aus);
    if !aus.iter().any(|z| z.pfad == DATEI) {
        aus.push(Ziel {
            pfad: DATEI.into(),
            titel: tr!("Dateiname des PDFs"),
            liste: false,
            art: Art::Text {
                widget: crate::TextWidget::Einzeilig,
                format: None,
                min_laenge: None,
                max_laenge: None,
                muster: None,
                standard: None,
            },
            stark: ["datei", "dateiname", "pdf", "pdfname"]
                .map(String::from)
                .to_vec(),
            schwach: Vec::new(),
        });
    }
    aus
}

/// Das Feld, das zum Spaltennamen passt (Index in `ziele`), ohne eigene Zuordnung. Ein Name, der
/// mehrere Felder treffen könnte, ordnet nichts zu. Eine Nummer am Ende (`Empfänger 2`) passt zu
/// einer Liste.
pub fn automatisch(spalte: &str, ziele: &[Ziel]) -> Option<usize> {
    fn suchen(n: &str, ziele: &[Ziel], nur_listen: bool) -> Option<usize> {
        let passend = |stark: bool| -> Vec<usize> {
            ziele
                .iter()
                .enumerate()
                .filter(|(_, z)| !nur_listen || z.liste)
                .filter(|(_, z)| {
                    if stark {
                        z.stark.iter().any(|k| k == n)
                    } else {
                        z.schwach.iter().any(|k| k == n)
                    }
                })
                .map(|(i, _)| i)
                .collect()
        };
        for stark in [true, false] {
            let p = passend(stark);
            match p.len() {
                0 => continue,
                1 => return Some(p[0]),
                _ => return None,
            }
        }
        None
    }
    let n = schluessel(spalte);
    if n.is_empty() {
        return None;
    }
    if let Some(i) = suchen(&n, ziele, false) {
        return Some(i);
    }
    let ohne = n.trim_end_matches(|c: char| c.is_ascii_digit());
    if ohne.len() < n.len() && !ohne.is_empty() {
        return suchen(ohne, ziele, true);
    }
    None
}

/// Eigene Zuordnung einer CSV: Spalte → Feldpfad, `""` = nicht verwenden.
pub type Zuordnung = BTreeMap<String, String>;

/// Schlüssel einer CSV in `tftn.toml`: ihr Dateiname (eine neue Ausfuhr gleichen Namens findet
/// ihre Zuordnung wieder, auch an anderem Ort).
pub fn dateischluessel(pfad: &std::path::Path) -> String {
    pfad.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Liest die eigene Zuordnung der CSV `datei` aus dem Text von `tftn.toml`.
pub fn zuordnung_lesen(tftn_toml: &str, datei: &str) -> Zuordnung {
    let Ok(doc) = tftn_toml.parse::<DocumentMut>() else {
        return Zuordnung::new();
    };
    let Some(t) = doc
        .get(ABSCHNITT)
        .and_then(Item::as_table_like)
        .and_then(|t| t.get(datei))
        .and_then(Item::as_table_like)
    else {
        return Zuordnung::new();
    };
    t.iter()
        .filter_map(|(k, v)| Some((k.to_string(), v.as_str()?.to_string())))
        .collect()
}

/// Setzt die Zuordnung einer Spalte: `feld` ist ein Feldpfad oder `""` (nicht verwenden, bleibt
/// gemerkt, damit die Spalte nicht mehr als offen gilt). Ein Feld, das der Name allein ergäbe, wird
/// nicht gemerkt (ein alter Eintrag fällt weg). Der Rest der Datei bleibt.
pub fn zuordnung_schreiben(
    tftn_toml: &str,
    datei: &str,
    spalte: &str,
    feld: &str,
    formular: &Formular,
) -> Result<String, String> {
    let z = ziele(formular);
    if !feld.is_empty() && !z.iter().any(|x| x.pfad == feld) {
        return Err(tr!("Unbekanntes Feld „{feld}“.", feld));
    }
    let auto = automatisch(spalte, &z).map(|i| z[i].pfad.as_str());
    let eigen = feld.is_empty() || auto != Some(feld);
    let mut doc = tftn_toml
        .parse::<DocumentMut>()
        .map_err(|e| e.message().to_string())?;
    if eigen {
        if !doc.contains_key(ABSCHNITT) {
            let mut t = Table::new();
            t.set_implicit(true);
            doc[ABSCHNITT] = Item::Table(t);
        }
        let aussen = doc[ABSCHNITT]
            .as_table_like_mut()
            .ok_or_else(|| tr!("[serie-csv] in tftn.toml ist keine Tabelle"))?;
        if aussen.get(datei).is_none() {
            aussen.insert(datei, Item::Table(Table::new()));
        }
        let innen = aussen
            .get_mut(datei)
            .and_then(Item::as_table_like_mut)
            .ok_or_else(|| tr!("[serie-csv] in tftn.toml ist keine Tabelle"))?;
        innen.insert(spalte, Item::Value(Value::from(feld)));
    } else if let Some(aussen) = doc.get_mut(ABSCHNITT).and_then(Item::as_table_like_mut) {
        if let Some(innen) = aussen.get_mut(datei).and_then(Item::as_table_like_mut) {
            innen.remove(spalte);
            if innen.is_empty() {
                aussen.remove(datei);
            }
        }
        if aussen.is_empty() {
            doc.remove(ABSCHNITT);
        }
    }
    Ok(doc.to_string())
}

// --- Umwandeln -----------------------------------------------------------------------------------

/// Ein umgewandelter Wert.
#[derive(Debug, Clone, PartialEq)]
pub enum Wert {
    Text(String),
    Ganz(i64),
    Zahl(f64),
    Schalter(bool),
    Datum { jahr: u16, monat: u8, tag: u8 },
    Liste(Vec<Wert>),
}

impl Wert {
    fn toml(&self) -> Value {
        match self {
            Wert::Text(s) => Value::from(s.as_str()),
            Wert::Ganz(i) => Value::from(*i),
            Wert::Zahl(f) => Value::from(*f),
            Wert::Schalter(b) => Value::from(*b),
            Wert::Datum { jahr, monat, tag } => Value::from(toml_edit::Datetime {
                date: Some(toml_edit::Date {
                    year: *jahr,
                    month: *monat,
                    day: *tag,
                }),
                time: None,
                offset: None,
            }),
            Wert::Liste(l) => {
                let mut a = toml_edit::Array::new();
                for w in l {
                    a.push(w.toml());
                }
                Value::Array(a)
            }
        }
    }

    /// So, wie der Wert ins Dokument geht (Liste: eine Zeile je Eintrag).
    pub fn text(&self) -> String {
        match self {
            Wert::Text(s) => s.clone(),
            Wert::Ganz(i) => i.to_string(),
            Wert::Zahl(f) => f.to_string(),
            Wert::Schalter(b) => b.to_string(),
            Wert::Datum { jahr, monat, tag } => format!("{jahr:04}-{monat:02}-{tag:02}"),
            Wert::Liste(l) => l.iter().map(Wert::text).collect::<Vec<_>>().join("\n"),
        }
    }
}

/// Zahl aus `1.234,50`, `1234,5`, `1234.50`, `1,234.50`, `-3`, `12 €`.
pub fn zahl_lesen(text: &str) -> Option<f64> {
    let s: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{202f}' | '\'' | '€'))
        .collect();
    let (komma, punkt) = (s.matches(',').count(), s.matches('.').count());
    let normal = match (komma, punkt) {
        (0, 0) | (0, 1) => s,
        (1, 0) => s.replace(',', "."),
        (0, _) => s.replace('.', ""),
        (_, 0) => s.replace(',', ""),
        _ if s.rfind(',') > s.rfind('.') => s.replace('.', "").replace(',', "."),
        _ => s.replace(',', ""),
    };
    let ziffern = normal.strip_prefix(['-', '+']).unwrap_or(&normal);
    if ziffern.is_empty()
        || !ziffern.chars().all(|c| c.is_ascii_digit() || c == '.')
        || !ziffern.chars().any(|c| c.is_ascii_digit())
    {
        return None;
    }
    normal.parse().ok()
}

fn tage_im_monat(jahr: u16, monat: u8) -> u8 {
    match monat {
        2 if (jahr.is_multiple_of(4) && !jahr.is_multiple_of(100)) || jahr.is_multiple_of(400) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Datum aus `31.01.2026`, `31.1.2026` oder `2026-01-31`; eine Uhrzeit dahinter (Excel) zählt nicht.
pub fn datum_lesen(text: &str) -> Option<(u16, u8, u8)> {
    let mut teile = text.split_whitespace();
    let d = teile.next()?;
    if teile.any(|t| !t.contains(':')) {
        return None;
    }
    let d = d.split_once('T').map(|(a, _)| a).unwrap_or(d);
    let zahlen = |trenn: char| -> Option<Vec<u32>> {
        let t: Vec<&str> = d.split(trenn).collect();
        if t.len() != 3
            || t.iter()
                .any(|x| x.is_empty() || !x.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        Some(t.iter().map(|x| x.parse().unwrap_or(0)).collect())
    };
    let (jahr, monat, tag) = if let Some(z) = zahlen('-').filter(|_| d.len() == 10) {
        (z[0], z[1], z[2])
    } else {
        let z = zahlen('.')?;
        if d.rsplit('.').next()?.len() != 4 {
            return None;
        }
        (z[2], z[1], z[0])
    };
    let (jahr, monat, tag) = (
        u16::try_from(jahr).ok()?,
        u8::try_from(monat).ok()?,
        u8::try_from(tag).ok()?,
    );
    ((1..=12).contains(&monat) && tag >= 1 && tag <= tage_im_monat(jahr, monat))
        .then_some((jahr, monat, tag))
}

fn schalter_lesen(text: &str) -> Option<bool> {
    match text.trim().to_lowercase().as_str() {
        "ja" | "j" | "x" | "wahr" | "true" | "1" | "yes" | "y" | "an" => Some(true),
        "nein" | "n" | "falsch" | "false" | "0" | "no" | "aus" => Some(false),
        _ => None,
    }
}

fn json_wert(v: &Json) -> Option<Wert> {
    Some(match v {
        Json::String(s) => Wert::Text(s.clone()),
        Json::Bool(b) => Wert::Schalter(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Wert::Ganz(i),
            None => Wert::Zahl(n.as_f64()?),
        },
        _ => return None,
    })
}

/// Eine Zelle nach dem Typ des Felds (ohne Listen; die zerlegt der Aufrufer).
pub fn umwandeln(art: &Art, text: &str) -> Result<Wert, String> {
    match art {
        Art::Text { .. } => Ok(Wert::Text(text.to_string())),
        Art::Datum { .. } => datum_lesen(text)
            .map(|(jahr, monat, tag)| Wert::Datum { jahr, monat, tag })
            .ok_or_else(|| {
                tr!(
                    "„{wert}“ ist kein Datum; erwartet wird etwa 31.01.2026 oder 2026-01-31.",
                    wert = text
                )
            }),
        Art::Zahl { ganzzahl, .. } => {
            let z = zahl_lesen(text).ok_or_else(|| tr!("„{wert}“ ist keine Zahl.", wert = text))?;
            if z.fract() == 0.0 && z.abs() < 9e15 {
                Ok(Wert::Ganz(z as i64))
            } else if *ganzzahl {
                Err(tr!("„{wert}“ ist keine ganze Zahl.", wert = text))
            } else {
                Ok(Wert::Zahl(z))
            }
        }
        Art::Schalter { .. } => schalter_lesen(text)
            .map(Wert::Schalter)
            .ok_or_else(|| tr!("„{wert}“ ist weder ja noch nein.", wert = text)),
        Art::Wahl {
            werte,
            beschriftungen,
            ..
        } => {
            let n = schluessel(text);
            let als_text = |v: &Json| match v {
                Json::String(s) => s.clone(),
                v => v.to_string(),
            };
            werte
                .iter()
                .enumerate()
                .find(|(i, v)| {
                    schluessel(&als_text(v)) == n
                        || beschriftungen.get(*i).is_some_and(|b| schluessel(b) == n)
                })
                .and_then(|(_, v)| json_wert(v))
                .ok_or_else(|| {
                    let moeglich: Vec<String> = werte
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            beschriftungen
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| als_text(v))
                        })
                        .collect();
                    tr!(
                        "„{wert}“ ist keine der Möglichkeiten ({liste}).",
                        wert = text,
                        liste = moeglich.join(", ")
                    )
                })
        }
        Art::Liste { element, .. } => {
            let mut aus = Vec::new();
            for z in text.lines().map(str::trim).filter(|z| !z.is_empty()) {
                aus.push(umwandeln(element, z)?);
            }
            Ok(Wert::Liste(aus))
        }
        Art::Gruppe { .. } | Art::Eintraege { .. } => {
            Err(tr!("Dieses Feld lässt sich nicht aus einer Spalte füllen."))
        }
    }
}

/// Eine Spalte, wie sie zugeordnet ist.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spalte {
    pub name: String,
    /// Feldpfad; `None`: nicht verwendet
    pub feld: Option<String>,
    /// aus der eigenen Zuordnung (sonst aus dem Namen)
    pub eigen: bool,
    /// erster Wert der Spalte (gekürzt), als Hilfe beim Zuordnen
    pub beispiel: String,
}

/// Ein Fehler in einer Zeile (Nummer wie in Excel, die Kopfzeile ist 1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fehler {
    pub zeile: usize,
    pub spalte: Option<String>,
    pub text: String,
}

impl std::fmt::Display for Fehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.spalte {
            Some(spalte) => write!(
                f,
                "{}",
                tr!(
                    "Zeile {zeile}, Spalte „{spalte}“: {text}",
                    zeile = self.zeile,
                    spalte,
                    text = self.text
                )
            ),
            None => write!(
                f,
                "{}",
                tr!(
                    "Zeile {zeile}: {text}",
                    zeile = self.zeile,
                    text = self.text
                )
            ),
        }
    }
}

/// Ein Datensatz aus einer Zeile.
#[derive(Debug, Clone, PartialEq)]
pub struct Satz {
    pub zeile: usize,
    /// Feldpfad → Wert, in der Reihenfolge der Felder
    pub werte: Vec<(String, Wert)>,
    pub fehler: Vec<Fehler>,
}

/// Was aus einer CSV wird: Zuordnung, Datensätze, Fehler.
#[derive(Debug, Clone, PartialEq)]
pub struct Bericht {
    pub trennzeichen: char,
    pub kodierung: Kodierung,
    pub spalten: Vec<Spalte>,
    pub felder: Vec<Ziel>,
    pub saetze: Vec<Satz>,
    /// Fehler der Zuordnung (etwa ein Feld, das zwei Spalten bekommen); verhindern die Serie
    pub zuordnungsfehler: Vec<String>,
}

impl Bericht {
    /// Die Werte des ersten Datensatzes: Feldtitel und Text, wie er ins Dokument geht.
    pub fn vorschau(&self) -> Vec<(String, String)> {
        let Some(s) = self.saetze.first() else {
            return Vec::new();
        };
        s.werte
            .iter()
            .map(|(pfad, w)| {
                let titel = self
                    .felder
                    .iter()
                    .find(|z| &z.pfad == pfad)
                    .map(|z| z.titel.clone())
                    .unwrap_or_else(|| pfad.clone());
                (titel, w.text())
            })
            .collect()
    }

    /// Alle Fehler der Zeilen.
    pub fn fehler(&self) -> Vec<&Fehler> {
        self.saetze.iter().flat_map(|s| &s.fehler).collect()
    }
}

fn beispiel(csv: &Csv, spalte: usize) -> String {
    let wert = csv
        .zeilen
        .iter()
        .filter_map(|(_, z)| z.get(spalte))
        .map(|z| z.trim())
        .find(|z| !z.is_empty())
        .unwrap_or("");
    let erste = wert.lines().next().unwrap_or("");
    let mut aus: String = erste.chars().take(40).collect();
    if aus.chars().count() < wert.chars().count() {
        aus.push('…');
    }
    aus
}

/// Ordnet die Spalten zu und wandelt jede Zeile um. `nachsichtig`: Was sich nicht umwandeln lässt,
/// bleibt Text statt Fehler (für Inkognito, das jeden Wert kennen muss).
pub fn auswerten(csv: &Csv, formular: &Formular, eigene: &Zuordnung, nachsichtig: bool) -> Bericht {
    let felder = ziele(formular);
    let mut zuordnungsfehler = Vec::new();
    // Zuerst die eigenen Zuordnungen, dann die aus den Namen für Felder, die noch frei sind.
    let mut ziel_je_spalte: Vec<Option<usize>> = vec![None; csv.spalten.len()];
    let mut eigen = vec![false; csv.spalten.len()];
    for (i, s) in csv.spalten.iter().enumerate() {
        let Some(feld) = eigene.get(s) else { continue };
        if feld.is_empty() {
            eigen[i] = true;
        } else if let Some(z) = felder.iter().position(|z| &z.pfad == feld) {
            eigen[i] = true;
            ziel_je_spalte[i] = Some(z);
        }
    }
    for (i, s) in csv.spalten.iter().enumerate() {
        if eigen[i] {
            continue;
        }
        if let Some(z) = automatisch(s, &felder)
            && (felder[z].liste || !ziel_je_spalte.contains(&Some(z)))
        {
            ziel_je_spalte[i] = Some(z);
        }
    }
    for (z, ziel) in felder.iter().enumerate() {
        let spalten: Vec<&str> = (0..csv.spalten.len())
            .filter(|&i| ziel_je_spalte[i] == Some(z))
            .map(|i| csv.spalten[i].as_str())
            .collect();
        if !ziel.liste && spalten.len() > 1 {
            zuordnungsfehler.push(tr!(
                "Das Feld „{feld}“ ist mehreren Spalten zugeordnet: {spalten}.",
                feld = ziel.titel,
                spalten = spalten.join(", ")
            ));
        }
    }
    let spalten = csv
        .spalten
        .iter()
        .enumerate()
        .map(|(i, name)| Spalte {
            name: name.clone(),
            feld: ziel_je_spalte[i].map(|z| felder[z].pfad.clone()),
            eigen: eigen[i],
            beispiel: beispiel(csv, i),
        })
        .collect();
    let mut saetze = Vec::new();
    for (nr, zellen) in &csv.zeilen {
        let mut fehler = Vec::new();
        if zellen
            .iter()
            .skip(csv.spalten.len())
            .any(|z| !z.trim().is_empty())
        {
            fehler.push(Fehler {
                zeile: *nr,
                spalte: None,
                text: tr!(
                    "Die Zeile hat mehr Zellen, als die Kopfzeile Spalten nennt ({anzahl}).",
                    anzahl = csv.spalten.len()
                ),
            });
        }
        let mut je_feld: BTreeMap<usize, Wert> = BTreeMap::new();
        for (i, ziel) in ziel_je_spalte.iter().enumerate() {
            let Some(z) = *ziel else { continue };
            let zelle = zellen.get(i).map(|z| z.trim()).unwrap_or("");
            if zelle.is_empty() {
                continue;
            }
            let wert = match umwandeln(&felder[z].art, zelle) {
                Ok(w) => w,
                Err(text) => {
                    if !nachsichtig {
                        fehler.push(Fehler {
                            zeile: *nr,
                            spalte: Some(csv.spalten[i].clone()),
                            text,
                        });
                        continue;
                    }
                    if felder[z].liste {
                        Wert::Liste(zelle.lines().map(|l| Wert::Text(l.into())).collect())
                    } else {
                        Wert::Text(zelle.to_string())
                    }
                }
            };
            match (je_feld.get_mut(&z), wert) {
                (Some(Wert::Liste(alt)), Wert::Liste(neu)) => alt.extend(neu),
                (None, w) => {
                    je_feld.insert(z, w);
                }
                // Zweite Spalte für ein einfaches Feld: schon als Zuordnungsfehler gemeldet.
                _ => {}
            }
        }
        let werte = je_feld
            .into_iter()
            .filter(|(_, w)| !matches!(w, Wert::Liste(l) if l.is_empty()))
            .map(|(z, w)| (felder[z].pfad.clone(), w))
            .collect();
        saetze.push(Satz {
            zeile: *nr,
            werte,
            fehler,
        });
    }
    Bericht {
        trennzeichen: csv.trennzeichen,
        kodierung: csv.kodierung,
        spalten,
        felder,
        saetze,
        zuordnungsfehler,
    }
}

/// Die Datensätze als TOML mit Abschnitten `[[serie]]` (wie eine Seriendatei von Hand).
pub fn serie_toml(saetze: &[Satz]) -> String {
    let mut liste = ArrayOfTables::new();
    for s in saetze {
        let mut t = Table::new();
        for (pfad, w) in &s.werte {
            let teile: Vec<&str> = pfad.split('.').collect();
            let (blatt, gruppen) = teile.split_last().expect("Pfad nicht leer");
            let mut ziel = &mut t;
            for g in gruppen {
                if !ziel.contains_key(g) {
                    ziel.insert(g, Item::Table(Table::new()));
                }
                ziel = ziel[*g].as_table_mut().expect("eben angelegt");
            }
            ziel.insert(blatt, Item::Value(w.toml()));
        }
        liste.push(t);
    }
    let mut doc = DocumentMut::new();
    doc.insert("serie", Item::ArrayOfTables(liste));
    doc.to_string()
}

/// Alle Felder, zu denen der Spaltenname passen könnte (auch mehrdeutig, etwa „Name“).
fn kandidaten(spalte: &str, ziele: &[Ziel]) -> Vec<usize> {
    let n = schluessel(spalte);
    let ohne = n.trim_end_matches(|c: char| c.is_ascii_digit());
    ziele
        .iter()
        .enumerate()
        .filter(|(_, z)| {
            let trifft = |k: &str| z.stark.iter().chain(&z.schwach).any(|x| x == k);
            !n.is_empty() && (trifft(&n) || (z.liste && !ohne.is_empty() && trifft(ohne)))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Für Inkognito: die Datensätze einer CSV als JSON (wie `inkognito::datensaetze`), auch mit
/// Werten, die sich nicht umwandeln lassen. Eine Spalte ohne Feld, deren Name auf Felder passt
/// (mehrdeutig wie „Name“ oder „nicht verwenden“), zählt für jedes davon: Liegt die CSV im
/// Projekt, kann ein Assistent sie lesen, und ihre Werte müssen bekannt sein.
pub fn inkognito_datensaetze(
    bytes: &[u8],
    formular: &Formular,
    eigene: &Zuordnung,
) -> Result<Vec<Json>, String> {
    let csv = lesen(bytes)?;
    let mut b = auswerten(&csv, formular, eigene, true);
    let frei: Vec<(usize, Vec<usize>)> = b
        .spalten
        .iter()
        .enumerate()
        .filter(|(_, s)| s.feld.is_none())
        .map(|(i, s)| (i, kandidaten(&s.name, &b.felder)))
        .filter(|(_, k)| !k.is_empty())
        .collect();
    for (nr, zellen) in &csv.zeilen {
        for (i, ziele) in &frei {
            let zelle = zellen.get(*i).map(|z| z.trim()).unwrap_or("");
            if zelle.is_empty() {
                continue;
            }
            let werte = ziele
                .iter()
                .map(|&z| {
                    let w = if b.felder[z].liste {
                        Wert::Liste(zelle.lines().map(|l| Wert::Text(l.trim().into())).collect())
                    } else {
                        Wert::Text(zelle.into())
                    };
                    (b.felder[z].pfad.clone(), w)
                })
                .collect();
            b.saetze.push(Satz {
                zeile: *nr,
                werte,
                fehler: Vec::new(),
            });
        }
    }
    crate::inkognito::datensaetze(&serie_toml(&b.saetze))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formular() -> Formular {
        crate::formular_lesen(
            r#"{
  "type": "object",
  "properties": {
    "nummer": { "type": "string", "title": "Rechnungsnummer" },
    "datum": { "type": "string", "title": "Rechnungsdatum", "format": "date" },
    "anrede": { "type": "string", "title": "Anrede" },
    "empfaenger": { "type": "array", "title": "Empfänger", "items": { "type": "string" } },
    "ust": { "type": "number", "title": "Umsatzsteuer in Prozent" },
    "zahlungsziel": { "type": "integer", "title": "Zahlungsziel in Tagen" },
    "bezahlt": { "type": "boolean", "title": "Bezahlt" },
    "sprache": { "type": "string", "title": "Sprache", "oneOf": [
      { "const": "de", "title": "Deutsch" }, { "const": "en", "title": "Englisch" } ] },
    "absender": { "type": "object", "title": "Absender", "properties": {
      "name": { "type": "string", "title": "Name" },
      "zeilen": { "type": "array", "title": "Anschrift", "items": { "type": "string" } } } },
    "patient": { "type": "object", "title": "Patient", "properties": {
      "name": { "type": "string", "title": "Name" },
      "geburtsdatum": { "type": "string", "title": "Geburtsdatum", "format": "date" } } },
    "posten": { "type": "array", "title": "Posten", "items": { "type": "object", "properties": {
      "text": { "type": "string", "title": "Bezeichnung" } } } }
  }
}"#,
        )
        .unwrap()
    }

    #[test]
    fn rfc4180_anfuehrungszeichen_umbrueche_leere() {
        let z = zerlegen(
            "a;b;c\r\n\"x;y\";\"sagt \"\"hallo\"\"\";\r\n\"Zeile 1\r\nZeile 2\";;z\n",
            ';',
        )
        .unwrap();
        assert_eq!(
            z,
            [
                vec!["a", "b", "c"],
                vec!["x;y", "sagt \"hallo\"", ""],
                vec!["Zeile 1\nZeile 2", "", "z"],
            ]
        );
        // Letzte Zeile ohne Umbruch, leere Zelle in Anführungszeichen, Anführungszeichen mitten drin.
        assert_eq!(
            zerlegen("a,b\n\"\",5\" Diskette", ',').unwrap(),
            [vec!["a", "b"], vec!["", "5\" Diskette"]]
        );
        let e = zerlegen("a;b\n1;2\n\"offen;3\n", ';').unwrap_err();
        assert!(e.contains("Zeile 3"), "{e}");
    }

    #[test]
    fn trennzeichen_aus_der_kopfzeile() {
        assert_eq!(trennzeichen("Name;Ort, Land;PLZ\n1,2,3,4,5"), ';');
        assert_eq!(trennzeichen("name,city,zip\n"), ',');
        assert_eq!(trennzeichen("Name\tOrt\n"), '\t');
        assert_eq!(trennzeichen("\"a;b\",c\n"), ',');
        assert_eq!(trennzeichen("Name"), ';');
    }

    #[test]
    fn kodierungen() {
        // Windows-1252: ä ö ü ß € „ “ – aus altem Excel
        let bytes = b"Name;Ort\nM\xfcller \x84Max\x93;Gr\xf6\xdfing \x96 \x80\n";
        let (t, k) = dekodieren(bytes);
        assert_eq!(k, Kodierung::Windows1252);
        assert_eq!(t, "Name;Ort\nMüller „Max“;Größing – €\n");
        let (t, k) = dekodieren("\u{feff}Name;Ort\nÄrztin;Wien".as_bytes());
        assert_eq!((t.as_str(), k), ("Name;Ort\nÄrztin;Wien", Kodierung::Utf8));
        let mut utf16 = vec![0xff, 0xfe];
        for e in "Name\tOrt\nÖ\tW".encode_utf16() {
            utf16.extend(e.to_le_bytes());
        }
        let csv = lesen(&utf16).unwrap();
        assert_eq!(csv.kodierung, Kodierung::Utf16);
        assert_eq!(csv.trennzeichen, '\t');
        assert_eq!(csv.zeilen, [(2, vec!["Ö".to_string(), "W".to_string()])]);
    }

    #[test]
    fn excel_deutsch_und_englisch() {
        // Deutsches Excel: Semikolon, Windows-1252, CRLF, Leerzeilen am Ende.
        let de = b"Anrede;Empf\xe4nger 1;Empf\xe4nger 2;Datum;USt\r\n\
                   Sehr geehrte Frau M\xfcller,;Frau;\"Dr. M\xfcller; Anna\";31.01.2026;\"1.234,50\"\r\n\
                   ;;;;\r\n;;;;\r\n";
        let csv = lesen(de).unwrap();
        assert_eq!(
            csv.spalten,
            ["Anrede", "Empfänger 1", "Empfänger 2", "Datum", "USt"]
        );
        assert_eq!(csv.zeilen.len(), 1, "leere Zeilen fallen weg");
        assert_eq!(csv.zeilen[0].1[2], "Dr. Müller; Anna");
        // Englisches Excel: Komma, UTF-8 mit BOM, Zeilenumbruch in der Zelle.
        let en = "\u{feff}Salutation,Recipient,Date\n\"Dear Ms Smith,\",\"Ms Smith\n1 High St\",2026-01-31\n";
        let csv = lesen(en.as_bytes()).unwrap();
        assert_eq!(csv.trennzeichen, ',');
        assert_eq!(csv.zeilen[0].1[1], "Ms Smith\n1 High St");
        // Doppelte Spalte, leere Datei.
        assert!(lesen(b"a;A\n1;2").unwrap_err().contains("zweimal"));
        assert!(lesen(b"").is_err());
    }

    #[test]
    fn spalten_finden_ihre_felder() {
        let z = ziele(&formular());
        let feld = |s: &str| automatisch(s, &z).map(|i| z[i].pfad.as_str());
        assert_eq!(feld("Empfänger"), Some("empfaenger"));
        assert_eq!(
            feld("EMPFAENGER 2"),
            Some("empfaenger"),
            "Nummer am Ende: Liste"
        );
        assert_eq!(feld("empfaenger_1"), Some("empfaenger"));
        assert_eq!(feld("Rechnungs-Nummer"), Some("nummer"), "über den Titel");
        assert_eq!(feld("patient.name"), Some("patient.name"));
        assert_eq!(feld("Patient Name"), Some("patient.name"));
        assert_eq!(
            feld("Geburtsdatum"),
            Some("patient.geburtsdatum"),
            "eindeutig"
        );
        assert_eq!(feld("Name"), None, "Absender oder Patient: mehrdeutig");
        assert_eq!(feld("Nummer 2"), None, "kein Listenfeld");
        assert_eq!(feld("Telefon"), None);
        assert_eq!(feld("Dateiname"), Some("datei"));
        assert!(
            !z.iter().any(|x| x.pfad.starts_with("posten")),
            "Einträge nicht"
        );
        assert_eq!(schluessel("Größe Ü"), "groesseue");
        assert_eq!(schluessel("Gru\u{308}n"), "gruen");
    }

    #[test]
    fn umwandeln_nach_typ() {
        let f = formular();
        let z = ziele(&f);
        let art = |p: &str| z.iter().find(|x| x.pfad == p).unwrap().art.clone();
        assert_eq!(umwandeln(&art("ust"), "1.234,50"), Ok(Wert::Zahl(1234.5)));
        assert_eq!(umwandeln(&art("ust"), "1234.50"), Ok(Wert::Zahl(1234.5)));
        assert_eq!(umwandeln(&art("ust"), "1,234.50"), Ok(Wert::Zahl(1234.5)));
        assert_eq!(umwandeln(&art("ust"), "20"), Ok(Wert::Ganz(20)));
        assert_eq!(umwandeln(&art("ust"), "-0,5 €"), Ok(Wert::Zahl(-0.5)));
        assert!(umwandeln(&art("ust"), "zwanzig").is_err());
        assert!(umwandeln(&art("ust"), "1e5").is_err());
        assert_eq!(umwandeln(&art("zahlungsziel"), "14"), Ok(Wert::Ganz(14)));
        assert!(umwandeln(&art("zahlungsziel"), "14,5").is_err());
        let datum = Wert::Datum {
            jahr: 2026,
            monat: 1,
            tag: 31,
        };
        assert_eq!(umwandeln(&art("datum"), "31.01.2026"), Ok(datum.clone()));
        assert_eq!(umwandeln(&art("datum"), "31.1.2026"), Ok(datum.clone()));
        assert_eq!(umwandeln(&art("datum"), "2026-01-31"), Ok(datum.clone()));
        assert_eq!(umwandeln(&art("datum"), "31.01.2026 00:00"), Ok(datum));
        assert!(umwandeln(&art("datum"), "31.02.2026").is_err());
        assert!(umwandeln(&art("datum"), "29.02.2025").is_err());
        assert!(umwandeln(&art("datum"), "29.02.2024").is_ok());
        assert!(
            umwandeln(&art("datum"), "31.01.26").is_err(),
            "Jahr vierstellig"
        );
        assert!(umwandeln(&art("datum"), "01/31/2026").is_err());
        assert_eq!(umwandeln(&art("bezahlt"), "Ja"), Ok(Wert::Schalter(true)));
        assert_eq!(umwandeln(&art("bezahlt"), "0"), Ok(Wert::Schalter(false)));
        assert_eq!(
            umwandeln(&art("sprache"), "Englisch"),
            Ok(Wert::Text("en".into()))
        );
        assert_eq!(
            umwandeln(&art("sprache"), "DE"),
            Ok(Wert::Text("de".into()))
        );
        let e = umwandeln(&art("sprache"), "Latein").unwrap_err();
        assert!(e.contains("Deutsch, Englisch"), "{e}");
        assert_eq!(
            umwandeln(&art("empfaenger"), "Frau\r\n\nAnna Muster\n"),
            Ok(Wert::Liste(vec![
                Wert::Text("Frau".into()),
                Wert::Text("Anna Muster".into())
            ]))
        );
    }

    #[test]
    fn auswerten_mit_zuordnung_und_fehlern() {
        let csv = lesen(
            "Nr;Empfänger 1;Empfänger 2;Datum;Telefon;Name;Datei\n\
             1;Frau;Anna Muster;31.01.2026;0664 123;Dr. A;Brief Muster\n\
             2;\"Herr\nBernd Beispiel\";;kein Datum;;;\n\
             3;;;;;;;zu viel\n"
                .as_bytes(),
        )
        .unwrap();
        let mut eigene = Zuordnung::new();
        eigene.insert("Nr".into(), "nummer".into());
        eigene.insert("Name".into(), "absender.name".into());
        eigene.insert("Datei".into(), "".into());
        let b = auswerten(&csv, &formular(), &eigene, false);
        let feld: Vec<(&str, Option<&str>, bool)> = b
            .spalten
            .iter()
            .map(|s| (s.name.as_str(), s.feld.as_deref(), s.eigen))
            .collect();
        assert_eq!(
            feld,
            [
                ("Nr", Some("nummer"), true),
                ("Empfänger 1", Some("empfaenger"), false),
                ("Empfänger 2", Some("empfaenger"), false),
                ("Datum", Some("datum"), false),
                ("Telefon", None, false),
                ("Name", Some("absender.name"), true),
                ("Datei", None, true),
            ]
        );
        assert_eq!(b.spalten[1].beispiel, "Frau");
        assert!(b.zuordnungsfehler.is_empty());
        assert_eq!(b.saetze.len(), 3);
        assert_eq!(
            b.vorschau(),
            [
                ("Rechnungsnummer".to_string(), "1".to_string()),
                ("Rechnungsdatum".into(), "2026-01-31".into()),
                ("Empfänger".into(), "Frau\nAnna Muster".into()),
                ("Absender › Name".into(), "Dr. A".into()),
            ]
        );
        let fehler: Vec<String> = b.fehler().iter().map(|f| f.to_string()).collect();
        assert_eq!(fehler.len(), 2, "{fehler:?}");
        assert!(
            fehler[0].starts_with("Zeile 3, Spalte „Datum“: „kein Datum“"),
            "{}",
            fehler[0]
        );
        assert!(fehler[1].starts_with("Zeile 4: "), "{}", fehler[1]);
        // Zeile 3: Empfänger aus den Zeilen einer Zelle, kein Datum (Fehler), Leeres weggelassen.
        assert_eq!(
            b.saetze[1].werte,
            [
                ("nummer".to_string(), Wert::Text("2".into())),
                (
                    "empfaenger".into(),
                    Wert::Liste(vec![
                        Wert::Text("Herr".into()),
                        Wert::Text("Bernd Beispiel".into())
                    ])
                ),
            ]
        );
        // Nachsichtig (Inkognito): Der Text bleibt.
        let n = auswerten(&csv, &formular(), &eigene, true);
        assert!(
            n.saetze[1]
                .werte
                .contains(&("datum".into(), Wert::Text("kein Datum".into())))
        );
    }

    #[test]
    fn ein_feld_zwei_spalten_ist_ein_zuordnungsfehler() {
        let csv = lesen(b"Anrede;Gruss;Text\nA;B;C\n").unwrap();
        let mut eigene = Zuordnung::new();
        eigene.insert("Gruss".into(), "anrede".into());
        eigene.insert("Text".into(), "anrede".into());
        let b = auswerten(&csv, &formular(), &eigene, false);
        assert_eq!(b.zuordnungsfehler.len(), 1, "{:?}", b.zuordnungsfehler);
        assert!(b.zuordnungsfehler[0].contains("Gruss, Text"));
        assert_eq!(
            b.spalten[0].feld, None,
            "der Name nimmt kein vergebenes Feld"
        );
        // Zwei Namen für dasselbe Feld: der erste bekommt es, der zweite bleibt sichtbar frei.
        let csv = lesen(b"Rechnungs-Nummer;Nummer\n1;2\n").unwrap();
        let b = auswerten(&csv, &formular(), &Zuordnung::new(), false);
        assert!(b.zuordnungsfehler.is_empty());
        assert_eq!(b.spalten[0].feld.as_deref(), Some("nummer"));
        assert_eq!(b.spalten[1].feld, None);
        assert!(
            lesen(b"anrede;Anrede \n1;2\n")
                .unwrap_err()
                .contains("zweimal")
        );
    }

    #[test]
    fn serie_toml_wie_von_hand() {
        let csv = lesen(
            "Rechnungsnummer;Empfänger;Datum;Absender Name;USt;Datei\n\
             2026-101;\"Frau\nLena Huber\";31.01.2026;Dr. X;10;2026-101 Huber\n"
                .as_bytes(),
        )
        .unwrap();
        let b = auswerten(&csv, &formular(), &Zuordnung::new(), false);
        let t = serie_toml(&b.saetze);
        let doc: DocumentMut = t.parse().unwrap();
        let s = doc["serie"].as_array_of_tables().unwrap().get(0).unwrap();
        assert_eq!(s["nummer"].as_str(), Some("2026-101"));
        assert_eq!(s["datei"].as_str(), Some("2026-101 Huber"));
        assert_eq!(s["ust"].as_integer(), Some(10));
        assert_eq!(s["absender"]["name"].as_str(), Some("Dr. X"));
        assert_eq!(
            s["datum"].as_datetime().map(|d| d.to_string()).as_deref(),
            Some("2026-01-31")
        );
        assert_eq!(s["empfaenger"].as_array().unwrap().len(), 2);
        // Inkognito liest dieselben Werte (als JSON).
        let j = inkognito_datensaetze(
            "Patient Name;Geburtsdatum\nAnna Muster;1.2.1970\n".as_bytes(),
            &formular(),
            &Zuordnung::new(),
        )
        .unwrap();
        assert_eq!(j.len(), 1);
        assert_eq!(j[0]["patient"]["name"], "Anna Muster");
        assert_eq!(j[0]["patient"]["geburtsdatum"], "1970-02-01");
        // „Name“ passt auf Absender und Patient (mehrdeutig, also nicht zugeordnet), „Notiz“ ist
        // abgewählt: Inkognito kennt die Werte trotzdem, für jedes mögliche Feld.
        let mut eigene = Zuordnung::new();
        eigene.insert("Anrede".into(), "".into());
        let j = inkognito_datensaetze(
            "Name;Anrede\nAnna Muster;Liebe Anna\n".as_bytes(),
            &formular(),
            &eigene,
        )
        .unwrap();
        let alle = serde_json::to_string(&j).unwrap();
        assert!(
            alle.contains(r#""patient":{"name":"Anna Muster"}"#),
            "{alle}"
        );
        assert!(
            alle.contains(r#""absender":{"name":"Anna Muster"}"#),
            "{alle}"
        );
        assert!(alle.contains(r#""anrede":"Liebe Anna""#), "{alle}");
    }

    #[test]
    fn zuordnung_in_tftn_toml() {
        let f = formular();
        let alt = "tftn = 1\n\n[inkognito]\n\"patient.name\" = true\n";
        let neu = zuordnung_schreiben(alt, "adressen.csv", "Nr", "nummer", &f).unwrap();
        let neu = zuordnung_schreiben(&neu, "adressen.csv", "Anrede", "", &f).unwrap();
        assert!(neu.starts_with(alt), "Rest bleibt: {neu}");
        assert!(neu.contains("[serie-csv.\"adressen.csv\"]"), "{neu}");
        let z = zuordnung_lesen(&neu, "adressen.csv");
        assert_eq!(z.get("Nr").map(String::as_str), Some("nummer"));
        assert_eq!(z.get("Anrede").map(String::as_str), Some(""));
        assert!(zuordnung_lesen(&neu, "andere.csv").is_empty());
        // „Nicht verwenden“ bleibt gemerkt, auch ohne passendes Feld (sonst gälte die Spalte als offen).
        let neu = zuordnung_schreiben(&neu, "adressen.csv", "Telefon", "", &f).unwrap();
        assert!(neu.contains("Telefon = \"\""), "{neu}");
        // Zurück auf das Feld, das der Name ergibt: Der Eintrag fällt weg, am Ende auch der Abschnitt.
        let neu = zuordnung_schreiben(&neu, "adressen.csv", "Anrede", "anrede", &f).unwrap();
        let neu = zuordnung_schreiben(&neu, "adressen.csv", "Telefon", "anrede", &f).unwrap();
        assert_eq!(
            zuordnung_lesen(&neu, "adressen.csv")
                .get("Telefon")
                .map(String::as_str),
            Some("anrede")
        );
        let neu = zuordnung_schreiben(&neu, "adressen.csv", "Nr", "", &f).unwrap();
        assert_eq!(
            zuordnung_lesen(&neu, "adressen.csv")
                .get("Nr")
                .map(String::as_str),
            Some("")
        );
        let ohne = zuordnung_schreiben(alt, "b.csv", "Anrede", "", &f).unwrap();
        let ohne = zuordnung_schreiben(&ohne, "b.csv", "Anrede", "anrede", &f).unwrap();
        assert_eq!(ohne, alt, "leerer Abschnitt fällt weg");
        assert!(zuordnung_schreiben(alt, "a.csv", "X", "gibtsnicht", &f).is_err());
    }
}
