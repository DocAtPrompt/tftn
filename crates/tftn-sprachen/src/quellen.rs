//! Welche Texte übersetzt werden müssen: alles in `tr!("…")` (Rust), `t("…")` und `n("…")`
//! (TypeScript) sowie die sichtbaren Texte von `ui/index.html`. Daraus schreibt `cargo xtask
//! sprachen` die Sprachdatei, und ein Test prüft, dass `sprachen/en.toml` vollständig ist.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// `{name}`-Platzhalter: Buchstaben, Ziffern, `_`, nicht mit einer Ziffer vorn.
pub fn ist_name(s: &str) -> bool {
    let mut z = s.chars();
    z.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && z.all(|c| c.is_alphanumeric() || c == '_')
}

/// Die Platzhalter in `text`.
pub fn platzhalter(text: &str) -> BTreeSet<String> {
    let mut aus = BTreeSet::new();
    let mut rest = text;
    while let Some(auf) = rest.find('{') {
        let nach = &rest[auf + 1..];
        if let Some(zu) = nach.find('}')
            && ist_name(&nach[..zu])
        {
            aus.insert(nach[..zu].to_string());
        }
        rest = nach;
    }
    aus
}

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Liest ein Rust-Zeichenkettenliteral ab `s[0] == '"'` (auch `r"…"`, `r#"…"#`).
fn rust_literal(s: &str) -> Option<String> {
    if let Some(roh) = s.strip_prefix('r') {
        let hashes = roh.chars().take_while(|&c| c == '#').count();
        let innen = roh[hashes..].strip_prefix('"')?;
        let ende = format!("\"{}", "#".repeat(hashes));
        return innen.find(&ende).map(|i| innen[..i].to_string());
    }
    let mut z = s.strip_prefix('"')?.chars().peekable();
    let mut aus = String::new();
    while let Some(c) = z.next() {
        match c {
            '"' => return Some(aus),
            '\\' => match z.next()? {
                'n' => aus.push('\n'),
                't' => aus.push('\t'),
                'r' => aus.push('\r'),
                '0' => aus.push('\0'),
                '\\' => aus.push('\\'),
                '"' => aus.push('"'),
                '\'' => aus.push('\''),
                '\n' | '\r' => {
                    while z.peek().is_some_and(|c| c.is_whitespace()) {
                        z.next();
                    }
                }
                'u' => {
                    z.next(); // {
                    let hex: String = z.by_ref().take_while(|&c| c != '}').collect();
                    aus.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            c => aus.push(c),
        }
    }
    None
}

/// Liest ein JavaScript-Literal ab `s[0]` (`"`, `'` oder `` ` `` ohne `${`).
fn js_literal(s: &str) -> Option<String> {
    let mut z = s.chars().peekable();
    let q = z.next()?;
    if !matches!(q, '"' | '\'' | '`') {
        return None;
    }
    let mut aus = String::new();
    while let Some(c) = z.next() {
        match c {
            c if c == q => return Some(aus),
            '$' if q == '`' && z.peek() == Some(&'{') => return None,
            '\\' => match z.next()? {
                'n' => aus.push('\n'),
                't' => aus.push('\t'),
                'r' => aus.push('\r'),
                '\n' => {}
                'u' => {
                    let hex: String = if z.peek() == Some(&'{') {
                        z.next();
                        z.by_ref().take_while(|&c| c != '}').collect()
                    } else {
                        z.by_ref().take(4).collect()
                    };
                    aus.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                c => aus.push(c),
            },
            c => aus.push(c),
        }
    }
    None
}

/// Liegt `pos` hinter `//` in derselben Zeile (Kommentar)?
fn im_kommentar(text: &str, pos: usize) -> bool {
    let anfang = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    text[anfang..pos].contains("//")
}

/// Texte in `tr!(…)`.
pub fn aus_rust(text: &str) -> Vec<String> {
    let mut aus = Vec::new();
    for (pos, _) in text.match_indices("tr!(") {
        if pos > 0 && text[..pos].chars().next_back().is_some_and(ident) {
            continue;
        }
        if im_kommentar(text, pos) {
            continue;
        }
        let rest = text[pos + 4..].trim_start();
        if let Some(l) = rust_literal(rest) {
            aus.push(l);
        }
    }
    aus
}

/// Ergebnis für TypeScript: die Texte und die Stellen `t(…)`, deren Text keiner prüfen kann.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TsTexte {
    pub texte: Vec<String>,
    /// Zeilennummern mit `t(` ohne festen Text (etwa `t(name)` oder `` t(`…${x}…`) ``).
    pub offen: Vec<usize>,
}

/// Texte in `t("…")` und `n("…")`.
pub fn aus_ts(text: &str) -> TsTexte {
    let mut aus = TsTexte::default();
    for name in ["t(", "n("] {
        for (pos, _) in text.match_indices(name) {
            let davor = text[..pos].chars().next_back();
            if davor.is_some_and(|c| ident(c) || c == '.') {
                continue;
            }
            if im_kommentar(text, pos) {
                continue;
            }
            let rest = text[pos + 2..].trim_start();
            match js_literal(rest) {
                Some(l) => aus.texte.push(l),
                None if name == "t(" => {
                    aus.offen.push(text[..pos].matches('\n').count() + 1);
                }
                None => {}
            }
        }
    }
    aus
}

/// Entschlüsselt die paar Zeichenreferenzen, die in `index.html` vorkommen können.
fn html_text(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&")
}

/// Zählt nur, was ein Kleinbuchstabe als Wort ausweist (nicht „TFTN“, „×“, „…“).
pub fn ist_wortlaut(s: &str) -> bool {
    s.chars().any(char::is_lowercase)
}

/// Attribute, deren Wert man sieht oder vorgelesen bekommt.
pub const HTML_ATTRIBUTE: [&str; 3] = ["title", "placeholder", "aria-label"];

/// Sichtbare Texte einer HTML-Seite: Textknoten und die `HTML_ATTRIBUTE`, ohne `<head>`/`<script>`.
pub fn aus_html(text: &str) -> Vec<String> {
    let rumpf = text
        .find("<body")
        .map_or(text, |i| &text[i..])
        .split("<script")
        .next()
        .unwrap_or_default();
    let mut aus = Vec::new();
    for teil in rumpf.split('<').skip(1) {
        let (marke, danach) = teil.split_once('>').unwrap_or((teil, ""));
        for a in HTML_ATTRIBUTE {
            for (i, _) in marke.match_indices(&format!("{a}=\"")) {
                if i > 0 && !marke[..i].ends_with(char::is_whitespace) {
                    continue;
                }
                let wert = &marke[i + a.len() + 2..];
                if let Some(e) = wert.find('"') {
                    aus.push(html_text(&wert[..e]));
                }
            }
        }
        let t = danach.trim();
        if !t.is_empty() {
            aus.push(html_text(t));
        }
    }
    aus.retain(|t| ist_wortlaut(t));
    aus
}

fn dateien(ordner: &Path, endung: &str, aus: &mut Vec<PathBuf>) {
    let Ok(e) = std::fs::read_dir(ordner) else {
        return;
    };
    let mut pfade: Vec<PathBuf> = e.flatten().map(|e| e.path()).collect();
    pfade.sort();
    for p in pfade {
        if p.is_dir() {
            dateien(&p, endung, aus);
        } else if p.to_str().is_some_and(|s| s.ends_with(endung)) {
            aus.push(p);
        }
    }
}

/// Alle Texte des Projekts unter `wurzel`, je mit den Dateien (relativ, mit `/`), in denen sie
/// stehen. Die Reihenfolge der Dateien ist fest (Rust-Crates, App, Oberfläche).
#[derive(Debug, Default)]
pub struct Bestand {
    pub texte: BTreeMap<String, Vec<String>>,
    /// Reihenfolge der Dateien, in der die Sprachdatei gegliedert wird.
    pub dateien: Vec<String>,
    /// `datei:zeile` mit `t(…)` ohne festen Text.
    pub offen: Vec<String>,
}

pub fn bestand(wurzel: &Path) -> Bestand {
    let mut b = Bestand::default();
    let rel = |p: &Path| {
        p.strip_prefix(wurzel)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let mut rs = Vec::new();
    if let Ok(e) = std::fs::read_dir(wurzel.join("crates")) {
        let mut crates: Vec<PathBuf> = e.flatten().map(|e| e.path()).collect();
        crates.sort();
        for c in crates {
            // Dieses Crate selbst: Beispiele in der Dokumentation sind keine Texte.
            if c.ends_with("tftn-sprachen") {
                continue;
            }
            dateien(&c.join("src"), ".rs", &mut rs);
        }
    }
    dateien(&wurzel.join("src-tauri/src"), ".rs", &mut rs);
    let mut ts = Vec::new();
    dateien(&wurzel.join("ui/src"), ".ts", &mut ts);
    // Tests und die Übersetzungsfunktionen selbst enthalten keine Texte der Oberfläche.
    ts.retain(|p| {
        let s = p.to_string_lossy();
        !s.ends_with(".test.ts") && !s.ends_with("sprache.ts")
    });

    let mut eintragen = |datei: String, texte: Vec<String>| {
        b.dateien.push(datei.clone());
        for t in texte {
            let d = b.texte.entry(t).or_default();
            if !d.contains(&datei) {
                d.push(datei.clone());
            }
        }
    };
    for p in rs {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        eintragen(rel(&p), aus_rust(&text));
    }
    let html = wurzel.join("ui/index.html");
    eintragen(
        rel(&html),
        aus_html(&std::fs::read_to_string(&html).unwrap_or_default()),
    );
    for p in ts {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        let t = aus_ts(&text);
        b.offen
            .extend(t.offen.iter().map(|z| format!("{}:{z}", rel(&p))));
        eintragen(rel(&p), t.texte);
    }
    // Mitgelieferte Dokumenttypen und Paketbeschreibungen: die Oberfläche zeigt ihre Texte an.
    let mut daten = Vec::new();
    if let Ok(e) = std::fs::read_dir(wurzel.join("typen")) {
        let mut typen: Vec<PathBuf> = e.flatten().map(|e| e.path()).collect();
        typen.sort();
        for t in typen {
            daten.push(t.join("typst.toml"));
            dateien(&t.join("tftn"), ".json", &mut daten);
        }
    }
    dateien(&wurzel.join("pakete"), ".toml", &mut daten);
    for p in daten {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let texte = if p.extension().is_some_and(|e| e == "json") {
            aus_schema(&text)
        } else {
            aus_manifest(&text)
        };
        eintragen(rel(&p), texte);
    }
    b
}

/// Sichtbare Texte eines Manifests (`typst.toml` eines Typs, Paketbeschreibung in `pakete/`):
/// `titel`, `gruppe`, `hilfe`, `braucht`, `beschriftungen` in jeder Tiefe und
/// `[package] description`.
pub fn aus_manifest(text: &str) -> Vec<String> {
    fn gang(wert: &toml::Value, aus: &mut Vec<String>) {
        match wert {
            toml::Value::Table(t) => {
                for (k, v) in t {
                    match (k.as_str(), v) {
                        ("titel" | "gruppe" | "hilfe", toml::Value::String(s)) => {
                            aus.push(s.clone())
                        }
                        ("braucht" | "beschriftungen", toml::Value::Array(a)) => {
                            aus.extend(a.iter().filter_map(|x| x.as_str().map(String::from)))
                        }
                        _ => gang(v, aus),
                    }
                }
            }
            toml::Value::Array(a) => a.iter().for_each(|x| gang(x, aus)),
            _ => {}
        }
    }
    let Ok(wert) = toml::from_str::<toml::Table>(text) else {
        return Vec::new();
    };
    let mut aus = Vec::new();
    if let Some(d) = wert
        .get("package")
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
    {
        aus.push(d.to_string());
    }
    gang(&toml::Value::Table(wert), &mut aus);
    aus.retain(|s| !s.trim().is_empty());
    aus
}

/// Sichtbare Texte eines Formular-Schemas: `title`, `description` und `x-tftn-gruppe` in jeder Tiefe.
pub fn aus_schema(text: &str) -> Vec<String> {
    fn gang(wert: &serde_json::Value, aus: &mut Vec<String>) {
        match wert {
            serde_json::Value::Object(o) => {
                for (k, v) in o {
                    match (k.as_str(), v) {
                        (
                            "title" | "description" | "x-tftn-gruppe",
                            serde_json::Value::String(s),
                        ) => aus.push(s.clone()),
                        _ => gang(v, aus),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| gang(x, aus)),
            _ => {}
        }
    }
    let Ok(wert) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let mut aus = Vec::new();
    gang(&wert, &mut aus);
    aus.retain(|s| !s.trim().is_empty());
    aus
}

/// TOML-Zeichenkette in Anführungszeichen.
pub fn toml_text(s: &str) -> String {
    let mut aus = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => aus.push_str("\\\\"),
            '"' => aus.push_str("\\\""),
            '\n' => aus.push_str("\\n"),
            '\t' => aus.push_str("\\t"),
            '\r' => aus.push_str("\\r"),
            c if c.is_control() => aus.push_str(&format!("\\u{:04X}", c as u32)),
            c => aus.push(c),
        }
    }
    aus.push('"');
    aus
}

/// Schreibt eine Sprachdatei: `kopf` (Kommentarzeilen), Name, dann alle Texte von `bestand`,
/// gegliedert nach der ersten Datei, in der sie stehen; Übersetzungen aus `alt`, sonst leer.
pub fn sprachdatei(
    kopf: &str,
    name: &str,
    bestand: &Bestand,
    alt: &std::collections::HashMap<String, String>,
) -> String {
    let mut aus = String::new();
    for z in kopf.lines() {
        aus.push_str(&format!("# {z}\n").replace("# \n", "#\n"));
    }
    aus.push_str(&format!(
        "\n[sprache]\nname = {}\n\n[texte]\n",
        toml_text(name)
    ));
    for d in &bestand.dateien {
        let hier: Vec<&String> = bestand
            .texte
            .iter()
            .filter(|(_, ds)| ds.first() == Some(d))
            .map(|(t, _)| t)
            .collect();
        if hier.is_empty() {
            continue;
        }
        aus.push_str(&format!("\n# {d}\n"));
        for t in hier {
            let z = alt.get(t).map_or("", String::as_str);
            aus.push_str(&format!("{} = {}\n", toml_text(t), toml_text(z)));
        }
    }
    aus
}
