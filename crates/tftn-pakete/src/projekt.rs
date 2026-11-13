//! Übernahme geprüfter Pakete aus dem Zwischenspeicher ins Projekt (Entwurf 8.1, 8.2).
//!
//! Die Funktionen nehmen die Projektwurzel statt `tftn_core::Projekt`, damit `tftn-holer` dieses
//! Crate ohne typst nutzen kann. Prüfsummen stehen in `tftn.toml` unter `[pakete]`
//! (`"@preview/<name>:<version>" = "sha256:<hex>"`).

use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::Serialize;

use crate::{PaketFehler, PaketSpec, Zwischenspeicher};

/// Höchstgröße einer README im Fach.
pub const README_MAX: usize = 200 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ProjektPaket {
    pub spec: String,
    pub name: String,
    pub version: String,
    /// Eintrag aus `[pakete]` in `tftn.toml`
    pub pruefsumme: Option<String>,
    pub readme: bool,
}

#[derive(Debug, Default)]
pub struct Uebernahme {
    pub kopiert: Vec<PaketSpec>,
    pub schon_da: Vec<PaketSpec>,
}

fn io(p: &Path) -> impl Fn(std::io::Error) -> PaketFehler + '_ {
    move |e| PaketFehler::Io(p.to_path_buf(), e)
}

fn paketordner(wurzel: &Path, s: &PaketSpec) -> PathBuf {
    wurzel
        .join("packages/preview")
        .join(&s.name)
        .join(s.version.to_string())
}

/// `tftn.toml` als Dokument (Kommentare und Formatierung bleiben).
fn tftn_toml(wurzel: &Path) -> Result<(PathBuf, String, toml_edit::DocumentMut), PaketFehler> {
    let pfad = wurzel.join("tftn.toml");
    let text = std::fs::read_to_string(&pfad).map_err(io(&pfad))?;
    let doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| PaketFehler::Projekt(format!("{}: {e}", pfad.display())))?;
    Ok((pfad, text, doc))
}

/// Hinzufügen und Entfernen lesen `tftn.toml`, ändern sie und schreiben sie zurück. Zwei Fenster
/// derselben App dürfen das nicht verschränkt tun, sonst geht ein `[pakete]`-Eintrag verloren.
fn sperre() -> std::sync::MutexGuard<'static, ()> {
    static SPERRE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SPERRE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn vermerkt(doc: &toml_edit::DocumentMut, s: &PaketSpec) -> Option<String> {
    doc.get("pakete")?
        .get(s.to_string())?
        .as_str()
        .map(str::to_string)
}

/// Prüft alle SHA gegen `tftn.toml` (Abweichung → `Err`, nichts geändert), kopiert dann fehlende
/// Ordner nach `packages/preview/<n>/<v>/` und trägt neue SHA in `[pakete]` ein.
pub fn uebernehmen(
    wurzel: &Path,
    zs: &Zwischenspeicher,
    pakete: &[(PaketSpec, String)],
) -> Result<Uebernahme, PaketFehler> {
    let _sperre = sperre();
    let (pfad, text, mut doc) = tftn_toml(wurzel)?;
    let mut quellen = Vec::new();
    for (s, sha) in pakete {
        let gefunden = format!("sha256:{sha}");
        if let Some(alt) = vermerkt(&doc, s)
            && alt != gefunden
        {
            return Err(PaketFehler::Pruefsumme {
                spec: s.to_string(),
                vermerkt: alt,
                gefunden,
            });
        }
        let quelle = zs
            .ordner(s)
            .filter(|_| zs.sha256(s).as_deref() == Some(sha.as_str()))
            .ok_or_else(|| {
                PaketFehler::Projekt(tr!(
                    "{paket} liegt nicht geprüft im Zwischenspeicher",
                    paket = s
                ))
            })?;
        quellen.push(quelle);
    }

    let mut u = Uebernahme::default();
    let mut angelegt: Vec<PathBuf> = Vec::new();
    let kopiert = (|| {
        for ((s, _), quelle) in pakete.iter().zip(&quellen) {
            let ziel = paketordner(wurzel, s);
            if ziel.exists() {
                u.schon_da.push(s.clone());
                continue;
            }
            let eltern = ziel.parent().expect("hat Eltern");
            std::fs::create_dir_all(eltern).map_err(io(eltern))?;
            let temp = eltern.join(format!(".tmp-{}-{}", s.version, std::process::id()));
            let _ = std::fs::remove_dir_all(&temp);
            let r = kopieren(quelle, &temp)
                .and_then(|()| crate::umbenennen(&temp, &ziel).map_err(io(&ziel)));
            if r.is_err() {
                let _ = std::fs::remove_dir_all(&temp);
            }
            r?;
            angelegt.push(ziel);
            u.kopiert.push(s.clone());
        }
        Ok::<(), PaketFehler>(())
    })();
    if let Err(e) = kopiert {
        rueckbau(&angelegt);
        return Err(e);
    }

    let mut neu = false;
    for (s, sha) in pakete {
        if vermerkt(&doc, s).is_none() {
            let tabelle = doc
                .entry("pakete")
                .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
                .as_table_like_mut() // auch `pakete = {}`
                .ok_or_else(|| {
                    PaketFehler::Projekt(tr!("[pakete] in tftn.toml ist keine Tabelle"))
                })?;
            tabelle.insert(&s.to_string(), toml_edit::value(format!("sha256:{sha}")));
            neu = true;
        }
    }
    if neu {
        // Scheitert das Schreiben, kommen die eben kopierten Ordner wieder weg: Ohne Prüfsumme
        // ließen sie sich über die Oberfläche nie nachtragen (Abschlussprüfung AP9).
        if let Err(e) = toml_schreiben(wurzel, &pfad, &text, &doc) {
            rueckbau(&angelegt);
            return Err(e);
        }
    }
    Ok(u)
}

