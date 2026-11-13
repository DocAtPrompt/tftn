//! `dokument.toml` lesen und verlustfrei schreiben (KONZEPT 4.7).
//!
//! Geschrieben wird mit `toml_edit`: Kommentare, Reihenfolge und Schlüssel, die das Schema nicht
//! kennt, bleiben byte-gleich. Ein vorhandener Schlüssel behält seinen Platz und den Kommentar
//! daneben, ein neuer kommt ans Ende seiner Tabelle.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tftn_sprachen::tr;

use serde::Deserialize;
use serde_json::Value as Json;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, TableLike, Value};

use crate::pruefen::{Kontext, Schritt, anzahl, wert_pruefen};
use crate::schema::{Art, Feld, Formular};

/// Änderung an einer Liste von Einträgen (Array of Tables, etwa `[[autoren]]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
pub enum EintragAktion {
    Hinzufuegen,
    Entfernen { index: usize },
    Verschieben { index: usize, nach: usize },
}

/// Inhalt von `dokument.toml`.
#[derive(Debug, Clone)]
pub struct Daten {
    doc: DocumentMut,
    /// Die Datei hatte CRLF (Windows): beim Ausgeben wieder so (toml_edit schreibt sonst LF).
    crlf: bool,
    /// Die Datei begann mit einem UTF-8-BOM: beim Ausgeben wieder davor (toml_edit lässt ihn weg).
    bom: bool,
}

impl Daten {
    /// Liest den Text; die Fehlermeldung nennt Zeile und Spalte.
    pub fn lesen(text: &str) -> Result<Daten, String> {
        let (bom, text) = match text.strip_prefix('\u{feff}') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        text.parse::<DocumentMut>()
            .map(|doc| Daten {
                doc,
                crlf: text.contains("\r\n"),
                bom,
            })
            .map_err(|e| {
                let start = e.span().map(|s| s.start).unwrap_or(0).min(text.len());
                let vorher = &text[..start];
                let zeile = vorher.matches('\n').count() + 1;
                let spalte = vorher.rsplit('\n').next().unwrap_or("").chars().count() + 1;
                tr!(
                    "Zeile {zeile}, Spalte {spalte}: {meldung}",
                    zeile,
                    spalte,
                    meldung = e.message().trim()
                )
            })
    }

    pub fn text(&self) -> String {
        let t = self.doc.to_string();
        let t = if self.crlf {
            // Zeilenumbrüche in mehrzeiligen Strings liest TOML als `\n` zurück; der Wert bleibt.
            t.replace("\r\n", "\n").replace('\n', "\r\n")
        } else {
            t
        };
        if self.bom { format!("\u{feff}{t}") } else { t }
    }

    /// Alle Werte als JSON-Objekt; Datumswerte als `JJJJ-MM-TT`.
    pub fn werte(&self) -> Json {
        item_json(self.doc.as_item())
    }

    /// Prüft den Wert gegen das Feld unter `pfad` und schreibt ihn; `None` entfernt den Schlüssel.
    /// Bei einem Fehler bleibt der Text unverändert.
    pub fn setzen(
        &mut self,
        f: &Formular,
        pfad: &[Schritt],
        wert: Option<Json>,
        k: &Kontext,
    ) -> Result<(), String> {
        let (feld, schluessel) = feld_an(&f.felder, pfad).ok_or_else(|| tr!("unbekanntes Feld"))?;
        if matches!(feld.art, Art::Gruppe { .. } | Art::Eintraege { .. }) {
            return Err(tr!("unbekanntes Feld"));
        }
        wert_pruefen(feld, wert.as_ref(), k)?;
        let neu = wert.map(|w| json_toml(&feld.art, &w)).transpose()?;
        let behaelter = behaelter(
            self.doc.as_table_mut(),
            &f.felder,
            &pfad[..pfad.len() - 1],
            neu.is_some(),
        )?;
        let Some(t) = behaelter else {
            return Ok(()); // Gruppe fehlt und es soll nur entfernt werden
        };
        match neu {
            None => {
                if let Some(rest) = entfernen(t, schluessel) {
                    // Nirgends in der Tabelle unterzubringen: ans Ende der Datei.
                    let alt = self.doc.trailing().as_str().unwrap_or("").to_string();
                    self.doc.set_trailing(format!("{alt}{rest}"));
                }
            }
            Some(mut v) => match t.get_mut(schluessel) {
                Some(Item::Value(alt)) => {
                    *v.decor_mut() = alt.decor().clone();
                    *alt = v;
                }
                Some(item) => *item = Item::Value(v),
                None => {
                    t.insert(schluessel, Item::Value(v));
                }
            },
        }
        Ok(())
    }

