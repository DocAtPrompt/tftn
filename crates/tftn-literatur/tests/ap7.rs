//! Abnahme AP7 / A5 (PLAN-M1): Suche in 5 000 Einträgen < 100 ms, eingefügtes Zitat kompiliert und
//! erscheint im Literaturverzeichnis, geänderte `.bib` ist nach dem Neuladen im Index.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tftn_core::vorschau::Vorschau;
use tftn_core::{Projekt, seitenzeilen};
use tftn_literatur::{Literatur, einfuegetext, ergaenzung};
use typst_syntax::Source;

const NACHNAMEN: [&str; 16] = [
    "Müller", "Gruber", "Huber", "Wagner", "Pichler", "Steiner", "Moser", "Mayer", "Hofer",
    "Leitner", "Berger", "Fuchs", "Eder", "Fischer", "Schmid", "Weiß",
];
const VORNAMEN: [&str; 8] = [
    "Anna",
    "Bärbel",
    "Christoph",
    "Doris",
    "Erich",
    "Jörg",
    "Lena",
    "Özlem",
];
const WOERTER: [&str; 20] = [
    "Dokumentation",
    "Befund",
    "Schriftsatz",
    "Allgemeinmedizin",
    "Prävention",
    "Leitlinie",
    "Qualität",
    "Versorgung",
    "Hausarzt",
    "Diagnostik",
    "Therapie",
    "Studie",
    "Übersicht",
    "Register",
    "Kohorte",
    "Evidenz",
    "Praxis",
    "Österreich",
    "Methoden",
    "Ergebnisse",
];

/// Erfundene, reproduzierbare `.bib` mit `n` Einträgen (lineare Kongruenz statt Zufall).
fn synthetische_bib(n: usize) -> String {
    let mut z: u64 = 42;
    let mut naechste = |m: usize| {
        z = z
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((z >> 33) as usize) % m
    };
    let mut bib = String::new();
    for i in 0..n {
        let name = NACHNAMEN[naechste(16)];
        let vor = VORNAMEN[naechste(8)];
        let zweit = NACHNAMEN[naechste(16)];
        let jahr = 1960 + naechste(67);
        let titel: Vec<&str> = (0..3 + naechste(4))
            .map(|_| WOERTER[naechste(20)])
            .collect();
        let schluessel = format!(
            "{}{}{i}",
            name.to_lowercase().replace(['ü', 'ß'], "u"),
            jahr
        );
        let art = ["article", "book", "incollection"][naechste(3)];
        writeln!(
            bib,
            "@{art}{{{schluessel},\n  title = {{{}}},\n  author = {{{name}, {vor} and {zweit}, Max}},\n  \
             date = {{{jahr}}},\n  journaltitle = {{Erfundene Zeitschrift {}}},\n  booktitle = {{Sammelband {}}},\n  \
             publisher = {{Beispielverlag}},\n  doi = {{10.99999/test.{i}}}\n}}\n",
            titel.join(" "),
            naechste(30),
            naechste(30)
        )
        .unwrap();
    }
    bib
}

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let p = e.unwrap().path();
        let ziel = nach.join(p.file_name().unwrap());
        if p.is_dir() {
            kopieren(&p, &ziel)
        } else {
            std::fs::copy(&p, &ziel).unwrap();
        }
    }
}

