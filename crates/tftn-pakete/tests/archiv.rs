//! Paketangabe, Archiv prüfen und entpacken (PLAN-M2 AP9 Task 1).

mod hilfe;

use hilfe::*;
use tar::EntryType;
use tftn_pakete::{Grenzen, PaketSpec, entpacken, sha256_hex};

fn spec() -> PaketSpec {
    PaketSpec::lesen("@preview/tftn-probe:0.1.0").unwrap()
}

#[test]
fn name_wird_geprueft() {
    let s = PaketSpec::lesen("@preview/cetz:0.4.2").unwrap();
    assert_eq!(s.name, "cetz");
    assert_eq!(s.version, semver::Version::new(0, 4, 2));
    assert_eq!(s.to_string(), "@preview/cetz:0.4.2");
    let lang = format!("@preview/{}:1.0.0", "a".repeat(65));
    for falsch in [
        "@preview/Cetz:0.4.2",
        "@preview/a/b:1.0.0",
        "@preview/-x:1.0.0",
        "@preview/x:1.0",
        "@preview/x:1.0.0-beta",
        "@local/x:1.0.0",
        "@preview/x",
        "@preview/x y:1.0.0",
        lang.as_str(),
    ] {
        assert!(PaketSpec::lesen(falsch).is_err(), "{falsch} angenommen");
    }
    assert!(PaketSpec::neu("cetz", "0.4.2").is_ok());
    assert!(PaketSpec::neu("cetz", "0.4").is_err());
}

#[test]
fn grenzen_wie_festgelegt() {
    let g = Grenzen::default();
    assert_eq!(g.archiv, 64 << 20);
    assert_eq!(g.entpackt, 256 << 20);
    assert_eq!(g.eintraege, 20_000);
}

#[test]
fn gutes_archiv() {
    let o = testordner("gutes-archiv");
    let daten = archiv(&[
        typst_toml("tftn-probe", "0.1.0"),
        datei("lib.typ", "#let probe(x) = x\n"),
        ordner_eintrag("src/"),
        datei("src/a.typ", "a"),
    ]);
    let ziel = o.join("ziel");
    let sha = entpacken(&daten, &spec(), &ziel, &Grenzen::default()).unwrap();
    assert_eq!(sha, sha256_hex(&daten));
    assert_eq!(sha.len(), 64);
    assert_eq!(
        std::fs::read_to_string(ziel.join("lib.typ")).unwrap(),
        "#let probe(x) = x\n"
    );
    assert_eq!(
        std::fs::read_to_string(ziel.join("src/a.typ")).unwrap(),
        "a"
    );

    // `./`-Präfix wie bei manchen tar-Werkzeugen
    let daten = archiv(&[
        ordner_eintrag("./"),
        typst_toml("tftn-probe", "0.1.0"),
        datei("./lib.typ", "x"),
    ]);
    let ziel = o.join("ziel2");
    entpacken(&daten, &spec(), &ziel, &Grenzen::default()).unwrap();
    assert_eq!(std::fs::read_to_string(ziel.join("lib.typ")).unwrap(), "x");
}

