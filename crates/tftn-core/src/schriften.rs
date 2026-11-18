//! Schriften einbinden: nur, was das Dokument benutzt (`fonts/` im Projekt).
//!
//! Von Textschriften kommt die ganze Familie mit, damit Fett und Kursiv später weiter gehen. Von
//! Formelschriften (Familie mit „Math“) nur die benutzten Schnitte; sie sind groß und Fett wird
//! selten gebraucht. Die Quelle ist der Vorrat der App (`schriftvorrat_setzen`).
//!
//! Weitere Schriften (Systemschriften oder Schriftdateien) kommen nie direkt in den Schriftpfad,
//! sondern werden als ganze Familie nach `fonts/` kopiert (`familie_einbinden`). So sehen Vorschau,
//! typst-CLI und Gegenprobe-Zeile dieselben Dateien, auch auf einem anderen Rechner.
//!
//! Fehlende Zeichen: Typst ersetzt still aus einer anderen Schrift oder zeichnet ein leeres
//! Kästchen. `ersatzzeichen` findet beides im gesetzten Dokument, `fehlende_zeichen` prüft eine
//! Schrift schon bei der Wahl gegen die Zeichen, die eine Sprache braucht.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use tftn_sprachen::tr;
use typst::foundations::Bytes;
use typst::layout::{Frame, FrameItem};
use typst::syntax::Span;
use typst::text::{Font, FontInfo, FontStyle, FontVariant, FontWeight};
use typst_layout::PagedDocument;

use crate::Projekt;
use crate::projekt::schriftvorrat;

/// Familie und Schnitt jeder Schrift, die im Satz vorkommt.
pub fn benutzt(doc: &PagedDocument) -> Vec<(String, FontVariant)> {
    fn sammeln(frame: &Frame, aus: &mut Vec<(String, FontVariant)>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => sammeln(&g.frame, aus),
                FrameItem::Text(t) => {
                    let eintrag = (t.font.info().family.clone(), t.font.info().variant);
                    if !aus.contains(&eintrag) {
                        aus.push(eintrag);
                    }
                }
                _ => {}
            }
        }
    }
    let mut aus = Vec::new();
    for seite in doc.pages() {
        sammeln(&seite.frame, &mut aus);
    }
    aus
}

/// Fingerabdruck der benutzten Schriften, unabhängig von der Reihenfolge. Ändert er sich nicht,
/// braucht `einbinden` nicht zu laufen (es liest jede Schriftdatei des Vorrats).
pub fn fingerabdruck(doc: &PagedDocument) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut einzeln: Vec<u64> = benutzt(doc)
        .iter()
        .map(|e| {
            let mut h = DefaultHasher::new();
            e.hash(&mut h);
            h.finish()
        })
        .collect();
    einzeln.sort_unstable();
    let mut h = DefaultHasher::new();
    einzeln.hash(&mut h);
    h.finish()
}

fn ist_formelschrift(familie: &str) -> bool {
    familie.contains("Math")
}

/// Kopiert, was `doc` benutzt und in `fonts/` noch fehlt, aus dem Vorrat; dazu die Lizenztexte
/// der Ordner, aus denen etwas kam. Liefert die neu kopierten Dateinamen.
pub fn einbinden(projekt: &Projekt, doc: &PagedDocument) -> std::io::Result<Vec<String>> {
    let benutzt = benutzt(doc);
    let familien: BTreeSet<&str> = benutzt
        .iter()
        .map(|(f, _)| f.as_str())
        .filter(|f| !ist_formelschrift(f))
        .collect();
    let ziel = projekt.schriften();
    let mut neu = Vec::new();
    for ordner in schriftvorrat() {
        let Ok(eintraege) = std::fs::read_dir(&ordner) else {
            continue;
        };
        let mut gebraucht = false;
        let mut dateien: Vec<_> = eintraege.filter_map(Result::ok).map(|e| e.path()).collect();
        dateien.sort();
        for datei in &dateien {
            if !passt(datei, &familien, &benutzt) {
                continue;
            }
            gebraucht = true;
            neu.extend(kopieren(datei, &ziel)?);
        }
        if gebraucht {
            for datei in dateien.iter().filter(|d| ist_lizenz(d)) {
                neu.extend(kopieren(datei, &ziel)?);
            }
        }
    }
    Ok(neu)
}

fn ist_lizenz(p: &Path) -> bool {
    p.file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("LIZENZEN"))
}

