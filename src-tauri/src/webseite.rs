//! Export als Webseite (HTML): typst setzt das Dokument als HTML (mit Vorlage, Zitaten,
//! Literaturverzeichnis, Formeln als MathML, Bilder eingebettet), TFTN fügt ein schlichtes Stilblatt
//! zum Lesen ein. Ergebnis ist eine einzige Datei ohne Verweise nach außen; das Seitenlayout (Ränder,
//! Kopfzeilen, Seitenzahlen) gibt es im Web nicht.

use std::path::Path;

use serde::Serialize;
use tftn_core::Projekt;
use tftn_sprachen::tr;

#[derive(Debug, Clone, Serialize)]
pub struct WebseiteJs {
    pub pfad: String,
    pub datei: String,
    /// Was auf der Webseite fehlt, etwa „overline (3×)“.
    pub fehlt: Vec<String>,
}

/// Lesbare Grundform: schmale Spalte, Serifenschrift des Systems, Abbildungen und Tabellen mittig.
/// Farben fest hell, wie das gedruckte Dokument.
pub const STIL: &str = r#"<style id="tftn">
/* TFTN: Grundform zum Lesen. Das Seitenlayout des Dokuments gibt es im Web nicht. */
:root { color-scheme: light; }
body { max-width: 42rem; margin: 2.5rem auto; padding: 0 1.25rem; font-family: Charter, "Bitstream Charter", "Sitka Text", Cambria, Georgia, serif; font-size: 1.0625rem; line-height: 1.55; color: #1a1a1a; background: #fff; hyphens: auto; overflow-wrap: break-word; }
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin: 1.6em 0 0.5em; hyphens: manual; }
h1 { font-size: 1.8rem; }
h2 { font-size: 1.4rem; }
h3 { font-size: 1.15rem; }
p { margin: 0 0 0.8em; }
a { color: #1f5fa8; }
img { max-width: 100%; height: auto; }
figure { margin: 1.5em 0; text-align: center; }
figcaption { font-size: 0.92em; color: #444; margin-top: 0.4em; }
table { border-collapse: collapse; margin: 0 auto; }
th, td { border: 1px solid #bbb; padding: 0.3em 0.6em; text-align: left; vertical-align: top; }
blockquote { margin: 1em 0; padding-left: 1em; border-left: 3px solid #ddd; color: #333; }
pre, code { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 0.92em; }
pre { overflow-x: auto; padding: 0.8em; background: #f6f6f6; }
math[display="block"] { margin: 1em 0; overflow-x: auto; }
section[role="doc-endnotes"], section[role="doc-bibliography"] { font-size: 0.92em; }
@media print { body { max-width: none; margin: 0; padding: 0; } }
</style>"#;

/// Fügt `STIL` am Ende von `<head>` ein (nach dem Stil von typst, damit er ihn ergänzt). Ohne
/// `</head>` kommt er an den Anfang; ist er schon drin, bleibt alles, wie es ist.
pub fn mit_stil(html: &str) -> String {
    if html.contains("<style id=\"tftn\">") {
        return html.to_string();
    }
    match html.find("</head>") {
        Some(i) => format!("{}{STIL}{}", &html[..i], &html[i..]),
        None => format!("{STIL}{html}"),
    }
}

/// Name der Webseite: wie der Dokumentordner.
pub fn dateiname(projekt: &Projekt) -> String {
    let name = projekt
        .wurzel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Dokument".into());
    format!("{name}.html")
}

pub fn exportieren(projekt: &Projekt, cli: &tftn_export::TypstCli) -> Result<WebseiteJs, String> {
    let datei = dateiname(projekt);
    let ziel = projekt.wurzel.join(&datei);
    // Je Lauf ein eigener Zwischenordner, auch bei zwei Exporten in derselben Sekunde.
    static LAUF: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "tftn-html-lauf-{}-{}-{}",
        std::process::id(),
        crate::fassungen::jetzt(),
        LAUF.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let ergebnis = (|| -> Result<WebseiteJs, String> {
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
        std::fs::write(&ziel, mit_stil(&text)).map_err(|e| format!("{}: {e}", ziel.display()))?;
        Ok(WebseiteJs {
            pfad: ziel.display().to_string(),
            datei: datei.clone(),
            fehlt: crate::word::fehlt_in_word(&warnungen),
        })
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ergebnis
}

/// Öffnet die Webseite im Standardbrowser. Nur Dateien mit `.html` im offenen Dokument.
pub fn im_browser(wurzel: &Path, pfad: &Path) -> Result<(), String> {
    let p = crate::word::zeigbar(wurzel, pfad)
        .filter(|p| p.extension().is_some_and(|e| e == "html"))
        .ok_or_else(|| tr!("Diese Datei gehört nicht zum offenen Dokument."))?;
    crate::netzprotokoll::mit_system_oeffnen(&p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn stil_ans_ende_des_kopfs_und_nur_einmal() {
        let html = "<html><head><style>mtd{}</style></head><body>x</body></html>";
        let mit = mit_stil(html);
        let stil = mit.find("<style id=\"tftn\">").unwrap();
        assert!(mit.find("mtd{}").unwrap() < stil);
        assert!(stil < mit.find("</head>").unwrap());
        assert_eq!(mit_stil(&mit), mit);
        assert!(mit_stil("<p>ohne Kopf</p>").starts_with("<style id=\"tftn\">"));
    }

    #[test]
    fn testartikel_als_webseite() {
        let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        let o = std::env::temp_dir().join(format!("tftn-html-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let ordner = o.join("Mein Artikel");
        crate::typen::kopieren(&quelle, &ordner).unwrap();
        let _ = std::fs::remove_dir_all(ordner.join("_renders"));
        let p = Projekt::oeffnen(&ordner).unwrap();
        let cli = tftn_export::TypstCli::finden(&p.konfig.typst).expect("typst-CLI nötig");
        let w = exportieren(&p, &cli).unwrap();
        assert_eq!(w.datei, "Mein Artikel.html");
        assert!(
            w.fehlt.iter().any(|f| f.starts_with("overline")),
            "{:?}",
            w.fehlt
        );
        let html = std::fs::read_to_string(&w.pfad).unwrap();
        assert!(
            html.contains("<title>Über die Reproduzierbarkeit"),
            "{html}"
        );
        assert!(html.contains("<style id=\"tftn\">"));
        assert!(html.contains("<math"), "Formeln als MathML");
        assert!(html.contains("role=\"doc-endnotes\""), "Fußnoten");
        // Eine Datei ohne Verweise nach außen: keine Skripte, Stilblätter oder Bilder von anderswo.
        for fremd in ["<script", "<link ", "src=\"http", "url(http"] {
            assert!(!html.contains(fremd), "{fremd}");
        }
        // Nur .html des offenen Dokuments öffnet sich im Browser.
        assert!(im_browser(&ordner, &ordner.join("main.typ")).is_err());
        assert!(im_browser(&ordner, Path::new("/etc/hosts")).is_err());
        let _ = std::fs::remove_dir_all(&o);
    }
}