/// Entfernt die eben angelegten Paketordner und danach leer gewordene Namensordner.
fn rueckbau(angelegt: &[PathBuf]) {
    for o in angelegt {
        let _ = std::fs::remove_dir_all(o);
        if let Some(name) = o.parent() {
            let _ = std::fs::remove_dir(name);
        }
    }
}

fn kopieren(von: &Path, nach: &Path) -> Result<(), PaketFehler> {
    std::fs::create_dir_all(nach).map_err(io(nach))?;
    for e in std::fs::read_dir(von).map_err(io(von))? {
        let e = e.map_err(io(von))?;
        let art = e.file_type().map_err(io(&e.path()))?;
        let ziel = nach.join(e.file_name());
        if art.is_dir() {
            kopieren(&e.path(), &ziel)?;
        } else if art.is_file() {
            std::fs::copy(e.path(), &ziel).map_err(io(&ziel))?;
        }
    }
    Ok(())
}

/// Wer `spec` importiert: Projektdateien (relativ, mit `/`) und andere Pakete (`@preview/x:1.0.0`).
pub fn benutzer(wurzel: &Path, spec: &PaketSpec) -> Vec<String> {
    let mut aus: Vec<String> = crate::importe::projekt_dateien(wurzel)
        .iter()
        .filter(|d| crate::importe::importe_der_datei(d).contains(spec))
        .map(|d| {
            let rel = d.strip_prefix(wurzel).unwrap_or(d);
            let teile: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            teile.join("/")
        })
        .collect();
    for p in im_projekt(wurzel) {
        if p.spec == spec.to_string() {
            continue;
        }
        let ordner = wurzel
            .join("packages/preview")
            .join(&p.name)
            .join(&p.version);
        if crate::importe(&ordner).is_ok_and(|(i, _)| i.contains(spec)) {
            aus.push(p.spec);
        }
    }
    aus
}