    /// Fügt einen Eintrag hinzu, entfernt oder verschiebt ihn. `pfad` zeigt auf die Liste.
    pub fn eintrag(
        &mut self,
        f: &Formular,
        pfad: &[Schritt],
        aktion: EintragAktion,
    ) -> Result<(), String> {
        let (feld, schluessel) = feld_an(&f.felder, pfad).ok_or_else(|| tr!("unbekanntes Feld"))?;
        let Art::Eintraege { felder, min, max } = &feld.art else {
            return Err(tr!("unbekanntes Feld"));
        };
        let t = behaelter(
            self.doc.as_table_mut(),
            &f.felder,
            &pfad[..pfad.len() - 1],
            true,
        )?
        .ok_or_else(|| tr!("Eintrag fehlt"))?;
        if t.get(schluessel).is_none() {
            t.insert(schluessel, Item::ArrayOfTables(ArrayOfTables::new()));
        }
        // Von Hand als Inline-Array geschrieben (`autoren = [{…}]`): in dieser Form bearbeiten.
        if let Some(Item::Value(Value::Array(arr))) = t.get_mut(schluessel) {
            if !arr.iter().all(Value::is_inline_table) {
                return Err(tr!("die Daten haben hier keine Liste von Einträgen"));
            }
            let n = arr.len();
            match aktion {
                EintragAktion::Hinzufuegen => {
                    anzahl(n + 1, None, *max)?;
                    let mut neu = neuer_eintrag(felder).into_inline_table();
                    neu.fmt();
                    arr.push(neu);
                }
                EintragAktion::Entfernen { index } => {
                    if index >= n {
                        return Err(tr!("Eintrag fehlt"));
                    }
                    anzahl(n - 1, *min, None)?;
                    arr.remove(index);
                }
                EintragAktion::Verschieben { index, nach } => {
                    if index >= n || nach >= n {
                        return Err(tr!("Eintrag fehlt"));
                    }
                    let v = arr.remove(index);
                    arr.insert_formatted(nach, v);
                }
            }
            arr.fmt();
            if arr.is_empty() {
                t.remove(schluessel);
            }
            return Ok(());
        }
        let liste = t
            .get_mut(schluessel)
            .and_then(Item::as_array_of_tables_mut)
            .ok_or_else(|| tr!("die Daten haben hier keine Liste von Einträgen"))?;
        let n = liste.len();
        match aktion {
            EintragAktion::Hinzufuegen => {
                anzahl(n + 1, None, *max)?;
                liste.push(neuer_eintrag(felder));
            }
            EintragAktion::Entfernen { index } => {
                if index >= n {
                    return Err(tr!("Eintrag fehlt"));
                }
                anzahl(n - 1, *min, None)?;
                liste.remove(index);
                if liste.is_empty() {
                    t.remove(schluessel);
                }
            }
            EintragAktion::Verschieben { index, nach } => {
                if index >= n || nach >= n {
                    return Err(tr!("Eintrag fehlt"));
                }
                // Die Reihenfolge im Text folgt den Positionen der Tabellen, nicht der Liste:
                // Einträge umstellen und die Positionen in der alten Reihenfolge neu vergeben.
                let positionen: Vec<Option<usize>> = liste.iter().map(Table::position).collect();
                let mut tabellen: Vec<Table> = liste.iter().cloned().collect();
                let t = tabellen.remove(index);
                tabellen.insert(nach, t);
                liste.clear();
                for (mut t, p) in tabellen.into_iter().zip(positionen) {
                    if let Some(p) = p {
                        t.set_position(p);
                    }
                    liste.push(t);
                }
            }
        }
        Ok(())
    }
}

/// Entfernt `schluessel`. Kommentarzeilen darüber gehören in toml_edit zum Schlüssel und gingen mit
/// ihm verloren; sie wandern vor den nächsten Wert der Tabelle, sonst hinter den vorigen. Gibt den
/// Kommentar zurück, wenn beides fehlt.
fn entfernen(t: &mut dyn TableLike, schluessel: &str) -> Option<String> {
    let kommentar = t
        .key(schluessel)
        .and_then(|k| k.leaf_decor().prefix())
        .and_then(|p| p.as_str())
        .filter(|p| p.contains('#'))
        .map(str::to_string);
    let werte: Vec<String> = t
        .iter()
        .filter(|(_, i)| i.is_value())
        .map(|(k, _)| k.to_string())
        .collect();
    let i = werte.iter().position(|k| k == schluessel);
    t.remove(schluessel);
    let kommentar = kommentar?;
    let naechster = i.and_then(|i| werte.get(i + 1));
    let voriger = i.and_then(|i| i.checked_sub(1)).and_then(|i| werte.get(i));
    if let Some(k) = naechster
        && let Some(mut key) = t.key_mut(k)
    {
        let alt = key
            .leaf_decor()
            .prefix()
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        key.leaf_decor_mut().set_prefix(format!("{kommentar}{alt}"));
        return None;
    }
    if let Some(k) = voriger
        && let Some(Item::Value(v)) = t.get_mut(k)
    {
        // Hinter dem Wert endet die Zeile; der Kommentar folgt als eigene Zeile(n).
        let alt = v
            .decor()
            .suffix()
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        let zeilen = kommentar.trim_end_matches(['\n', '\r']);
        v.decor_mut().set_suffix(format!("{alt}\n{zeilen}"));
        return None;
    }
    Some(kommentar)
}

