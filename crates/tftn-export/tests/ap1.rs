//! Abnahmeprüfungen AP1 (PLAN-M1): Export = Gegenprobe, falsche Version, Vorschau = Export.
//!
//! Braucht die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seiten_png};
use tftn_export::{ExportFehler, STANDARD_FRIST, TypstCli, ausfuehren, exportieren};

fn testartikel() -> Projekt {
    let ordner = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    Projekt::oeffnen(&ordner).expect("Testartikel öffnen")
}

fn cli(projekt: &Projekt) -> TypstCli {
    TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig")
}

fn arbeitsordner(name: &str) -> PathBuf {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).unwrap();
    ordner
}

/// (a) Zwei Exporte und die Gegenprobe-Zeile ergeben dieselbe Datei.
#[test]
fn export_ist_bytegleich_und_wiederholbar() {
    let projekt = testartikel();
    let cli = cli(&projekt);
    let ordner = arbeitsordner("ap1-a");
    let eins = ordner.join("eins.pdf");
    let zwei = ordner.join("zwei.pdf");
    exportieren(&projekt, &cli, &eins, &[]).unwrap();
    exportieren(&projekt, &cli, &zwei, &[]).unwrap();
    assert_eq!(std::fs::read(&eins).unwrap(), std::fs::read(&zwei).unwrap());
}

/// (b) Eine CLI mit anderer Version wird abgelehnt.
#[test]
fn falsche_version_wird_abgelehnt() {
    let projekt = testartikel();
    let cli = cli(&projekt);
    match TypstCli::pruefen(&cli.pfad, "0.14.0") {
        Err(ExportFehler::FalscheVersion {
            gefunden, erwartet, ..
        }) => {
            assert_eq!(gefunden, projekt.konfig.typst);
            assert_eq!(erwartet, "0.14.0");
        }
        anders => panic!("FalscheVersion erwartet, bekam {anders:?}"),
    }
}

/// (c) Die Seitenbilder der Vorschau-World sind pixelgleich mit `typst compile --format png`.
/// Verglichen werden die entpackten Pixel: Die CLI (Homebrew wie offizielles Release) komprimiert
/// das PNG anders als unser Bau, die IDAT-Bytes weichen ab, die Pixel nicht (2026-10-31).
#[test]
fn vorschau_gleich_export() {
    let projekt = testartikel();
    let cli = cli(&projekt);
    let ordner = arbeitsordner("ap1-c");
    let muster = ordner.join("seite-{0p}.png");
    let zusatz: Vec<String> = ["--format", "png", "--ppi", "144"]
        .map(String::from)
        .to_vec();
    ausfuehren(&projekt, &cli, &muster, &zusatz, STANDARD_FRIST).unwrap();

    let world = ProjektWorld::neu(&projekt).unwrap();
    let dokument = kompilieren(&world)
        .dokument
        .expect("Testartikel kompiliert");
    let vorschau = seiten_png(&dokument, 144.0);

    let mut cli_seiten: Vec<PathBuf> = std::fs::read_dir(&ordner)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    cli_seiten.sort();
    assert_eq!(
        cli_seiten.len(),
        vorschau.len(),
        "Seitenzahl Vorschau ≠ Export"
    );
    for (nr, (datei, bild)) in cli_seiten.iter().zip(&vorschau).enumerate() {
        let export = tiny_skia::Pixmap::decode_png(&std::fs::read(datei).unwrap()).unwrap();
        let vorschau = tiny_skia::Pixmap::decode_png(bild).unwrap();
        assert_eq!(
            (export.width(), export.height()),
            (vorschau.width(), vorschau.height()),
            "Seite {}: Größe",
            nr + 1
        );
        assert!(
            export.data() == vorschau.data(),
            "Seite {}: Pixel weichen ab",
            nr + 1
        );
    }
}

/// Die Vorprüfung meldet fehlende Pakete, bevor die CLI ins Netz gehen könnte.
#[test]
fn fehlendes_paket_stoppt_den_export() {
    let ordner = arbeitsordner("ap1-paket");
    std::fs::write(
        ordner.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(
        ordner.join("main.typ"),
        "#import \"@preview/gibt-es-nicht:0.1.0\": *\nHallo\n",
    )
    .unwrap();
    let projekt = Projekt::oeffnen(&ordner).unwrap();
    let cli = cli(&projekt);
    match exportieren(&projekt, &cli, &ordner.join("x.pdf"), &[]) {
        Err(ExportFehler::FehlendePakete(p)) => {
            assert_eq!(p, vec!["@preview/gibt-es-nicht:0.1.0".to_string()]);
        }
        anders => panic!("FehlendePakete erwartet, bekam {anders:?}"),
    }
    assert!(!ordner.join("x.pdf").exists());
    assert!(!ordner.join("packages").exists(), "nichts heruntergeladen");
}
