//! Eigene Textbausteine (`textbausteine.toml` im Ordner für eigene Anpassungen): Standardsätze und
//! -absätze, die der Benutzer selbst pflegt. Format wie `[[baustein]]` in Paketbeschreibungen, nur
//! ohne `funktion` und `parameter`: `titel` und `einfuegen` (mit `${feld:Name}`, `${cursor}`,
//! `${auswahl}`), optional `id`, `gruppe`, `kuerzel` und `modus` (Standard: nur im Text).
//!
//! Eine fehlerhafte Datei bringt keine Bausteine und eine Meldung mit Zeile; ein fehlerhafter Eintrag
//! wird mit Meldung übersprungen, die übrigen bleiben. Angehängt wird an den Text der Datei, damit
//! Kommentare und Reihenfolge erhalten bleiben.

use std::path::Path;

use serde::Deserialize;
use tftn_manifest::Modus;
use tftn_sprachen::tr;

use crate::{Eintrag, Herkunft, Schnipsel};

/// Dateiname im Ordner für eigene Anpassungen.
pub const DATEI: &str = "textbausteine.toml";

/// Namensraum der Ids im Katalog: `eigene/<id>`.
pub const NAMENSRAUM: &str = "eigene";

/// Inhalt einer neuen Datei: nur Kommentare, damit nichts erscheint, was der Benutzer nicht schrieb.
pub const VORLAGE: &str = r#"# Eigene Textbausteine für TFTN
#
# Erscheinen im Menü nach „/“ (in einer Leerzeile), in der Palette „Baustein einfügen“ und in der
# Seitenleiste unter „Einfügen“, in jedem Dokumenttyp. Änderungen wirken sofort, ohne Neustart.
#
# Je Baustein ein Block [[baustein]] mit:
#   titel      Name im Menü (Pflicht)
#   einfuegen  der Text (Pflicht); mehrzeilig zwischen ''' und '''
#   gruppe     Überschrift zum Ordnen und Suchen (optional)
#   kuerzel    Tastenkürzel, z. B. "Mod-Alt-1" (Mod = Cmd am Mac, Strg unter Windows; optional)
#   id         fester Name für Kürzel-Einstellungen (optional; sonst aus Gruppe und Titel)
#
# Lücken zum Ausfüllen: ${feld:Name} oder ${feld:Name:Vorgabe}; Tab springt zur nächsten.
# ${cursor} setzt die Schreibmarke, ${auswahl} nimmt markierten Text auf.
#
# Beispiel (zum Ausprobieren die Rauten am Zeilenanfang entfernen):
#
# [[baustein]]
# titel = "Procedere Kontrolle"
# gruppe = "Procedere"
# einfuegen = '''
# Kontrolle in ${feld:Wochen:6} Wochen, bei Beschwerden früher.
# ${cursor}'''
#
# [[baustein]]
# titel = "Grußformel"
# gruppe = "Brief"
# kuerzel = "Mod-Alt-g"
# einfuegen = '''
# Mit freundlichen Grüßen
#
# ${feld:Name}'''
"#;

#[derive(Deserialize)]
struct Datei {
    #[serde(default)]
    baustein: Vec<toml::Value>,
}

#[derive(Deserialize)]
struct Eigener {
    id: Option<String>,
    titel: String,
    einfuegen: String,
    gruppe: Option<String>,
    kuerzel: Option<String>,
    #[serde(default = "nur_markup")]
    modus: Vec<Modus>,
}

fn nur_markup() -> Vec<Modus> {
    vec![Modus::Markup]
}

/// Was in der Datei steht: die Bausteine und, was nicht ging.
#[derive(Debug, Clone, Default)]
pub struct Gelesen {
    pub eintraege: Vec<Eintrag>,
    /// Die ganze Datei ist unlesbar (Zeile und Grund); dann gibt es keine Einträge.
    pub fehler: Option<String>,
    /// Übersprungene Einträge
    pub warnungen: Vec<String>,
}

impl Gelesen {
    /// Alle Meldungen in einer Zeile, für Einstellungen und Hinweis; `None`, wenn alles stimmt.
    pub fn meldung(&self) -> Option<String> {
        let alle: Vec<&str> = self
            .fehler
            .iter()
            .chain(&self.warnungen)
            .map(String::as_str)
            .collect();
        (!alle.is_empty()).then(|| alle.join(" · "))
    }
}

