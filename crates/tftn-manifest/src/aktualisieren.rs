//! Dokumenttyp eines Projekts aktualisieren (Spec 9, PLAN-M2 AP8).
//!
//! Ein Projekt hat eine vendorte Kopie seines Typs (KONZEPT 2.5). Gibt es in den Quellen eine neuere
//! Version, kann sie auf Klick übernommen werden: vendoren (die alte Kopie bleibt liegen), fehlende
//! Schriften kopieren, `tftn.toml` → `typ` umstellen. Die Importstellen (`"@local/<name>:<alt>"`)
//! werden gesucht, aber hier nicht umgeschrieben: Die Hauptdatei ändert nur der Editor.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tftn_sprachen::tr;

use tftn_core::Projekt;
use typst_syntax::{LinkedNode, SyntaxKind};

use crate::anlegen::{dateien, io, kopieren, spec_lesen, vendoren};
use crate::typen::{TypEintrag, Typquelle, verfuegbar};
use crate::{ManifestFehler, Typpaket};

/// Eine Importstelle: Bytes von `@local/<name>:<alt>` (ohne Anführungszeichen) in `datei`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Importstelle {
    /// relativ zum Projekt, mit `/`
    pub datei: String,
    pub von: usize,
    pub bis: usize,
}

#[derive(Debug)]
pub struct Aktualisierung {
    /// `@local/<name>:<version>`
    pub alt: String,
    pub neu: String,
    pub stellen: Vec<Importstelle>,
    /// neu nach `fonts/` kopierte Schriften
    pub schriften: Vec<String>,
}

/// Name und Version des Projekttyps aus `tftn.toml`.
fn projekttyp(projekt: &Projekt) -> Option<(String, semver::Version)> {
    let typ = projekt.konfig.typ.as_deref()?;
    let (name, version) = spec_lesen(typ);
    Some((name.to_string(), semver::Version::parse(version?).ok()?))
}

/// Die neuere Version des Projekttyps in den Quellen, falls es eine gibt. Es zählt der Eintrag,
/// der in der Liste gewinnt (eigener Ordner vor TFTN).
pub fn neuere_version(projekt: &Projekt, quellen: &[Typquelle]) -> Option<TypEintrag> {
    let (name, version) = projekttyp(projekt)?;
    let (liste, _) = verfuegbar(quellen);
    liste
        .into_iter()
        .find(|t| t.name == name)
        .filter(|t| t.version > version)
}

/// Vendort `neu` samt Abhängigkeiten, kopiert fehlende Schriften und stellt `tftn.toml` um.
/// Ändert keine `.typ`-Datei; die Importstellen stehen im Ergebnis.
pub fn typ_aktualisieren(
    projekt: &Projekt,
    neu: &TypEintrag,
    quellen: &[Typquelle],
) -> Result<Aktualisierung, ManifestFehler> {
    let alt = projekt
        .konfig
        .typ
        .clone()
        .ok_or_else(|| ManifestFehler::Inhalt(tr!("tftn.toml nennt keinen Dokumenttyp")))?;
    let paket = Typpaket::lesen(&neu.ordner)?;
    let neu_spec = paket.spec();
    let wurzel = &projekt.wurzel;
    let stellen = importstellen(wurzel, &alt)?;

    vendoren(wurzel, &paket, quellen, BTreeSet::new())?;

    // Schriften: nur fehlende kopieren, vorhandene bleiben unangetastet.
    let mut schriften = Vec::new();
    let manifest = paket.manifest()?;
    if !manifest.schriften.is_empty() {
        let fonts = wurzel.join("fonts");
        std::fs::create_dir_all(&fonts).map_err(io(&fonts))?;
        for s in &manifest.schriften {
            let Some(name) = Path::new(s).file_name() else {
                continue;
            };
            let ziel = fonts.join(name);
            if !ziel.exists() {
                let quelle = paket.ordner.join(s);
                std::fs::copy(&quelle, &ziel).map_err(io(&quelle))?;
                schriften.push(name.to_string_lossy().into_owned());
            }
        }
    }

    // tftn.toml: nur `typ`, Kommentare und übrige Zeilen bleiben.
    let datei = wurzel.join("tftn.toml");
    let text = std::fs::read_to_string(&datei).map_err(io(&datei))?;
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .map_err(|e| ManifestFehler::Format(datei.clone(), format!("{e}")))?;
    match doc.get_mut("typ") {
        Some(toml_edit::Item::Value(v)) => {
            let decor = v.decor().clone();
            *v = toml_edit::Value::from(neu_spec.as_str());
            *v.decor_mut() = decor;
        }
        _ => doc["typ"] = toml_edit::value(neu_spec.as_str()),
    }
    std::fs::write(&datei, doc.to_string()).map_err(io(&datei))?;

    Ok(Aktualisierung {
        alt,
        neu: neu_spec,
        stellen,
        schriften,
    })
}

