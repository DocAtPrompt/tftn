//! Schriften einbinden: nur, was das Dokument benutzt (`fonts/` im Projekt).
//!
//! Von Textschriften kommt die ganze Familie mit, damit Fett und Kursiv später weiter gehen. Von
//! Formelschriften (Familie mit „Math“) nur die benutzten Schnitte; sie sind groß und Fett wird
//! selten gebraucht. Die Quelle ist der Vorrat der App (`schriftvorrat_setzen`).

use std::collections::BTreeSet;
use std::path::Path;

use typst::foundations::Bytes;
use typst::layout::{Frame, FrameItem};
use typst::text::{Font, FontVariant};
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