/// Die Datei enthält eine benutzte Textfamilie oder einen benutzten Formelschnitt.
fn passt(datei: &Path, familien: &BTreeSet<&str>, benutzt: &[(String, FontVariant)]) -> bool {
    let endung = datei
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !matches!(endung.as_str(), "otf" | "ttf" | "otc" | "ttc") {
        return false;
    }
    let Ok(daten) = std::fs::read(datei) else {
        return false;
    };
    Font::iter(Bytes::new(daten)).any(|f| {
        let info = f.info();
        if ist_formelschrift(&info.family) {
            benutzt
                .iter()
                .any(|(fam, var)| *fam == info.family && *var == info.variant)
        } else {
            familien.contains(info.family.as_str())
        }
    })
}

/// Kopiert `datei` nach `ziel`, falls dort noch nicht vorhanden; liefert den Namen, wenn kopiert.
fn kopieren(datei: &Path, ziel: &Path) -> std::io::Result<Option<String>> {
    let Some(name) = datei.file_name() else {
        return Ok(None);
    };
    let nach = ziel.join(name);
    if nach.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(ziel)?;
    std::fs::copy(datei, &nach)?;
    Ok(Some(name.to_string_lossy().into_owned()))
}

// --- Weitere Schriften ------------------------------------------------------------------------

/// Eine Schriftfamilie außerhalb des Projekts: auf dem Rechner installiert oder aus einer Datei.
#[derive(Debug, Clone)]
pub struct Familie {
    pub name: String,
    /// Dateien mit Schnitten der Familie (sortiert, ohne Doppel).
    pub dateien: Vec<PathBuf>,
    /// Anzahl der Schnitte (normal, fett, kursiv …).
    pub schnitte: usize,
    /// Ein Schnitt der Familie, möglichst der normale (für die Zeichenprüfung).
    pub info: FontInfo,
}

impl Familie {
    /// Gesamtgröße der Dateien in Byte.
    pub fn groesse(&self) -> u64 {
        self.dateien
            .iter()
            .filter_map(|d| std::fs::metadata(d).ok())
            .map(|m| m.len())
            .sum()
    }
}

/// Fasst gefundene Schnitte zu Familien zusammen, nach Namen sortiert. Versteckte Systemschriften
/// (Name mit „.“ vorne) fehlen.
fn familien_bilden(
    schnitte: impl IntoIterator<Item = (typst_kit::fonts::FontPath, FontInfo)>,
) -> Vec<Familie> {
    let mut nach_name: BTreeMap<String, Familie> = BTreeMap::new();
    for (pfad, info) in schnitte {
        if info.family.starts_with('.') || info.family.is_empty() {
            continue;
        }
        let f = nach_name
            .entry(info.family.clone())
            .or_insert_with(|| Familie {
                name: info.family.clone(),
                dateien: Vec::new(),
                schnitte: 0,
                info: info.clone(),
            });
        f.schnitte += 1;
        if !f.dateien.contains(&pfad.path) {
            f.dateien.push(pfad.path);
        }
        if ist_normal(&info.variant) && !ist_normal(&f.info.variant) {
            f.info = info;
        }
    }
    let mut aus: Vec<Familie> = nach_name.into_values().collect();
    for f in &mut aus {
        f.dateien.sort();
    }
    aus
}

fn ist_normal(v: &FontVariant) -> bool {
    v.style == FontStyle::Normal && v.weight == FontWeight::REGULAR
}

/// Die auf diesem Rechner installierten Schriftfamilien (ohne Netz; liest die Schriftordner des
/// Systems, dauert einen Moment).
pub fn systemfamilien() -> Vec<Familie> {
    familien_bilden(typst_kit::fonts::system())
}

/// Die Familien in einer Schriftdatei, mit den Schnitten aus demselben Ordner (eine gewählte
/// `…-Regular.otf` bringt so Fett und Kursiv daneben mit).
pub fn dateifamilien(datei: &Path) -> Vec<Familie> {
    let Ok(daten) = std::fs::read(datei) else {
        return Vec::new();
    };
    let namen: BTreeSet<String> = FontInfo::iter(&daten).map(|i| i.family).collect();
    let ordner = datei.parent().unwrap_or(Path::new("."));
    let mut schnitte: Vec<_> = typst_kit::fonts::scan(ordner)
        .filter(|(_, i)| namen.contains(&i.family))
        .collect();
    if schnitte.is_empty() {
        // Ordner nicht lesbar: wenigstens die Datei selbst.
        schnitte = FontInfo::iter(&daten)
            .enumerate()
            .map(|(i, info)| {
                (
                    typst_kit::fonts::FontPath {
                        path: datei.to_path_buf(),
                        index: i as u32,
                    },
                    info,
                )
            })
            .collect();
    }
    familien_bilden(schnitte)
}

