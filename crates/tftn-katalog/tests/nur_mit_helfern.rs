//! Abnahme M2 (Entwurf 11, PLAN-M2 AP11): Der Testartikel entsteht nur über Helfer-Schnittstellen,
//! danach wird jedes Element über einen Helfer geändert.
//!
//! Text kommt ausschließlich aus Bausteinen, Feldwerten (Klartext, Zahlen, Dateinamen) und Patches.
//! Braucht für die Gegenprobe die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

mod hilfe;

use std::path::Path;

use hilfe::*;
use serde_json::json;
use tftn_core::Projekt;
use tftn_edit::{
    MarkupAktion, MarkupStelle, Patch, TabellenAktion as A, Wert, angebote_im_bereich, anwenden,
    farbe_patch, formel_einfuegen, formel_patch, markup_patch, markup_stellen, tabelle_am,
    tabelle_patch, wert_patch, zitat_patch,
};
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren, exportieren};
use tftn_formular::{Daten, Kontext, Schritt, formular_lesen};
use tftn_katalog::{Katalog, import_fuer};
use tftn_literatur::{einfuegetext, eingebaute_stile, ergaenzung};
use typst_syntax::Source;

const QUELLE: &str = "beispielEinfuehrung2026";

/// Schnipsel mit Werten in den Feldern, wie nach dem Durchlaufen mit Tab.
fn gefuellt(k: &Katalog, id: &str, auswahl: &str, werte: &[&str]) -> String {
    let e = k.finden(id).unwrap_or_else(|| panic!("{id} fehlt"));
    let mut ein = e.schnipsel.einsetzen(auswahl);
    let mut paare: Vec<_> = ein.felder.iter().zip(werte).collect();
    paare.sort_by_key(|(f, _)| std::cmp::Reverse(f.bereich.start));
    for (f, w) in paare {
        ein.text.replace_range(f.bereich.clone(), w);
    }
    ein.text
}

/// Baustein ans Ende, samt nötigem Import.
fn baustein(text: &mut String, k: &Katalog, id: &str, auswahl: &str, werte: &[&str]) {
    let e = k.finden(id).unwrap();
    if let Some(f) = import_fuer(&Source::detached(text.clone()), e) {
        text.insert_str(f.byte, &f.text);
    }
    text.push_str("\n\n");
    text.push_str(&gefuellt(k, id, auswahl, werte));
    text.push('\n');
}

/// Baustein innerhalb des letzten Diagramms, das mit `anker` beginnt.
fn innen(text: &mut String, k: &Katalog, anker: &str, id: &str, werte: &[&str]) {
    let von = text.rfind(anker).unwrap();
    let zu = von + text[von..].find("\n})").unwrap();
    let neu = format!("\n  {}", gefuellt(k, id, "", werte));
    text.insert_str(zu, &neu);
}

fn patch(text: &mut String, p: Patch) {
    *text = anwenden(text, &p);
}

/// Stelle der gewünschten Art beim letzten Vorkommen von `ab`.
fn stelle(text: &str, ab: &str, art: fn(&MarkupStelle) -> bool) -> (Source, MarkupStelle) {
    let q = Source::detached(text.to_string());
    let s = markup_stellen(&q, text.rfind(ab).unwrap())
        .into_iter()
        .find(art)
        .unwrap_or_else(|| panic!("keine Stelle bei {ab}"));
    (q, s)
}

/// Setzt den Wert beim ersten passenden Aufruf ab dem letzten Vorkommen von `ab`.
fn wert(k: &Katalog, text: &mut String, ab: &str, funktion: &str, name: &str, w: Wert, soll: &str) {
    let q = Source::detached(text.clone());
    let von = text.rfind(ab).unwrap_or_else(|| panic!("{ab} fehlt"));
    let alle = angebote_im_bereich(&q, von..text.len(), &parameter(k));
    let a = alle
        .iter()
        .find(|a| a.funktion == funktion && a.parameter == name)
        .unwrap_or_else(|| panic!("{funktion}.{name}: {alle:?}"));
    assert_eq!(a.gesperrt, None, "{funktion}.{name}");
    let p = wert_patch(&q, a, &a.art, &w).unwrap();
    patch(text, p);
    assert!(text.contains(soll), "{soll}\n{text}");
}

