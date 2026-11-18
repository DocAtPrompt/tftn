//! Jeder mitgelieferte Dokumenttyp lässt sich auf Deutsch und Englisch anlegen und übersetzt ohne
//! Fehler; sein Formular ist gültig, und die Daten der Vorlagen bestehen die Prüfung.

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seitenzeilen};
use tftn_formular::{Daten, Kontext, alles_pruefen, formular_lesen};
use tftn_manifest::Typpaket;
use tftn_manifest::anlegen::projekt_anlegen_sprache;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn typen() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types")
}

/// Alle Typen mit Vorlage (`[template]`), also ohne Bibliotheken wie tftn-kern.
fn mit_vorlage() -> Vec<String> {
    let mut aus: Vec<String> = std::fs::read_dir(typen())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|o| {
            std::fs::read_to_string(o.join("typst.toml")).is_ok_and(|t| t.contains("\n[template]"))
        })
        .map(|o| o.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    aus.sort();
    aus
}

fn text(projekt: &Projekt) -> String {
    let world = ProjektWorld::neu(projekt).unwrap();
    let ergebnis = kompilieren(&world);
    let fehler: Vec<_> = ergebnis.meldungen.iter().filter(|m| m.fehler).collect();
    assert!(
        fehler.is_empty(),
        "{}: {fehler:?}",
        projekt.wurzel.display()
    );
    seitenzeilen(&ergebnis.dokument.expect("kompiliert"))
        .concat()
        .join("\n")
}

#[test]
fn neue_typen_sind_da() {
    let alle = mit_vorlage();
    for typ in [
        "tftn-befund",
        "tftn-bericht",
        "tftn-brief",
        "tftn-fachartikel",
        "tftn-fachbuch",
        "tftn-lebenslauf",
        "tftn-leer",
        "tftn-plakat",
        "tftn-rechnung",
    ] {
        assert!(alle.iter().any(|t| t == typ), "{typ} fehlt in {alle:?}");
    }
}

#[test]
fn jeder_typ_deutsch_und_englisch() {
    // Deutsche Wörter, die in keinem englischen Dokument vorkommen sollen.
    let deutsch = [
        "Jänner",
        "Dezember",
        "Seite ",
        "Inhalt",
        "Literatur",
        "Betreff",
        "Mit freundlichen",
        "geboren",
        "Hinweis:",
    ];
    for typ in mit_vorlage() {
        let paket = Typpaket::lesen(&typen().join(&typ)).unwrap();
        let manifest = paket.manifest().unwrap();
        assert_eq!(
            manifest.vorlagen.get("en").map(String::as_str),
            Some("vorlage-en"),
            "{typ}: englische Vorlage fehlt"
        );
        let formular = manifest.formular.as_ref().map(|pfad| {
            let json = std::fs::read_to_string(paket.ordner.join(pfad)).unwrap();
            formular_lesen(&json).unwrap_or_else(|e| panic!("{typ}: {e:?}"))
        });
        for sprache in [None, Some("en")] {
            let ordner = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("alle-typen-{typ}-{}", sprache.unwrap_or("de")));
            let _ = std::fs::remove_dir_all(&ordner);
            let p = projekt_anlegen_sprache(&typen(), &typ, &ordner, Some(ZEITSTEMPEL), sprache)
                .unwrap_or_else(|e| panic!("{typ}: {e:?}"))
                .projekt;
            let t = text(&p);
            assert!(typ == "tftn-leer" || !t.trim().is_empty(), "{typ}: leer");
            if sprache == Some("en") {
                for wort in deutsch {
                    assert!(
                        !t.contains(wort),
                        "{typ}: „{wort}“ im englischen Text:\n{t}"
                    );
                }
            }
            if let Some(f) = &formular {
                let daten = manifest.daten.as_deref().unwrap_or("dokument.toml");
                let roh = std::fs::read_to_string(p.wurzel.join(daten)).unwrap();
                let werte = Daten::lesen(&roh).unwrap().werte();
                let k = Kontext {
                    wurzel: Some(&p.wurzel),
                    stile: &["american-psychological-association"],
                };
                let fehler: Vec<_> = alles_pruefen(f, &werte, &k)
                    .into_iter()
                    .filter(|m| m.fehler)
                    .collect();
                assert!(fehler.is_empty(), "{typ} {sprache:?}: {fehler:?}");
            }
            let _ = std::fs::remove_dir_all(&ordner);
        }
    }
}

