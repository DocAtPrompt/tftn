//! Werte am Feld prüfen, bevor sie geschrieben werden (KONZEPT 4.7, 6).
//!
//! Meldungen nennen nie den eingegebenen Wert: Felder mit `x-tftn-sensibel` dürfen nirgends
//! auftauchen, auch nicht in einer Fehlermeldung.

use std::path::{Component, Path};
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::{Art, Feld, Formular, TextWidget};

/// Ein Schritt im Pfad zu einem Wert: Schlüssel oder Index in einer Liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Schritt {
    Name(String),
    Index(usize),
}

/// Pfad zu einem Wert, als JSON etwa `["autoren", 0, "name"]`.
pub type Pfad = Vec<Schritt>;

/// Was die Prüfung über das Projekt wissen muss.
#[derive(Debug, Clone, Copy)]
pub struct Kontext<'a> {
    /// Projektordner für `literaturdatei` und `.csl`-Stile; ohne ihn wird das Vorhandensein nicht geprüft.
    pub wurzel: Option<&'a Path>,
    /// Eingebaute Zitierstile von Typst (`tftn_literatur::eingebaute_stile`).
    pub stile: &'a [&'a str],
}

/// Ergebnis der Prüfung eines ganzen Dokuments.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Meldung {
    pub pfad: Pfad,
    pub text: String,
    /// `false`: nur ein Hinweis (etwa „Pflichtfeld leer“)
    pub fehler: bool,
}

/// Prüft einen Wert gegen ein Feld; `None` heißt leer (der Schlüssel wird entfernt).
pub fn wert_pruefen(feld: &Feld, wert: Option<&Value>, k: &Kontext) -> Result<(), String> {
    match wert {
        None if feld.pflicht => Err(tr!("Pflichtfeld")),
        None => Ok(()),
        Some(w) => art_pruefen(&feld.art, w, k),
    }
}

/// Prüft das ganze Dokument (Werte als JSON-Objekt), für die Anzeige beim Laden.
pub fn alles_pruefen(f: &Formular, werte: &Value, k: &Kontext) -> Vec<Meldung> {
    let mut m = Vec::new();
    felder_pruefen(&f.felder, werte, &mut Vec::new(), k, &mut m);
    m
}

fn felder_pruefen(
    felder: &[Feld],
    obj: &Value,
    pfad: &mut Pfad,
    k: &Kontext,
    m: &mut Vec<Meldung>,
) {
    for feld in felder {
        pfad.push(Schritt::Name(feld.name.clone()));
        let wert = obj.get(&feld.name);
        match (wert, &feld.art) {
            (None, _) => {
                if feld.pflicht {
                    melden(m, pfad, &tr!("Pflichtfeld"), true);
                }
            }
            (Some(w @ Value::Object(_)), Art::Gruppe { felder }) => {
                felder_pruefen(felder, w, pfad, k, m)
            }
            (Some(Value::Array(a)), Art::Eintraege { felder, min, max }) => {
                if let Err(e) = anzahl(a.len(), *min, *max) {
                    melden(m, pfad, &e, true);
                }
                for (i, e) in a.iter().enumerate() {
                    pfad.push(Schritt::Index(i));
                    if e.is_object() {
                        felder_pruefen(felder, e, pfad, k, m);
                    } else {
                        melden(m, pfad, &tr!("Eintrag erwartet"), true);
                    }
                    pfad.pop();
                }
            }
            (Some(Value::String(s)), Art::Text { .. }) if s.is_empty() && feld.pflicht => {
                melden(m, pfad, &tr!("Pflichtfeld leer"), false)
            }
            (Some(w), art) => {
                if let Err(e) = art_pruefen(art, w, k) {
                    melden(m, pfad, &e, true);
                }
            }
        }
        pfad.pop();
    }
}

fn melden(m: &mut Vec<Meldung>, pfad: &Pfad, text: &str, fehler: bool) {
    m.push(Meldung {
        pfad: pfad.clone(),
        text: text.to_string(),
        fehler,
    });
}