fn tabelle(text: &mut String, aktion: A) {
    let q = Source::detached(text.clone());
    let byte = text.rfind("table(").unwrap();
    let s = tabelle_am(&q, byte).expect("Tabelle").expect("bearbeitbar");
    let p = tabelle_patch(&q, &s, &aktion).unwrap();
    patch(text, p);
}

fn ist_formel(s: &MarkupStelle) -> bool {
    matches!(s, MarkupStelle::Formel { .. })
}

fn ist_zitat(s: &MarkupStelle) -> bool {
    matches!(s, MarkupStelle::Zitat { .. })
}

fn ist_ueberschrift(s: &MarkupStelle) -> bool {
    matches!(s, MarkupStelle::Ueberschrift { .. })
}

/// Fach „Dokument“: ein Feld setzen und schreiben.
fn titelei(p: &Projekt, setzen: &[(&str, serde_json::Value)]) {
    let schema =
        std::fs::read_to_string(repo().join("typen/tftn-fachartikel/tftn/formular.schema.json"))
            .unwrap();
    let f = formular_lesen(&schema).unwrap();
    let stile = eingebaute_stile();
    let k = Kontext {
        wurzel: Some(&p.wurzel),
        stile: &stile,
    };
    let pfad = p.wurzel.join("dokument.toml");
    let mut d = Daten::lesen(&std::fs::read_to_string(&pfad).unwrap()).unwrap();
    for (feld, w) in setzen {
        d.setzen(
            &f,
            &[Schritt::Name((*feld).to_string())],
            Some(w.clone()),
            &k,
        )
        .unwrap();
    }
    std::fs::write(pfad, d.text()).unwrap();
}

