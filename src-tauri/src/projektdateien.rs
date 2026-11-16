//! Dateien im Projektordner für MCP: auflisten, lesen, schreiben.
//!
//! Die Grenze ist der Projektordner. Nicht erreichbar sind versteckte Ordner und Dateien (`.git`,
//! `.fassungen`), `_`-Ordner (`_lokal`) und `packages/` (vendorte Pakete, nur über TFTN). Geschrieben
//! werden nur Textdateien; die Hauptdatei geht über den Editor, `tftn.toml` gar nicht, die
//! Datendatei über das Formular. Vor dem Überschreiben kommt der alte Inhalt nach
//! `.fassungen/dateien/`, weil Rückgängig im Editor nur für die Hauptdatei gilt.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;

/// Endungen, die gelesen und geschrieben werden dürfen.
const TEXT: [&str; 8] = ["typ", "txt", "bib", "yaml", "yml", "md", "csv", "json"];
/// Nur lesen.
const NUR_LESEN: [&str; 1] = ["toml"];
/// Höchstens so viele Bytes werden gelesen oder geschrieben.
pub const GRENZE: usize = 1_000_000;
/// Sicherungen vor dem Überschreiben, unter `.fassungen/`.
pub const SICHERUNGEN: &str = ".fassungen/dateien";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Eintrag {
    /// relativ zum Projektordner, mit `/`
    pub pfad: String,
    pub bytes: u64,
    /// lesbar (Text) …
    pub text: bool,
    /// … und schreibbar
    pub schreibbar: bool,
}

fn endung(pfad: &Path) -> String {
    pfad.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn ist_text(pfad: &Path) -> bool {
    let e = endung(pfad);
    TEXT.contains(&e.as_str()) || NUR_LESEN.contains(&e.as_str())
}

fn ausgeschlossen(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('_')
}

/// Alle erreichbaren Dateien, sortiert (höchstens 4 Ebenen tief und 500 Einträge).
pub fn auflisten(wurzel: &Path) -> Vec<Eintrag> {
    let mut aus = Vec::new();
    sammeln(wurzel, wurzel, 4, &mut aus);
    aus.sort_by(|a, b| a.pfad.cmp(&b.pfad));
    aus.truncate(500);
    aus
}

fn sammeln(wurzel: &Path, ordner: &Path, tiefe: usize, aus: &mut Vec<Eintrag>) {
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return;
    };
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        if ausgeschlossen(&name) || (ordner == wurzel && name == "packages") {
            continue;
        }
        let Ok(art) = e.file_type() else { continue };
        let p = e.path();
        if art.is_dir() {
            if tiefe > 1 {
                sammeln(wurzel, &p, tiefe - 1, aus);
            }
        } else if art.is_file() {
            let rel = p.strip_prefix(wurzel).unwrap_or(&p);
            let text = ist_text(&p);
            aus.push(Eintrag {
                pfad: rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
                bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                text,
                schreibbar: text && TEXT.contains(&endung(&p).as_str()),
            });
        }
    }
}

/// Prüft einen relativen Pfad und liefert den absoluten. Kein `..`, nichts Absolutes, nichts
/// Verstecktes, kein `packages/`; auch über symbolische Verweise nicht hinaus.
pub fn pfad(wurzel: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().replace('\\', "/");
    if rel.is_empty() {
        return Err("`pfad` ist leer".into());
    }
    let p = Path::new(&rel);
    let mut teile = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(t) => teile.push(t.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => {
                return Err(format!(
                    "„{rel}“: nur Pfade innerhalb des Projektordners, relativ und ohne „..“"
                ));
            }
        }
    }
    if teile.is_empty() {
        return Err("`pfad` ist leer".into());
    }
    if teile.iter().any(|t| ausgeschlossen(t)) || teile[0] == "packages" {
        return Err(format!(
            "„{rel}“ ist nicht erreichbar (versteckte und `_`-Ordner, `packages/`)"
        ));
    }
    let ziel = teile.iter().fold(wurzel.to_path_buf(), |p, t| p.join(t));
    // Symbolische Verweise: Der nächste vorhandene Vorfahr muss im Projekt liegen.
    let echt_wurzel = wurzel.canonicalize().map_err(|e| e.to_string())?;
    let vorhanden = ziel
        .ancestors()
        .find(|a| a.exists())
        .unwrap_or(wurzel)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !vorhanden.starts_with(&echt_wurzel) {
        return Err(format!("„{rel}“ führt aus dem Projektordner hinaus"));
    }
    Ok(ziel)
}

