//! Latenzmessung ohne Oberfläche (PLAN-M1 4.4): `cargo xtask messen`.
//!
//! Baut aus dem Testartikel einen Fachartikel von gut 15 Seiten, öffnet ihn wie die App und tippt
//! 500 Zeichen in den dritten Absatz. Je Tastendruck gemessen: Spiegel aktualisieren,
//! Syntaxhervorhebung des sichtbaren Bereichs, Kompilieren, Rastern der geänderten sichtbaren Seiten
//! (144 ppi, PNG). Nicht enthalten: Übertragung an den Webview und Zeichnen (das misst die App).
//! Ergebnis als JSON nach `target/messung/<rechner>.json`. Frist: 120 s (Projektregel).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use tftn_core::Projekt;
use tftn_core::spiegel::{Aenderung, Spiegel};
use tftn_core::vorschau::Vorschau;
use tftn_manifest::anlegen::projekt_anlegen;

const FRIST: Duration = Duration::from_secs(120);
const ZEICHEN: usize = 500;
/// Abschriften des Testartikels; ergibt gut 15 Seiten.
const ABSCHRIFTEN: usize = 9;
const PPI: f64 = 144.0;
/// Sichtbarer Bereich im Editor um die Schreibmarke, in UTF-16-Einheiten.
const SICHTBAR: usize = 3000;
const LABELS: [&str; 7] = [
    "einleitung",
    "methode",
    "tab-jahrgaenge",
    "gl-mittel",
    "gl-ersparnis",
    "ergebnisse",
    "diskussion",
];

#[derive(Serialize)]
struct Kennzahl {
    p50: f64,
    p95: f64,
    max: f64,
    budget_p50: Option<f64>,
    budget_p95: Option<f64>,
    eingehalten: Option<bool>,
}

fn kennzahl(mut werte: Vec<f64>, budget: Option<(f64, f64)>) -> Kennzahl {
    werte.sort_by(f64::total_cmp);
    let q = |p: f64| werte[((werte.len() - 1) as f64 * p).round() as usize];
    let (p50, p95) = (q(0.5), q(0.95));
    Kennzahl {
        p50,
        p95,
        max: *werte.last().unwrap(),
        budget_p50: budget.map(|b| b.0),
        budget_p95: budget.map(|b| b.1),
        eingehalten: budget.map(|(b50, b95)| p50 <= b50 && p95 <= b95),
    }
}

#[derive(Serialize)]
struct Bericht {
    rechner: String,
    system: String,
    seiten: usize,
    tastendruecke: usize,
    oeffnen_bis_erste_seite_ms: f64,
    oeffnen_budget_ms: (f64, f64),
    taste_spiegel: Kennzahl,
    taste_hervorhebung: Kennzahl,
    taste_kompilieren: Kennzahl,
    taste_rastern: Kennzahl,
    taste_bis_seite: Kennzahl,
    gerasterte_seiten_je_taste: f64,
    hinweis: &'static str,
}

fn quelle() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Fachartikel aus `ABSCHRIFTEN` Abschriften des Testartikels (Labels je Abschrift umbenannt).
fn langer_artikel(ziel: &Path) -> Projekt {
    let _ = std::fs::remove_dir_all(ziel);
    let typen = quelle().join("typen");
    projekt_anlegen(&typen, "tftn-fachartikel", ziel, Some(1_793_404_800))
        .expect("Projekt anlegen");
    let korpus = quelle().join("tests/korpus/testartikel");
    for datei in ["dokument.toml", "literatur.bib", "trennungen.txt"] {
        std::fs::copy(korpus.join(datei), ziel.join(datei)).unwrap();
    }
    let text = std::fs::read_to_string(korpus.join("main.typ")).unwrap();
    let anfang = text.find("= Einleitung").unwrap();
    let ende = text.find("#bibliography").unwrap();
    let mut neu = text[..anfang].to_string();
    for k in 1..=ABSCHRIFTEN {
        let mut teil = text[anfang..ende].to_string();
        for l in LABELS {
            teil = teil.replace(&format!("<{l}>"), &format!("<{l}-{k}>"));
            teil = teil.replace(&format!("@{l}"), &format!("@{l}-{k}"));
        }
        neu.push_str(&teil);
    }
    neu.push_str(&text[ende..]);
    std::fs::write(ziel.join("main.typ"), neu).unwrap();
    Projekt::oeffnen(ziel).unwrap()
}

fn rechner() -> String {
    let cpu = if cfg!(target_os = "macos") {
        std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    } else {
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default()
    };
    let name = std::env::var("TFTN_RECHNER").unwrap_or_else(|_| std::env::consts::OS.to_string());
    format!(
        "{name} ({cpu}, {} Kerne)",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0)
    )
}

