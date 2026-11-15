//! „Eigener Dokumenttyp …“: Ein vorhandener Typ (mitgeliefert oder eigen) wird unter eigenem Namen
//! in den Ordner für eigene Typen kopiert. Danach ist er ein ganz gewöhnlicher Typ: Vorgaben in
//! `vorlage/dokument.toml`, Layout in `lib.typ`, wählbar über „Neu“ → „Aus Vorlage …“.
//!
//! Die Kopie ist eigenständig: Schriften, die der Vorbild-Typ aus Nachbarordnern holt (`../…`),
//! liegen danach in ihrem eigenen `schriften/`, und die Vorlagen importieren den neuen Namen.

use std::path::{Path, PathBuf};

use tftn_sprachen::tr;

use crate::typen::{Herkunft, Typquelle, finden_in};

/// Paketname aus dem Titel: klein, Umlaute ausgeschrieben, sonst nur `a-z`, `0-9` und `-`.
/// `None`, wenn nichts übrig bleibt.
pub fn kennung_aus_titel(titel: &str) -> Option<String> {
    let mut k = String::new();
    for c in titel.trim().to_lowercase().chars() {
        match c {
            'ä' => k.push_str("ae"),
            'ö' => k.push_str("oe"),
            'ü' => k.push_str("ue"),
            'ß' => k.push_str("ss"),
            c if c.is_ascii_alphanumeric() => k.push(c),
            _ if !k.ends_with('-') && !k.is_empty() => k.push('-'),
            _ => {}
        }
    }
    let k = k.trim_end_matches('-').to_string();
    if k.is_empty() {
        None
    } else if k.starts_with(|c: char| c.is_ascii_digit()) {
        Some(format!("eigen-{k}"))
    } else {
        Some(k)
    }
}

/// Kopiert den Typ `basis` als neuen Typ `titel` in den eigenen Ordner (die Quelle mit
/// `Herkunft::Eigen`) und gibt dessen Ordner zurück. Version des neuen Typs: 0.1.0.
pub fn eigenen_typ_anlegen(
    quellen: &[Typquelle],
    basis: &str,
    titel: &str,
) -> Result<PathBuf, String> {
    let titel = titel.trim();
    let kennung = kennung_aus_titel(titel)
        .ok_or_else(|| tr!("Bitte einen Namen für den neuen Dokumenttyp angeben."))?;
    let eigen = quellen
        .iter()
        .find(|q| q.herkunft == Herkunft::Eigen)
        .map(|q| q.ordner.clone())
        .ok_or_else(|| tr!("Kein Ordner für eigene Dokumenttypen."))?;
    let ziel = eigen.join(&kennung);
    if ziel.exists() || finden_in(quellen, &kennung, None).is_ok() {
        return Err(tr!(
            "Einen Dokumenttyp {kennung} gibt es schon – bitte einen anderen Namen wählen.",
            kennung
        ));
    }
    let vorbild = finden_in(quellen, basis, None)
        .map_err(|_| tr!("Dokumenttyp {basis} nicht gefunden.", basis))?;
    if vorbild.toml.tool.tftn.is_none() || vorbild.toml.template.is_none() {
        return Err(tr!("{basis} ist kein wählbarer Dokumenttyp.", basis));
    }
    let ergebnis = kopie_anpassen(&vorbild.ordner, &ziel, &kennung, titel, &vorbild);
    if ergebnis.is_err() {
        let _ = std::fs::remove_dir_all(&ziel);
    }
    ergebnis.map(|()| ziel)
}

fn kopie_anpassen(
    von: &Path,
    ziel: &Path,
    kennung: &str,
    titel: &str,
    vorbild: &crate::Typpaket,
) -> Result<(), String> {
    let e = |e: std::io::Error| e.to_string();
    ordner_kopieren(von, ziel).map_err(e)?;

    // Manifest mit toml_edit: Kommentare und Reihenfolge bleiben.
    let pfad = ziel.join("typst.toml");
    let mut doc: toml_edit::DocumentMut = std::fs::read_to_string(&pfad)
        .map_err(e)?
        .parse()
        .map_err(|f: toml_edit::TomlError| f.to_string())?;
    doc["package"]["name"] = toml_edit::value(kennung);
    doc["package"]["version"] = toml_edit::value("0.1.0");
    doc["package"]["description"] = toml_edit::value(titel);
    doc["tool"]["tftn"]["titel"] = toml_edit::value(titel);
    if let Some(liste) = doc["tool"]["tftn"]["schriften"].as_array_mut() {
        for wert in liste.iter_mut() {
            let Some(s) = wert.as_str() else { continue };
            if !s.starts_with("../") {
                continue;
            }
            let quelle = von.join(s);
            let name = quelle
                .file_name()
                .ok_or_else(|| tr!("Schrift {pfad} ohne Dateinamen", pfad = s))?
                .to_string_lossy()
                .into_owned();
            std::fs::create_dir_all(ziel.join("schriften")).map_err(e)?;
            std::fs::copy(&quelle, ziel.join("schriften").join(&name))
                .map_err(|f| format!("{}: {f}", quelle.display()))?;
            let neu = toml_edit::Value::from(format!("schriften/{name}"));
            *wert = neu.decorated(
                wert.decor()
                    .prefix()
                    .and_then(|p| p.as_str())
                    .unwrap_or(" "),
                "",
            );
        }
    }
    std::fs::write(&pfad, doc.to_string()).map_err(e)?;

    // Vorlagen importieren den neuen Typ statt des Vorbilds.
    let alt = format!(
        "@local/{}:{}",
        vorbild.toml.package.name, vorbild.toml.package.version
    );
    let neu = format!("@local/{kennung}:0.1.0");
    let mut ordner = vec![
        vorbild
            .toml
            .template
            .as_ref()
            .map(|t| t.path.clone())
            .unwrap_or_default(),
    ];
    if let Some(m) = &vorbild.toml.tool.tftn {
        ordner.extend(m.vorlagen.values().cloned());
    }
    for o in ordner {
        importe_ersetzen(&ziel.join(o), &alt, &neu).map_err(e)?;
    }
    Ok(())
}

fn ordner_kopieren(von: &Path, nach: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(nach)?;
    for eintrag in std::fs::read_dir(von)? {
        let eintrag = eintrag?;
        let ziel = nach.join(eintrag.file_name());
        if eintrag.file_type()?.is_dir() {
            ordner_kopieren(&eintrag.path(), &ziel)?;
        } else {
            std::fs::copy(eintrag.path(), ziel)?;
        }
    }
    Ok(())
}

fn importe_ersetzen(ordner: &Path, alt: &str, neu: &str) -> std::io::Result<()> {
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return Ok(());
    };
    for eintrag in eintraege {
        let p = eintrag?.path();
        if p.is_dir() {
            importe_ersetzen(&p, alt, neu)?;
        } else if p.extension().is_some_and(|x| x == "typ") {
            let text = std::fs::read_to_string(&p)?;
            if text.contains(alt) {
                std::fs::write(&p, text.replace(alt, neu))?;
            }
        }
    }
    Ok(())
}
