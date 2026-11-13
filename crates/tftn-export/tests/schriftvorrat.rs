//! Dokumente ohne eingebundene Schriften nehmen den Vorrat der App: Vorschau, Export und
//! Gegenprobe-Zeile nennen dieselben Ordner.

use std::path::Path;

use tftn_core::gegenprobe::gegenprobe_argumente;
use tftn_core::{Projekt, ProjektWorld, kompilieren, schriftvorrat_setzen};
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren, exportieren};

#[test]
fn ohne_eingebundene_schriften_gilt_der_vorrat() {
    let vorrat = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../types/tftn-fachartikel/schriften")
        .canonicalize()
        .unwrap();
    // Alle Tests setzen denselben Vorrat: Sie laufen parallel in einem Prozess.
    schriftvorrat_setzen(vec![
        vorrat.clone(),
        vorrat.join("../../tftn-leer/schriften"),
    ]);
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join("schriftvorrat");
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(
        o.join("main.typ"),
        "#set text(font: \"Libertinus Serif\")\nHallo\n",
    )
    .unwrap();
    let p = Projekt::oeffnen(&o).unwrap();

    let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
    assert!(k.meldungen.iter().all(|m| !m.fehler), "{:?}", k.meldungen);
    assert!(
        k.meldungen.iter().all(|m| !m.text.contains("font")),
        "{:?}",
        k.meldungen
    );

    let args = gegenprobe_argumente(&p, "x.pdf", &[]);
    let i = args.iter().position(|a| a == "--font-path").unwrap();
    assert_eq!(args[i + 1], vorrat.to_string_lossy());

    let cli = TypstCli::finden(&p.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    exportieren(&p, &cli, Path::new("a.pdf"), &[]).unwrap();
    ausfuehren(&p, &cli, &p.wurzel.join("b.pdf"), &[], STANDARD_FRIST).unwrap();
    assert_eq!(
        std::fs::read(p.wurzel.join("a.pdf")).unwrap(),
        std::fs::read(p.wurzel.join("b.pdf")).unwrap()
    );

    // Mit eingebundenen Schriften kommt `fonts` zuerst, und die Zeile zum Abtippen nennt nur sie.
    std::fs::create_dir_all(o.join("fonts")).unwrap();
    let args = gegenprobe_argumente(&p, "x.pdf", &[]);
    let i = args.iter().position(|a| a == "--font-path").unwrap();
    assert_eq!(args[i + 1], "fonts");
    let zeile = tftn_core::gegenprobe::gegenprobe_zeile(&p, "x.pdf", &[]);
    assert!(zeile.contains("--font-path fonts"), "{zeile}");
    assert!(!zeile.contains(&*vorrat.to_string_lossy()), "{zeile}");
}

fn projekt_mit(name: &str, text: &str) -> Projekt {
    let typen = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../types")
        .canonicalize()
        .unwrap();
    schriftvorrat_setzen(vec![
        typen.join("tftn-fachartikel/schriften"),
        typen.join("tftn-leer/schriften"),
    ]);
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(o.join("main.typ"), text).unwrap();
    Projekt::oeffnen(&o).unwrap()
}

fn satz(p: &Projekt) -> tftn_core::PagedDocument {
    let k = kompilieren(&ProjektWorld::neu(p).unwrap());
    assert!(k.meldungen.iter().all(|m| !m.fehler), "{:?}", k.meldungen);
    k.dokument.unwrap()
}

fn dateien(p: &Projekt) -> (Vec<String>, u64) {
    let mut namen = Vec::new();
    let mut summe = 0;
    for e in std::fs::read_dir(p.schriften()).unwrap() {
        let e = e.unwrap();
        summe += e.metadata().unwrap().len();
        namen.push(e.file_name().to_string_lossy().into_owned());
    }
    namen.sort();
    (namen, summe)
}

#[test]
fn einbinden_nur_was_benutzt_wird() {
    let p = projekt_mit("einbinden-sparsam", "Hallo *fett* und _kursiv_, $x^2$.\n");
    let doc = satz(&p);
    tftn_core::schriften::einbinden(&p, &doc).unwrap();
    let (namen, summe) = dateien(&p);
    // Die ganze Textfamilie, von der Formelschrift nur der benutzte Schnitt, keine Codeschrift.
    assert_eq!(
        namen
            .iter()
            .filter(|n| n.starts_with("LibertinusSerif"))
            .count(),
        6,
        "{namen:?}"
    );
    assert_eq!(
        namen.iter().filter(|n| n.starts_with("NewCMMath")).count(),
        1,
        "{namen:?}"
    );
    assert!(
        !namen
            .iter()
            .any(|n| n.starts_with("DejaVu") || n.starts_with("Source")),
        "{namen:?}"
    );
    assert!(namen.iter().any(|n| n == "LIZENZEN.txt"), "{namen:?}");
    assert!(summe < 3_500_000, "{summe} Bytes: {namen:?}");
    // Danach übersetzt es mit den eigenen Schriften gleich.
    assert_eq!(satz(&p).pages().len(), doc.pages().len());
}

#[test]
fn einbinden_ergaenzt_eine_neue_schrift() {
    let p = projekt_mit("einbinden-neu", "Hallo\n");
    tftn_core::schriften::einbinden(&p, &satz(&p)).unwrap();
    std::fs::write(p.haupt(), "#set text(font: \"Source Sans 3\")\nHallo\n").unwrap();
    let neu = tftn_core::schriften::einbinden(&p, &satz(&p)).unwrap();
    assert_eq!(
        neu.iter().filter(|n| n.starts_with("SourceSans3")).count(),
        4,
        "{neu:?}"
    );
    assert!(neu.iter().any(|n| n == "LIZENZEN-Source.txt"), "{neu:?}");
    // Nichts doppelt
    assert!(
        tftn_core::schriften::einbinden(&p, &satz(&p))
            .unwrap()
            .is_empty()
    );
}

/// Gleiche Schriften, anderer Text: gleicher Fingerabdruck; eine neue Schrift ändert ihn.
#[test]
fn fingerabdruck_nur_bei_neuer_schrift_anders() {
    let a = projekt_mit("abdruck-a", "Hallo *fett*.\n");
    let b = projekt_mit("abdruck-b", "Ganz *anderer* Text, länger.\n");
    let c = projekt_mit("abdruck-c", "Hallo *fett* und $x^2$.\n");
    let fa = tftn_core::schriften::fingerabdruck(&satz(&a));
    assert_eq!(fa, tftn_core::schriften::fingerabdruck(&satz(&b)));
    assert_ne!(fa, tftn_core::schriften::fingerabdruck(&satz(&c)));
}

/// Echte Fehler von typst kommen deutsch an, mit Tipp und dem Wortlaut von typst.
#[test]
fn meldungen_auf_deutsch() {
    let faelle = [
        ("deutsch-formel", "$ab$\n", "Unbekannter Name „ab“."),
        (
            "deutsch-bild",
            "#image(\"bilder/fehlt.png\")\n",
            "Datei nicht gefunden:",
        ),
        ("deutsch-marke", "Siehe @nirgends.\n", "Verweis ins Leere"),
        (
            "deutsch-wert",
            "#text(size: \"groß\")[x]\n",
            "Hier gehört eine Länge",
        ),
        (
            "deutsch-klammer",
            "#text(fill: red\n",
            "Klammer nicht geschlossen.",
        ),
    ];
    for (name, text, erwartet) in faelle {
        let p = projekt_mit(name, text);
        let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
        let m = k.meldungen.iter().find(|m| m.fehler).expect(name);
        assert!(m.text.starts_with(erwartet), "{name}: {}", m.text);
        assert!(m.text.contains("(typst: "), "{name}: {}", m.text);
    }
}

/// Die Sprache, in der der meiste Text gesetzt ist, auch wenn eine Vorlage sie setzt.
#[test]
fn hauptsprache_des_satzes() {
    let faelle = [
        ("sprache-standard", "Hallo Welt.\n", ("en", None)),
        (
            "sprache-at",
            "#set text(lang: \"de\", region: \"AT\")\nGrüß Gott im Jänner.\n",
            ("de", Some("AT")),
        ),
        (
            "sprache-gemischt",
            "#set text(lang: \"fr\")\nUn long texte en français ici. #text(lang: \"de\")[Kurz.]\n",
            ("fr", None),
        ),
    ];
    for (name, text, (lang, region)) in faelle {
        let (l, r) = tftn_core::sprache::hauptsprache(&satz(&projekt_mit(name, text))).unwrap();
        assert_eq!((l.as_str(), r.as_deref()), (lang, region), "{name}");
    }
}
