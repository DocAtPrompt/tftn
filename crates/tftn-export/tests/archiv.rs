//! Archivpaket: Das Zip enthält, was das Dokument braucht (auch Schriften aus dem Vorrat der App),
//! aber nichts aus `_lokal`, `_renders`, `export` oder versteckten Ordnern. Ausgepackt und ohne die
//! Schriften der App gesetzt, ergibt es dasselbe PDF (gleiche SHA-256).

use std::path::Path;

use sha2::{Digest, Sha256};
use tftn_core::Projekt;
use tftn_core::gegenprobe::gegenprobe_argumente_eigen;
use tftn_export::archiv::archiv_erzeugen;
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren_argumente, zip};

fn schreiben(o: &Path, name: &str, inhalt: &str) {
    let p = o.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, inhalt).unwrap();
}

#[test]
fn archiv_ist_vollstaendig_und_setzt_dasselbe_pdf() {
    let basis = Path::new(env!("CARGO_TARGET_TMPDIR")).join("archiv");
    let _ = std::fs::remove_dir_all(&basis);
    let o = basis.join("Befund Huber");
    schreiben(
        &o,
        "tftn.toml",
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    );
    schreiben(
        &o,
        "main.typ",
        "#set text(font: \"Libertinus Serif\", lang: \"de\")\n= Grüße\nStraße, $a^2 + b^2 = c^2$\n#image(\"abbildungen/punkt.svg\", width: 1cm)\n",
    );
    schreiben(
        &o,
        "abbildungen/punkt.svg",
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><circle cx=\"5\" cy=\"5\" r=\"4\"/></svg>",
    );
    schreiben(&o, "_lokal/serie/geheim.toml", "name = \"x\"\n");
    schreiben(&o, "_renders/probe.pdf", "x");
    schreiben(&o, "export/alt.pdf", "x");
    schreiben(&o, ".fassungen/1.typ", "x");
    schreiben(&o, "abbildungen/_renders/y.png", "x");

    // Schriften nur aus dem Vorrat, wie bei einem neuen Dokument in der App.
    let vorrat =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types/tftn-fachartikel/schriften");
    tftn_core::schriftvorrat_setzen(vec![vorrat]);
    let p = Projekt::oeffnen(&o).unwrap();
    let cli = TypstCli::finden(&p.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let a = archiv_erzeugen(&p, &cli, "9.9.9").unwrap();

    assert!(
        a.relativ.starts_with("export/Befund-Huber-archiv-"),
        "{}",
        a.relativ
    );
    assert!(a.relativ.ends_with(".zip"));
    assert_eq!(a.pdf, "Befund-Huber.pdf");
    assert_eq!(a.standard, "a-2b");
    assert!(
        !o.join("fonts").exists(),
        "das Projekt selbst bleibt unverändert"
    );

    let inhalt = zip::inhalt(&std::fs::read(&a.pfad).unwrap()).unwrap();
    assert_eq!(inhalt.len(), a.dateien);
    let paket = a
        .relativ
        .trim_start_matches("export/")
        .trim_end_matches(".zip");
    let namen: Vec<&str> = inhalt
        .iter()
        .map(|(n, _)| {
            n.strip_prefix(&format!("{paket}/"))
                .expect("alles im Paketordner")
        })
        .collect();
    assert_eq!(namen[0], "LIESMICH.txt");
    assert_eq!(namen[1], "Befund-Huber.pdf");
    for n in [
        "main.typ",
        "tftn.toml",
        "abbildungen/punkt.svg",
        "fonts/LibertinusSerif-Regular.otf",
        "fonts/LIZENZEN.txt",
    ] {
        assert!(namen.contains(&n), "{n} fehlt: {namen:?}");
    }
    assert!(
        namen.iter().any(|n| n.starts_with("fonts/NewCMMath")),
        "Formelschrift fehlt: {namen:?}"
    );
    for n in &namen {
        assert!(
            !n.contains("_lokal")
                && !n.contains("_renders")
                && !n.starts_with("export/")
                && !n.starts_with('.'),
            "{n} gehört nicht ins Archiv"
        );
    }
    let liesmich = String::from_utf8(inhalt[0].1.clone()).unwrap();
    assert!(liesmich.contains(&a.sha256), "{liesmich}");
    assert!(liesmich.contains("TFTN 9.9.9"));
    assert!(
        liesmich.contains("--font-path fonts --pdf-standard a-2b main.typ Befund-Huber.pdf"),
        "{liesmich}"
    );
    assert!(liesmich.contains("$env:SOURCE_DATE_EPOCH="));
    assert!(
        !liesmich.contains("schriften"),
        "keine Ordner der App: {liesmich}"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&inhalt[1].1)),
        a.sha256,
        "SHA-256 in der Liesmich gehört zum PDF im Zip"
    );

    // Ein zweiter Lauf am selben Tag ersetzt das Zip.
    let b = archiv_erzeugen(&p, &cli, "9.9.9").unwrap();
    assert_eq!(b.pfad, a.pfad);
    assert_eq!(b.sha256, a.sha256);

    // Auspacken und ohne den Vorrat der App neu setzen: dasselbe PDF.
    let aus = basis.join("ausgepackt");
    for (name, daten) in &inhalt {
        let ziel = aus.join(name);
        std::fs::create_dir_all(ziel.parent().unwrap()).unwrap();
        std::fs::write(ziel, daten).unwrap();
    }
    let ordner = aus.join(paket);
    std::fs::remove_file(ordner.join(&a.pdf)).unwrap();
    tftn_core::schriftvorrat_setzen(Vec::new());
    let q = Projekt::oeffnen(&ordner).unwrap();
    let zusatz = vec!["--pdf-standard".to_string(), "a-2b".to_string()];
    ausfuehren_argumente(
        &q,
        &cli,
        &gegenprobe_argumente_eigen(&q, &a.pdf, &zusatz),
        STANDARD_FRIST,
    )
    .unwrap();
    let neu = std::fs::read(ordner.join(&a.pdf)).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&neu)),
        a.sha256,
        "neu gesetztes PDF weicht ab"
    );
}
