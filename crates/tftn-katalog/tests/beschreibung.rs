//! Paketbeschreibungen und Pakete im Projekt (PLAN-M2 AP1, Task 4).

use std::path::{Path, PathBuf};

use tftn_katalog::{beschreibungen_lesen, projektpakete};

fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten")
}

/// Leerer Ordner unter `CARGO_TARGET_TMPDIR`.
fn tmp(name: &str) -> PathBuf {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("beschreibung-{name}"));
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).unwrap();
    ordner
}

#[test]
fn beschreibung_wird_gelesen() {
    let (b, warnungen) = beschreibungen_lesen(&daten().join("beschreibungen"));
    assert!(warnungen.is_empty(), "{warnungen:?}");
    assert_eq!(b[0].paket, "demo");
    assert!(b[0].versionen.matches(&semver::Version::new(0, 1, 3)));
    assert!(!b[0].versionen.matches(&semver::Version::new(0, 2, 0)));
    assert_eq!(b[0].baustein[0].id, "kasten");
}

#[test]
fn kaputte_datei_ist_warnung() {
    let ordner = tmp("kaputt");
    std::fs::write(ordner.join("x.toml"), "paket = ").unwrap();
    let (b, warnungen) = beschreibungen_lesen(&ordner);
    assert!(b.is_empty());
    assert!(warnungen[0].contains("x.toml"), "{warnungen:?}");
}

#[test]
fn pakete_im_projekt() {
    let p = tmp("projekt");
    std::fs::create_dir_all(p.join("packages/preview/demo/0.1.3")).unwrap();
    std::fs::write(
        p.join("packages/preview/demo/0.1.3/typst.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.3\"\nentrypoint = \"lib.typ\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(p.join("packages/preview/ohne-toml/1.0.0")).unwrap();
    assert_eq!(
        projektpakete(&p.join("packages")),
        vec![("demo".to_string(), semver::Version::new(0, 1, 3))]
    );
}

const GUT: &str = r#"paket = "demo"
versionen = ">=0.1.0, <0.2.0"
import = '#import "@preview/demo:{version}"'
"#;

#[test]
fn beschreibung_neue_schluessel() {
    let ordner = tmp("neue-schluessel");
    std::fs::write(
        ordner.join("demo.toml"),
        format!("{GUT}titel = \"Demo-Helfer\"\nempfohlen = \"0.1.5\"\nauch = [\"basis\"]\n"),
    )
    .unwrap();
    let (b, w) = beschreibungen_lesen(&ordner);
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(b[0].titel.as_deref(), Some("Demo-Helfer"));
    assert_eq!(b[0].empfohlen, Some(semver::Version::new(0, 1, 5)));
    assert_eq!(b[0].auch, vec!["basis".to_string()]);
}

/// Review Focus 3: Jeder Fehler ist eine Warnung mit Dateiname, der Rest bleibt.
#[test]
fn eigene_beschreibung_fehlerfaelle() {
    let ordner = tmp("fehlerfaelle");
    let kopf = "paket = \"demo\"\nversionen = \">=0.1.0, <0.2.0\"\n";
    let import = "import = '#import \"@preview/demo:{version}\"'\n";
    for (datei, text) in [
        ("a-toml.toml", "paket = ".to_string()),
        (
            "b-bereich.toml",
            format!("paket = \"demo\"\nversionen = \"nicht gültig\"\n{import}"),
        ),
        (
            "c-fremd.toml",
            format!("{kopf}import = '#import \"@preview/anderes:{{version}}\"'\n"),
        ),
        (
            "d-zwei.toml",
            format!(
                "{kopf}import = '''#import \"@preview/demo:{{version}}\"\n#import \"@preview/demo:{{version}}\"'''\n"
            ),
        ),
        (
            "e-empfohlen.toml",
            format!("{kopf}{import}empfohlen = \"0.2.0\"\n"),
        ),
        (
            "f-kein-import.toml",
            format!("{kopf}import = 'kein Import'\n"),
        ),
        ("g-auch.toml", format!("{kopf}{import}auch = [\"A/b\"]\n")),
    ] {
        std::fs::write(ordner.join(datei), text).unwrap();
    }
    std::fs::write(ordner.join("z-gut.toml"), GUT).unwrap();
    let (b, w) = beschreibungen_lesen(&ordner);
    assert_eq!(b.len(), 1, "{w:?}");
    assert!(b[0].datei.ends_with("z-gut.toml"));
    assert_eq!(w.len(), 7, "{w:?}");
    for name in [
        "a-toml",
        "b-bereich",
        "c-fremd",
        "d-zwei",
        "e-empfohlen",
        "f-kein-import",
        "g-auch",
    ] {
        assert!(w.iter().any(|z| z.contains(name)), "{name}: {w:?}");
    }
}

#[test]
fn fehlender_ordner_ist_keine_warnung() {
    let (b, w) = beschreibungen_lesen(&tmp("fehlt").join("gibtsnicht"));
    assert!(b.is_empty() && w.is_empty(), "{w:?}");
}