/// Legt eine Rechnung mit eigener `dokument.toml` an und gibt ihren Text zurück.
fn rechnung(name: &str, dokument: Option<&str>, sprache: Option<&str>) -> String {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("rechnung-{name}"));
    let _ = std::fs::remove_dir_all(&ordner);
    let p = projekt_anlegen_sprache(
        &typen(),
        "tftn-rechnung",
        &ordner,
        Some(ZEITSTEMPEL),
        sprache,
    )
    .unwrap()
    .projekt;
    if let Some(d) = dokument {
        std::fs::write(p.wurzel.join("dokument.toml"), d).unwrap();
    }
    let t = text(&p);
    let _ = std::fs::remove_dir_all(&ordner);
    t
}

#[test]
fn rechnung_rechnet() {
    // Vorlage: 2 × 120 + 250 + 36 × 0,50 = 508,00 netto, 20 % USt 101,60, gesamt 609,60.
    let t = rechnung("vorlage", None, None);
    for s in [
        "2026-001",
        "508,00 €",
        "USt 20 %",
        "101,60 €",
        "609,60 €",
        "14. Februar 2026",
    ] {
        assert!(t.contains(s), "„{s}“ fehlt:\n{t}");
    }
    let en = rechnung("vorlage-en", None, Some("en"));
    for s in [
        "Invoice",
        "508.00 €",
        "VAT 20 %",
        "609.60 €",
        "14 February 2026",
    ] {
        assert!(en.contains(s), "„{s}“ fehlt:\n{en}");
    }
}

#[test]
fn rechnung_rundet_je_zeile_und_ohne_ust() {
    // 3 × 0,335 = 1,005 → 1,01; 1 × 1234,5 → 1.234,50; ohne USt nur der Gesamtbetrag.
    let t = rechnung(
        "runden",
        Some(
            "nummer = \"R-7\"\ndatum = 2026-03-01\nempfaenger = [\"A\"]\nust = 0\n\
             steuerhinweis = \"Ohne Umsatzsteuer.\"\n\
             posten = [{ text = \"x\", menge = 3, preis = 0.335 }, { text = \"y\", preis = 1234.5 }]\n",
        ),
        None,
    );
    for s in [
        "R-7",
        "1,01",
        "1.234,50",
        "1.235,51 €",
        "Ohne Umsatzsteuer.",
    ] {
        assert!(t.contains(s), "„{s}“ fehlt:\n{t}");
    }
    assert!(!t.contains("USt"), "{t}");
    assert!(
        !t.contains("Zahlbar"),
        "ohne Zahlungsziel keine Frist:\n{t}"
    );
}

#[test]
fn rechnung_mit_steuerbefreiung() {
    // Heilbehandlung: ust = 20 bleibt stehen, gilt aber nicht; der Hinweis kommt von selbst.
    let posten = "nummer = \"A-1\"\ndatum = 2026-03-01\nempfaenger = [\"A\"]\nust = 20\n\
                  posten = [{ text = \"Untersuchung\", preis = 150 }]\n";
    let t = rechnung(
        "heilbehandlung",
        Some(&format!("{posten}befreiung = \"heilbehandlung\"\n")),
        None,
    );
    assert!(t.contains("150,00 €") && !t.contains("USt 20"), "{t}");
    assert!(t.contains("§ 6 Abs. 1 Z 19 UStG"), "{t}");
    let k = rechnung(
        "kleinunternehmer",
        Some(&format!(
            "{posten}befreiung = \"kleinunternehmer\"\nsprache = \"en\"\n"
        )),
        None,
    );
    assert!(k.contains("small business") && !k.contains("VAT 20"), "{k}");
    // Ein eigener Steuerhinweis geht vor.
    let e = rechnung(
        "eigener-hinweis",
        Some(&format!(
            "{posten}befreiung = \"heilbehandlung\"\nsteuerhinweis = \"Eigener Hinweis.\"\n"
        )),
        None,
    );
    assert!(e.contains("Eigener Hinweis.") && !e.contains("Z 19"), "{e}");
    // Ohne Befreiung wie bisher.
    let o = rechnung("ohne-befreiung", Some(posten), None);
    assert!(o.contains("USt 20 %") && o.contains("180,00 €"), "{o}");
}

/// Bilder (`FrameItem::Image`) auf allen Seiten, auch in Gruppen.
fn bilder_zaehlen(dokument: &tftn_core::PagedDocument) -> usize {
    fn im(frame: &typst::layout::Frame) -> usize {
        use typst::layout::FrameItem;
        frame
            .items()
            .map(|(_, item)| match item {
                FrameItem::Image(..) => 1,
                FrameItem::Group(g) => im(&g.frame),
                _ => 0,
            })
            .sum()
    }
    dokument.pages().iter().map(|s| im(&s.frame)).sum()
}