#[test]
fn boese_archive() {
    let gut = || typst_toml("tftn-probe", "0.1.0");
    let klein = Grenzen {
        archiv: 64 << 20,
        entpackt: 1 << 20,
        eintraege: 10,
    };
    let faelle: Vec<(&str, Vec<u8>, Grenzen, &str)> = vec![
        (
            "eltern",
            archiv(&[gut(), datei("../x", "x")]),
            Grenzen::default(),
            "Pfad",
        ),
        (
            "tief",
            archiv(&[gut(), datei("a/../../x", "x")]),
            Grenzen::default(),
            "Pfad",
        ),
        (
            "absolut",
            archiv(&[gut(), datei("/etc/x", "x")]),
            Grenzen::default(),
            "Pfad",
        ),
        (
            "laufwerk",
            archiv(&[gut(), datei("C:x", "x")]),
            Grenzen::default(),
            "Pfad",
        ),
        (
            "rueckstrich",
            archiv(&[gut(), datei("a\\b", "x")]),
            Grenzen::default(),
            "Pfad",
        ),
        (
            "symlink",
            archiv(&[gut(), verweis(EntryType::Symlink, "s", "/etc/passwd")]),
            Grenzen::default(),
            "Verknüpfung",
        ),
        (
            "hardlink",
            archiv(&[gut(), verweis(EntryType::Link, "h", "typst.toml")]),
            Grenzen::default(),
            "Verknüpfung",
        ),
        (
            "geraet",
            archiv(&[gut(), verweis(EntryType::Char, "c", "")]),
            Grenzen::default(),
            "Eintragsart",
        ),
        (
            "ohne-toml",
            archiv(&[datei("lib.typ", "x")]),
            Grenzen::default(),
            "typst.toml fehlt",
        ),
        (
            "name",
            archiv(&[typst_toml("anders", "0.1.0")]),
            Grenzen::default(),
            "Name",
        ),
        (
            "version",
            archiv(&[typst_toml("tftn-probe", "0.2.0")]),
            Grenzen::default(),
            "Version",
        ),
        (
            "bombe",
            archiv_mit_nullen(&[gut()], "null.bin", 2 << 20),
            klein.clone(),
            "zu groß",
        ),
        (
            "viele",
            archiv(
                &std::iter::once(gut())
                    .chain((0..10).map(|i| datei(&format!("d{i}"), "")))
                    .collect::<Vec<_>>(),
            ),
            klein,
            "zu viele Einträge",
        ),
        (
            "kein-gzip",
            b"kein archiv".to_vec(),
            Grenzen::default(),
            "Archiv",
        ),
    ];
    for (name, daten, g, grund) in faelle {
        let o = testordner(&format!("boese-{name}"));
        let ziel = o.join("pakete/ziel");
        let fehler = entpacken(&daten, &spec(), &ziel, &g)
            .expect_err(name)
            .to_string();
        assert!(fehler.contains(grund), "{name}: {fehler}");
        assert!(!ziel.exists(), "{name}: Ziel blieb liegen");
        assert_eq!(inhalt(&o.join("pakete")), Vec::<String>::new(), "{name}");
        assert_eq!(inhalt(&o), vec!["pakete".to_string()], "{name}");
    }
    // Archiv selbst über der Grenze
    let o = testordner("boese-archivgroesse");
    let g = Grenzen {
        archiv: 10,
        ..Grenzen::default()
    };
    let fehler = entpacken(&archiv(&[gut()]), &spec(), &o.join("ziel"), &g).unwrap_err();
    assert!(fehler.to_string().contains("zu groß"), "{fehler}");
    assert!(!o.join("ziel").exists());
}

#[test]
fn ziel_nicht_leer() {
    let o = testordner("nicht-leer");
    let ziel = o.join("ziel");
    std::fs::create_dir_all(&ziel).unwrap();
    std::fs::write(ziel.join("da.txt"), "bleibt").unwrap();
    let daten = archiv(&[typst_toml("tftn-probe", "0.1.0")]);
    assert!(entpacken(&daten, &spec(), &ziel, &Grenzen::default()).is_err());
    assert_eq!(
        std::fs::read_to_string(ziel.join("da.txt")).unwrap(),
        "bleibt"
    );
}

/// Abschlussprüfung: tar liest Erweiterungsköpfe (GNU-LongName, PAX) selbst ganz in den Speicher;
/// auch sie müssen an die Grenze stoßen.
#[test]
fn bombe_im_erweiterungskopf() {
    let klein = Grenzen {
        archiv: 64 << 20,
        entpackt: 1 << 20,
        eintraege: 10,
    };
    for art in [
        tar::EntryType::GNULongName,
        tar::EntryType::XHeader,
        tar::EntryType::Directory,
    ] {
        let o = testordner(&format!("bombe-kopf-{art:?}"));
        let daten = archiv_mit_kopf(art, 8 << 20);
        let ziel = o.join("ziel");
        let fehler = entpacken(&daten, &spec(), &ziel, &klein)
            .unwrap_err()
            .to_string();
        assert!(
            fehler.contains("zu groß"),
            "{art:?}: {}",
            fehler.chars().take(200).collect::<String>()
        );
        assert!(!ziel.exists());
    }
}

/// Abschlussprüfung AP9: Namen, die unter Windows nicht gehen, lehnt TFTN überall ab – ein Paket
/// soll auf beiden Rechnern gleich entpacken.
#[test]
fn windows_namen_abgelehnt() {
    let gut = || typst_toml("tftn-probe", "0.1.0");
    for (i, name) in [
        "a<b.typ",
        "a>b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "CON",
        "nul.typ",
        "aux.txt",
        "com1",
        "LPT9.typ",
        "ende.",
        "ende ",
        "ordner./x.typ",
    ]
    .iter()
    .enumerate()
    {
        let o = testordner(&format!("windows-{i}"));
        let fehler = entpacken(
            &archiv(&[gut(), datei(name, "x")]),
            &spec(),
            &o.join("ziel"),
            &Grenzen::default(),
        )
        .expect_err(name)
        .to_string();
        assert!(fehler.contains("Pfad"), "{name}: {fehler}");
        assert!(!o.join("ziel").exists(), "{name}");
    }
    // Ähnliche, erlaubte Namen
    for (i, name) in ["console.typ", "nullen.typ", "a.b.typ", "com10.typ"]
        .iter()
        .enumerate()
    {
        let o = testordner(&format!("windows-gut-{i}"));
        entpacken(
            &archiv(&[gut(), datei(name, "x")]),
            &spec(),
            &o.join("ziel"),
            &Grenzen::default(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}