/// Neuer Eintrag: Felder mit Vorgabe, Pflichtfelder ohne Vorgabe leer (Festlegung 7).
fn neuer_eintrag(felder: &[Feld]) -> Table {
    let mut t = Table::new();
    for f in felder {
        let v: Option<Value> = match &f.art {
            Art::Text { standard, .. } => standard
                .clone()
                .or_else(|| f.pflicht.then(String::new))
                .map(Value::from),
            Art::Zahl { standard, .. } => standard.or_else(|| f.pflicht.then_some(0.0)).map(zahl),
            Art::Schalter { standard } => standard
                .or_else(|| f.pflicht.then_some(false))
                .map(Value::from),
            Art::Wahl {
                werte, standard, ..
            } => standard
                .clone()
                .or_else(|| f.pflicht.then(|| werte[0].clone()))
                .and_then(|w| json_toml(&f.art, &w).ok()),
            Art::Datum { standard } => standard
                .as_ref()
                .and_then(|s| json_toml(&f.art, &Json::from(s.clone())).ok()),
            Art::Liste { .. } => f.pflicht.then(|| Value::Array(toml_edit::Array::new())),
            Art::Gruppe { .. } | Art::Eintraege { .. } => None,
        };
        if let Some(v) = v {
            t.insert(&f.name, Item::Value(v));
        }
    }
    t
}

fn zahl(z: f64) -> Value {
    if z.fract() == 0.0 && z.abs() < 9.0e15 {
        Value::from(z as i64)
    } else {
        Value::from(z)
    }
}

/// Das Feld unter `pfad` und sein Schlüssel. Pfade: `name`, `gruppe/name`, `liste/i/name`.
pub(crate) fn feld_an<'a>(felder: &'a [Feld], pfad: &'a [Schritt]) -> Option<(&'a Feld, &'a str)> {
    let Some(Schritt::Name(n)) = pfad.first() else {
        return None;
    };
    let feld = felder.iter().find(|f| &f.name == n)?;
    match (&pfad[1..], &feld.art) {
        ([], _) => Some((feld, n)),
        ([Schritt::Index(_), rest @ ..], Art::Eintraege { felder, .. }) if !rest.is_empty() => {
            feld_an(felder, rest)
        }
        (rest @ [Schritt::Name(_), ..], Art::Gruppe { felder }) => feld_an(felder, rest),
        _ => None,
    }
}

/// Die Tabelle, in der der letzte Schlüssel des Pfads liegt (`vor` ist der Pfad ohne ihn).
/// Fehlt eine Gruppe, wird sie mit `anlegen` als `[name]` angelegt, sonst `Ok(None)`.
fn behaelter<'a>(
    mut t: &'a mut dyn TableLike,
    felder: &[Feld],
    vor: &[Schritt],
    anlegen: bool,
) -> Result<Option<&'a mut dyn TableLike>, String> {
    let mut felder = felder;
    let mut i = 0;
    while i < vor.len() {
        let Schritt::Name(n) = &vor[i] else {
            return Err(tr!("unbekanntes Feld"));
        };
        let feld = felder
            .iter()
            .find(|f| &f.name == n)
            .ok_or_else(|| tr!("unbekanntes Feld"))?;
        match &feld.art {
            Art::Gruppe { felder: unter } => {
                if t.get(n).is_none() {
                    if !anlegen {
                        return Ok(None);
                    }
                    t.insert(n, Item::Table(Table::new()));
                }
                t = t
                    .get_mut(n)
                    .and_then(Item::as_table_like_mut)
                    .ok_or_else(|| tr!("die Daten haben hier keine Tabelle"))?;
                felder = unter;
                i += 1;
            }
            Art::Eintraege { felder: unter, .. } => {
                let Some(Schritt::Index(j)) = vor.get(i + 1) else {
                    return Err(tr!("unbekanntes Feld"));
                };
                let eintrag: &mut dyn TableLike = match t.get_mut(n) {
                    Some(Item::ArrayOfTables(a)) => a.get_mut(*j).map(|t| t as &mut dyn TableLike),
                    Some(Item::Value(Value::Array(a))) => a
                        .get_mut(*j)
                        .and_then(Value::as_inline_table_mut)
                        .map(|t| t as &mut dyn TableLike),
                    _ => None,
                }
                .ok_or_else(|| tr!("Eintrag fehlt"))?;
                t = eintrag;
                felder = unter;
                i += 2;
            }
            _ => return Err(tr!("unbekanntes Feld")),
        }
    }
    Ok(Some(t))
}

