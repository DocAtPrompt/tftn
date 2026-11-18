//! Import über Pandoc: .docx, .odt, .rtf, .md, .html, .epub, .tex → Typst; dazu eingefügter Text
//! (Markdown, HTML) aus der Zwischenablage (`umwandeln_text`).
//!
//! Pandoc läuft mit `--sandbox` (liest nur die angegebene Datei, holt nichts aus dem Netz). Bilder
//! in der Datei (docx, odt, epub) packt Pandoc nach `bilder/`; Bilder, auf die .md oder .html nur
//! verweisen, kopiert TFTN selbst, sofern sie neben der Datei liegen. Bilder aus dem Netz werden
//! nicht geholt, dafür gibt es einen Hinweis.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tftn_sprachen::tr;
use typst::syntax::ast::{self, AstNode};
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
        "Für den Import braucht TFTN das Programm Pandoc (ab 3.1.2). Mac: „brew install pandoc“, Windows: „winget install --id JohnMacFarlane.Pandoc“, Linux: „sudo apt install pandoc“. Danach TFTN neu starten."
    )
}

/// Meldung, wenn Pandoc für Word fehlt.
pub fn fehlt_fuer_word() -> String {
    tr!(
        "Für Word braucht TFTN das Programm Pandoc (ab 3.1.2). Mac: „brew install pandoc“, Windows: „winget install --id JohnMacFarlane.Pandoc“, Linux: „sudo apt install pandoc“. Danach TFTN neu starten."
    )
}