/// Alle String-Literale `"<spec>"` in `.typ`-Dateien des Projekts (ohne `packages/` und ohne Ordner,
/// die mit `_` oder `.` beginnen). Kommentare zählen nicht.
pub fn importstellen(wurzel: &Path, spec: &str) -> Result<Vec<Importstelle>, ManifestFehler> {
    let mut stellen = Vec::new();
    for datei in dateien(wurzel)? {
        let rel = datei.strip_prefix(wurzel).expect("liegt im Projekt");
        let erster = rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned());
        if datei.extension().is_none_or(|e| e != "typ")
            || erster
                .as_deref()
                .is_some_and(|e| e == "packages" || e.starts_with('_') || e.starts_with('.'))
        {
            continue;
        }
        let text = std::fs::read_to_string(&datei).map_err(io(&datei))?;
        if !text.contains(spec) {
            continue;
        }
        let name: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let quelle = typst_syntax::Source::detached(text);
        sammeln(
            &LinkedNode::new(quelle.root()),
            spec,
            &name.join("/"),
            &mut stellen,
        );
    }
    Ok(stellen)
}

/// Bytebereiche (ohne Anführungszeichen) aller String-Literale `"<spec>"` in `text`, etwa in der
/// Quelle des Editors, die von der Datei auf der Platte abweichen kann.
pub fn stellen_in_text(text: &str, spec: &str) -> Vec<std::ops::Range<usize>> {
    let quelle = typst_syntax::Source::detached(text);
    let mut aus = Vec::new();
    sammeln(&LinkedNode::new(quelle.root()), spec, "", &mut aus);
    aus.into_iter().map(|s| s.von..s.bis).collect()
}

fn sammeln(knoten: &LinkedNode, spec: &str, datei: &str, aus: &mut Vec<Importstelle>) {
    if knoten.kind() == SyntaxKind::Str
        && knoten.get().leaf_text().as_str() == format!("\"{spec}\"")
    {
        let r = knoten.range();
        aus.push(Importstelle {
            datei: datei.to_string(),
            von: r.start + 1,
            bis: r.end - 1,
        });
    }
    for kind in knoten.children() {
        sammeln(&kind, spec, datei, aus);
    }
}

/// Schreibt die Importstellen der Dateien, für die `nur` wahr ist, auf `a.neu` um: alle oder keine.
/// Erst werden alle neuen Texte berechnet und in Temp-Dateien geschrieben, dann getauscht; scheitert
/// ein Tausch, bekommen die schon getauschten Dateien ihren alten Text zurück.
pub fn stellen_umschreiben(
    wurzel: &Path,
    a: &Aktualisierung,
    nur: impl Fn(&str) -> bool,
) -> Result<(), ManifestFehler> {
    let mut je_datei: BTreeMap<&str, Vec<&Importstelle>> = BTreeMap::new();
    for s in &a.stellen {
        je_datei.entry(s.datei.as_str()).or_default().push(s);
    }
    struct Neu {
        pfad: std::path::PathBuf,
        temp: std::path::PathBuf,
        alt: String,
        neu: String,
    }
    let mut dateien = Vec::new();
    for (datei, mut stellen) in je_datei {
        if !nur(datei) {
            continue;
        }
        let pfad = wurzel.join(datei);
        let alt = std::fs::read_to_string(&pfad).map_err(io(&pfad))?;
        let mut neu = alt.clone();
        stellen.sort_by_key(|s| std::cmp::Reverse(s.von));
        for s in stellen {
            if neu.get(s.von..s.bis) != Some(a.alt.as_str()) {
                return Err(ManifestFehler::Inhalt(tr!(
                    "{datei} hat sich geändert; bitte neu vergleichen",
                    datei
                )));
            }
            neu.replace_range(s.von..s.bis, &a.neu);
        }
        let name = pfad
            .file_name()
            .map_or_else(Default::default, |n| n.to_string_lossy().into_owned());
        let temp = pfad.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
        dateien.push(Neu {
            pfad,
            temp,
            alt,
            neu,
        });
    }
    let aufraeumen = |dateien: &[Neu]| {
        for d in dateien {
            let _ = std::fs::remove_file(&d.temp);
        }
    };
    for d in &dateien {
        if let Err(e) = std::fs::write(&d.temp, &d.neu) {
            aufraeumen(&dateien);
            return Err(io(&d.temp)(e));
        }
    }
    for (i, d) in dateien.iter().enumerate() {
        if let Err(e) = tftn_pakete::umbenennen(&d.temp, &d.pfad) {
            for frueher in &dateien[..i] {
                let _ = std::fs::write(&frueher.pfad, &frueher.alt);
            }
            aufraeumen(&dateien);
            return Err(io(&d.pfad)(e));
        }
    }
    Ok(())
}

/// Kopie des Projekts in `ziel` für die Probe: ohne Ordner, die mit `_` oder `.` beginnen, und ohne
/// `export/` (Erzeugtes und Abgenommenes gehört nicht in die Probe).
pub fn probekopie(projekt: &Projekt, ziel: &Path) -> Result<Projekt, ManifestFehler> {
    let mut aus = BTreeSet::new();
    let eintraege = std::fs::read_dir(&projekt.wurzel).map_err(io(&projekt.wurzel))?;
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('_') || name.starts_with('.') || name == "export" {
            aus.insert(std::path::PathBuf::from(name));
        }
    }
    kopieren(&projekt.wurzel, ziel, &aus)?;
    Projekt::oeffnen(ziel).map_err(|e| ManifestFehler::Inhalt(e.to_string()))
}