/// Text einer Datei (Textendung, UTF-8, höchstens `GRENZE` Bytes).
pub fn lesen(wurzel: &Path, rel: &str) -> Result<String, String> {
    let p = pfad(wurzel, rel)?;
    if !ist_text(&p) {
        return Err(format!(
            "„{rel}“ ist keine Textdatei; lesbar sind {}",
            [TEXT.as_slice(), NUR_LESEN.as_slice()].concat().join(", ")
        ));
    }
    let bytes = std::fs::read(&p).map_err(|e| format!("„{rel}“: {e}"))?;
    if bytes.len() > GRENZE {
        return Err(format!("„{rel}“ ist größer als {GRENZE} Bytes"));
    }
    String::from_utf8(bytes).map_err(|_| format!("„{rel}“ ist kein UTF-8-Text"))
}

/// Darf `rel` geschrieben werden? `haupt` und `daten`: Dateinamen relativ zum Projekt.
pub fn schreiben_pruefen(
    wurzel: &Path,
    rel: &str,
    haupt: &str,
    daten: &str,
) -> Result<PathBuf, String> {
    let p = pfad(wurzel, rel)?;
    let gleich = |a: &str| pfad(wurzel, a).is_ok_and(|x| x == p);
    if gleich(haupt) {
        return Err(format!(
            "„{rel}“ ist die Hauptdatei: bitte `text_ersetzen` oder `quelltext_setzen` (dann \
             erscheint die Änderung im Editor und lässt sich rückgängig machen)"
        ));
    }
    if gleich(daten) {
        return Err(format!(
            "„{rel}“ sind die Formulardaten: bitte `formular_setzen`"
        ));
    }
    if !TEXT.contains(&endung(&p).as_str()) {
        return Err(format!(
            "„{rel}“: schreibbar sind nur Textdateien mit {}",
            TEXT.join(", ")
        ));
    }
    Ok(p)
}

/// Schreibt `text` nach `ziel` (Ordner werden angelegt). Gab es die Datei, kommt ihr alter Inhalt
/// vorher nach `SICHERUNGEN` (Name: `<zeit>-<pfad mit __>`). Antwort: der Pfad der Sicherung.
pub fn schreiben(
    wurzel: &Path,
    rel: &str,
    ziel: &Path,
    text: &str,
    zeit: u64,
) -> Result<Option<String>, String> {
    if text.len() > GRENZE {
        return Err(format!("Text größer als {GRENZE} Bytes"));
    }
    let sicherung = match std::fs::read(ziel) {
        Ok(alt) if alt == text.as_bytes() => return Ok(None),
        Ok(alt) => Some(sichern(wurzel, rel, &alt, zeit)?),
        Err(_) => None,
    };
    if let Some(eltern) = ziel.parent() {
        std::fs::create_dir_all(eltern).map_err(|e| format!("„{rel}“: {e}"))?;
    }
    tftn_formular::atomar_schreiben(ziel, text).map_err(|e| format!("„{rel}“: {e}"))?;
    Ok(sicherung)
}

/// Legt `alt` (den bisherigen Inhalt von `rel`) unter `SICHERUNGEN` ab; Antwort: der Pfad.
pub fn sichern(wurzel: &Path, rel: &str, alt: &[u8], zeit: u64) -> Result<String, String> {
    let basis = rel.trim().replace(['/', '\\'], "__");
    let ordner = wurzel.join(SICHERUNGEN);
    std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
    // Zwei Änderungen in derselben Sekunde: die zweite Sicherung darf die erste nicht ersetzen.
    let mut name = format!("{zeit}-{basis}");
    let mut n = 2;
    while ordner.join(&name).exists() {
        name = format!("{zeit}-{n}-{basis}");
        n += 1;
    }
    std::fs::write(ordner.join(&name), alt).map_err(|e| e.to_string())?;
    Ok(format!("{SICHERUNGEN}/{name}"))
}