/// „mindestens 1 Eintrag“, „höchstens 3 Einträge“
fn eintraege(n: u64) -> String {
    if n == 1 {
        tr!("1 Eintrag")
    } else {
        tr!("{n} Einträge", n)
    }
}

pub(crate) fn anzahl(n: usize, min: Option<u64>, max: Option<u64>) -> Result<(), String> {
    if let Some(min) = min
        && (n as u64) < min
    {
        return Err(tr!("mindestens {anzahl}", anzahl = eintraege(min)));
    }
    if let Some(max) = max
        && (n as u64) > max
    {
        return Err(tr!("höchstens {anzahl}", anzahl = eintraege(max)));
    }
    Ok(())
}

/// Zahl für Meldungen: ohne `.0`, mit Komma.
fn zahl_text(z: f64) -> String {
    let t = if z.fract() == 0.0 && z.abs() < 1e15 {
        format!("{}", z as i64)
    } else {
        format!("{z}")
    };
    t.replace('.', ",")
}

/// Prüft einen vorhandenen Wert gegen eine Art.
pub(crate) fn art_pruefen(art: &Art, w: &Value, k: &Kontext) -> Result<(), String> {
    match art {
        Art::Text {
            widget,
            format,
            min_laenge,
            max_laenge,
            muster,
            ..
        } => {
            let s = w.as_str().ok_or_else(|| tr!("Text erwartet"))?;
            if *widget == TextWidget::Einzeilig && (s.contains('\n') || s.contains('\r')) {
                return Err(tr!("nur eine Zeile"));
            }
            let n = s.chars().count() as u64;
            if let Some(min) = min_laenge
                && n < *min
            {
                return Err(tr!("mindestens {min} Zeichen", min));
            }
            if let Some(max) = max_laenge
                && n > *max
            {
                return Err(tr!("höchstens {max} Zeichen", max));
            }
            if let Some(m) = muster {
                // JSON Schema: `pattern` sucht irgendwo im Text (nicht verankert).
                let re = regex::Regex::new(m).map_err(|_| tr!("Muster im Schema ungültig"))?;
                if !re.is_match(s) {
                    return Err(tr!("entspricht nicht dem Muster"));
                }
            }
            if *widget == TextWidget::Markup {
                markup_pruefen(s)?;
            }
            match format.as_deref() {
                None => Ok(()),
                Some(f) => format_pruefen(f, s, k),
            }
        }
        Art::Zahl {
            ganzzahl, min, max, ..
        } => {
            let z = w.as_f64().ok_or_else(|| tr!("Zahl erwartet"))?;
            if *ganzzahl && !(w.is_i64() || w.is_u64() || z.fract() == 0.0) {
                return Err(tr!("ganze Zahl erwartet"));
            }
            let zu_klein = min.is_some_and(|m| z < m);
            let zu_gross = max.is_some_and(|m| z > m);
            if zu_klein || zu_gross {
                return Err(match (min, max) {
                    (Some(a), Some(b)) => {
                        tr!(
                            "Zahl zwischen {von} und {bis}",
                            von = zahl_text(*a),
                            bis = zahl_text(*b)
                        )
                    }
                    (Some(a), None) => tr!("Zahl ab {wert}", wert = zahl_text(*a)),
                    (None, Some(b)) => tr!("Zahl bis {wert}", wert = zahl_text(*b)),
                    (None, None) => unreachable!(),
                });
            }
            Ok(())
        }
        Art::Schalter { .. } => w
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| tr!("ja oder nein erwartet")),
        Art::Datum { .. } => {
            let s = w.as_str().ok_or_else(|| tr!("kein gültiges Datum"))?;
            if datum_gueltig(s) {
                Ok(())
            } else {
                Err(tr!("kein gültiges Datum"))
            }
        }
        Art::Wahl { werte, .. } => {
            if werte.contains(w) {
                Ok(())
            } else {
                Err(tr!("kein Wert der Auswahl"))
            }
        }
        Art::Liste { element, min, max } => {
            let a = w.as_array().ok_or_else(|| tr!("Liste erwartet"))?;
            anzahl(a.len(), *min, *max)?;
            for (i, e) in a.iter().enumerate() {
                art_pruefen(element, e, k)
                    .map_err(|m| tr!("Eintrag {nr}: {meldung}", nr = i + 1, meldung = m))?;
            }
            Ok(())
        }
        Art::Gruppe { felder } => {
            let o = w.as_object().ok_or_else(|| tr!("Gruppe erwartet"))?;
            for f in felder {
                wert_pruefen(f, o.get(&f.name), k)
                    .map_err(|m| tr!("{feld}: {meldung}", feld = f.titel, meldung = m))?;
            }
            Ok(())
        }
        Art::Eintraege { felder, min, max } => {
            let a = w.as_array().ok_or_else(|| tr!("Liste erwartet"))?;
            anzahl(a.len(), *min, *max)?;
            for (i, e) in a.iter().enumerate() {
                let o = e
                    .as_object()
                    .ok_or_else(|| tr!("Eintrag {nr}: Eintrag erwartet", nr = i + 1))?;
                for f in felder {
                    wert_pruefen(f, o.get(&f.name), k).map_err(|m| {
                        tr!(
                            "Eintrag {nr}, {feld}: {meldung}",
                            nr = i + 1,
                            feld = f.titel,
                            meldung = m
                        )
                    })?;
                }
            }
            Ok(())
        }
    }
}

