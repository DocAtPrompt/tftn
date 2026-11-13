//! Export nach Word (.docx) zum Gegenlesen: typst setzt das Dokument als HTML (mit der Vorlage,
//! Zitaten und Literaturverzeichnis, Formeln als MathML), Pandoc macht daraus .docx. Das Layout der
//! Seite (Ränder, Kopfzeilen, Schrift) kommt nicht mit.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use tftn_core::Projekt;
use tftn_sprachen::tr;

#[derive(Debug, Clone, Serialize)]
pub struct WordJs {
    pub pfad: String,
    pub datei: String,
    /// Was in Word fehlt, etwa „overline (3×)“.
    pub fehlt: Vec<String>,
}

/// Ersetzt jedes Stück von `anfang` bis einschließlich des nächsten `ende`, für das `f` etwas liefert.
fn stuecke(s: &str, anfang: &str, ende: &str, f: impl Fn(&str) -> Option<String>) -> String {
    let mut aus = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(anfang) {
        let Some(j) = rest[i..].find(ende) else { break };
        let stueck = &rest[i..i + j + ende.len()];
        aus.push_str(&rest[..i]);
        match f(stueck) {
            Some(neu) => aus.push_str(&neu),
            None => aus.push_str(stueck),
        }
        rest = &rest[i + j + ende.len()..];
    }
    aus.push_str(rest);
    aus
}

fn wert<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let a = s.find(&format!("{name}=\""))? + name.len() + 2;
    let l = s[a..].find('"')?;
    Some(&s[a..a + l])
}

/// HTML von typst so umschreiben, dass Pandoc Fußnoten, Bildgrößen und Autoren erkennt.
pub fn html_fuer_pandoc(html: &str) -> String {
    // Fußnoten: typst schreibt <sup role="doc-noteref"><a …>, Pandoc erwartet <a role="doc-noteref"><sup>.
    let s = stuecke(html, "<sup id=\"", "</sup>", |t| {
        if !t.contains("role=\"doc-noteref\"") {
            return None;
        }
        let id = wert(t, "id")?;
        let ziel = wert(t, "href")?;
        let a = t.find("<a ")?;
        let zahl = &t[a + t[a..].find('>')? + 1..t.find("</a>")?];
        Some(format!(
            "<a href=\"{ziel}\" id=\"{id}\" class=\"footnote-ref\" role=\"doc-noteref\"><sup>{zahl}</sup></a>"
        ))
    });
    let s = stuecke(&s, "<sup role=\"doc-backlink\">", "</sup>", |_| {
        Some(String::new())
    });
    let s = s.replace(
        "<section role=\"doc-endnotes\">",
        "<section class=\"footnotes\" role=\"doc-endnotes\">",
    );
    let s = s.replace("<meta name=\"authors\"", "<meta name=\"author\"");
    // Bilder: typst gibt Pixelmaße an und die gewünschte Breite nur im Stil.
    stuecke(&s, "<img ", ">", |t| {
        let mut neu = t.to_string();
        for name in ["width", "height"] {
            if let Some(w) = wert(&neu, name) {
                neu = neu.replacen(&format!(" {name}=\"{w}\""), "", 1);
            }
        }
        let stil = wert(t, "style").unwrap_or("");
        let prozent = stil.split(';').find_map(|teil| {
            let w = teil.trim().strip_prefix("width:")?.trim();
            w.ends_with('%').then(|| w.to_string())
        });
        if let Some(p) = prozent {
            neu = neu.replacen("<img ", &format!("<img width=\"{p}\" "), 1);
        }
        Some(neu)
    })
}

/// Aus den Warnungen von typst: was beim HTML- bzw. MathML-Export wegfiel, mit Anzahl.
pub fn fehlt_in_word(stderr: &str) -> Vec<String> {
    let mut zaehler: Vec<(String, usize)> = Vec::new();
    for zeile in stderr.lines() {
        let Some(w) = zeile.strip_prefix("warning: ") else {
            continue;
        };
        let Some(was) = w
            .strip_suffix(" was ignored during HTML export")
            .or_else(|| w.strip_suffix(" was ignored during MathML export"))
        else {
            continue;
        };
        // Das Seitenlayout fehlt in Word immer; das sagt die Oberfläche ohnehin.
        if was.starts_with("page set rule") || was.contains("set rule") && was.contains("page") {
            continue;
        }
        match zaehler.iter_mut().find(|(n, _)| n == was) {
            Some((_, z)) => *z += 1,
            None => zaehler.push((was.to_string(), 1)),
        }
    }
    zaehler
        .into_iter()
        .map(|(n, z)| if z > 1 { format!("{n} ({z}×)") } else { n })
        .collect()
}

/// Name der Word-Datei: wie der Dokumentordner.
pub fn dateiname(projekt: &Projekt) -> String {
    let name = projekt
        .wurzel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Dokument".into());
    format!("{name}.docx")
}

