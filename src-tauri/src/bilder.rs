//! Bilder in den Text ziehen oder einfügen: Die Datei landet unter `bilder/` im Dokument.

use std::path::Path;

use tftn_sprachen::tr;

pub const ENDUNGEN: [&str; 7] = ["png", "jpg", "jpeg", "gif", "svg", "webp", "pdf"];
/// Größer wird kein Bild übernommen.
pub const HOECHSTENS: usize = 50 * 1024 * 1024;

/// Dateiname für Typst: Leerzeichen werden `-`, nur Buchstaben, Ziffern, `-`, `_`; Endung klein.
/// `None`, wenn die Endung kein Bildformat ist, das Typst setzt.
pub fn dateiname(name: &str) -> Option<String> {
    let (stamm, endung) = name.rsplit_once('.')?;
    let endung = endung.to_lowercase();
    if !ENDUNGEN.contains(&endung.as_str()) {
        return None;
    }
    let mut s = String::new();
    for c in stamm.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            s.push(c);
        } else if (c.is_whitespace() || c == '.') && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-');
    let s = if s.is_empty() { "bild" } else { s };
    Some(format!("{s}.{endung}"))
}

/// Legt `daten` als `bilder/<name>` ab und liefert den Pfad für `image(…)`. Gibt es die Datei mit
/// gleichem Inhalt schon, wird sie genommen; mit anderem Inhalt bekommt die neue `-2`, `-3` …
pub fn ablegen(projekt: &Path, name: &str, daten: &[u8]) -> Result<String, String> {
    let name = dateiname(name).ok_or_else(|| {
        tr!(
            "„{name}“ ist kein Bild, das Typst setzen kann ({endungen}).",
            name,
            endungen = ENDUNGEN.join(", ")
        )
    })?;
    if daten.len() > HOECHSTENS {
        return Err(tr!("„{name}“ ist größer als 50 MB.", name));
    }
    if daten.is_empty() {
        return Err(tr!("„{name}“ ist leer.", name));
    }
    let ordner = projekt.join("bilder");
    std::fs::create_dir_all(&ordner).map_err(|e| format!("bilder/: {e}"))?;
    let (stamm, endung) = name.rsplit_once('.').expect("dateiname hat eine Endung");
    for n in 1.. {
        let kandidat = if n == 1 {
            name.clone()
        } else {
            format!("{stamm}-{n}.{endung}")
        };
        let pfad = ordner.join(&kandidat);
        match std::fs::read(&pfad) {
            Ok(vorhanden) if vorhanden == daten => return Ok(format!("bilder/{kandidat}")),
            Ok(_) => continue,
            Err(_) => {
                std::fs::write(&pfad, daten).map_err(|e| format!("bilder/{kandidat}: {e}"))?;
                return Ok(format!("bilder/{kandidat}"));
            }
        }
    }
    unreachable!()
}

/// `%XX`-Folgen zurück in UTF-8 (Kopfzeilen tragen nur ASCII; die Oberfläche kodiert den Namen).
pub fn prozent_dekodieren(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut aus = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            aus.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            aus.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(aus).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projekt(name: &str) -> std::path::PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-bilder-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        o
    }

    #[test]
    fn dateinamen_fuer_typst() {
        assert_eq!(
            dateiname("Mein Foto 1.JPG").as_deref(),
            Some("Mein-Foto-1.jpg")
        );
        assert_eq!(
            dateiname("Übersicht (neu).png").as_deref(),
            Some("Übersicht-neu.png")
        );
        assert_eq!(dateiname("a.b.c.svg").as_deref(), Some("a-b-c.svg"));
        assert_eq!(dateiname("\"#?.png").as_deref(), Some("bild.png"));
        assert_eq!(dateiname("text.docx"), None);
        assert_eq!(dateiname("ohne-endung"), None);
    }

    #[test]
    fn gleicher_inhalt_einmal_anderer_mit_nummer() {
        let p = projekt("nummer");
        assert_eq!(ablegen(&p, "a.png", b"1").unwrap(), "bilder/a.png");
        assert_eq!(ablegen(&p, "a.png", b"1").unwrap(), "bilder/a.png");
        assert_eq!(ablegen(&p, "a.png", b"2").unwrap(), "bilder/a-2.png");
        assert_eq!(ablegen(&p, "a.png", b"3").unwrap(), "bilder/a-3.png");
        assert_eq!(std::fs::read(p.join("bilder/a-2.png")).unwrap(), b"2");
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn kein_bild_leer_zu_gross() {
        let p = projekt("abgelehnt");
        assert!(ablegen(&p, "brief.docx", b"x").is_err());
        assert!(ablegen(&p, "leer.png", b"").is_err());
        assert!(ablegen(&p, "riesig.png", &vec![0; HOECHSTENS + 1]).is_err());
        assert!(!p.join("bilder").exists());
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn prozent() {
        assert_eq!(
            prozent_dekodieren("%C3%9Cbersicht%20neu.png").as_deref(),
            Some("Übersicht neu.png")
        );
        assert_eq!(prozent_dekodieren("a.png").as_deref(), Some("a.png"));
        assert_eq!(prozent_dekodieren("%zz"), None);
        assert_eq!(prozent_dekodieren("%C3"), None);
    }
}
