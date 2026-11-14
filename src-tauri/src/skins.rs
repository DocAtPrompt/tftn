//! Skins: Farben und Editorschrift der Oberfläche aus kleinen TOML-Dateien. Mitgelieferte liegen in
//! `skins/` (im Bündel), eigene im Ordner `skins` des Einstellungsordners; eigene haben Vorrang.
//! Aus jeder Datei werden nur geprüfte Werte zu CSS-Variablen; was nicht passt, wird gemeldet und
//! übersprungen, damit keine Datei beliebiges CSS einschleusen kann.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tftn_sprachen::tr;

/// Name der Vorlage für einen eigenen Skin.
pub const VORLAGE: &str = "eigener";

/// Farbvariablen der Oberfläche (Abschnitt `[farben]`).
const FARBEN: [&str; 7] = [
    "grund", "text", "leise", "linie", "akzent", "warn", "flaeche",
];
/// Farben der Syntaxhervorhebung (Abschnitt `[syntax]`).
const SYNTAX: [&str; 8] = [
    "marker", "link", "raw", "key", "zahl", "text", "funktion", "pol",
];

/// Ein gelesener Skin: CSS-Variablen (`--grund` → `#fbfaf7`) und was übersprungen wurde.
#[derive(Debug, Clone, Serialize)]
pub struct Skin {
    pub kennung: String,
    pub name: String,
    /// `hell` oder `dunkel`: für Bedienelemente des Systems (`color-scheme`).
    pub art: String,
    pub variablen: BTreeMap<String, String>,
    pub warnungen: Vec<String>,
}

/// Eintrag der Auswahlliste.
#[derive(Debug, Clone, Serialize)]
pub struct SkinEintrag {
    pub kennung: String,
    pub name: String,
}

/// Mitgelieferte Skins; im Entwicklungsbau direkt aus dem Quellbaum.
pub fn mitgeliefert(resource_dir: Option<PathBuf>) -> PathBuf {
    match resource_dir {
        Some(r) if !cfg!(debug_assertions) => r.join("skins"),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../skins"),
    }
}

/// Ordner für eigene Skins.
pub fn eigene(einstellungsordner: &Path) -> PathBuf {
    einstellungsordner.join("skins")
}

