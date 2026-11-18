//! Export als E-Book (EPUB 3) auf demselben Weg wie Word (`word::ueber_pandoc`): typst setzt das
//! Dokument als HTML (mit Vorlage, Zitaten, Literaturverzeichnis, Formeln als MathML, Bilder
//! eingebettet), Pandoc macht daraus im abgeschotteten Modus ein EPUB mit Titelseite,
//! Inhaltsverzeichnis, einer Datei je Hauptkapitel, Bildern und Formeln als MathML. Das Seitenlayout
//! (Ränder, Kopfzeilen, Seitenzahlen) gibt es im E-Book nicht; Schrift und Größe wählt der Leser.
//!
//! Pandoc liest mit `--sandbox` nur die genannte HTML-Datei, auch kein Stilblatt (`--css`) und kein
//! Titelbild (`--epub-cover-image`) von der Platte. Das Stilblatt kommt deshalb als Text in den Kopf
//! jeder Seite (`-V header-includes`), ein Titelbild gibt es nicht.

use tftn_core::Projekt;

use crate::word::{WordJs, stuecke, wert};

/// Schlichte Grundform zum Lesen, ergänzt Pandocs eigenes Stilblatt. Keine Schriften und Farben:
/// die wählt der Leser (auch Nachtmodus). Ohne `"`, `<` und `&`, weil der Text als Variable an Pandoc
/// geht und unverändert in `<style>` landet.
pub const STIL: &str = "\
body { line-height: 1.5; hyphens: auto; -webkit-hyphens: auto; overflow-wrap: break-word; }
h1, h2, h3, h4, h5, h6 { line-height: 1.25; hyphens: manual; -webkit-hyphens: manual; text-align: left; page-break-after: avoid; break-after: avoid; }
p { margin: 0 0 0.6em; }
img, svg { max-width: 100%; height: auto; }
figure { margin: 1.2em 0; text-align: center; page-break-inside: avoid; break-inside: avoid; }
figcaption { font-size: 0.9em; margin-top: 0.4em; }
table { border-collapse: collapse; margin: 1em auto; max-width: 100%; }
th, td { border: 1px solid #999; padding: 0.25em 0.5em; text-align: left; vertical-align: top; }
blockquote { margin: 1em 0; padding-left: 1em; border-left: 3px solid #ccc; }
pre { white-space: pre-wrap; font-size: 0.9em; }
math[display=block] { margin: 1em 0; overflow-x: auto; }
section.footnotes, aside.footnote, section[role=doc-endnotes], section[role=doc-bibliography] { font-size: 0.9em; }
";

/// Name des E-Books: wie der Dokumentordner.
pub fn dateiname(projekt: &Projekt) -> String {
    crate::word::dateiname_mit(projekt, "epub")
}

/// Kennung des Buchs (`dc:identifier`), aus Ordnername und Titel: bleibt bei jedem neuen Export
/// gleich, damit ein Lesegerät das Buch ersetzt statt es doppelt zu führen. Ohne sie gäbe Pandoc im
/// abgeschotteten Modus jedem Buch dieselbe.
pub fn kennung(ordner: &str, titel: &str) -> String {
    let h = tftn_pakete::archiv::sha256_hex(format!("tftn-epub\n{ordner}\n{titel}").as_bytes());
    let variante = (u8::from_str_radix(&h[16..17], 16).unwrap_or(0) & 0x3) | 0x8;
    format!(
        "urn:uuid:{}-{}-5{}-{variante:x}{}-{}",
        &h[0..8],
        &h[8..12],
        &h[13..16],
        &h[17..20],
        &h[20..32]
    )
}

/// HTML von typst für das EPUB: wie für Word (`html_fuer_pandoc`), dazu je Autor ein eigenes
/// `<meta name="author">` (typst trennt sie mit Komma), Datum und Kennung im Kopf.
pub fn html_fuer_epub(html: &str, datum: &str, ordner: &str) -> String {
    let s = crate::word::html_fuer_pandoc(html);
    let s = stuecke(&s, "<meta name=\"author\" ", ">", |t| {
        let namen = wert(t, "content")?;
        Some(
            namen
                .split(", ")
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(|n| format!("<meta name=\"author\" content=\"{n}\">"))
                .collect(),
        )
    });
    let titel = s
        .find("<title>")
        .and_then(|a| {
            let rest = &s[a + 7..];
            rest.find("</title>").map(|e| &rest[..e])
        })
        .unwrap_or("");
    let mut kopf = String::new();
    if !s.contains("<meta name=\"date\"") {
        kopf.push_str(&format!("<meta name=\"date\" content=\"{datum}\">"));
    }
    if !s.contains("<meta name=\"identifier\"") {
        kopf.push_str(&format!(
            "<meta name=\"identifier\" content=\"{}\">",
            kennung(ordner, titel)
        ));
    }
    match s.find("</head>") {
        Some(i) => format!("{}{kopf}{}", &s[..i], &s[i..]),
        None => format!("{kopf}{s}"),
    }
}

/// Datum des Buchs: das Formularfeld, das der Dokumenttyp als Pandoc-`date` auszeichnet
/// (`x-tftn-pandoc`), sonst der Zeitstempel des Projekts. Immer `JJJJ-MM-TT`.
pub fn datum(projekt: &Projekt) -> String {
    datum_aus_formular(projekt).unwrap_or_else(|| crate::pakete::datum(projekt.epoche()))
}

fn datum_aus_formular(projekt: &Projekt) -> Option<String> {
    let f = crate::formular::FormularSitzung::neu(projekt);
    let feld = f
        .schema()?
        .felder
        .iter()
        .find(|x| x.pandoc.as_deref() == Some("date"))?;
    let daten: toml::Table = std::fs::read_to_string(f.datei()).ok()?.parse().ok()?;
    let (j, m, t) = match daten.get(&feld.name)? {
        toml::Value::Datetime(d) => {
            let d = d.date?;
            (d.year, d.month, d.day)
        }
        toml::Value::String(s) => tftn_formular::csv::datum_lesen(s)?,
        _ => return None,
    };
    Some(format!("{j:04}-{m:02}-{t:02}"))
}

/// Die Schalter für Pandoc nach `-t epub3` (ab Pandoc 3.1.2, wie der Import):
/// typst setzt Überschriften der ersten Ebene als `<h2>` (`<h1>` ist dem Titel vorbehalten), daher
/// eine Ebene hinauf; je Hauptkapitel eine Datei, Inhaltsverzeichnis über zwei Ebenen.
pub fn pandoc_schalter() -> Vec<String> {
    [
        "--shift-heading-level-by=-1",
        "--split-level=1",
        "--toc",
        "--toc-depth=2",
        "--mathml",
        "-V",
    ]
    .into_iter()
    .map(String::from)
    .chain([format!("header-includes=<style>\n{STIL}</style>")])
    .collect()
}

pub fn exportieren(
    projekt: &Projekt,
    cli: &tftn_export::TypstCli,
    pandoc: &std::path::Path,
) -> Result<WordJs, String> {
    let datei = dateiname(projekt);
    let ziel = projekt.wurzel.join(&datei);
    let ordner = projekt
        .wurzel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tag = datum(projekt);
    let fehlt = crate::word::ueber_pandoc(
        projekt,
        cli,
        pandoc,
        "epub3",
        &pandoc_schalter(),
        |html| html_fuer_epub(html, &tag, &ordner),
        &ziel,
    )?;
    Ok(WordJs {
        pfad: ziel.display().to_string(),
        datei,
        fehlt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn autoren_einzeln_datum_und_kennung_im_kopf() {
        let html = "<html><head><title>Buch &amp; Co</title><meta name=\"authors\" content=\"A, B &amp; C\"></head><body></body></html>";
        let neu = html_fuer_epub(html, "2026-01-15", "Mein Buch");
        assert!(
            neu.contains(
                "<meta name=\"author\" content=\"A\"><meta name=\"author\" content=\"B &amp; C\">"
            ),
            "{neu}"
        );
        assert!(neu.contains("<meta name=\"date\" content=\"2026-01-15\">"));
        let k = kennung("Mein Buch", "Buch &amp; Co");
        assert!(neu.contains(&format!(
            "<meta name=\"identifier\" content=\"{k}\"></head>"
        )));
        // Zweimal angewandt ändert sich am Kopf nichts mehr.
        assert_eq!(html_fuer_epub(&neu, "2027-01-01", "Mein Buch"), neu);
    }

    #[test]
    fn kennung_ist_eine_feste_uuid() {
        let k = kennung("Ordner", "Titel");
        assert_eq!(k, kennung("Ordner", "Titel"));
        assert_ne!(k, kennung("Ordner", "Anderer Titel"));
        assert_ne!(k, kennung("Anderer Ordner", "Titel"));
        let u = k.strip_prefix("urn:uuid:").unwrap();
        let teile: Vec<&str> = u.split('-').collect();
        assert_eq!(
            teile.iter().map(|t| t.len()).collect::<Vec<_>>(),
            [8, 4, 4, 4, 12]
        );
        assert!(teile[2].starts_with('5'));
        assert!(matches!(
            teile[3].chars().next(),
            Some('8' | '9' | 'a' | 'b')
        ));
    }

    #[test]
    fn stil_ohne_zeichen_die_im_kopf_stoeren() {
        for z in ['"', '<', '&'] {
            assert!(!STIL.contains(z), "{z}");
        }
        assert!(STIL.contains("img, svg { max-width: 100%"));
        let s = pandoc_schalter();
        assert!(s.last().unwrap().starts_with("header-includes=<style>"));
        assert!(s.last().unwrap().ends_with("</style>"));
    }

    /// Ein Eintrag aus dem EPUB.
    fn eintrag<'a>(dateien: &'a [(String, Vec<u8>)], name: &str) -> &'a str {
        let (_, d) = dateien
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} fehlt"));
        std::str::from_utf8(d).unwrap()
    }

    /// Der Testartikel (mit einer Abbildung dazu) ganz durch typst und Pandoc; ohne Pandoc
    /// übersprungen (die CI hat keines).
    #[test]
    fn testartikel_als_epub() {
        let Some(pandoc) = crate::import::pandoc_finden() else {
            eprintln!("Pandoc fehlt – EPUB-Export übersprungen");
            return;
        };
        let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        let o = std::env::temp_dir().join(format!("tftn-epub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let ordner = o.join("Mein Artikel");
        crate::typen::kopieren(&quelle, &ordner).unwrap();
        let _ = std::fs::remove_dir_all(ordner.join("_renders"));
        std::fs::create_dir_all(ordner.join("bilder")).unwrap();
        std::fs::write(
            ordner.join("bilder/balken.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"20\"><rect width=\"40\" height=\"20\" fill=\"#48c\"/></svg>",
        )
        .unwrap();
        let main = ordner.join("main.typ");
        let text = std::fs::read_to_string(&main).unwrap();
        let text = text.replacen(
            "= Diskussion",
            "#figure(image(\"bilder/balken.svg\", width: 60%), caption: [Balken])\n\n= Diskussion",
            1,
        );
        assert!(
            text.contains("bilder/balken.svg"),
            "keine Stelle für das Bild"
        );
        std::fs::write(&main, text).unwrap();
        let p = Projekt::oeffnen(&ordner).unwrap();
        let cli = tftn_export::TypstCli::finden(&p.konfig.typst).expect("typst-CLI nötig");
        let e = exportieren(&p, &cli, &pandoc).unwrap();
        assert_eq!(e.datei, "Mein Artikel.epub");
        assert!(
            e.fehlt.iter().any(|f| f.starts_with("overline")),
            "{:?}",
            e.fehlt
        );
        let roh = std::fs::read(&e.pfad).unwrap();
        // Ein EPUB ist ein Zip, das mit der unkomprimierten Datei `mimetype` beginnt.
        assert_eq!(&roh[..4], b"PK\x03\x04");
        assert_eq!(&roh[30..38], b"mimetype");
        assert_eq!(&roh[38..58], b"application/epub+zip");
        let dateien = tftn_export::zip::inhalt(&roh).unwrap();
        let namen: Vec<&str> = dateien.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(namen[0], "mimetype");
        // Metadaten: Titel, beide Autoren, Sprache, Datum aus dem Formular, feste Kennung.
        let opf = eintrag(&dateien, "EPUB/content.opf");
        assert!(
            opf.contains(">Über die Reproduzierbarkeit ärztlicher Dokumentation</dc:title>"),
            "{opf}"
        );
        assert!(opf.contains(">Erika Musterfrau</dc:creator>"), "{opf}");
        assert!(opf.contains(">Jörg Österreicher</dc:creator>"), "{opf}");
        assert!(opf.contains("<dc:language>de-AT</dc:language>"), "{opf}");
        assert!(opf.contains(">2026-01-15</dc:date>"), "{opf}");
        assert!(
            opf.contains(&kennung(
                "Mein Artikel",
                "Über die Reproduzierbarkeit ärztlicher Dokumentation"
            )),
            "{opf}"
        );
        // Inhaltsverzeichnis mit Kapiteln und Unterabschnitten, je Hauptkapitel eine Datei.
        let nav = eintrag(&dateien, "EPUB/nav.xhtml");
        assert!(nav.contains("epub:type=\"toc\""), "{nav}");
        assert!(nav.contains("2 Methode</a>"), "{nav}");
        assert!(nav.contains("2.1 Datengrundlage</a>"), "{nav}");
        let kapitel = namen
            .iter()
            .filter(|n| n.starts_with("EPUB/text/ch"))
            .count();
        assert!(kapitel >= 5, "{namen:?}");
        // Das Bild steckt im Buch, die Formeln sind MathML, das Stilblatt ist im Kopf.
        assert!(
            namen.iter().any(|n| n.starts_with("EPUB/media/")),
            "{namen:?}"
        );
        let texte: String = dateien
            .iter()
            .filter(|(n, _)| n.starts_with("EPUB/text/"))
            .map(|(_, d)| String::from_utf8_lossy(d).into_owned())
            .collect();
        assert!(texte.contains("<img src=\"../media/"), "Bild fehlt");
        assert!(texte.contains("<math"), "Formeln als MathML");
        assert!(texte.contains("img, svg { max-width: 100%"), "Stilblatt");
        // Nichts verweist nach außen.
        for fremd in ["src=\"http", "href=\"http://fonts", "url(http"] {
            assert!(!texte.contains(fremd), "{fremd}");
        }
        let _ = std::fs::remove_dir_all(&o);
    }
}