#[test]
fn testartikel_nur_mit_helfern() {
    let (p, k, warnungen) = fall("nur-mit-helfern");
    assert!(warnungen.is_empty(), "{warnungen:?}");
    let daten = p.wurzel.join("dokument.toml");
    let mit_notiz = format!(
        "# Eigene Notiz\n{}",
        std::fs::read_to_string(&daten).unwrap()
    );
    std::fs::write(&daten, mit_notiz).unwrap();
    std::fs::create_dir_all(p.wurzel.join("bilder")).unwrap();
    std::fs::write(
        p.wurzel.join("bilder/bild.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"20\">\
         <rect width=\"40\" height=\"20\" fill=\"gray\"/></svg>",
    )
    .unwrap();

    // --- Anlegen --------------------------------------------------------------------------------
    titelei(
        &p,
        &[
            ("titel", json!("Rücklaufquoten im Vergleich")),
            ("datum", json!("2026-03-01")),
        ],
    );
    let mut t = hauptdatei(&p);
    uebersetzt(&p, &t);

    baustein(&mut t, &k, "typst/ueberschrift", "", &["Abschnitt Eins"]);
    t.push_str("\nErster Absatz");
    baustein(&mut t, &k, "typst/ueberschrift", "", &["Abschnitt Zwei"]);
    baustein(
        &mut t,
        &k,
        "typst/unterueberschrift",
        "",
        &["Unterabschnitt"],
    );
    uebersetzt(&p, &t);

    baustein(
        &mut t,
        &k,
        "typst/tabelle",
        "",
        &["Gruppe", "Anzahl", "Eins", "Probe"],
    );
    tabelle(
        &mut t,
        A::Zelltext {
            zeile: 1,
            spalte: 1,
            inhalt: "12".into(),
        },
    );
    tabelle(&mut t, A::ZeileEinfuegen { vor: 2 });
    tabelle(
        &mut t,
        A::Zelltext {
            zeile: 2,
            spalte: 0,
            inhalt: "Summe".into(),
        },
    );
    tabelle(
        &mut t,
        A::Verbinden {
            zeile: 2,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
    );
    assert!(t.contains("colspan: 2"), "{t}");
    uebersetzt(&p, &t);

    baustein(
        &mut t,
        &k,
        "typst/abbildung",
        "",
        &["bilder/bild.svg", "Ein graues Rechteck"],
    );
    uebersetzt(&p, &t);

    t.push_str("\nEs gilt ");
    let q = Source::detached(t.clone());
    let bruch = gefuellt(&k, "typst/mathe-bruch", "", &["a", "b"]);
    let eingefuegt = formel_einfuegen(&q, t.len(), &bruch, false).unwrap();
    patch(&mut t, eingefuegt);
    t.push_str(" immer.\n");
    baustein(
        &mut t,
        &k,
        "typst/formel-mehrzeilig",
        "",
        &["x", "y + 1", "z"],
    );
    uebersetzt(&p, &t);

    baustein(
        &mut t,
        &k,
        "tftn-fachartikel/hinweis",
        "Bitte beachten",
        &[],
    );
    t.push_str("\nEin Satz");
    baustein(&mut t, &k, "typst/fussnote", "Eine Anmerkung", &[]);
    t.push_str("\nWie beschrieben");
    let q = Source::detached(t.clone());
    let zitat = einfuegetext(&q, t.len(), QUELLE, Some("3")).unwrap();
    assert_eq!(zitat, format!(" @{QUELLE}[S. 3]"));
    t.push_str(&zitat);
    t.push_str(".\n");
    uebersetzt(&p, &t);

    baustein(&mut t, &k, "@preview/cetz-plot/linien", "", &[]);
    baustein(
        &mut t,
        &k,
        "@preview/fletcher/diagramm",
        "",
        &["Beginn", "Schluss"],
    );
    innen(
        &mut t,
        &k,
        "#fletcher.diagram",
        "@preview/fletcher/knoten",
        &["2, 0", "Weiter"],
    );
    innen(
        &mut t,
        &k,
        "#fletcher.diagram",
        "@preview/fletcher/kante",
        &["1, 0", "2, 0"],
    );
    baustein(&mut t, &k, "@preview/cetz/zeichnung", "", &["3, 2"]);
    innen(
        &mut t,
        &k,
        "#cetz.canvas",
        "@preview/cetz/kreis",
        &["1, 1", "0.5"],
    );
    innen(
        &mut t,
        &k,
        "#cetz.canvas",
        "@preview/cetz/text",
        &["2, 2", "Marke"],
    );
    uebersetzt(&p, &t);

    // --- Ändern ---------------------------------------------------------------------------------
    titelei(&p, &[("titel", json!("Rücklaufquoten neu betrachtet"))]);
    let geschrieben = std::fs::read_to_string(&daten).unwrap();
    assert!(geschrieben.contains("Rücklaufquoten neu betrachtet"));
    assert!(geschrieben.contains("# Eigene Notiz"), "{geschrieben}");

    let (q, s) = stelle(&t, "Abschnitt Zwei", ist_ueberschrift);
    patch(
        &mut t,
        markup_patch(&q, &s, MarkupAktion::EbenePlus).unwrap(),
    );
    assert!(t.contains("== Abschnitt Zwei"), "{t}");

    tabelle(&mut t, A::SpalteEinfuegen { vor: 2 });
    tabelle(
        &mut t,
        A::Zelltext {
            zeile: 1,
            spalte: 2,
            inhalt: "neu".into(),
        },
    );
    assert!(t.contains("[neu]"), "{t}");

    wert(
        &k,
        &mut t,
        "#figure(\n  image",
        "figure",
        "caption",
        Wert::Inhalt("Das Rechteck".into()),
        "[Das Rechteck]",
    );
    wert(
        &k,
        &mut t,
        "#figure(\n  image",
        "image",
        "width",
        Wert::Prozent(50.0),
        "50%",
    );

    let (q, s) = stelle(&t, "frac(a", ist_formel);
    let wurzel = gefuellt(&k, "typst/mathe-wurzel", "", &["x"]);
    patch(&mut t, formel_patch(&q, &s, &wurzel).unwrap());
    let (q, s) = stelle(&t, "sqrt(x", ist_formel);
    patch(
        &mut t,
        markup_patch(&q, &s, MarkupAktion::FormelUmschalten).unwrap(),
    );
    assert!(t.contains("$ sqrt(x) $"), "{t}");

    let (q, s) = stelle(&t, "y + 1", ist_formel);
    let zeile = gefuellt(&k, "typst/mathe-zeile", "", &[]);
    let punkt = gefuellt(&k, "typst/mathe-ausrichten", "", &[]);
    let mehrzeilig = format!("p {punkt}= q {zeile}{punkt}= r {zeile}{punkt}= s");
    patch(&mut t, formel_patch(&q, &s, &mehrzeilig).unwrap());
    assert!(
        t.contains("= r") && t.contains('&') && t.contains("\\\n"),
        "{t}"
    );

    let hinweis = "#hinweis[Bitte";
    wert(
        &k,
        &mut t,
        hinweis,
        "hinweis",
        "art",
        Wert::Text("warnung".into()),
        "art: \"warnung\"",
    );
    let hinweis = "#hinweis(art: \"warnung\"";
    let q = Source::detached(t.clone());
    let von = t.rfind(hinweis).unwrap();
    let alle = angebote_im_bereich(&q, von..t.len(), &parameter(&k));
    let farbe = alle
        .iter()
        .find(|a| a.funktion == "hinweis" && a.parameter == "farbe")
        .unwrap();
    patch(&mut t, farbe_patch(&q, farbe, "#aabbcc").unwrap());
    assert!(t.contains("#aabbcc"), "{t}");

    wert(
        &k,
        &mut t,
        "#footnote[Eine",
        "footnote",
        "text",
        Wert::Inhalt("Eine neue Anmerkung".into()),
        "[Eine neue Anmerkung]",
    );

    let (q, s) = stelle(&t, &format!("@{QUELLE}[S. 3]."), ist_zitat);
    let zusatz = ergaenzung(Some("12"));
    patch(
        &mut t,
        zitat_patch(&q, &s, QUELLE, zusatz.as_deref()).unwrap(),
    );
    assert!(t.contains(&format!("@{QUELLE}[S. 12].")), "{t}");
    uebersetzt(&p, &t);

    let punkt = |x: f64, y: f64| Wert::Punkt(vec![Wert::Zahl(x), Wert::Zahl(y)]);
    let daten_reihe = Wert::Liste(vec![punkt(0.0, 1.0), punkt(5.0, 6.0)]);
    let plot = "#cetz.canvas";
    let erste = t.find(plot).unwrap();
    assert!(t[erste..].contains("cetz-plot.plot"), "{t}");
    wert(
        &k,
        &mut t,
        "cetz-plot.plot.add",
        "cetz-plot.plot.add",
        "daten",
        daten_reihe,
        "(5, 6)",
    );
    wert(
        &k,
        &mut t,
        "#fletcher.diagram",
        "fletcher.node",
        "text",
        Wert::Inhalt("Anfang".into()),
        "[Anfang]",
    );
    wert(
        &k,
        &mut t,
        "#fletcher.diagram",
        "fletcher.edge",
        "marks",
        Wert::Text("<|-|>".into()),
        "<|-|>",
    );
    wert(
        &k,
        &mut t,
        "cetz.draw.line",
        "cetz.draw.line",
        "nach",
        punkt(4.0, 3.0),
        "(4, 3)",
    );
    uebersetzt(&p, &t);

    // --- Ergebnis -------------------------------------------------------------------------------
    std::fs::write(p.wurzel.join(&p.konfig.haupt), &t).unwrap();
    let cli = TypstCli::finden(&p.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let (pdf, gegen) = (Path::new("artikel.pdf"), p.wurzel.join("gegenprobe.pdf"));
    exportieren(&p, &cli, pdf, &[]).unwrap();
    ausfuehren(&p, &cli, &gegen, &[], STANDARD_FRIST).unwrap();
    assert_eq!(
        std::fs::read(p.wurzel.join(pdf)).unwrap(),
        std::fs::read(&gegen).unwrap()
    );
}
