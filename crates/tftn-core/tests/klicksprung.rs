//! Abnahme AP5 (PLAN-M1): 20 Klickpunkte im Text aus `main.typ`, mindestens 19 richtig;
//! Klicks auf Formularwerte und Vorlagentext ergeben `Sprung::Vorlage`, keinen falschen Sprung.
//!
//! Die Klickpunkte kommen unabhängig von typst-ide aus dem Seitenlayout: Wo ein Wort gesetzt ist,
//! wird mitten auf seinen ersten Buchstaben geklickt. Erwartet wird ein Sprung in den Bereich der
//! Quelle, aus dem das Wort stammt.

use std::path::Path;

use tftn_core::Projekt;
use tftn_core::vorschau::{Sprung, Vorschau};
use typst::layout::{Frame, FrameItem};
use typst::syntax::Source;
use typst::text::TextItem;

struct Fundstelle<'a> {
    seite: usize,
    x: f64,
    y: f64,
    item: &'a TextItem,
}

fn sammeln<'a>(frame: &'a Frame, seite: usize, dx: f64, dy: f64, aus: &mut Vec<Fundstelle<'a>>) {
    for (pos, item) in frame.items() {
        let (x, y) = (dx + pos.x.to_pt(), dy + pos.y.to_pt());
        match item {
            FrameItem::Group(g) => sammeln(
                &g.frame,
                seite,
                x + g.transform.tx.to_pt(),
                y + g.transform.ty.to_pt(),
                aus,
            ),
            FrameItem::Text(t) => aus.push(Fundstelle {
                seite,
                x,
                y,
                item: t,
            }),
            _ => {}
        }
    }
}

/// Klickpunkt mitten auf dem ersten Glyph der `n`-ten gesetzten Fundstelle von `wort`.
fn klickpunkt(stellen: &[Fundstelle], wort: &str, n: usize) -> Option<(usize, f64, f64)> {
    let mut gefunden = 0;
    for f in stellen {
        let text = f.item.text.as_str();
        for (idx, _) in text.match_indices(wort) {
            if gefunden == n {
                let groesse = f.item.size;
                let mut x = f.x;
                for g in &f.item.glyphs {
                    let breite = g.x_advance.at(groesse).to_pt();
                    if g.range().start >= idx {
                        return Some((f.seite, x + breite / 2.0, f.y - groesse.to_pt() * 0.35));
                    }
                    x += breite;
                }
            }
            gefunden += 1;
        }
    }
    None
}

fn bereich(quelle: &str, schnipsel: &str) -> std::ops::Range<usize> {
    assert_eq!(
        quelle.matches(schnipsel).count(),
        1,
        "Schnipsel nicht eindeutig: {schnipsel}"
    );
    let start = quelle.find(schnipsel).unwrap();
    start..start + schnipsel.len()
}

#[test]
fn zwanzig_klickpunkte() {
    let projekt = Projekt::oeffnen(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel"),
    )
    .unwrap();
    let mut vorschau = Vorschau::neu(&projekt).unwrap();
    let text = std::fs::read_to_string(projekt.haupt()).unwrap();
    let quelle = Source::new(vorschau.world().haupt(), text.clone());
    assert!(vorschau.lauf(1, Some(quelle.clone())).gelungen);

    let dokument = &vorschau.gut().unwrap().dokument;
    let mut stellen = Vec::new();
    for (nr, seite) in dokument.pages().iter().enumerate() {
        sammeln(&seite.frame, nr, 0.0, 0.0, &mut stellen);
    }
    stellen.sort_by(|a, b| {
        (a.seite, a.y, a.x)
            .partial_cmp(&(b.seite, b.y, b.x))
            .unwrap()
    });

    // (Art, gesetztes Wort, Fundstelle Nr., eindeutiger Quelltext-Ausschnitt)
    let ziele: [(&str, &str, usize, &str); 20] = [
        ("Fließtext", "Briefköpfe", 0, "Briefköpfe, Schriftgrößen"),
        ("Fließtext", "Durchschuss", 0, "Zeilenlänge, Durchschuss"),
        ("Fließtext", "Jahrgängen", 0, "aus drei Jahrgängen"),
        ("Fließtext", "Klartextwerkzeuge", 0, "für Klartextwerkzeuge"),
        ("Fließtext", "Hausschrift", 0, "eine neue Hausschrift"),
        ("Fließtext", "Öffnungszeiten", 0, "Öffnungszeiten, Füße"),
        ("Fließtext", "Register", 0, "Register, Querverweise"),
        ("Überschrift", "Methode", 0, "= Methode"),
        ("Überschrift", "Zitierweise", 0, "== Zitierweise"),
        ("Überschrift", "Diskussion", 0, "= Diskussion"),
        ("Fußnote", "Überweisungen", 0, "Für Überweisungen gilt"),
        ("Fußnote", "Mehrere", 0, "Mehrere Quellen an einer Stelle"),
        ("Zitat", "Smith", 0, "@smithTypesettingClinicalReports2023"),
        (
            "Zitat",
            "Lüddecke",
            1,
            "@lueddeckeZitierenInDerMedizin2020[S. 52]",
        ),
        ("Tabellenzelle", "402", 0, "[402]"),
        ("Tabellenzelle", "Jahrgang", 0, "[*Jahrgang*]"),
        ("Tabellenzelle", "Summe", 0, "[Summe/Ø]"),
        ("Formel", "Δ", 0, "$ Delta = (overline"),
        ("Formel", "≈", 0, "approx"),
        (
            "Code",
            "Libertinus",
            0,
            "#set text(font: \"Libertinus Serif\", size: 11pt, lang: \"de\", region: \"AT\")\n```",
        ),
    ];

    let mut richtig = 0;
    for (art, wort, n, schnipsel) in ziele {
        let erwartet = bereich(&text, schnipsel);
        let (seite, x, y) =
            klickpunkt(&stellen, wort, n).unwrap_or_else(|| panic!("{wort} nicht gesetzt"));
        let sprung = vorschau.klick(seite, x, y);
        let ok = matches!(sprung, Sprung::Haupt(b) if erwartet.contains(&b));
        // Gegenrichtung: Die Quellstelle führt auf dieselbe Seite.
        let zurueck = vorschau.stellen(&quelle, erwartet.start + schnipsel.find(wort).unwrap_or(0));
        let zurueck_ok = zurueck.iter().any(|(s, _, _)| *s == seite);
        println!(
            "{:<14} {wort:<18} S.{} ({x:6.1}, {y:6.1}) → {sprung:?} {}  zurück: {}",
            art,
            seite + 1,
            if ok { "richtig" } else { "FALSCH" },
            if zurueck_ok { "ja" } else { "nein" }
        );
        if ok {
            richtig += 1;
        }
    }
    println!("{richtig} von 20 richtig");
    assert!(richtig >= 19, "nur {richtig} von 20 Klickpunkten richtig");

    // Formularwerte (dokument.toml) und Vorlagentext: kein Sprung nach main.typ.
    for (wort, n) in [
        ("Reproduzierbarkeit", 0),
        ("Jänner", 0),
        ("Textverarbeitungen", 0),
        ("Schlagwörter", 0),
    ] {
        let (seite, x, y) = klickpunkt(&stellen, wort, n).unwrap();
        let sprung = vorschau.klick(seite, x, y);
        println!("Formular/Vorlage {wort:<18} → {sprung:?}");
        assert_eq!(sprung, Sprung::Vorlage, "{wort}");
    }
}