/// Liest `ordner/textbausteine.toml`; fehlt die Datei, gibt es nichts (und keinen Fehler).
pub fn im_ordner(ordner: &Path) -> Gelesen {
    match std::fs::read_to_string(ordner.join(DATEI)) {
        Ok(text) => lesen(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Gelesen::default(),
        Err(e) => Gelesen {
            fehler: Some(format!("{DATEI}: {e}")),
            ..Gelesen::default()
        },
    }
}

/// Liest den Text einer `textbausteine.toml`.
pub fn lesen(text: &str) -> Gelesen {
    let datei: Datei = match toml::from_str(text) {
        Ok(d) => d,
        Err(e) => {
            return Gelesen {
                fehler: Some(fehlertext(text, &e)),
                ..Gelesen::default()
            };
        }
    };
    let mut aus = Gelesen::default();
    for (i, wert) in datei.baustein.into_iter().enumerate() {
        let nr = i + 1;
        let titel_von = wert
            .get("titel")
            .and_then(toml::Value::as_str)
            .map(str::to_string);
        let wo = match &titel_von {
            Some(t) => tr!(
                "{datei}, Baustein {nr} („{titel}“)",
                datei = DATEI,
                nr,
                titel = t
            ),
            None => tr!("{datei}, Baustein {nr}", datei = DATEI, nr),
        };
        let b: Eigener = match wert.try_into() {
            Ok(b) => b,
            Err(e) => {
                aus.warnungen.push(tr!(
                    "{wo}: {fehler} – übersprungen",
                    wo,
                    fehler = feldfehler(e.message())
                ));
                continue;
            }
        };
        match eintrag(b) {
            Ok(e) if aus.eintraege.iter().any(|x| x.id == e.id) => aus.warnungen.push(tr!(
                "{wo}: Id „{id}“ doppelt, der erste bleibt",
                wo,
                id = &e.id[NAMENSRAUM.len() + 1..]
            )),
            Ok(e) => aus.eintraege.push(e),
            Err(grund) => aus
                .warnungen
                .push(tr!("{wo}: {grund} – übersprungen", wo, grund)),
        }
    }
    aus
}

fn eintrag(b: Eigener) -> Result<Eintrag, String> {
    let titel = b.titel.trim().to_string();
    if titel.is_empty() {
        return Err(tr!("Titel fehlt"));
    }
    let schnipsel = Schnipsel::lesen(&b.einfuegen).map_err(|f| {
        tr!(
            "einfuegen: {grund} (Zeichen {stelle})",
            grund = f.grund,
            stelle = b.einfuegen[..f.stelle].chars().count() + 1
        )
    })?;
    let id = match b.id.as_deref().map(str::trim) {
        Some("") => return Err(tr!("id ist leer")),
        Some(id) => id.to_string(),
        None => kennung(&match &b.gruppe {
            Some(g) => format!("{g} {titel}"),
            None => titel.clone(),
        }),
    };
    if b.modus.is_empty() {
        return Err(tr!("modus ist leer"));
    }
    Ok(Eintrag {
        id: format!("{NAMENSRAUM}/{id}"),
        titel,
        gruppe: b
            .gruppe
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty()),
        herkunft: Herkunft::Eigene,
        funktion: None,
        schnipsel,
        modus: b.modus,
        kuerzel: b.kuerzel.filter(|k| !k.trim().is_empty()),
        parameter: Default::default(),
    })
}

/// Id aus einem Titel: Kleinbuchstaben und Ziffern, alles andere wird zu `-`.
pub fn kennung(titel: &str) -> String {
    let mut aus = String::new();
    for z in titel.chars().flat_map(char::to_lowercase) {
        if z.is_alphanumeric() {
            aus.push(z);
        } else if !aus.is_empty() && !aus.ends_with('-') {
            aus.push('-');
        }
    }
    while aus.ends_with('-') {
        aus.pop();
    }
    if aus.is_empty() {
        "baustein".into()
    } else {
        aus
    }
}

/// „missing field `titel`“ → „titel fehlt“; andere Meldungen bleiben, wie sie sind.
fn feldfehler(meldung: &str) -> String {
    match meldung
        .trim()
        .strip_prefix("missing field `")
        .and_then(|r| r.strip_suffix('`'))
    {
        Some(feld) => tr!("{feld} fehlt", feld),
        None => meldung.trim().to_string(),
    }
}

fn fehlertext(text: &str, e: &toml::de::Error) -> String {
    let zeile = e
        .span()
        .map(|s| text[..s.start.min(text.len())].matches('\n').count() + 1);
    match zeile {
        Some(z) => tr!(
            "{datei}, Zeile {zeile}: {meldung}",
            datei = DATEI,
            zeile = z,
            meldung = e.message().trim()
        ),
        None => tr!(
            "{datei}: {meldung}",
            datei = DATEI,
            meldung = e.message().trim()
        ),
    }
}