/// Word legt neben einer geöffneten Datei `~$…` an (bei langen Namen ohne die ersten zwei Zeichen).
fn in_word_offen(ziel: &Path) -> bool {
    let (Some(ordner), Some(name)) = (ziel.parent(), ziel.file_name()) else {
        return false;
    };
    let name = name.to_string_lossy();
    let kurz: String = name.chars().skip(2).collect();
    [format!("~${name}"), format!("~${kurz}")]
        .iter()
        .any(|n| ordner.join(n).exists())
}

pub fn exportieren(
    projekt: &Projekt,
    cli: &tftn_export::TypstCli,
    pandoc: &Path,
) -> Result<WordJs, String> {
    let datei = dateiname(projekt);
    let ziel = projekt.wurzel.join(&datei);
    if in_word_offen(&ziel) {
        return Err(tr!(
            "„{datei}“ ist in Word geöffnet. Bitte dort schließen und noch einmal exportieren.",
            datei
        ));
    }
    // Je Lauf ein eigener Zwischenordner, auch bei zwei Exporten in derselben Sekunde.
    static LAUF: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "tftn-word-lauf-{}-{}-{}",
        std::process::id(),
        crate::fassungen::jetzt(),
        LAUF.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let ergebnis = (|| -> Result<WordJs, String> {
        let html = tmp.join("dokument.html");
        let zusatz = ["--features", "html", "--format", "html"].map(String::from);
        let warnungen =
            tftn_export::ausfuehren(projekt, cli, &html, &zusatz, tftn_export::STANDARD_FRIST)
                .map_err(|e| {
                    tr!(
                        "typst konnte das Dokument nicht als HTML setzen: {fehler}",
                        fehler = e
                    )
                })?;
        let text = std::fs::read_to_string(&html).map_err(|e| e.to_string())?;
        let umgeschrieben = tmp.join("fuer-pandoc.html");
        std::fs::write(&umgeschrieben, html_fuer_pandoc(&text)).map_err(|e| e.to_string())?;
        let docx = tmp.join("dokument.docx");
        let aus = Command::new(pandoc)
            .arg("--sandbox")
            .args(["-f", "html", "-t", "docx", "-o"])
            .arg(&docx)
            .arg(&umgeschrieben)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| tr!("Pandoc ließ sich nicht starten: {fehler}", fehler = e))?;
        if !aus.status.success() {
            return Err(tr!(
                "Pandoc meldet: {meldung}",
                meldung = String::from_utf8_lossy(&aus.stderr).trim()
            ));
        }
        std::fs::copy(&docx, &ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
        Ok(WordJs {
            pfad: ziel.display().to_string(),
            datei: datei.clone(),
            fehlt: fehlt_in_word(&warnungen),
        })
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ergebnis
}

/// Zeigt `pfad` im Finder bzw. Explorer.
pub fn im_ordner_zeigen(pfad: &Path) -> Result<(), String> {
    let mut befehl = if cfg!(target_os = "macos") {
        let mut b = Command::new("open");
        b.arg("-R").arg(pfad);
        b
    } else if cfg!(windows) {
        let mut b = Command::new("explorer");
        b.arg(format!("/select,{}", pfad.display()));
        b
    } else {
        let mut b = Command::new("xdg-open");
        b.arg(pfad.parent().unwrap_or(pfad));
        b
    };
    befehl.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Nur Dateien im offenen Dokument dürfen gezeigt werden.
pub fn zeigbar(wurzel: &Path, pfad: &Path) -> Option<PathBuf> {
    let p = pfad.canonicalize().ok()?;
    p.starts_with(wurzel).then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fussnoten_fuer_pandoc() {
        let html = "<p>Text<sup id=\"loc-1\" role=\"doc-noteref\"><a href=\"#loc-9\">1</a></sup></p>\
            <section role=\"doc-endnotes\"><ol><li id=\"loc-9\"><sup role=\"doc-backlink\"><a href=\"#loc-1\">1</a></sup>Fußnote.</li></ol></section>";
        let neu = html_fuer_pandoc(html);
        assert!(neu.contains("<a href=\"#loc-9\" id=\"loc-1\" class=\"footnote-ref\" role=\"doc-noteref\"><sup>1</sup></a>"), "{neu}");
        assert!(neu.contains("<li id=\"loc-9\">Fußnote.</li>"), "{neu}");
        assert!(
            neu.contains("<section class=\"footnotes\" role=\"doc-endnotes\">"),
            "{neu}"
        );
    }

    #[test]
    fn andere_hochstellungen_bleiben() {
        let html = "x<sup>2</sup> und <sup id=\"a\">b</sup>";
        assert_eq!(html_fuer_pandoc(html), html);
    }

    #[test]
    fn bildbreite_aus_dem_stil() {
        let html =
            "<img src=\"data:x\" width=\"24\" height=\"16\" style=\"display: block; width: 80%\">";
        assert_eq!(
            html_fuer_pandoc(html),
            "<img width=\"80%\" src=\"data:x\" style=\"display: block; width: 80%\">"
        );
        let ohne = "<img src=\"data:x\" width=\"24\" height=\"16\">";
        assert_eq!(html_fuer_pandoc(ohne), "<img src=\"data:x\">");
    }

    #[test]
    fn autoren() {
        assert_eq!(
            html_fuer_pandoc("<meta name=\"authors\" content=\"A, B\">"),
            "<meta name=\"author\" content=\"A, B\">"
        );
    }

    /// Der Testartikel ganz durch typst und Pandoc; ohne Pandoc übersprungen (die CI hat keines).
    #[test]
    fn testartikel_nach_word() {
        let Some(pandoc) = crate::import::pandoc_finden() else {
            eprintln!("Pandoc fehlt – Word-Export übersprungen");
            return;
        };
        let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        let o = std::env::temp_dir().join(format!("tftn-word-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let ordner = o.join("Mein Artikel");
        crate::typen::kopieren(&quelle, &ordner).unwrap();
        let _ = std::fs::remove_dir_all(ordner.join("_renders"));
        let p = Projekt::oeffnen(&ordner).unwrap();
        let cli = tftn_export::TypstCli::finden(&p.konfig.typst).expect("typst-CLI nötig");
        let w = exportieren(&p, &cli, &pandoc).unwrap();
        assert_eq!(w.datei, "Mein Artikel.docx");
        assert!(
            w.fehlt.iter().any(|f| f.starts_with("overline")),
            "{:?}",
            w.fehlt
        );
        // Zurück nach Markdown: Titel, echte Fußnoten, Literatur, Formel, Tabelle.
        let md = Command::new(&pandoc)
            .args(["-f", "docx", "-t", "markdown", "-s"])
            .arg(&w.pfad)
            .output()
            .unwrap();
        let md = String::from_utf8_lossy(&md.stdout);
        assert!(md.contains("title: Über die Reproduzierbarkeit"), "{md}");
        assert!(md.contains("[^1]: Für Überweisungen"), "{md}");
        assert!(md.contains("## Literatur"), "{md}");
        assert!(md.contains("\\frac{1}{n}"), "{md}");
        assert!(md.contains("Jahrgang"), "{md}");
        // Ist die Datei in Word offen, wird nicht überschrieben.
        std::fs::write(ordner.join("~$in Artikel.docx"), b"").unwrap();
        assert!(
            exportieren(&p, &cli, &pandoc)
                .unwrap_err()
                .contains("in Word geöffnet")
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Ein leeres Dokument mit Kopfzeile, Seitenzahl und Randtext; ohne Pandoc übersprungen.
    #[test]
    fn leeres_dokument_mit_randtext_nach_word() {
        let Some(pandoc) = crate::import::pandoc_finden() else {
            eprintln!("Pandoc fehlt – Word-Export übersprungen");
            return;
        };
        let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../types");
        let o = std::env::temp_dir().join(format!("tftn-word-leer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let q = crate::typen::quellen(&typen, &o);
        tftn_core::schriftvorrat_setzen(crate::typen::schriftvorrat(&q));
        let ordner = crate::typen::entwurf_sichern(&q, &o.join("Notiz")).unwrap();
        let main = ordner.join("main.typ");
        let vorlage = std::fs::read_to_string(&main).unwrap();
        std::fs::write(
            &main,
            format!(
                "{vorlage}#set page(numbering: \"1\", header: kopfzeile(aussen: \"{{kapitel}}\"))\n\
                 = Kapitel\nText mit Randtext#randnotiz[am Rand] und weiter.\n"
            ),
        )
        .unwrap();
        let p = Projekt::oeffnen(&ordner).unwrap();
        let cli = tftn_export::TypstCli::finden(&p.konfig.typst).expect("typst-CLI nötig");
        let w = exportieren(&p, &cli, &pandoc).unwrap();
        let md = Command::new(&pandoc)
            .args(["-f", "docx", "-t", "markdown"])
            .arg(&w.pfad)
            .output()
            .unwrap();
        let md = String::from_utf8_lossy(&md.stdout);
        assert!(md.contains("Kapitel"), "{md}");
        assert!(md.contains("am Rand"), "Randtext fehlt: {md}");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn was_in_word_fehlt() {
        let stderr = "warning: html export is under active development and incomplete\n\
            warning: page set rule was ignored during HTML export\n\
            warning: overline was ignored during MathML export\n\
            warning: overline was ignored during MathML export\n\
            warning: place was ignored during HTML export\n\
            warning: something else\n";
        assert_eq!(fehlt_in_word(stderr), ["overline (2×)", "place"]);
    }
}
