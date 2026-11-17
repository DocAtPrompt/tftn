//! Eine weitere Schrift kommt als Familie nach `fonts/`: Danach setzen Vorschau und typst-CLI
//! pixelgleich mit ihr, und die Gegenprobe-Zeile nennt nur `fonts`.

use std::path::{Path, PathBuf};

use tftn_core::gegenprobe::gegenprobe_zeile;
use tftn_core::schriften::{benutzt, dateifamilien, familie_einbinden};
use tftn_core::{Projekt, ProjektWorld, kompilieren, seiten_png};
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren};

#[test]
fn eingebundene_schrift_setzt_in_vorschau_und_cli_gleich() {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join("schriftwahl");
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(
        o.join("main.typ"),
        "#set text(font: \"Source Sans 3\", lang: \"de\")\n= Grüße\nStraße, Äpfel, „Öl“ – *fett* und _kursiv_.\n",
    )
    .unwrap();
    let p = Projekt::oeffnen(&o).unwrap();

    let datei = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../types/tftn-leer/schriften/SourceSans3-Regular.otf");
    let familie = dateifamilien(&datei)
        .into_iter()
        .find(|f| f.name == "Source Sans 3")
        .expect("Source Sans 3 in der Datei");
    assert_eq!(familie.dateien.len(), 4, "{:?}", familie.dateien);
    assert_eq!(familie_einbinden(&p, &familie).unwrap().len(), 4);

    let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
    let doc = k.dokument.expect("übersetzt");
    assert!(k.meldungen.is_empty(), "{:?}", k.meldungen);
    assert!(benutzt(&doc).iter().all(|(f, _)| f == "Source Sans 3"));
    assert!(gegenprobe_zeile(&p, "x.pdf", &[]).contains("--font-path fonts main.typ"));

    let cli = TypstCli::finden(&p.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let bilder = o.join("_renders");
    std::fs::create_dir_all(&bilder).unwrap();
    let zusatz: Vec<String> = ["--format", "png", "--ppi", "144"]
        .map(String::from)
        .to_vec();
    ausfuehren(
        &p,
        &cli,
        &bilder.join("seite-{0p}.png"),
        &zusatz,
        STANDARD_FRIST,
    )
    .unwrap();
    let mut cli_seiten: Vec<PathBuf> = std::fs::read_dir(&bilder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    cli_seiten.sort();
    let vorschau = seiten_png(&doc, 144.0);
    assert_eq!(cli_seiten.len(), vorschau.len());
    for (datei, bild) in cli_seiten.iter().zip(&vorschau) {
        let export = tiny_skia::Pixmap::decode_png(&std::fs::read(datei).unwrap()).unwrap();
        let vorschau = tiny_skia::Pixmap::decode_png(bild).unwrap();
        assert!(export.data() == vorschau.data(), "Pixel weichen ab");
    }
}