/// Hängt einen Baustein an den Text der Datei an (`None`: Datei fehlt, dann mit [`VORLAGE`]
/// davor). Gibt den neuen Text und die Id (ohne Namensraum) zurück. Eine fehlerhafte Datei wird
/// nicht angefasst; was davor steht, bleibt Zeichen für Zeichen erhalten.
pub fn anhaengen(
    alt: Option<&str>,
    titel: &str,
    gruppe: Option<&str>,
    text: &str,
) -> Result<(String, String), String> {
    let titel = titel.trim();
    if titel.is_empty() {
        return Err(tr!("Bitte einen Titel angeben."));
    }
    if text.trim().is_empty() {
        return Err(tr!("Kein Text markiert."));
    }
    let gruppe = gruppe.map(str::trim).filter(|g| !g.is_empty());
    let alt = alt.unwrap_or(VORLAGE);
    let vorher = lesen(alt);
    if let Some(f) = vorher.fehler {
        return Err(tr!(
            "{fehler} – bitte erst die Datei reparieren; sie bleibt unverändert.",
            fehler = f
        ));
    }
    let basis = kennung(titel);
    let belegt = |id: &str| {
        vorher
            .eintraege
            .iter()
            .any(|e| e.id[NAMENSRAUM.len() + 1..] == *id)
    };
    let id = (1..)
        .map(|n| {
            if n == 1 {
                basis.clone()
            } else {
                format!("{basis}-{n}")
            }
        })
        .find(|id| !belegt(id))
        .expect("eine freie Id gibt es immer");
    let mut block = String::from("[[baustein]]\n");
    block.push_str(&format!("id = {}\n", einfach(&id)));
    block.push_str(&format!("titel = {}\n", einfach(titel)));
    if let Some(g) = gruppe {
        block.push_str(&format!("gruppe = {}\n", einfach(g)));
    }
    let neu = |wert: String| {
        let mut t = alt.to_string();
        if !t.is_empty() && !t.ends_with('\n') {
            t.push('\n');
        }
        if !t.is_empty() && !t.ends_with("\n\n") {
            t.push('\n');
        }
        t.push_str(&block);
        t.push_str(&format!("einfuegen = {wert}\n"));
        t
    };
    // Mehrzeilig lesbar, wenn es geht; sonst als Zeichenkette mit Escapes. Geprüft wird, ob die
    // Datei danach genau diesen Baustein enthält.
    let versuche = mehrzeilig(text)
        .into_iter()
        .chain(std::iter::once(einfach(text)));
    let mut grund = None;
    for wert in versuche {
        let kandidat = neu(wert);
        let g = lesen(&kandidat);
        if let Some(f) = g.fehler {
            grund = Some(f);
            continue;
        }
        let voll = format!("{NAMENSRAUM}/{id}");
        match g.eintraege.iter().find(|e| e.id == voll) {
            Some(e) if Schnipsel::lesen(text).is_ok_and(|s| s == e.schnipsel) => {
                return Ok((kandidat, id));
            }
            Some(_) => grund = Some(tr!("Text verändert sich beim Speichern")),
            None => {
                grund = Some(
                    g.warnungen
                        .last()
                        .cloned()
                        .unwrap_or_else(|| tr!("Baustein fehlt nach dem Anhängen")),
                )
            }
        }
    }
    Err(tr!(
        "Nicht gespeichert: {grund}",
        grund = grund.unwrap_or_default()
    ))
}

/// Zeichenkette mit Escapes (`"…"`).
fn einfach(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// Mehrzeilige wörtliche Zeichenkette (`'''…'''`), wenn der Text das zulässt.
fn mehrzeilig(s: &str) -> Option<String> {
    let geht = s.contains('\n')
        && !s.contains("'''")
        && !s.ends_with('\'')
        && !s.chars().any(|z| z.is_control() && z != '\n' && z != '\t');
    geht.then(|| format!("'''\n{s}'''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kennung_aus_titel() {
        assert_eq!(kennung("Procedere Kontrolle"), "procedere-kontrolle");
        assert_eq!(kennung("  Grüße, Dr. Ü!  "), "grüße-dr-ü");
        assert_eq!(kennung("…"), "baustein");
    }

    #[test]
    fn vorlage_ist_leer_und_gueltig() {
        let g = lesen(VORLAGE);
        assert!(g.fehler.is_none() && g.warnungen.is_empty());
        assert!(g.eintraege.is_empty());
    }

    #[test]
    fn beispiel_der_vorlage_geht_ohne_rauten() {
        let ohne: String = VORLAGE
            .lines()
            .skip_while(|z| !z.starts_with("# [[baustein]]"))
            .map(|z| z.strip_prefix("# ").or(z.strip_prefix('#')).unwrap_or(z))
            .collect::<Vec<_>>()
            .join("\n");
        let g = lesen(&ohne);
        assert_eq!(g.meldung(), None);
        assert_eq!(g.eintraege.len(), 2);
        assert_eq!(g.eintraege[0].id, "eigene/procedere-procedere-kontrolle");
        assert_eq!(g.eintraege[1].kuerzel.as_deref(), Some("Mod-Alt-g"));
    }
}