/// Meldung, wenn Pandoc zum Umwandeln eingefügten Textes fehlt.
pub fn fehlt_fuer_umwandeln() -> String {
    tr!(
        "Zum Umwandeln braucht TFTN das Programm Pandoc (ab 3.1.2). Mac: „brew install pandoc“, Windows: „winget install --id JohnMacFarlane.Pandoc“, Linux: „sudo apt install pandoc“. Danach TFTN neu starten."
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
        .arg(format!("--resource-path={}", ordner.display()));
    let (text, meldung) = laufen(pandoc, befehl, ziel, None)?;
    let mut hinweise = pandoc_hinweise(&meldung);
    let text = marken_entfernen(&text);
    let text = bilder_uebernehmen(&text, ordner, ziel, &mut hinweise);
    Ok(Umgewandelt { text, hinweise })
}

/// Text aus der Zwischenablage (Markdown wie auf GitHub, sonst HTML) nach Typst, für das Dokument
/// in `ziel`. Eingebettete Bilder landen unter `ziel/bilder/`, Bilder aus dem Netz nicht (Hinweis).
pub fn umwandeln_text(
    pandoc: &Path,
    text: &str,
    html: bool,
    ziel: &Path,
) -> Result<Umgewandelt, String> {
    let mut befehl = Command::new(pandoc);
    befehl.args(["-f", if html { "html" } else { "gfm" }]);
    let (aus, meldung) = laufen(pandoc, befehl, ziel, Some(text.to_string()))?;
    let mut hinweise = pandoc_hinweise(&meldung);
    let aus = marken_entfernen(&aus);
    let aus = bilder_uebernehmen(&aus, ziel, ziel, &mut hinweise);
    Ok(Umgewandelt {
        text: vereinfachen(&aus),
        hinweise,
    })
}

fn pandoc_hinweise(meldung: &str) -> Vec<String> {
    meldung
        .lines()
        .filter(|z| !z.trim().is_empty())
        .map(|z| format!("Pandoc: {}", z.trim()))
        .collect()
}

/// Pandoc mit den gemeinsamen Schaltern nach Typst laufen lassen, mit Frist; `eingabe` geht über
/// stdin. Liefert Ausgabe und Meldungen.
fn laufen(
    pandoc: &Path,
    mut befehl: Command,
    ziel: &Path,
    eingabe: Option<String>,
) -> Result<(String, String), String> {
    befehl
        .args([
            "--sandbox",
            "-t",
            "typst",
            "--wrap=none",
            "--extract-media=bilder",
        ])
        .current_dir(ziel)
        .stdin(if eingabe.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
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
    if let (Some(text), Some(mut ein)) = (eingabe, kind.stdin.take()) {
        // Eigener Faden: Pandoc kann schon schreiben, bevor es alles gelesen hat.
        std::thread::spawn(move || {
            let _ = std::io::Write::write_all(&mut ein, text.as_bytes());
        });
    }
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
    Ok((text, meldung))
}

/// Pandocs Typst lesbarer machen: `#strong[x]` → `*x*`, `#emph[x]` → `_x_` (nur wo das an
/// Wortgrenzen sicher geht) und Tabellen ohne die Hülle `#figure(align(center)[…], kind: table)`.
/// Ergäbe das mehr Syntaxfehler als vorher, bleibt der Text, wie er war.
pub fn vereinfachen(text: &str) -> String {
    let mut t = text.to_string();
    for _ in 0..8 {
        let source = Source::detached(t.as_str());
        let mut stellen: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        kurzformen(&LinkedNode::new(source.root()), &t, &mut stellen);
        if stellen.is_empty() {
            break;
        }
        stellen.sort_by_key(|(b, _)| std::cmp::Reverse(b.start));
        for (b, neu) in stellen {
            t.replace_range(b, &neu);
        }
    }
    let fehler = |s: &str| Source::detached(s).root().errors_and_warnings().0.len();
    if fehler(&t) > fehler(text) {
        return text.to_string();
    }
    t
}

/// Sucht von außen nach innen; was ersetzt wird, wird nicht weiter durchsucht (der nächste
/// Durchgang nimmt das Innere).
fn kurzformen(k: &LinkedNode, text: &str, aus: &mut Vec<(std::ops::Range<usize>, String)>) {
    use typst::syntax::SyntaxKind;
    if let Some(aufruf) = k.cast::<ast::FuncCall>()
        && let Some(hash) = k.prev_sibling()
        && hash.kind() == SyntaxKind::Hash
        && hash.offset() + 1 == k.offset()
        && let ast::Expr::Ident(name) = aufruf.callee()
    {
        let von = hash.offset();
        let bis = k.offset() + k.len();
        let neu = match name.as_str() {
            "strong" => kurz(&aufruf, text, von, bis, '*'),
            "emph" => kurz(&aufruf, text, von, bis, '_'),
            "figure" => tabelle_auspacken(&aufruf),
            _ => None,
        };
        if let Some(neu) = neu {
            aus.push((von..bis, neu));
            return;
        }
    }
    for c in k.children() {
        kurzformen(&c, text, aus);
    }
}

/// `#strong[x]` → `*x*`: genau ein Inhaltsblock, einzeilig, ohne Rand-Leerzeichen und ohne das
/// Zeichen selbst, davor und dahinter kein Wortzeichen.
fn kurz(
    aufruf: &ast::FuncCall,
    text: &str,
    von: usize,
    bis: usize,
    zeichen: char,
) -> Option<String> {
    let mut args = aufruf.args().items();
    let (Some(ast::Arg::Pos(ast::Expr::ContentBlock(block))), None) = (args.next(), args.next())
    else {
        return None;
    };
    let innen = block.body().to_untyped().full_text();
    let innen = innen.as_str();
    let grenze = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != zeichen);
    let ok = !innen.is_empty()
        && innen.trim() == innen
        && !innen.contains(zeichen)
        && !innen.contains('\n')
        && grenze(text[..von].chars().next_back())
        && grenze(text[bis..].chars().next());
    ok.then(|| format!("{zeichen}{innen}{zeichen}"))
}

/// `#figure(align(center)[#table(…)], kind: table)` ohne Unterschrift → `#table(…)`.
fn tabelle_auspacken(aufruf: &ast::FuncCall) -> Option<String> {
    let mut inhalt = None;
    for arg in aufruf.args().items() {
        match arg {
            ast::Arg::Named(n) if n.name().as_str() == "kind" => {
                if !matches!(n.expr(), ast::Expr::Ident(i) if i.as_str() == "table") {
                    return None;
                }
            }
            ast::Arg::Pos(ast::Expr::FuncCall(a)) if inhalt.is_none() => inhalt = Some(a),
            _ => return None,
        }
    }
    let ausrichtung = inhalt?;
    if !matches!(ausrichtung.callee(), ast::Expr::Ident(i) if i.as_str() == "align") {
        return None;
    }
    let mut args = ausrichtung.args().items();
    let (Some(ast::Arg::Pos(_)), Some(ast::Arg::Pos(ast::Expr::ContentBlock(block))), None) =
        (args.next(), args.next(), args.next())
    else {
        return None;
    };
    // Im Block steht nur `#table(…)`.
    let teile: Vec<_> = block
        .body()
        .to_untyped()
        .children()
        .filter(|c| !c.kind().is_trivia())
        .collect();
    let [hash, tabelle] = teile.as_slice() else {
        return None;
    };
    if hash.kind() != typst::syntax::SyntaxKind::Hash {
        return None;
    }
    let t = tabelle.cast::<ast::FuncCall>()?;
    if !matches!(t.callee(), ast::Expr::Ident(i) if i.as_str() == "table") {
        return None;
    }
    let roh = tabelle.full_text();
    let mut zeilen = Vec::new();
    for (i, z) in roh.lines().enumerate() {
        if i > 0 && nur_auto_ausrichtung(z) {
            continue;
        }
        zeilen.push(if i > 0 {
            z.strip_prefix("  ").unwrap_or(z)
        } else {
            z
        });
    }
    Some(format!("#{}", zeilen.join("\n")))
}

/// `align: (auto,auto,),` – Pandocs Vorgabe, die nichts bewirkt.
fn nur_auto_ausrichtung(zeile: &str) -> bool {
    let Some(innen) = zeile
        .trim()
        .strip_prefix("align: (")
        .and_then(|r| r.strip_suffix("),"))
    else {
        return false;
    };
    innen.split(',').all(|w| matches!(w.trim(), "" | "auto"))
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
    fn fett_und_kursiv_werden_kurz() {
        assert_eq!(
            vereinfachen("Text mit #strong[fett] und #emph[kursiv].\n"),
            "Text mit *fett* und _kursiv_.\n"
        );
        assert_eq!(vereinfachen("#strong[#emph[x]]\n"), "*_x_*\n");
    }

    #[test]
    fn fett_bleibt_lang_wo_kurz_nicht_geht() {
        for t in [
            "ab#strong[c]d\n",
            "#strong[a*b]\n",
            "#emph[a\\_b]\n",
            "#strong[ x]\n",
            "#strong[]\n",
            "#strong[a\nb]\n",
            "#strong(delta: 300)[x]\n",
        ] {
            assert_eq!(vereinfachen(t), t);
        }
    }

    #[test]
    fn tabelle_ohne_huelle() {
        let pandoc = "#figure(\n  align(center)[#table(\n    columns: 2,\n    align: (auto,auto,),\n    table.header([A], [B],),\n    table.hline(),\n    [1], [2],\n  )]\n  , kind: table\n  )\n";
        assert_eq!(
            vereinfachen(pandoc),
            "#table(\n  columns: 2,\n  table.header([A], [B],),\n  table.hline(),\n  [1], [2],\n)\n"
        );
        // Mit Unterschrift bleibt die Abbildung.
        let mit = "#figure(\n  align(center)[#table(\n    columns: 1,\n    [1],\n  )]\n  , caption: [Zahlen]\n  , kind: table\n  )\n";
        assert_eq!(vereinfachen(mit), mit);
    }

    /// Markdown und HTML aus der Zwischenablage; ohne Pandoc übersprungen.
    #[test]
    fn text_aus_der_zwischenablage() {
        let Some(pandoc) = pandoc_finden() else {
            eprintln!("Pandoc fehlt – Umwandeln übersprungen");
            return;
        };
        let o = std::env::temp_dir().join(format!("tftn-umwandeln-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        let md = "# Titel\n\nText mit **fett**, *kursiv* und [Link](https://x.at).\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n![Bild](https://x.at/b.png)\n";
        let u = umwandeln_text(&pandoc, md, false, &o).unwrap();
        assert!(u.text.starts_with("= Titel\n"), "{}", u.text);
        assert!(!u.text.contains("<titel>"), "{}", u.text);
        assert!(
            u.text.contains("*fett*") && u.text.contains("_kursiv_"),
            "{}",
            u.text
        );
        assert!(
            u.text.contains("#link(\"https://x.at\")[Link]"),
            "{}",
            u.text
        );
        assert!(
            u.text.contains("#table(") && !u.text.contains("#figure"),
            "{}",
            u.text
        );
        assert!(u.text.contains("bilder/fehlt.svg"), "{}", u.text);
        assert_eq!(u.hinweise.len(), 1, "{:?}", u.hinweise);
        let html = "<h2>Zwei</h2><p>Ein <b>fetter</b> Satz.</p><ul><li>a</li><li>b</li></ul>";
        let u = umwandeln_text(&pandoc, html, true, &o).unwrap();
        assert!(u.text.contains("== Zwei"), "{}", u.text);
        assert!(u.text.contains("*fetter*"), "{}", u.text);
        assert!(u.text.contains("- a\n- b"), "{}", u.text);
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
        let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../types");
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