/// `#include "…"`-Ziele der Hauptdatei mit ihrer Zeile (ab 1), in ihrer Reihenfolge (nur
/// Zeichenketten-Literale am Zeilenanfang).
pub fn eingebunden(quelle: &str) -> Vec<(usize, String)> {
    let mut aus = Vec::new();
    for (i, zeile) in quelle.lines().enumerate() {
        let z = zeile.trim_start();
        if let Some(rest) = z.strip_prefix("#include") {
            let rest = rest.trim_start();
            if let Some(innen) = rest.strip_prefix('"')
                && let Some(ende) = innen.find('"')
            {
                aus.push((i + 1, innen[..ende].to_string()));
            }
        }
    }
    aus
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordner(name: &str) -> PathBuf {
        let o =
            std::env::temp_dir().join(format!("tftn-projektdateien-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        for (p, t) in [
            (
                "main.typ",
                "#include \"kapitel/01.typ\"\n  #include \"kapitel/02.typ\" // zwei\n",
            ),
            ("kapitel/01.typ", "= Eins\n"),
            ("kapitel/02.typ", "= Zwei\n"),
            ("dokument.toml", "titel = \"T\"\n"),
            ("tftn.toml", "tftn = 1\n"),
            ("bilder/a.png", "PNG"),
            ("packages/local/x/lib.typ", "geheim"),
            (".fassungen/1.typ", "alt"),
            ("_lokal/log.txt", "log"),
        ] {
            let d = o.join(p);
            std::fs::create_dir_all(d.parent().unwrap()).unwrap();
            std::fs::write(d, t).unwrap();
        }
        o
    }

    #[test]
    fn auflisten_ohne_pakete_versteckte_und_lokales() {
        let o = ordner("liste");
        let l = auflisten(&o);
        let namen: Vec<&str> = l.iter().map(|e| e.pfad.as_str()).collect();
        assert_eq!(
            namen,
            [
                "bilder/a.png",
                "dokument.toml",
                "kapitel/01.typ",
                "kapitel/02.typ",
                "main.typ",
                "tftn.toml"
            ]
        );
        let bild = &l[0];
        assert!(!bild.text && !bild.schreibbar);
        let toml = &l[1];
        assert!(toml.text && !toml.schreibbar);
        assert!(l[2].schreibbar);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn pfade_bleiben_im_projekt() {
        let o = ordner("pfade");
        assert!(pfad(&o, "kapitel/01.typ").is_ok());
        assert!(pfad(&o, "./kapitel/neu.typ").is_ok(), "neue Datei");
        for schlecht in [
            "",
            "../x.typ",
            "kapitel/../../x.typ",
            "/etc/passwd",
            ".fassungen/1.typ",
            "_lokal/log.txt",
            "packages/local/x/lib.typ",
            "kapitel/.versteckt.typ",
        ] {
            assert!(pfad(&o, schlecht).is_err(), "{schlecht}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(std::env::temp_dir(), o.join("raus")).unwrap();
            assert!(pfad(&o, "raus/x.typ").is_err(), "Verweis nach draußen");
        }
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn lesen_nur_text() {
        let o = ordner("lesen");
        assert_eq!(lesen(&o, "kapitel/01.typ").unwrap(), "= Eins\n");
        assert_eq!(lesen(&o, "tftn.toml").unwrap(), "tftn = 1\n");
        assert!(lesen(&o, "bilder/a.png").is_err());
        assert!(lesen(&o, "kapitel/fehlt.typ").is_err());
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn schreiben_mit_sicherung_und_sperren() {
        let o = ordner("schreiben");
        let pruef = |r: &str| schreiben_pruefen(&o, r, "main.typ", "dokument.toml");
        assert!(pruef("main.typ").unwrap_err().contains("text_ersetzen"));
        assert!(pruef("./main.typ").is_err());
        assert!(
            pruef("dokument.toml")
                .unwrap_err()
                .contains("formular_setzen")
        );
        assert!(pruef("tftn.toml").is_err());
        assert!(pruef("bilder/b.png").is_err());
        // Überschreiben: alter Inhalt gesichert
        let z = pruef("kapitel/01.typ").unwrap();
        let s = schreiben(&o, "kapitel/01.typ", &z, "= Eins neu\n", 42).unwrap();
        assert_eq!(s.as_deref(), Some(".fassungen/dateien/42-kapitel__01.typ"));
        assert_eq!(std::fs::read_to_string(&z).unwrap(), "= Eins neu\n");
        assert_eq!(
            std::fs::read_to_string(o.join(".fassungen/dateien/42-kapitel__01.typ")).unwrap(),
            "= Eins\n"
        );
        // Gleiche Sekunde: eigene Sicherung, die erste bleibt
        let s2 = schreiben(&o, "kapitel/01.typ", &z, "= Eins neuer\n", 42).unwrap();
        assert_eq!(
            s2.as_deref(),
            Some(".fassungen/dateien/42-2-kapitel__01.typ")
        );
        assert_eq!(
            std::fs::read_to_string(o.join(".fassungen/dateien/42-kapitel__01.typ")).unwrap(),
            "= Eins\n"
        );
        schreiben(&o, "kapitel/01.typ", &z, "= Eins neu\n", 42).unwrap();
        // Unverändert: keine Sicherung
        assert_eq!(
            schreiben(&o, "kapitel/01.typ", &z, "= Eins neu\n", 43).unwrap(),
            None
        );
        // Neu, samt Ordner
        let n = pruef("kapitel/neu/03.typ").unwrap();
        assert_eq!(
            schreiben(&o, "kapitel/neu/03.typ", &n, "= Drei\n", 44).unwrap(),
            None
        );
        assert_eq!(lesen(&o, "kapitel/neu/03.typ").unwrap(), "= Drei\n");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn eingebundene_dateien() {
        assert_eq!(
            eingebunden(
                "#include \"a.typ\"\n  #include \"k/b.typ\" // x\n#include(x)\n// #include \"c.typ\"\n"
            ),
            [(1, "a.typ".to_string()), (2, "k/b.typ".to_string())]
        );
    }
}