/// Entfernt `spec` und danach seine Abhängigkeiten, die dadurch niemand mehr braucht (Entwurf 6,
/// PLAN-M2 AP10). Abgelehnt, solange jemand `spec` importiert. Die Einträge in `[pakete]` werden
/// erst nach den Ordnern angepasst, auch wenn unterwegs ein Ordner nicht wegging.
pub fn entfernen(wurzel: &Path, spec: &PaketSpec) -> Result<Vec<PaketSpec>, PaketFehler> {
    let _sperre = sperre();
    let (pfad, text, mut doc) = tftn_toml(wurzel)?;
    if !paketordner(wurzel, spec).is_dir() {
        return Err(PaketFehler::Projekt(tr!(
            "{spec} liegt nicht im Projekt",
            spec
        )));
    }
    let wer = benutzer(wurzel, spec);
    if !wer.is_empty() {
        return Err(PaketFehler::Projekt(tr!(
            "{spec} wird noch importiert von: {wer}",
            spec,
            wer = wer.join(", ")
        )));
    }
    let mut weg: Vec<PaketSpec> = Vec::new();
    let mut stapel = vec![spec.clone()];
    let mut ergebnis = Ok(());
    while let Some(s) = stapel.pop() {
        let ordner = paketordner(wurzel, &s);
        if weg.contains(&s) || !ordner.is_dir() || (s != *spec && !benutzer(wurzel, &s).is_empty())
        {
            continue;
        }
        let abhaengig: Vec<PaketSpec> = crate::importe(&ordner)
            .map(|(i, _)| i.into_iter().collect())
            .unwrap_or_default();
        let geloescht = crate::wiederholen(10, std::time::Duration::from_millis(100), || {
            std::fs::remove_dir_all(&ordner)
        });
        if let Err(e) = geloescht {
            ergebnis = Err(PaketFehler::Io(ordner, e));
            break;
        }
        if let Some(name) = ordner.parent() {
            let _ = std::fs::remove_dir(name);
        }
        weg.push(s);
        stapel.extend(abhaengig.into_iter().rev());
    }
    let mut geaendert = false;
    if let Some(t) = doc.get_mut("pakete").and_then(|t| t.as_table_like_mut()) {
        for s in &weg {
            geaendert |= t.remove(&s.to_string()).is_some();
        }
    }
    if geaendert {
        toml_schreiben(wurzel, &pfad, &text, &doc)?;
    }
    ergebnis.map(|()| weg)
}

/// Schreibt `doc` über eine temporäre Datei; CRLF bleibt.
fn toml_schreiben(
    wurzel: &Path,
    pfad: &Path,
    text: &str,
    doc: &toml_edit::DocumentMut,
) -> Result<(), PaketFehler> {
    let mut aus = doc.to_string();
    if text.contains("\r\n") {
        aus = aus.replace("\r\n", "\n").replace('\n', "\r\n");
    }
    let temp = wurzel.join(format!(".tftn.toml.{}.tmp", std::process::id()));
    let geschrieben = std::fs::write(&temp, aus)
        .map_err(io(&temp))
        .and_then(|()| crate::umbenennen(&temp, pfad).map_err(io(pfad)));
    if geschrieben.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    geschrieben
}

/// Universe-Pakete unter `packages/preview/`, sortiert nach Name und Version.
pub fn im_projekt(wurzel: &Path) -> Vec<ProjektPaket> {
    let doc = tftn_toml(wurzel).ok().map(|(_, _, d)| d);
    let mut aus = Vec::new();
    let Ok(namen) = std::fs::read_dir(wurzel.join("packages/preview")) else {
        return Vec::new();
    };
    for name in namen.filter_map(Result::ok) {
        let Ok(versionen) = std::fs::read_dir(name.path()) else {
            continue;
        };
        for v in versionen.filter_map(Result::ok) {
            let (n, ver) = (
                name.file_name().to_string_lossy().into_owned(),
                v.file_name().to_string_lossy().into_owned(),
            );
            let Ok(s) = PaketSpec::neu(&n, &ver) else {
                continue;
            };
            if !v.path().join("typst.toml").is_file() {
                continue;
            }
            aus.push((
                s.clone(),
                ProjektPaket {
                    spec: s.to_string(),
                    name: n,
                    version: ver,
                    pruefsumme: doc.as_ref().and_then(|d| vermerkt(d, &s)),
                    readme: readme_datei(&v.path()).is_some(),
                },
            ));
        }
    }
    aus.sort_by(|a, b| a.0.cmp(&b.0));
    aus.into_iter().map(|(_, p)| p).collect()
}

fn readme_datei(ordner: &Path) -> Option<PathBuf> {
    ["README.md", "readme.md", "Readme.md"]
        .iter()
        .map(|n| ordner.join(n))
        .find(|p| p.is_file())
}

/// README des Pakets (Projekt vor Zwischenspeicher), höchstens 200 KB.
pub fn readme(wurzel: &Path, zs: &Zwischenspeicher, s: &PaketSpec) -> Option<String> {
    let ordner = Some(paketordner(wurzel, s))
        .filter(|o| o.is_dir())
        .or_else(|| zs.ordner(s))?;
    let daten = std::fs::read(readme_datei(&ordner)?).ok()?;
    if daten.len() > README_MAX {
        let mut text = String::from_utf8_lossy(&daten[..README_MAX]).into_owned();
        text.push_str(&tr!("\n\n… (gekürzt)"));
        Some(text)
    } else {
        Some(String::from_utf8_lossy(&daten).into_owned())
    }
}