fn markup_pruefen(s: &str) -> Result<(), String> {
    let (fehler, _) = typst_syntax::parse(s).errors_and_warnings();
    match fehler.first() {
        None => Ok(()),
        Some(e) => Err(tr!("Markup fehlerhaft: {meldung}", meldung = e.message)),
    }
}

fn format_pruefen(format: &str, s: &str, k: &Kontext) -> Result<(), String> {
    let ok = |b: bool, text: String| if b { Ok(()) } else { Err(text) };
    match format {
        "date" => ok(datum_gueltig(s), tr!("kein gültiges Datum")),
        "email" => ok(email_gueltig(s), tr!("keine gültige E-Mail-Adresse")),
        "uri" => ok(uri_gueltig(s), tr!("keine gültige Adresse (URI)")),
        "orcid" => ok(orcid_gueltig(s), tr!("keine gültige ORCID")),
        "iban" => ok(iban_gueltig(s), tr!("keine gültige IBAN")),
        "uid-at" => ok(uid_gueltig(s), tr!("keine gültige UID (ATU + 8 Ziffern)")),
        "sv-nummer" => ok(
            sv_gueltig(s),
            tr!("keine gültige Sozialversicherungsnummer"),
        ),
        "literaturdatei" => {
            let klein = s.to_lowercase();
            if ![".bib", ".yaml", ".yml"].iter().any(|e| klein.ends_with(e)) {
                return Err(tr!("Literaturdatei muss .bib, .yaml oder .yml sein"));
            }
            projektdatei(s, k)
        }
        "zitierstil" => {
            if s.to_lowercase().ends_with(".csl") {
                projektdatei(s, k)
            } else {
                ok(k.stile.contains(&s), tr!("unbekannter Zitierstil"))
            }
        }
        _ => Ok(()),
    }
}

/// Relativer Pfad im Projekt ohne `..`, als Datei vorhanden (ohne Projektordner nur die Form).
fn projektdatei(s: &str, k: &Kontext) -> Result<(), String> {
    let p = Path::new(s);
    let sicher = !s.is_empty()
        && !s.starts_with('/')
        && !s.starts_with('\\')
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    let da = match k.wurzel {
        Some(w) => sicher && w.join(p).is_file(),
        None => sicher,
    };
    if da {
        Ok(())
    } else {
        Err(tr!("Datei nicht im Projekt gefunden"))
    }
}

