//! Weitere Schriften fürs Dokument: eine installierte Schrift oder eine Schriftdatei, als ganze
//! Familie nach `fonts/` kopiert (`tftn_core::schriften::familie_einbinden`). Systemschriften
//! kommen nie direkt in den Schriftpfad; so sehen Vorschau, typst-CLI und Gegenprobe-Zeile dieselben
//! Dateien, auch auf einem anderen Rechner.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;
use tftn_core::schriften::{self, Familie};
use tftn_sprachen::tr;
use typst::World as _;

use crate::Zustand;

/// Die Systemschriften, einmal je Lauf der App gesucht (das Lesen dauert einen Moment).
static SYSTEM: Mutex<Option<Vec<Familie>>> = Mutex::new(None);

#[derive(Serialize)]
pub struct FamilieJs {
    name: String,
    schnitte: usize,
    /// Byte
    groesse: u64,
    /// Zeichen, die das Dokument in seiner Sprache braucht und die der Schrift fehlen.
    fehlend: String,
    /// Das Dokument kennt die Familie schon (eingebunden oder aus dem Vorrat).
    vorhanden: bool,
    /// Schriftdatei, aus der die Familie kommt; `None`: installiert.
    datei: Option<String>,
}

/// Sprache des Dokuments und die Familien, die es schon kennt.
fn dokument(zustand: &Zustand) -> Result<(String, BTreeSet<String>), String> {
    let sprache = zustand
        .sitzung
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| crate::reiter::haupt_spiegel(s).quelle().clone())
        .ok_or_else(|| tr!("kein Dokument geöffnet"))?;
    let sprache = tftn_edit::seite_lesen(&sprache)
        .sprache
        .unwrap_or_else(|| "en".into());
    let vorhanden = zustand
        .vorschau
        .lock()
        .unwrap()
        .clone()
        .map(|v| {
            v.lock()
                .unwrap()
                .world()
                .book()
                .families()
                .map(|(n, _)| n.to_string())
                .collect()
        })
        .unwrap_or_default();
    Ok((sprache, vorhanden))
}

fn js(f: &Familie, sprache: &str, vorhanden: &BTreeSet<String>, datei: Option<&str>) -> FamilieJs {
    FamilieJs {
        name: f.name.clone(),
        schnitte: f.schnitte,
        groesse: f.groesse(),
        fehlend: schriften::fehlende_zeichen(&f.info, sprache)
            .into_iter()
            .collect(),
        vorhanden: vorhanden.contains(&f.name),
        datei: datei.map(String::from),
    }
}

/// Die auf diesem Rechner installierten Schriftfamilien; `neu` sucht noch einmal.
#[tauri::command(async)]
pub fn schriften_suchen(
    neu: Option<bool>,
    zustand: State<'_, Zustand>,
) -> Result<Vec<FamilieJs>, String> {
    let (sprache, vorhanden) = dokument(&zustand)?;
    let mut gemerkt = SYSTEM.lock().unwrap();
    if neu == Some(true) {
        *gemerkt = None;
    }
    let liste = gemerkt.get_or_insert_with(schriften::systemfamilien);
    Ok(liste
        .iter()
        .map(|f| js(f, &sprache, &vorhanden, None))
        .collect())
}

/// Die Familien in einer Schriftdatei (samt Schnitten aus demselben Ordner).
#[tauri::command(async)]
pub fn schriftdatei_lesen(
    pfad: String,
    zustand: State<'_, Zustand>,
) -> Result<Vec<FamilieJs>, String> {
    let (sprache, vorhanden) = dokument(&zustand)?;
    let familien = schriften::dateifamilien(Path::new(&pfad));
    if familien.is_empty() {
        return Err(tr!(
            "{datei} ist keine lesbare Schriftdatei (.otf, .ttf, .otc, .ttc).",
            datei = pfad
        ));
    }
    Ok(familien
        .iter()
        .map(|f| js(f, &sprache, &vorhanden, Some(&pfad)))
        .collect())
}

#[derive(Serialize)]
pub struct EingebundenJs {
    /// Neu nach `fonts/` kopierte Dateien.
    dateien: Vec<String>,
    /// Byte
    groesse: u64,
}

/// Kopiert die Familie `name` (installiert oder aus `datei`) nach `fonts/`, liest die Schriften der
/// Vorschau neu und übersetzt noch einmal. Die Schrift setzt danach die Oberfläche (Fach „Seite“).
#[tauri::command(async)]
pub fn schrift_hinzufuegen(
    name: String,
    datei: Option<String>,
    zustand: State<'_, Zustand>,
) -> Result<EingebundenJs, String> {
    let familie = match &datei {
        Some(d) => schriften::dateifamilien(Path::new(d))
            .into_iter()
            .find(|f| f.name == name),
        None => SYSTEM
            .lock()
            .unwrap()
            .get_or_insert_with(schriften::systemfamilien)
            .iter()
            .find(|f| f.name == name)
            .cloned(),
    }
    .ok_or_else(|| {
        tr!(
            "Die Schrift „{name}“ ist nicht mehr zu finden.",
            name = name
        )
    })?;

    let sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_ref()
        .ok_or_else(|| tr!("kein Dokument geöffnet"))?;
    let erstmals = !s.projekt.schriften().is_dir();
    let dateien = schriften::familie_einbinden(&s.projekt, &familie).map_err(|e| e.to_string())?;
    let vorschau = zustand.vorschau.lock().unwrap().clone();
    if let Some(v) = &vorschau {
        let mut v = v.lock().unwrap();
        // Ab jetzt hat das Dokument `fonts/`, und die Gegenprobe-Zeile nennt nur noch diesen Ordner:
        // Was es bisher aus dem Vorrat benutzt, kommt gleich mit.
        if erstmals
            && let Some(gut) = v.gut()
            && let Err(e) = schriften::einbinden(&s.projekt, &gut.dokument)
        {
            eprintln!("Bisherige Schriften nicht eingebunden: {e}");
        }
        v.schriften_neu_lesen(&s.projekt);
    }
    tftn_manifest::anlegen::readme_schriften_abgleichen(&s.projekt.wurzel)
        .map_err(|e| e.to_string())?;
    crate::kompilieren_lassen(&zustand, s);
    Ok(EingebundenJs {
        dateien,
        groesse: familie.groesse(),
    })
}