/// Zeichen, die ein Text in dieser Sprache braucht, über die Buchstaben A–Z hinaus.
pub fn pflichtzeichen(sprache: &str) -> &'static str {
    match sprache {
        "de" => "äöüÄÖÜß„“‚‘’–—…€§°",
        "fr" => "àâæçéèêëîïôœùûüÿÀÂÆÇÉÈÊËÎÏÔŒÙÛÜŸ«»’–—…€",
        "it" => "àèéìíîòóùúÀÈÉÌÒÙ«»’–—…€",
        "es" => "áéíñóúüÁÉÍÑÓÚÜ¿¡«»’–—…€",
        _ => "“”‘’–—…€",
    }
}

/// Zeichen aus A–Z, a–z, 0–9 und `pflichtzeichen(sprache)`, die der Schrift fehlen.
pub fn fehlende_zeichen(info: &FontInfo, sprache: &str) -> Vec<char> {
    ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .chain(pflichtzeichen(sprache).chars())
        .filter(|c| !info.coverage.contains(*c as u32))
        .collect()
}

/// Liest `fsType` aus der OS/2-Tabelle: Die Lizenz verbietet das Einbetten (Bit 1, „Restricted
/// License“) oder erlaubt nur Bitmaps (Bit 9). Ohne OS/2-Tabelle gilt Einbetten als erlaubt.
pub fn einbettung_gesperrt(daten: &[u8], index: u32) -> bool {
    fn u16_an(d: &[u8], i: usize) -> Option<u16> {
        Some(u16::from_be_bytes(d.get(i..i + 2)?.try_into().ok()?))
    }
    fn u32_an(d: &[u8], i: usize) -> Option<u32> {
        Some(u32::from_be_bytes(d.get(i..i + 4)?.try_into().ok()?))
    }
    let fs_type = (|| {
        let anfang = if daten.get(0..4)? == b"ttcf" {
            u32_an(daten, 12 + 4 * index as usize)? as usize
        } else {
            0
        };
        let tabellen = u16_an(daten, anfang + 4)? as usize;
        (0..tabellen).find_map(|t| {
            let eintrag = anfang + 12 + 16 * t;
            (daten.get(eintrag..eintrag + 4)? == b"OS/2")
                .then(|| u32_an(daten, eintrag + 8))
                .flatten()
                .and_then(|os2| u16_an(daten, os2 as usize + 8))
        })
    })();
    fs_type.is_some_and(|f| f & 0x000F == 0x0002 || f & 0x0200 != 0)
}

/// Warum eine Familie nicht eingebunden wurde.
#[derive(Debug)]
pub enum EinbindeFehler {
    /// Die Lizenz der Schrift verbietet das Einbetten in PDFs.
    Gesperrt(String),
    Datei(PathBuf, std::io::Error),
}

impl std::fmt::Display for EinbindeFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gesperrt(name) => f.write_str(&tr!(
                "Die Lizenz der Schrift „{name}“ verbietet das Einbetten in PDFs. TFTN bindet sie deshalb nicht ein.",
                name = name
            )),
            Self::Datei(p, e) => f.write_str(&tr!(
                "{datei} lässt sich nicht kopieren: {fehler}",
                datei = p.display(),
                fehler = e
            )),
        }
    }
}

impl std::error::Error for EinbindeFehler {}

/// Kopiert alle Dateien der Familie nach `fonts/` im Projekt. Liefert die neu kopierten
/// Dateinamen. Verbietet die Lizenz das Einbetten, wird nichts kopiert.
pub fn familie_einbinden(
    projekt: &Projekt,
    familie: &Familie,
) -> Result<Vec<String>, EinbindeFehler> {
    for datei in &familie.dateien {
        let daten = std::fs::read(datei).map_err(|e| EinbindeFehler::Datei(datei.clone(), e))?;
        let anzahl = Font::iter(Bytes::new(daten.clone())).count().max(1) as u32;
        let gesperrt = (0..anzahl).any(|i| {
            FontInfo::new(&daten, i).is_some_and(|info| info.family == familie.name)
                && einbettung_gesperrt(&daten, i)
        });
        if gesperrt {
            return Err(EinbindeFehler::Gesperrt(familie.name.clone()));
        }
    }
    let ziel = projekt.schriften();
    let mut neu = Vec::new();
    for datei in &familie.dateien {
        neu.extend(kopieren(datei, &ziel).map_err(|e| EinbindeFehler::Datei(datei.clone(), e))?);
    }
    Ok(neu)
}

// --- Fehlende Zeichen im Dokument -------------------------------------------------------------