/// JSON-Wert (schon geprüft) → TOML-Wert der Art.
fn json_toml(art: &Art, w: &Json) -> Result<Value, String> {
    Ok(match (art, w) {
        (Art::Datum { .. }, Json::String(s)) => {
            let teil = |r: std::ops::Range<usize>| s.get(r);
            let (Some(year), Some(month), Some(day)) = (
                teil(0..4).and_then(|t| t.parse::<u16>().ok()),
                teil(5..7).and_then(|t| t.parse::<u8>().ok()),
                teil(8..10).and_then(|t| t.parse::<u8>().ok()),
            ) else {
                return Err(tr!("kein gültiges Datum"));
            };
            Value::from(toml_edit::Datetime {
                date: Some(toml_edit::Date { year, month, day }),
                time: None,
                offset: None,
            })
        }
        (Art::Liste { element, .. }, Json::Array(a)) => {
            let mut arr = toml_edit::Array::new();
            for e in a {
                arr.push(json_toml(element, e)?);
            }
            arr.fmt();
            Value::Array(arr)
        }
        (_, Json::String(s)) => Value::from(s.as_str()),
        (_, Json::Bool(b)) => Value::from(*b),
        (_, Json::Number(n)) => match n.as_i64() {
            Some(i) => Value::from(i),
            None => zahl(n.as_f64().ok_or_else(|| tr!("Zahl erwartet"))?),
        },
        _ => return Err(tr!("Wert passt nicht zum Feld")),
    })
}

fn wert_json(v: &Value) -> Json {
    match v {
        Value::String(s) => Json::from(s.value().as_str()),
        Value::Integer(i) => Json::from(*i.value()),
        Value::Float(f) => serde_json::Number::from_f64(*f.value())
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Value::Boolean(b) => Json::from(*b.value()),
        Value::Datetime(d) => {
            let d = d.value();
            match (d.date, d.time) {
                (Some(t), None) => Json::from(format!("{:04}-{:02}-{:02}", t.year, t.month, t.day)),
                _ => Json::from(d.to_string()),
            }
        }
        Value::Array(a) => Json::Array(a.iter().map(wert_json).collect()),
        Value::InlineTable(t) => Json::Object(
            t.iter()
                .map(|(k, v)| (k.to_string(), wert_json(v)))
                .collect(),
        ),
    }
}

fn item_json(i: &Item) -> Json {
    match i {
        Item::None => Json::Null,
        Item::Value(v) => wert_json(v),
        Item::Table(t) => Json::Object(
            t.iter()
                .map(|(k, v)| (k.to_string(), item_json(v)))
                .collect(),
        ),
        Item::ArrayOfTables(a) => Json::Array(
            a.iter()
                .map(|t| item_json(&Item::Table(t.clone())))
                .collect(),
        ),
    }
}

/// `[literatur].dateien`, falls vorhanden (für das Laden der Literatur in der App).
pub fn literatur_dateien(text: &str) -> Option<Vec<String>> {
    let doc = text.parse::<DocumentMut>().ok()?;
    let a = doc.get("literatur")?.get("dateien")?.as_array()?;
    Some(
        a.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    )
}

/// Prüfsumme des Dateitexts (FNV-1a, 64 Bit) als Hex; stabil über Läufe und Rechner.
pub fn stand(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Schreibt atomar: erst eine temporäre Datei mit eindeutigem Namen im selben Ordner, dann
/// umbenennen. Scheitert das Umbenennen, wird die temporäre Datei gelöscht.
pub fn atomar_schreiben(pfad: &Path, text: &str) -> std::io::Result<()> {
    static ZAEHLER: AtomicU64 = AtomicU64::new(0);
    let ordner = pfad.parent().unwrap_or(Path::new("."));
    let name = pfad
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = ordner.join(format!(
        ".{name}.{}-{}.tmp",
        std::process::id(),
        ZAEHLER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, pfad).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}
