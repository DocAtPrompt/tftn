//! Import über Pandoc: .docx, .odt, .rtf, .md, .html, .epub, .tex → Typst.
//!
//! Pandoc läuft mit `--sandbox` (liest nur die angegebene Datei, holt nichts aus dem Netz). Bilder
//! in der Datei (docx, odt, epub) packt Pandoc nach `bilder/`; Bilder, auf die .md oder .html nur
//! verweisen, kopiert TFTN selbst, sofern sie neben der Datei liegen. Bilder aus dem Netz werden
//! nicht geholt, dafür gibt es einen Hinweis.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tftn_sprachen::tr;
use typst::syntax::ast;
use typst::syntax::{LinkedNode, Source};

/// Dateiendungen, die der Import annimmt.
pub const ENDUNGEN: &[&str] = &[
    "docx", "odt", "rtf", "md", "markdown", "html", "htm", "epub", "tex", "latex",
];

/// Pandoc darf so lange laufen, dann wird abgebrochen.
const FRIST: Duration = Duration::from_secs(120);

/// Meldung, wenn Pandoc für den Import fehlt.
pub fn fehlt() -> String {
    tr!(
        "Für den Import braucht TFTN das Programm Pandoc (ab 3.1.2). Mac: „brew install pandoc“, Windows: „winget install --id JohnMacFarlane.Pandoc“. Danach TFTN neu starten."
    )
}

/// Meldung, wenn Pandoc für Word fehlt.
pub fn fehlt_fuer_word() -> String {
    tr!(
        "Für Word braucht TFTN das Programm Pandoc (ab 3.1.2). Mac: „brew install pandoc“, Windows: „winget install --id JohnMacFarlane.Pandoc“. Danach TFTN neu starten."
    )
}

/// Pandoc: `TFTN_PANDOC`, der Suchpfad, dann die üblichen Orte (eine App vom Mac hat Homebrew
/// nicht im Suchpfad).
pub fn pandoc_finden() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("TFTN_PANDOC").map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    let name = if cfg!(windows) {
        "pandoc.exe"
    } else {
        "pandoc"
    };
    let mut orte: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    orte.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    if let Some(lokal) = std::env::var_os("LOCALAPPDATA") {
        orte.push(PathBuf::from(lokal).join("Pandoc"));
    }
    if let Some(programme) = std::env::var_os("ProgramFiles") {
        orte.push(PathBuf::from(programme).join("Pandoc"));
    }
    orte.into_iter().map(|o| o.join(name)).find(|p| p.is_file())
}

pub struct Umgewandelt {
    pub text: String,
    pub hinweise: Vec<String>,
}