/// Ein Zeichen, das nicht aus der Schrift seiner Umgebung kommt.
#[derive(Debug, Clone, PartialEq)]
pub struct Ersatz {
    pub zeichen: char,
    /// Die Schrift der Umgebung, der das Zeichen fehlt.
    pub schrift: String,
    /// Die Schrift, aus der es stattdessen kommt; `None`: keine Schrift hat es (leeres Kästchen).
    pub ersatz: Option<String>,
    /// Erste Stelle im Quelltext: Knoten und Byte darin.
    pub span: Span,
    pub versatz: u16,
}

/// Höchstens so viele verschiedene Zeichen werden gemeldet.
pub const ERSATZ_HOECHSTENS: usize = 12;

/// Zeichen, die Typst aus einer Ersatzschrift nimmt oder gar nicht hat. Die Schrift der Umgebung
/// ist die mit den meisten Zeichen im selben Rahmen; ein Lauf in einer anderen Schrift zählt nur,
/// wenn ihr dessen Zeichen fehlen (Code in Monospace oder eine Überschrift in einer anderen
/// Schrift sind kein Ersatz). Jedes Zeichen wird einmal gemeldet, an seiner ersten Stelle.
///
/// `formel` sagt, ob eine Stelle in einer Formel liegt: Dort kommen Zeichen wie 𝑎 absichtlich aus
/// der Formelschrift; gemeldet werden nur leere Kästchen, und die Formel zählt nicht zur Umgebung.
pub fn ersatzzeichen(doc: &PagedDocument, formel: &dyn Fn(Span) -> bool) -> Vec<Ersatz> {
    let mut aus: Vec<Ersatz> = Vec::new();
    for seite in doc.pages() {
        rahmen_pruefen(&seite.frame, None, formel, &mut aus);
    }
    aus
}

/// Ein Lauf gehört zu einer Formel, wenn seine Stelle in `$…$` liegt. Bei Akzenten wie `hat(beta)`
/// setzt Typst die Basis ohne Stelle (losgelöste Spanne); dann zählt ein Lauf in einer Formelschrift
/// ebenfalls als Formel. Ohne Stelle in einer gewöhnlichen Schrift bleibt es Text.
fn in_formel(t: &typst::text::TextItem, formel: &dyn Fn(Span) -> bool) -> bool {
    t.glyphs.first().is_some_and(|g| {
        formel(g.span.0)
            || (g.span.0.is_detached()
                && t.font
                    .font()
                    .info()
                    .flags
                    .contains(typst::text::FontFlags::MATH))
    })
}

fn rahmen_pruefen(
    frame: &Frame,
    aussen: Option<&Font>,
    formel: &dyn Fn(Span) -> bool,
    aus: &mut Vec<Ersatz>,
) {
    // Herrschende Schrift dieses Rahmens: die meisten Glyphen; ohne Text die von außen.
    let mut zaehler: Vec<(&Font, usize)> = Vec::new();
    for (_, item) in frame.items() {
        if let FrameItem::Text(t) = item
            && !in_formel(t, formel)
        {
            let font = t.font.font();
            match zaehler
                .iter_mut()
                .find(|(f, _)| f.info().family == font.info().family)
            {
                Some((_, n)) => *n += t.glyphs.len(),
                None => zaehler.push((font, t.glyphs.len())),
            }
        }
    }
    let herrschend = zaehler
        .iter()
        .max_by_key(|(_, n)| *n)
        .map(|(f, _)| *f)
        .or(aussen);
    for (_, item) in frame.items() {
        match item {
            FrameItem::Group(g) => rahmen_pruefen(&g.frame, herrschend, formel, aus),
            FrameItem::Text(t) => {
                if aus.len() >= ERSATZ_HOECHSTENS {
                    return;
                }
                let font = t.font.font();
                let mathe = in_formel(t, formel);
                for g in &t.glyphs {
                    let Some(zeichen) = t.text.get(g.range()).and_then(|s| s.chars().next()) else {
                        continue;
                    };
                    if zeichen.is_whitespace() || zeichen.is_control() {
                        continue;
                    }
                    let fall = if g.id == 0 {
                        Some((herrschend.unwrap_or(font).info().family.clone(), None))
                    } else if mathe {
                        None
                    } else {
                        herrschend
                            .filter(|h| h.info().family != font.info().family)
                            .filter(|h| !h.info().coverage.contains(zeichen as u32))
                            .map(|h| (h.info().family.clone(), Some(font.info().family.clone())))
                    };
                    if let Some((schrift, ersatz)) = fall
                        && !aus.iter().any(|e| e.zeichen == zeichen)
                        && aus.len() < ERSATZ_HOECHSTENS
                    {
                        aus.push(Ersatz {
                            zeichen,
                            schrift,
                            ersatz,
                            span: g.span.0,
                            versatz: g.span.1,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}
