//! Mitgelieferte Paketbeschreibungen gegen die echten Pakete (PLAN-M2 AP10, Task 4; „fertig, wenn“):
//! je ein Daten-, Fluss- und frei gezeichnetes Diagramm nur aus Bausteinen.
//!
//! Braucht für die Gegenprobe die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

mod hilfe;

use std::path::Path;

use hilfe::*;
use tftn_core::Projekt;
use tftn_edit::{Wert, angebote_im_bereich, anwenden, wert_patch};
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren, exportieren};
use tftn_katalog::{Katalog, pruefen};
use typst_syntax::Source;

#[test]
fn mitgelieferte_beschreibungen_bestehen() {
    let (p, k, w) = fall("bestehen");
    assert!(w.is_empty(), "{w:?}");
    let befunde = pruefen(&p, &k).unwrap();
    assert_eq!(befunde, vec![]);
    let zahl = |vorne: &str| {
        k.eintraege()
            .iter()
            .filter(|e| e.id.starts_with(vorne))
            .count()
    };
    assert!(zahl("@preview/cetz/") >= 6);
    assert!(zahl("@preview/fletcher/") >= 3);
    assert!(zahl("@preview/cetz-plot/") >= 9);
}

#[test]
fn drei_diagramme_nur_aus_bausteinen() {
    let (p, k, _) = fall("drei");
    let leer = hauptdatei(&p);
    let mut text = leer.clone();
    anhaengen(&mut text, &k, "@preview/cetz-plot/linien");
    anhaengen(&mut text, &k, "@preview/fletcher/diagramm");
    anhaengen(&mut text, &k, "@preview/cetz/zeichnung");
    uebersetzt(&p, &text);

    let cli = TypstCli::finden(&p.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let (leer_pdf, voll, gegen) = (
        Path::new("leer.pdf"),
        Path::new("voll.pdf"),
        p.wurzel.join("gegenprobe.pdf"),
    );
    exportieren(&p, &cli, leer_pdf, &[]).unwrap();
    std::fs::write(p.wurzel.join(&p.konfig.haupt), &text).unwrap();
    exportieren(&p, &cli, voll, &[]).unwrap();
    ausfuehren(&p, &cli, &gegen, &[], STANDARD_FRIST).unwrap();
    let pdf = std::fs::read(p.wurzel.join(voll)).unwrap();
    assert_eq!(pdf, std::fs::read(&gegen).unwrap());
    assert_ne!(pdf, std::fs::read(p.wurzel.join(leer_pdf)).unwrap());
}

/// Setzt einen Wert über das Widget und prüft, dass es danach übersetzt.
fn aendern(
    p: &Projekt,
    k: &Katalog,
    text: &str,
    funktion: &str,
    name: &str,
    wert: Wert,
    erwartet: &str,
) {
    let q = Source::detached(text.to_string());
    let alle = angebote_im_bereich(&q, 0..text.len(), &parameter(k));
    let a = alle
        .iter()
        .find(|a| a.funktion == funktion && a.parameter == name)
        .unwrap_or_else(|| panic!("{funktion}.{name}: {alle:?}"));
    assert_eq!(a.gesperrt, None, "{funktion}.{name}");
    let patch = wert_patch(&q, a, &a.art, &wert).unwrap();
    let neu = anwenden(text, &patch);
    assert!(neu.contains(erwartet), "{neu}");
    uebersetzt(p, &neu);
}

#[test]
fn diagramm_werte_aendern() {
    let (p, k, _) = fall("werte");
    let punkt = |x: f64, y: f64| Wert::Punkt(vec![Wert::Zahl(x), Wert::Zahl(y)]);

    let mut t = hauptdatei(&p);
    anhaengen(&mut t, &k, "@preview/cetz/zeichnung");
    aendern(
        &p,
        &k,
        &t,
        "cetz.draw.line",
        "nach",
        punkt(3.0, 4.0),
        "(3, 4)",
    );

    let mut t = hauptdatei(&p);
    anhaengen(&mut t, &k, "@preview/fletcher/diagramm");
    aendern(
        &p,
        &k,
        &t,
        "fletcher.node",
        "text",
        Wert::Inhalt("Beginn".into()),
        "[Beginn]",
    );
    aendern(
        &p,
        &k,
        &t,
        "fletcher.edge",
        "marks",
        Wert::Text("<|-|>".into()),
        "<|-|>",
    );

    let mut t = hauptdatei(&p);
    anhaengen(&mut t, &k, "@preview/cetz-plot/linien");
    let liste = Wert::Liste(vec![punkt(0.0, 1.0), punkt(5.0, 6.0)]);
    aendern(&p, &k, &t, "cetz-plot.plot.add", "daten", liste, "(5, 6)");
    aendern(
        &p,
        &k,
        &t,
        "cetz-plot.plot.plot",
        "x-label",
        Wert::Inhalt("Zeit".into()),
        "[Zeit]",
    );
}

#[test]
fn importierte_form_as_und_items() {
    let (p, k, _) = fall("formen");
    let mut t = format!(
        "#import \"@preview/cetz:0.5.3\": canvas\n{}",
        hauptdatei(&p)
    );
    anhaengen(&mut t, &k, "@preview/cetz/zeichnung");
    assert_eq!(t.matches("@preview/cetz:0.5.3").count(), 2, "{t}");
    uebersetzt(&p, &t);
}

#[test]
fn plot_und_zeichnung_mit_vorhandenem_cetz() {
    let (p, k, _) = fall("zusammen");
    let mut t = hauptdatei(&p);
    anhaengen(&mut t, &k, "@preview/cetz/zeichnung");
    anhaengen(&mut t, &k, "@preview/cetz-plot/kreis");
    anhaengen(&mut t, &k, "@preview/cetz-plot/linien");
    assert_eq!(t.matches("#import \"@preview/cetz:").count(), 1, "{t}");
    uebersetzt(&p, &t);
}