/// Wandelt `datei` nach Typst; Bilder landen unter `ziel/bilder/`.
pub fn umwandeln(pandoc: &Path, datei: &Path, ziel: &Path) -> Result<Umgewandelt, String> {
    let endung = datei
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !ENDUNGEN.contains(&endung.as_str()) {
        return Err(tr!(
            "Dieses Format kann der Import nicht lesen (.{endung}). Möglich: {moeglich}.",
            endung,
            moeglich = ENDUNGEN
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let datei = datei
        .canonicalize()
        .map_err(|e| format!("{}: {e}", datei.display()))?;
    let ordner = datei.parent().unwrap_or(Path::new("."));
    let mut befehl = Command::new(pandoc);
    befehl
        .arg(&datei)
        .args([
            "--sandbox",
            "-t",
            "typst",
            "--wrap=none",
            "--extract-media=bilder",
        ])
        .arg(format!("--resource-path={}", ordner.display()))
        .current_dir(ziel)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        befehl.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut kind = befehl.spawn().map_err(|e| {
        tr!(
            "Pandoc ({pfad}) startet nicht: {fehler}",
            pfad = pandoc.display(),
            fehler = e
        )
    })?;
    let mut aus = kind.stdout.take().expect("umgeleitet");
    let mut fehler = kind.stderr.take().expect("umgeleitet");
    let leser = std::thread::spawn(move || {
        let mut t = String::new();
        let _ = std::io::Read::read_to_string(&mut aus, &mut t);
        t
    });
    let fehlerleser = std::thread::spawn(move || {
        let mut t = String::new();
        let _ = std::io::Read::read_to_string(&mut fehler, &mut t);
        t
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = kind.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > FRIST {
            let _ = kind.kill();
            let _ = kind.wait();
            return Err(tr!(
                "Pandoc hat nach {sekunden} s nicht fertig.",
                sekunden = FRIST.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = leser.join().unwrap_or_default();
    let meldung = fehlerleser.join().unwrap_or_default();
    if !status.success() {
        return Err(tr!("Pandoc meldet: {meldung}", meldung = meldung.trim()));
    }
    let mut hinweise: Vec<String> = meldung
        .lines()
        .filter(|z| !z.trim().is_empty())
        .map(|z| format!("Pandoc: {}", z.trim()))
        .collect();
    let text = marken_entfernen(&text);
    let text = bilder_uebernehmen(&text, ordner, ziel, &mut hinweise);
    Ok(Umgewandelt { text, hinweise })
}

/// Pandoc setzt hinter jede Überschrift eine Marke (`<einleitung>`). Wird sie nirgends benutzt,
/// stört sie nur beim Schreiben.
pub fn marken_entfernen(text: &str) -> String {
    let zeilen: Vec<&str> = text.lines().collect();
    let mut aus = Vec::with_capacity(zeilen.len());
    for (i, z) in zeilen.iter().enumerate() {
        let marke = z.trim();
        let nach_ueberschrift = i > 0 && zeilen[i - 1].trim_start().starts_with('=');
        if nach_ueberschrift
            && marke.starts_with('<')
            && marke.ends_with('>')
            && !marke.contains(' ')
        {
            let name = &marke[1..marke.len() - 1];
            let benutzt =
                text.matches(&format!("@{name}")).count() > 0 || text.matches(marke).count() > 1;
            if !benutzt {
                continue;
            }
        }
        aus.push(*z);
    }
    let mut t = aus.join("\n");
    if text.ends_with('\n') {
        t.push('\n');
    }
    t
}

/// Bilder, die nur verlinkt sind: neben der Quelldatei suchen, nach `bilder/` kopieren und den
/// Pfad anpassen. Bilder aus dem Netz bleiben weg (Hinweis).
pub fn bilder_uebernehmen(
    text: &str,
    quelle: &Path,
    ziel: &Path,
    hinweise: &mut Vec<String>,
) -> String {
    let source = Source::detached(text);
    let mut aenderungen: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    fn gehen(k: &LinkedNode, aus: &mut Vec<(std::ops::Range<usize>, String)>) {
        if let Some(aufruf) = k.cast::<ast::FuncCall>()
            && matches!(aufruf.callee(), ast::Expr::Ident(i) if i.as_str() == "image")
            && let Some(ast::Arg::Pos(ast::Expr::Str(s))) = aufruf.args().items().next()
        {
            let bereich = k
                .children()
                .find(|c| c.kind() == typst::syntax::SyntaxKind::Args)
                .and_then(|a| {
                    a.children()
                        .find(|c| c.kind() == typst::syntax::SyntaxKind::Str)
                })
                .map(|c| c.range());
            if let Some(b) = bereich {
                aus.push((b, s.get().to_string()));
            }
        }
        for c in k.children() {
            gehen(&c, aus);
        }
    }
    let mut stellen = Vec::new();
    gehen(&LinkedNode::new(source.root()), &mut stellen);
    for (bereich, pfad) in stellen {
        if pfad.starts_with("http://") || pfad.starts_with("https://") {
            hinweise.push(tr!("Bild aus dem Netz nicht übernommen: {pfad}", pfad));
            aenderungen.push((bereich, "\"bilder/fehlt.svg\"".into()));
            continue;
        }
        if ziel.join(&pfad).is_file() {
            continue;
        }
        let von = quelle.join(&pfad);
        let Some(name) = von.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        if !von.is_file() {
            hinweise.push(tr!("Bild nicht gefunden: {pfad}", pfad));
            aenderungen.push((bereich, "\"bilder/fehlt.svg\"".into()));
            continue;
        }
        let ordner = ziel.join("bilder");
        let _ = std::fs::create_dir_all(&ordner);
        let mut nach = ordner.join(&name);
        let mut n = 2;
        while nach.exists() {
            nach = ordner.join(format!("{n}-{name}"));
            n += 1;
        }
        if std::fs::copy(&von, &nach).is_ok() {
            let neu = format!("bilder/{}", nach.file_name().unwrap().to_string_lossy());
            aenderungen.push((bereich, format!("\"{}\"", neu.replace('"', "\\\""))));
        }
    }
    if aenderungen.iter().any(|(_, t)| t.contains("fehlt.svg")) {
        let _ = std::fs::create_dir_all(ziel.join("bilder"));
        let _ = std::fs::write(
            ziel.join("bilder/fehlt.svg"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"40\"><rect width=\"120\" height=\"40\" fill=\"#eee\" stroke=\"#999\"/><text x=\"60\" y=\"25\" font-size=\"12\" text-anchor=\"middle\" fill=\"#666\">Bild fehlt</text></svg>",
        );
    }
    let mut t = text.to_string();
    aenderungen.sort_by_key(|(b, _)| std::cmp::Reverse(b.start));
    for (b, neu) in aenderungen {
        t.replace_range(b, &neu);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marken_hinter_ueberschriften() {
        let t = "= Titel\n<titel>\nText\n== Zwei\n<zwei>\nSiehe @zwei.\n";
        assert_eq!(
            marken_entfernen(t),
            "= Titel\nText\n== Zwei\n<zwei>\nSiehe @zwei.\n"
        );
    }

    #[test]
    fn verlinkte_bilder_werden_kopiert() {
        let o = std::env::temp_dir().join(format!("tftn-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let (quelle, ziel) = (o.join("quelle"), o.join("ziel"));
        std::fs::create_dir_all(&quelle).unwrap();
        std::fs::create_dir_all(&ziel).unwrap();
        std::fs::write(quelle.join("a.png"), b"x").unwrap();
        let mut h = Vec::new();
        let t = "#image(\"a.png\")\n#image(\"https://x.org/b.png\")\n#image(\"weg.png\")\n";
        let neu = bilder_uebernehmen(t, &quelle, &ziel, &mut h);
        assert_eq!(
            neu,
            "#image(\"bilder/a.png\")\n#image(\"bilder/fehlt.svg\")\n#image(\"bilder/fehlt.svg\")\n"
        );
        assert!(ziel.join("bilder/a.png").is_file());
        assert!(ziel.join("bilder/fehlt.svg").is_file());
        assert_eq!(h.len(), 2, "{h:?}");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn unbekanntes_format() {
        let e = umwandeln(Path::new("pandoc"), Path::new("x.pages"), Path::new("."));
        assert!(e.err().unwrap().contains(".pages"));
    }

    /// Alle Beispieldateien aus `tests/korpus/import` mit dem installierten Pandoc; ohne Pandoc
    /// übersprungen (die CI hat keines).
    #[test]
    fn beispiele_uebersetzen() {
        let Some(pandoc) = pandoc_finden() else {
            eprintln!("Pandoc fehlt – Import-Beispiele übersprungen");
            return;
        };
        let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../typen");
        let korpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/import");
        let o = std::env::temp_dir().join(format!("tftn-import-bsp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        let q = crate::typen::quellen(&typen, &o);
        tftn_core::schriftvorrat_setzen(crate::typen::schriftvorrat(&q));
        for endung in ["md", "docx", "odt", "html", "epub", "tex", "rtf"] {
            let ziel = crate::typen::entwurf_sichern(&q, &o.join(endung)).unwrap();
            let u = umwandeln(&pandoc, &korpus.join(format!("beispiel.{endung}")), &ziel)
                .unwrap_or_else(|e| panic!("{endung}: {e}"));
            assert!(u.text.contains("Rücklaufquoten"), "{endung}: {}", u.text);
            assert!(!u.text.contains("<methode>"), "{endung}: Marke blieb");
            // RTF liest Pandoc nur mit Grundformatierung (keine Fußnoten, Formeln als Text).
            if endung != "rtf" {
                assert!(u.text.contains("#footnote"), "{endung}");
                assert!(u.text.contains("image(\"bilder/"), "{endung}: {}", u.text);
            }
            assert!(u.hinweise.is_empty(), "{endung}: {:?}", u.hinweise);
            let main = ziel.join("main.typ");
            let vorlage = std::fs::read_to_string(&main).unwrap();
            std::fs::write(&main, format!("{vorlage}\n{}", u.text)).unwrap();
            let p = tftn_core::Projekt::oeffnen(&ziel).unwrap();
            let k = tftn_core::kompilieren(&tftn_core::ProjektWorld::neu(&p).unwrap());
            assert!(
                k.meldungen.iter().all(|m| !m.fehler),
                "{endung}: {:?}\n{}",
                k.meldungen,
                u.text
            );
        }
        let _ = std::fs::remove_dir_all(&o);
    }
}