/// Setzt das Projekt ohne Fehler; Text und Zahl der Bilder.
fn setzen(projekt: &Projekt) -> (String, usize) {
    let world = ProjektWorld::neu(projekt).unwrap();
    let ergebnis = kompilieren(&world);
    let fehler: Vec<_> = ergebnis.meldungen.iter().filter(|m| m.fehler).collect();
    assert!(
        fehler.is_empty(),
        "{}: {fehler:?}",
        projekt.wurzel.display()
    );
    let dokument = ergebnis.dokument.expect("kompiliert");
    let text = seitenzeilen(&dokument).concat().join("\n");
    (text, bilder_zaehlen(&dokument))
}

#[test]
fn logo_und_unterschrift_als_bild() {
    const LOGO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40"><rect width="120" height="40" fill="blue"/></svg>"#;
    const UNTERSCHRIFT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="60"><path d="M5 45 C 30 5, 50 60, 70 30" stroke="black" fill="none"/></svg>"#;
    // Brief und Befund: Logo und Unterschrift; Rechnung: nur das Logo.
    for (typ, erwartet) in [("tftn-brief", 2), ("tftn-befund", 2), ("tftn-rechnung", 1)] {
        for sprache in [None, Some("en")] {
            let ordner = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("logo-{typ}-{}", sprache.unwrap_or("de")));
            let _ = std::fs::remove_dir_all(&ordner);
            let p = projekt_anlegen_sprache(&typen(), typ, &ordner, Some(ZEITSTEMPEL), sprache)
                .unwrap()
                .projekt;
            let (_, ohne) = setzen(&p);
            assert_eq!(ohne, 0, "{typ}: die Vorlage hat kein Bild");

            std::fs::create_dir_all(p.wurzel.join("bilder")).unwrap();
            std::fs::write(p.wurzel.join("bilder/logo.svg"), LOGO).unwrap();
            std::fs::write(p.wurzel.join("bilder/unterschrift.svg"), UNTERSCHRIFT).unwrap();
            let daten = p.wurzel.join("dokument.toml");
            let alt = std::fs::read_to_string(&daten).unwrap();
            std::fs::write(
                &daten,
                format!(
                    "logo = \"bilder/logo.svg\"\nlogo_hoehe = 2\n\
                     unterschrift_bild = \"bilder/unterschrift.svg\"\n{alt}"
                ),
            )
            .unwrap();
            let (text, n) = setzen(&p);
            assert_eq!(n, erwartet, "{typ} {sprache:?}:\n{text}");
            assert!(!text.contains("path(p)"), "{typ}: kein Hinweis:\n{text}");

            // Das Formular nimmt die Felder an (Format `bild`, Datei im Projekt).
            let paket = Typpaket::lesen(&typen().join(typ)).unwrap();
            let manifest = paket.manifest().unwrap();
            let json =
                std::fs::read_to_string(paket.ordner.join(manifest.formular.as_ref().unwrap()))
                    .unwrap();
            let f = formular_lesen(&json).unwrap();
            let werte = Daten::lesen(&std::fs::read_to_string(&daten).unwrap())
                .unwrap()
                .werte();
            let k = Kontext {
                wurzel: Some(&p.wurzel),
                stile: &[],
            };
            let fehler: Vec<_> = alles_pruefen(&f, &werte, &k)
                .into_iter()
                .filter(|m| m.fehler)
                .collect();
            assert!(fehler.is_empty(), "{typ}: {fehler:?}");

            // Eine main.typ von vor 0.3.0 (ohne `pfad`) setzt weiter, statt der Bilder ein Hinweis.
            let haupt = p.wurzel.join("main.typ");
            let main = std::fs::read_to_string(&haupt).unwrap();
            assert!(main.contains(", pfad: p => path(p))"), "{main}");
            std::fs::write(&haupt, main.replace(", pfad: p => path(p))", ")")).unwrap();
            let (text, n) = setzen(&p);
            assert_eq!(n, 0, "{typ}: ohne pfad kein Bild");
            // Zeilen aus Spalten mischen sich; je Bild ein Hinweis mit `path(p)`.
            let wort = if sprache.is_some() {
                "needs"
            } else {
                "braucht"
            };
            assert!(
                text.matches("path(p)").count() == erwartet && text.contains(wort),
                "{typ}: je Bild ein Hinweis:\n{text}"
            );
            let _ = std::fs::remove_dir_all(&ordner);
        }
    }
}