fn tmp(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn suche_in_5000_eintraegen() {
    let ordner = tmp("ap7-suche");
    let datei = ordner.join("gross.bib");
    let korpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel/literatur.bib");
    let mut bib = synthetische_bib(5000);
    bib.push_str(&std::fs::read_to_string(korpus).unwrap());
    std::fs::write(&datei, bib).unwrap();

    let t = Instant::now();
    let (lit, fehler) = Literatur::laden(std::slice::from_ref(&datei));
    let laden = t.elapsed().as_secs_f64() * 1000.0;
    assert!(fehler.is_empty(), "{fehler:?}");
    assert_eq!(lit.len(), 5006);

    let anfragen = [
        "lüddecke 2020",
        "lueddecke",
        "luddecke",
        "smith",
        "müller 1987",
        "weiss",
        "weiß",
        "prävention leitlinie",
        "österreich kohorte",
        "gruber",
        "2001",
        "doku",
        "10.99999/test.4711",
        "fischer anna",
        "huber register 2010",
        "zitieren medizin",
        "satz schrift",
        "befund",
        "leitn",
        "sammelband 7",
    ];
    let mut zeiten = Vec::new();
    for runde in 0..5 {
        for a in anfragen {
            let t = Instant::now();
            let treffer = lit.suchen(a, 30);
            zeiten.push(t.elapsed().as_secs_f64() * 1000.0);
            if runde == 0 {
                println!(
                    "{a:<24} {:>3} Treffer, erster: {:?}",
                    treffer.len(),
                    treffer.first().map(|t| &t.schluessel)
                );
            }
        }
    }
    zeiten.sort_by(f64::total_cmp);
    let p95 = zeiten[(zeiten.len() as f64 * 0.95) as usize];
    println!(
        "laden {laden:.0} ms · Suche p50 {:.2} ms, p95 {p95:.2} ms, max {:.2} ms",
        zeiten[zeiten.len() / 2],
        zeiten.last().unwrap()
    );
    assert!(p95 < 100.0, "Suche p95 {p95:.1} ms");
    assert!(laden < 2000.0, "Neuladen {laden:.0} ms");

    // Richtige Treffer zuerst
    assert_eq!(
        lit.suchen("lüddecke 2020", 5)[0].schluessel,
        "lueddeckeZitierenInDerMedizin2020"
    );
    assert_eq!(
        lit.suchen("luddecke", 5)[0].schluessel,
        "lueddeckeZitierenInDerMedizin2020"
    );
    assert_eq!(
        lit.suchen("smithTyp", 5)[0].schluessel,
        "smithTypesettingClinicalReports2023"
    );
    assert_eq!(
        lit.suchen("10.99999/test.4711", 5)[0].doi.as_deref(),
        Some("10.99999/test.4711")
    );
}

#[test]
fn eingefuegtes_zitat_kompiliert_und_steht_im_verzeichnis() {
    let ordner = tmp("ap7-einfuegen");
    let korpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    for d in [
        "main.typ",
        "dokument.toml",
        "literatur.bib",
        "trennungen.txt",
        "tftn.toml",
    ] {
        std::fs::copy(korpus.join(d), ordner.join(d)).unwrap();
    }
    for d in ["fonts", "packages"] {
        kopieren(&korpus.join(d), &ordner.join(d));
    }
    // Neuer Eintrag von außen (wie ein Zotero-Auto-Export)
    let bib = ordner.join("literatur.bib");
    let mut text = std::fs::read_to_string(&bib).unwrap();
    text.push_str(
        "\n@book{oberhauserNeuesBuch2025,\n  title = {Ein ganz neues Buch über Satz},\n  \
         author = {Oberhauser, Wilma},\n  date = {2025},\n  publisher = {Beispielverlag},\n  location = {Linz}\n}\n",
    );
    let (vorher, _) = Literatur::laden(std::slice::from_ref(&bib));
    std::fs::write(&bib, &text).unwrap();
    let t = Instant::now();
    let (lit, fehler) = Literatur::laden(std::slice::from_ref(&bib));
    println!(
        "Neuladen nach Änderung: {:.1} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );
    assert!(fehler.is_empty());
    assert!(!vorher.enthaelt("oberhauserNeuesBuch2025") || vorher.len() == lit.len());
    let treffer = lit.suchen("oberhauser", 5);
    assert_eq!(treffer[0].schluessel, "oberhauserNeuesBuch2025");

    // Zitat mit Seite am Ende eines Satzes einfügen
    let projekt = Projekt::oeffnen(&ordner).unwrap();
    let mut vorschau = Vorschau::neu(&projekt).unwrap();
    let id = vorschau.world().haupt();
    let haupt = std::fs::read_to_string(projekt.haupt()).unwrap();
    let pos = haupt.find("Druckvorstufe hinzu").unwrap() + "Druckvorstufe hinzu".len();
    let quelle = Source::new(id, haupt.clone());
    let einfuegen = einfuegetext(&quelle, pos, &treffer[0].schluessel, Some("33")).unwrap();
    // Direkt nach „hinzu“: mit Leerzeichen davor
    assert_eq!(einfuegen, " @oberhauserNeuesBuch2025[S. 33]");
    let neu = format!("{}{}{}", &haupt[..pos], einfuegen, &haupt[pos..]);

    let lauf = vorschau.lauf(1, Some(Source::new(id, neu)));
    for m in &lauf.meldungen {
        println!("{m}");
    }
    assert!(lauf.gelungen);
    // Zeilen ohne Trenner verbinden: Das Zitat darf umbrechen. Typst fasst es mit dem
    // benachbarten Zitat zusammen: „(Große, 2019, Kap. 9; Oberhauser, 2025, S. 33)“.
    let text = seitenzeilen(&vorschau.gut().unwrap().dokument)
        .concat()
        .join("");
    assert!(text.contains("Oberhauser, 2025, S. 33"), "Zitat fehlt");
    assert!(
        text.contains("Ein ganz neues Buch über Satz"),
        "Eintrag fehlt im Literaturverzeichnis"
    );
}

#[test]
fn ergaenzung_regel() {
    assert_eq!(ergaenzung(Some("12")).as_deref(), Some("S. 12"));
    assert_eq!(ergaenzung(Some(" 12–14 ")).as_deref(), Some("S. 12–14"));
    assert_eq!(ergaenzung(Some("Kap. 9")).as_deref(), Some("Kap. 9"));
    assert_eq!(ergaenzung(Some("S. 3")).as_deref(), Some("S. 3"));
    assert_eq!(ergaenzung(Some("  ")), None);
    assert_eq!(ergaenzung(None), None);
}