/// `JJJJ-MM-TT` mit echtem Kalendertag.
pub(crate) fn datum_gueltig(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let zahl = |r: std::ops::Range<usize>| -> Option<u32> {
        let t = &s[r];
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    let (Some(j), Some(m), Some(t)) = (zahl(0..4), zahl(5..7), zahl(8..10)) else {
        return false;
    };
    let schalt = (j % 4 == 0 && j % 100 != 0) || j % 400 == 0;
    let tage = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if schalt => 29,
        2 => 28,
        _ => return false,
    };
    j >= 1 && (1..=tage).contains(&t)
}

fn email_gueltig(s: &str) -> bool {
    let Some((lokal, domain)) = s.split_once('@') else {
        return false;
    };
    !lokal.is_empty()
        && !domain.contains('@')
        && !s.chars().any(char::is_whitespace)
        && domain.contains('.')
        && domain.split('.').all(|t| !t.is_empty())
}

fn uri_gueltig(s: &str) -> bool {
    let Some((schema, rest)) = s.split_once(':') else {
        return false;
    };
    let mut z = schema.chars();
    z.next().is_some_and(|c| c.is_ascii_alphabetic())
        && z.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        && !rest.is_empty()
        && !s.chars().any(char::is_whitespace)
}

/// ISO 7064 11,2 über die ersten 15 Ziffern.
fn orcid_gueltig(s: &str) -> bool {
    let teile: Vec<&str> = s.split('-').collect();
    if teile.len() != 4 || teile.iter().any(|t| t.len() != 4) {
        return false;
    }
    let z: Vec<char> = teile.concat().chars().collect();
    if !z[..15].iter().all(char::is_ascii_digit) {
        return false;
    }
    let mut summe = 0u32;
    for c in &z[..15] {
        summe = (summe + c.to_digit(10).unwrap_or(0)) * 2;
    }
    let pruef = (12 - summe % 11) % 11;
    let erwartet = if pruef == 10 {
        'X'
    } else {
        char::from_digit(pruef, 10).unwrap_or('?')
    };
    z[15].to_ascii_uppercase() == erwartet
}

/// ISO 13616: umgestellt und in Ziffern, Rest mod 97 gleich 1.
fn iban_gueltig(s: &str) -> bool {
    let t: String = s
        .chars()
        .filter(|c| *c != ' ')
        .collect::<String>()
        .to_ascii_uppercase();
    if !(15..=34).contains(&t.len()) || !t.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let b = t.as_bytes();
    if !b[0].is_ascii_alphabetic()
        || !b[1].is_ascii_alphabetic()
        || !b[2].is_ascii_digit()
        || !b[3].is_ascii_digit()
    {
        return false;
    }
    let mut rest = 0u32;
    for c in t[4..].chars().chain(t[..4].chars()) {
        let wert = c.to_digit(36).unwrap_or(0);
        for ziffer in wert.to_string().chars() {
            rest = (rest * 10 + ziffer.to_digit(10).unwrap_or(0)) % 97;
        }
    }
    rest == 1
}

/// Österreichische UID: `ATU` und 8 Ziffern, die letzte ist die Prüfziffer.
fn uid_gueltig(s: &str) -> bool {
    let Some(z) = s.strip_prefix("ATU") else {
        return false;
    };
    if z.len() != 8 || !z.bytes().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let d: Vec<u32> = z.bytes().map(|c| u32::from(c - b'0')).collect();
    let mut r = 0;
    for (i, &x) in d[..7].iter().enumerate() {
        r += if i % 2 == 0 {
            x
        } else {
            (2 * x) / 10 + (2 * x) % 10
        };
    }
    (10 - (r + 4) % 10) % 10 == d[7]
}

/// Österreichische Sozialversicherungsnummer: 10 Ziffern, die vierte ist die Prüfziffer.
fn sv_gueltig(s: &str) -> bool {
    if s.len() != 10 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let d: Vec<u32> = s.bytes().map(|c| u32::from(c - b'0')).collect();
    const GEWICHTE: [u32; 10] = [3, 7, 9, 0, 5, 8, 4, 2, 1, 6];
    let summe: u32 = d.iter().zip(GEWICHTE).map(|(x, g)| x * g).sum();
    let pruef = summe % 11;
    pruef != 10 && pruef == d[3]
}