fn ist_farbe(w: &str) -> bool {
    let Some(h) = w.strip_prefix('#') else {
        return false;
    };
    matches!(h.len(), 3 | 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Schriftliste wie `Iosevka, "SF Mono", monospace`: nur Buchstaben, Ziffern, Leerzeichen und
/// `,"'-_.` – nichts, womit eine CSS-Regel enden könnte.
fn ist_schrift(w: &str) -> bool {
    !w.trim().is_empty()
        && w.len() <= 200
        && w.chars()
            .all(|c| c.is_alphanumeric() || " ,\"'-_.".contains(c))
}

/// Liest einen Skin aus `text`; unpassende Werte werden gemeldet und übersprungen.
pub fn lesen(kennung: &str, text: &str) -> Skin {
    let mut skin = Skin {
        kennung: kennung.to_string(),
        name: kennung.to_string(),
        art: "hell".into(),
        variablen: BTreeMap::new(),
        warnungen: Vec::new(),
    };
    let tabelle: toml::Table = match toml::from_str(text) {
        Ok(t) => t,
        Err(e) => {
            skin.warnungen.push(tr!(
                "Skin {kennung}: {fehler}",
                kennung,
                fehler = e.message()
            ));
            return skin;
        }
    };
    let mut warnen = |was: String| {
        skin.warnungen
            .push(tr!("Skin {kennung}: {was} – übersprungen", kennung, was))
    };
    for (schluessel, wert) in &tabelle {
        match (schluessel.as_str(), wert) {
            ("name", toml::Value::String(n)) => skin.name = n.clone(),
            ("art", toml::Value::String(a)) if a == "hell" || a == "dunkel" => skin.art = a.clone(),
            ("farben" | "syntax" | "editor", toml::Value::Table(_)) => {}
            _ => warnen(schluessel.clone()),
        }
    }
    for (abschnitt, namen, praefix) in [
        ("farben", &FARBEN[..], "--"),
        ("syntax", &SYNTAX[..], "--syntax-"),
    ] {
        let Some(toml::Value::Table(t)) = tabelle.get(abschnitt) else {
            continue;
        };
        for (n, w) in t {
            match w.as_str() {
                Some(f) if namen.contains(&n.as_str()) && ist_farbe(f) => {
                    skin.variablen
                        .insert(format!("{praefix}{n}"), f.to_string());
                }
                _ => warnen(format!("{abschnitt}.{n}")),
            }
        }
    }
    if let Some(toml::Value::Table(t)) = tabelle.get("editor") {
        for (n, w) in t {
            let wert = match (n.as_str(), w) {
                ("schrift", toml::Value::String(s)) if ist_schrift(s) => {
                    Some(("--editor-schrift", s.clone()))
                }
                ("groesse", toml::Value::Integer(g)) if (8..=40).contains(g) => {
                    Some(("--editor-groesse", format!("{g}px")))
                }
                ("zeilenabstand", v) => v
                    .as_float()
                    .or_else(|| v.as_integer().map(|i| i as f64))
                    .filter(|z| (1.0..=3.0).contains(z))
                    .map(|z| ("--editor-zeilenabstand", z.to_string())),
                _ => None,
            };
            match wert {
                Some((var, w)) => {
                    skin.variablen.insert(var.into(), w);
                }
                None => warnen(format!("editor.{n}")),
            }
        }
    }
    skin
}

/// Alle Skins, nach Namen sortiert; ein eigener ersetzt einen mitgelieferten mit gleicher Kennung.
pub fn verfuegbar(mitgeliefert: &Path, eigene: &Path) -> Vec<SkinEintrag> {
    let mut alle: BTreeMap<String, String> = BTreeMap::new();
    for ordner in [mitgeliefert, eigene] {
        let Ok(dateien) = std::fs::read_dir(ordner) else {
            continue;
        };
        for d in dateien.filter_map(Result::ok) {
            let p = d.path();
            if p.extension().is_some_and(|e| e == "toml")
                && let Some(k) = p.file_stem().and_then(|s| s.to_str())
            {
                let text = std::fs::read_to_string(&p).unwrap_or_default();
                alle.insert(k.to_string(), lesen(k, &text).name);
            }
        }
    }
    let mut liste: Vec<_> = alle
        .into_iter()
        .map(|(kennung, name)| SkinEintrag { kennung, name })
        .collect();
    liste.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    liste
}

/// Der Skin `kennung`, eigener vor mitgeliefertem; `None`, wenn es ihn nicht gibt.
pub fn laden(mitgeliefert: &Path, eigene: &Path, kennung: &str) -> Option<Skin> {
    if kennung.contains(['/', '\\']) || kennung.starts_with('.') {
        return None;
    }
    [eigene, mitgeliefert].iter().find_map(|o| {
        let text = std::fs::read_to_string(o.join(format!("{kennung}.toml"))).ok()?;
        Some(lesen(kennung, &text))
    })
}

/// Legt `eigener.toml` (Kopie von „Papier“) im eigenen Ordner an, wenn es ihn noch nicht gibt.
pub fn vorlage_anlegen(mitgeliefert: &Path, eigene: &Path) -> Result<PathBuf, String> {
    let ziel = eigene.join(format!("{VORLAGE}.toml"));
    if ziel.exists() {
        return Ok(ziel);
    }
    std::fs::create_dir_all(eigene).map_err(|e| format!("{}: {e}", eigene.display()))?;
    let quelle = mitgeliefert.join("papier.toml");
    let text =
        std::fs::read_to_string(&quelle).map_err(|e| format!("{}: {e}", quelle.display()))?;
    let text = text.replacen("name = \"Papier\"", "name = \"Eigener Skin\"", 1);
    std::fs::write(&ziel, text).map_err(|e| format!("{}: {e}", ziel.display()))?;
    Ok(ziel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_skins() -> PathBuf {
        mitgeliefert(None)
    }

    #[test]
    fn gueltige_datei_wird_zu_variablen() {
        let s = lesen(
            "papier",
            r##"
name = "Papier"
art = "hell"
[farben]
grund = "#f4ecd8"
text = "#3b2f1e"
[syntax]
marker = "#a0522d"
[editor]
schrift = "Iosevka, \"SF Mono\", monospace"
groesse = 15
zeilenabstand = 1.6
"##,
        );
        assert_eq!(s.name, "Papier");
        assert_eq!(s.art, "hell");
        assert!(s.warnungen.is_empty(), "{:?}", s.warnungen);
        assert_eq!(s.variablen["--grund"], "#f4ecd8");
        assert_eq!(s.variablen["--text"], "#3b2f1e");
        assert_eq!(s.variablen["--syntax-marker"], "#a0522d");
        assert_eq!(
            s.variablen["--editor-schrift"],
            "Iosevka, \"SF Mono\", monospace"
        );
        assert_eq!(s.variablen["--editor-groesse"], "15px");
        assert_eq!(s.variablen["--editor-zeilenabstand"], "1.6");
        assert!(
            !s.variablen.contains_key("--leise"),
            "nicht gesetzt: Standard gilt"
        );
    }

    #[test]
    fn kein_css_einschleusen() {
        let s = lesen(
            "boese",
            r##"
name = "Böse"
[farben]
grund = "red; } body { display: none"
text = "#123"
[editor]
schrift = "x } body { display: none"
groesse = 400
"##,
        );
        assert!(!s.variablen.contains_key("--grund"));
        assert!(!s.variablen.contains_key("--editor-schrift"));
        assert!(!s.variablen.contains_key("--editor-groesse"));
        assert_eq!(s.variablen["--text"], "#123");
        assert_eq!(s.warnungen.len(), 3, "{:?}", s.warnungen);
        assert!(s.warnungen.iter().any(|w| w.contains("grund")));
    }

    #[test]
    fn unbekanntes_und_kaputtes_wird_gemeldet() {
        let s = lesen("x", "name = \"X\"\n[farben]\nhimmel = \"#fff\"\n");
        assert!(
            s.warnungen.iter().any(|w| w.contains("himmel")),
            "{:?}",
            s.warnungen
        );
        let k = lesen("kaputt", "name = \n");
        assert_eq!(k.name, "kaputt");
        assert!(k.variablen.is_empty());
        assert_eq!(k.warnungen.len(), 1);
        // Ohne Namen gilt die Kennung, ohne Art „hell“.
        let o = lesen("ohne", "");
        assert_eq!((o.name.as_str(), o.art.as_str()), ("ohne", "hell"));
    }

    #[test]
    fn mitgelieferte_skins() {
        let t = std::env::temp_dir().join(format!("tftn-skins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        let liste = verfuegbar(&repo_skins(), &t);
        let kennungen: Vec<_> = liste.iter().map(|s| s.kennung.as_str()).collect();
        for k in ["dunkel", "hell", "kontrast", "papier"] {
            assert!(kennungen.contains(&k), "{k} fehlt: {kennungen:?}");
        }
        for s in &liste {
            let skin = laden(&repo_skins(), &t, &s.kennung).unwrap();
            assert!(
                skin.warnungen.is_empty(),
                "{}: {:?}",
                s.kennung,
                skin.warnungen
            );
            // Text auf Grund und auf Fläche gut lesbar (WCAG AA: 4,5:1), Leises wenigstens 3:1.
            let v = &skin.variablen;
            for (vorne, hinten, mindestens) in [
                ("--text", "--grund", 4.5),
                ("--text", "--flaeche", 4.5),
                ("--leise", "--grund", 3.0),
                ("--akzent", "--grund", 3.0),
            ] {
                let k = kontrast(&v[vorne], &v[hinten]);
                assert!(
                    k >= mindestens,
                    "{}: {vorne} auf {hinten} nur {k:.2}",
                    s.kennung
                );
            }
            for name in [
                "marker", "link", "raw", "key", "zahl", "text", "funktion", "pol",
            ] {
                let k = kontrast(&v[&format!("--syntax-{name}")], &v["--grund"]);
                assert!(k >= 3.0, "{}: Syntax {name} nur {k:.2}", s.kennung);
            }
        }
    }

    #[test]
    fn eigener_skin_hat_vorrang_und_vorlage_wird_angelegt() {
        let t = std::env::temp_dir().join(format!("tftn-skins-eigen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("hell.toml"), "name = \"Mein Hell\"\n").unwrap();
        let liste = verfuegbar(&repo_skins(), &t);
        assert_eq!(liste.iter().filter(|s| s.kennung == "hell").count(), 1);
        assert_eq!(
            liste.iter().find(|s| s.kennung == "hell").unwrap().name,
            "Mein Hell"
        );
        let p = vorlage_anlegen(&repo_skins(), &t).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains("[farben]"));
        assert!(
            verfuegbar(&repo_skins(), &t)
                .iter()
                .any(|s| s.kennung == "eigener")
        );
        assert!(laden(&repo_skins(), &t, "gibt-es-nicht").is_none());
        let _ = std::fs::remove_dir_all(&t);
    }

    /// Kontrastverhältnis nach WCAG 2 für `#rgb`/`#rrggbb`.
    fn kontrast(a: &str, b: &str) -> f64 {
        let hell = |f: &str| {
            let h = f.trim_start_matches('#');
            let h: String = if h.len() == 3 {
                h.chars().flat_map(|c| [c, c]).collect()
            } else {
                h[..6].to_string()
            };
            let k = |i: usize| {
                let c = u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
                if c <= 0.03928 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * k(0) + 0.7152 * k(2) + 0.0722 * k(4)
        };
        let (x, y) = (hell(a), hell(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }
}