fn main() {
    let start = Instant::now();
    let ausgabe = quelle().join("target/messung");
    std::fs::create_dir_all(&ausgabe).unwrap();
    let projekt = langer_artikel(&ausgabe.join("projekt"));

    // Öffnen bis erste Seite (kalter Cache)
    let t = Instant::now();
    let mut vorschau = Vorschau::neu(&projekt).unwrap();
    let text = std::fs::read_to_string(projekt.haupt()).unwrap();
    let mut spiegel = Spiegel::neu(vorschau.world().haupt(), &text);
    let lauf = vorschau.lauf(0, Some(spiegel.quelle().clone()));
    assert!(
        lauf.gelungen,
        "Messartikel kompiliert nicht: {:?}",
        lauf.meldungen
    );
    vorschau.seite_png(0, lauf.seiten[0].hash, PPI).unwrap();
    let oeffnen = t.elapsed().as_secs_f64() * 1000.0;
    let seiten = lauf.seiten.len();

    // Schreibmarke im dritten Absatz der ersten Abschrift
    let byte = spiegel.text().find("an. @methode-1").expect("Absatz 3") + 3;
    let mut pos16 = spiegel.quelle().lines().byte_to_utf16(byte).unwrap();
    let einfuegen: Vec<char> = "Sorgfältig geprüfte Größen ändern nichts am Ergebnis. "
        .chars()
        .collect();

    let (mut sp, mut hv, mut ko, mut ra, mut ge) = (vec![], vec![], vec![], vec![], vec![]);
    let mut gerastert = 0usize;
    for i in 0..ZEICHEN {
        if start.elapsed() > FRIST {
            eprintln!(
                "Frist von {} s überschritten nach {i} Tasten, Abbruch.",
                FRIST.as_secs()
            );
            std::process::exit(2);
        }
        let c = einfuegen[i % einfuegen.len()].to_string();
        let t0 = Instant::now();
        let rev = spiegel.rev();
        spiegel
            .anwenden(
                rev,
                &[Aenderung {
                    von: pos16,
                    bis: pos16,
                    text: c.clone(),
                }],
            )
            .unwrap();
        pos16 += c.encode_utf16().count();
        let t1 = Instant::now();
        let h = spiegel.hervorhebung(pos16.saturating_sub(SICHTBAR / 2)..pos16 + SICHTBAR / 2);
        std::hint::black_box(h);
        let t2 = Instant::now();
        let lauf = vorschau.lauf(spiegel.rev(), Some(spiegel.quelle().clone()));
        let t3 = Instant::now();
        // Sichtbar: die Seite der Schreibmarke und die folgende.
        let cursor = spiegel.quelle().lines().utf16_to_byte(pos16).unwrap();
        let gut = vorschau.gut().unwrap();
        let seite = typst_ide::jump_from_cursor(&gut.dokument, spiegel.quelle(), cursor)
            .first()
            .map(|p| p.page.get() - 1)
            .unwrap_or(0);
        for nr in lauf
            .geaendert
            .iter()
            .copied()
            .filter(|&n| n == seite || n == seite + 1)
        {
            vorschau.seite_png(nr, lauf.seiten[nr].hash, PPI).unwrap();
            gerastert += 1;
        }
        let t4 = Instant::now();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.0;
        sp.push(ms(t0, t1));
        hv.push(ms(t1, t2));
        ko.push(ms(t2, t3));
        ra.push(ms(t3, t4));
        ge.push(ms(t0, t1) + ms(t2, t4));
    }

    let bericht = Bericht {
        rechner: rechner(),
        system: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        seiten,
        tastendruecke: ZEICHEN,
        oeffnen_bis_erste_seite_ms: oeffnen,
        oeffnen_budget_ms: (1500.0, 3000.0),
        taste_spiegel: kennzahl(sp, None),
        taste_hervorhebung: kennzahl(hv, Some((16.0, 33.0))),
        taste_kompilieren: kennzahl(ko, None),
        taste_rastern: kennzahl(ra, None),
        taste_bis_seite: kennzahl(ge, Some((100.0, 250.0))),
        gerasterte_seiten_je_taste: gerastert as f64 / ZEICHEN as f64,
        hinweis: "Ohne Übertragung an den Webview und Zeichnen; Widget- und Zitat-Messung folgen mit AP6/AP7.",
    };
    let json = serde_json::to_string_pretty(&bericht).unwrap();
    let name = std::env::var("TFTN_RECHNER").unwrap_or_else(|_| std::env::consts::OS.to_string());
    std::fs::write(ausgabe.join(format!("{name}.json")), &json).unwrap();

    let zeile = |name: &str, k: &Kennzahl| {
        let urteil = match k.eingehalten {
            Some(true) => "eingehalten",
            Some(false) => "VERFEHLT",
            None => "",
        };
        println!(
            "{name:<24} p50 {:>7.1} ms   p95 {:>7.1} ms   max {:>7.1} ms   {urteil}",
            k.p50, k.p95, k.max
        );
    };
    println!(
        "{} · {} Seiten · {} Tasten",
        bericht.rechner, seiten, ZEICHEN
    );
    println!(
        "Öffnen → erste Seite     {:>7.1} ms (Budget p50 1500 ms)   {}",
        oeffnen,
        if oeffnen <= 1500.0 {
            "eingehalten"
        } else {
            "VERFEHLT"
        }
    );
    zeile("Taste → Spiegel", &bericht.taste_spiegel);
    zeile("Taste → Hervorhebung", &bericht.taste_hervorhebung);
    zeile("Taste → kompiliert", &bericht.taste_kompilieren);
    zeile("Rastern (geändert+sichtb.)", &bericht.taste_rastern);
    zeile("Taste → Seite (ohne UI)", &bericht.taste_bis_seite);
    println!(
        "gerasterte Seiten je Taste: {:.2}",
        bericht.gerasterte_seiten_je_taste
    );
    println!(
        "Bericht: {}",
        ausgabe.join(format!("{name}.json")).display()
    );
}
