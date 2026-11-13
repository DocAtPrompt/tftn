//! Katalog aus Typst, Dokumenttyp und Paketbeschreibungen (PLAN-M2 AP1, Task 5).

use std::path::{Path, PathBuf};

use tftn_core::Projekt;
use tftn_katalog::{Herkunft, Katalog, Quellen, eingebaut};
use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn daten() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/daten")
}

fn tmp(name: &str) -> PathBuf {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).unwrap();
    ordner
}

fn neues_projekt(name: &str) -> Projekt {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types");
    projekt_anlegen(&typen, "tftn-fachartikel", &ordner, Some(ZEITSTEMPEL))
        .unwrap()
        .projekt
}

/// Legt `packages/preview/<name>/<version>/typst.toml` an.
fn paket_ablegen(projekt: &Projekt, name: &str, version: &str) {
    let ordner = projekt.pakete().join("preview").join(name).join(version);
    std::fs::create_dir_all(&ordner).unwrap();
    std::fs::write(
        ordner.join("typst.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"lib.typ\"\n"
        ),
    )
    .unwrap();
}

#[test]
fn fachartikel_hat_typst_und_hinweis() {
    let projekt = neues_projekt("katalog-fachartikel");
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[],
    });
    assert!(w.is_empty(), "{w:?}");
    assert!(k.finden("typst/tabelle").is_some());
    let h = k.finden("tftn-fachartikel/hinweis").unwrap();
    assert!(
        matches!(&h.herkunft, Herkunft::Dokumenttyp { spec } if spec == "@local/tftn-fachartikel:0.3.0")
    );
    let widgets = k.widgets();
    assert!(widgets.iter().any(|w| w.parameter.funktion == "hinweis"
        && w.parameter.parameter == "farbe"
        && w.parameter.art.name() == "farbe"
        && w.baustein == "Hinweiskasten"));
}

#[test]
fn ohne_dokumenttyp_nur_typst() {
    let (k, _) = Katalog::bauen(&Quellen {
        projekt: None,
        beschreibungen: &[],
    });
    assert_eq!(k.eintraege().len(), eingebaut().len());
}

#[test]
fn projekt_ohne_typ_hat_typst_bausteine() {
    let projekt = neues_projekt("katalog-ohne-typ");
    let toml = projekt.wurzel.join("tftn.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    let ohne: String = text
        .lines()
        .filter(|z| !z.starts_with("typ "))
        .map(|z| format!("{z}\n"))
        .collect();
    assert_ne!(ohne, text, "tftn.toml hatte keine typ-Zeile");
    std::fs::write(&toml, ohne).unwrap();
    let projekt = Projekt::oeffnen(&projekt.wurzel).unwrap();
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[],
    });
    assert_eq!(k.eintraege().len(), eingebaut().len());
    assert!(!w.is_empty(), "fehlender Typ soll gemeldet werden");
}

#[test]
fn kaputter_baustein_faellt_weg_mit_warnung() {
    let projekt = neues_projekt("katalog-kaputt");
    let toml = projekt
        .pakete()
        .join("local/tftn-fachartikel/0.3.0/typst.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str("\n[[tool.tftn.baustein]]\nid = \"kaputt\"\nfunktion = \"hinweis\"\ntitel = \"K\"\neinfuegen = \"#hinweis[${feld:X\"\n");
    std::fs::write(&toml, text).unwrap();
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[],
    });
    assert!(k.finden("tftn-fachartikel/hinweis").is_some());
    assert!(k.finden("tftn-fachartikel/kaputt").is_none());
    assert!(w.iter().any(|w| w.starts_with("kaputt:")), "{w:?}");
}

#[test]
fn paketbeschreibung_nur_bei_passender_version() {
    let ordner = [daten().join("beschreibungen")];

    let projekt = neues_projekt("katalog-paket");
    paket_ablegen(&projekt, "demo", "0.1.3");
    let (k, _) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &ordner,
    });
    let e = k.finden("@preview/demo/kasten").unwrap();
    assert!(
        matches!(&e.herkunft, Herkunft::Paket { import, .. } if import == "#import \"@preview/demo:0.1.3\": kasten")
    );

    let projekt = neues_projekt("katalog-paket-alt");
    paket_ablegen(&projekt, "demo", "0.2.0");
    let (k, _) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &ordner,
    });
    assert!(k.finden("@preview/demo/kasten").is_none());
}

#[test]
fn eigener_ordner_hat_vorrang() {
    let projekt = neues_projekt("katalog-vorrang");
    paket_ablegen(&projekt, "demo", "0.1.3");
    let eigen = tmp("katalog-eigen");
    std::fs::write(
        eigen.join("demo.toml"),
        std::fs::read_to_string(daten().join("beschreibungen/demo-0.1.toml"))
            .unwrap()
            .replace("Demo-Kasten", "Mein Kasten"),
    )
    .unwrap();
    let ordner = [eigen, daten().join("beschreibungen")];
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &ordner,
    });
    assert_eq!(
        k.finden("@preview/demo/kasten").unwrap().titel,
        "Mein Kasten"
    );
    assert!(w.iter().any(|w| w.contains("überdeckt")), "{w:?}");
}

#[test]
fn doppelte_id_im_manifest_wird_gemeldet_erster_bleibt() {
    let projekt = neues_projekt("katalog-doppelt");
    let toml = projekt
        .pakete()
        .join("local/tftn-fachartikel/0.3.0/typst.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str("\n[[tool.tftn.baustein]]\nid = \"hinweis\"\nfunktion = \"hinweis\"\ntitel = \"Zweiter\"\neinfuegen = \"#hinweis[]\"\n");
    std::fs::write(&toml, text).unwrap();
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[],
    });
    let gleich: Vec<_> = k
        .eintraege()
        .iter()
        .filter(|e| e.id == "tftn-fachartikel/hinweis")
        .collect();
    assert_eq!(gleich.len(), 1);
    assert_eq!(gleich[0].titel, "Hinweiskasten");
    assert!(w.iter().any(|w| w.contains("doppelt")), "{w:?}");
}

#[test]
fn zwei_paketversionen_neueste_gewinnt() {
    let projekt = neues_projekt("katalog-zwei-versionen");
    paket_ablegen(&projekt, "demo", "0.1.3");
    paket_ablegen(&projekt, "demo", "0.1.10");
    let ordner = [daten().join("beschreibungen")];
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &ordner,
    });
    let e = k.finden("@preview/demo/kasten").unwrap();
    assert!(
        matches!(&e.herkunft, Herkunft::Paket { import, .. } if import == "#import \"@preview/demo:0.1.10\": kasten"),
        "{:?}",
        e.herkunft
    );
    assert_eq!(
        k.eintraege()
            .iter()
            .filter(|e| e.id == "@preview/demo/kasten")
            .count(),
        1
    );
    // Mehrere Versionen sind normal: Abhängigkeiten bringen ihre eigenen mit.
    assert!(!w.iter().any(|w| w.contains("doppelt")), "{w:?}");
}

/// Testpaket mit einer `lib.typ`.
fn paket_mit_quelle(projekt: &Projekt, name: &str, version: &str, lib: &str) {
    paket_ablegen(projekt, name, version);
    let ordner = projekt.pakete().join("preview").join(name).join(version);
    std::fs::write(ordner.join("lib.typ"), lib).unwrap();
}

fn beschreibung(ordner: &Path, datei: &str, text: &str) {
    std::fs::write(ordner.join(datei), text).unwrap();
}

const DEMO: &str = r##"paket = "demo"
versionen = ">=0.1.0, <0.2.0"
import = '#import "@preview/demo:{version}"'
titel = "Demo-Helfer"
empfohlen = "0.1.3"

[[baustein]]
id = "kasten"
funktion = "demo.kasten"
titel = "Kasten"
einfuegen = "#demo.kasten[${auswahl}]"
"##;

#[test]
fn eigene_beschreibung_hat_vorrang() {
    let projekt = neues_projekt("katalog-beschr-vorrang");
    paket_ablegen(&projekt, "demo", "0.1.3");
    let eigen = tmp("katalog-beschr-vorrang-eigen");
    let mit = tmp("katalog-beschr-vorrang-mit");
    beschreibung(
        &eigen,
        "demo.toml",
        &DEMO.replace("titel = \"Kasten\"", "titel = \"Eigener Kasten\""),
    );
    beschreibung(&mit, "demo.toml", DEMO);
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[eigen, mit],
    });
    assert_eq!(
        k.finden("@preview/demo/kasten").unwrap().titel,
        "Eigener Kasten"
    );
    assert!(w.iter().any(|z| z.contains("überdeckt")), "{w:?}");
}

#[test]
fn holbar_und_helfer() {
    let ordner = tmp("katalog-holbar-b");
    beschreibung(&ordner, "demo.toml", DEMO);
    let bauen = |projekt: &Projekt| {
        Katalog::bauen(&Quellen {
            projekt: Some(projekt),
            beschreibungen: std::slice::from_ref(&ordner),
        })
        .0
    };
    let v = |t: &str| semver::Version::parse(t).unwrap();

    let projekt = neues_projekt("katalog-holbar-1");
    let k = bauen(&projekt);
    assert_eq!(k.holbar().len(), 1);
    assert_eq!(k.holbar()[0].paket, "demo");
    assert_eq!(k.holbar()[0].spec, "@preview/demo:0.1.3");
    assert_eq!(k.holbar()[0].titel, "Demo-Helfer");
    assert_eq!(k.holbar()[0].bausteine, vec!["Kasten".to_string()]);
    assert!(k.hat_helfer("demo", &v("0.1.9")));
    assert!(!k.hat_helfer("demo", &v("0.2.0")));
    assert!(!k.hat_helfer("anderes", &v("0.1.3")));

    let projekt = neues_projekt("katalog-holbar-2");
    paket_ablegen(&projekt, "demo", "0.1.3");
    let k = bauen(&projekt);
    assert!(k.holbar().is_empty());
    assert!(k.finden("@preview/demo/kasten").is_some());

    let projekt = neues_projekt("katalog-holbar-3");
    paket_ablegen(&projekt, "demo", "0.2.0");
    let k = bauen(&projekt);
    assert_eq!(k.holbar().len(), 1, "nur als Abhängigkeit da");
    assert!(k.finden("@preview/demo/kasten").is_none());

    let projekt = neues_projekt("katalog-holbar-4");
    paket_ablegen(&projekt, "demo", "0.2.0");
    std::fs::write(
        projekt.wurzel.join("kapitel.typ"),
        "#import \"@preview/demo:0.2.0\"\n",
    )
    .unwrap();
    let k = bauen(&projekt);
    assert!(
        k.holbar().is_empty(),
        "das Projekt nutzt bewusst eine andere Version"
    );
}

#[test]
fn auch_importiert_die_version_des_pakets() {
    let projekt = neues_projekt("katalog-auch");
    paket_ablegen(&projekt, "demo", "0.1.3");
    paket_ablegen(&projekt, "demo", "0.2.0");
    paket_mit_quelle(
        &projekt,
        "demo-plot",
        "0.1.0",
        "#import \"@preview/demo:0.1.3\"\n",
    );
    let ordner = tmp("katalog-auch-b");
    beschreibung(
        &ordner,
        "plot.toml",
        r##"paket = "demo-plot"
versionen = "0.1"
import = '#import "@preview/demo-plot:{version}"'
auch = ["demo"]

[[baustein]]
id = "p"
funktion = "demo-plot.p"
titel = "P"
einfuegen = "#demo-plot.p()"
"##,
    );
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[ordner],
    });
    assert!(w.is_empty(), "{w:?}");
    let e = k.finden("@preview/demo-plot/p").unwrap();
    let f = tftn_katalog::import_fuer(&typst_syntax::Source::detached("Text"), e).unwrap();
    assert_eq!(
        f.text,
        "#import \"@preview/demo-plot:0.1.0\"\n#import \"@preview/demo:0.1.3\"\n"
    );
}

#[test]
fn auch_paket_fehlt_ist_warnung() {
    let projekt = neues_projekt("katalog-auch-fehlt");
    paket_mit_quelle(&projekt, "demo-plot", "0.1.0", "");
    let ordner = tmp("katalog-auch-fehlt-b");
    beschreibung(
        &ordner,
        "plot.toml",
        r##"paket = "demo-plot"
versionen = "0.1"
import = '#import "@preview/demo-plot:{version}"'
auch = ["demo"]

[[baustein]]
id = "p"
funktion = "demo-plot.p"
titel = "P"
einfuegen = "#demo-plot.p()"
"##,
    );
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[ordner],
    });
    assert!(k.finden("@preview/demo-plot/p").is_none());
    assert!(w.iter().any(|z| z.contains("demo")), "{w:?}");
}

#[test]
fn symbole_uebersetzen() {
    let gruppen = tftn_katalog::symbole();
    let alle: Vec<&str> = gruppen
        .iter()
        .flat_map(|g| g.symbole.iter().map(|s| s.text.as_str()))
        .collect();
    assert!(alle.len() >= 70, "{}", alle.len());
    let mut sortiert = alle.clone();
    sortiert.sort();
    sortiert.dedup();
    assert_eq!(sortiert.len(), alle.len());

    let projekt = neues_projekt("katalog-symbole");
    let mut world = tftn_core::ProjektWorld::neu(&projekt).unwrap();
    let text = format!("$ {} $", alle.join(" quad "));
    world.quelle_setzen(typst_syntax::Source::new(world.haupt(), text));
    let e = tftn_core::kompilieren(&world);
    assert!(e.meldungen.is_empty(), "{:?}", e.meldungen);
    assert!(e.dokument.is_some());
}

/// Abschlussprüfung AP10: Eine eigene Datei ergänzt die mitgelieferte; je Id gewinnt die erste.
#[test]
fn eigene_beschreibung_ergaenzt() {
    let projekt = neues_projekt("katalog-ergaenzt");
    paket_ablegen(&projekt, "demo", "0.1.3");
    let eigen = tmp("katalog-ergaenzt-eigen");
    let mit = tmp("katalog-ergaenzt-mit");
    beschreibung(
        &eigen,
        "demo.toml",
        &DEMO.replace("id = \"kasten\"", "id = \"mein\"").replace(
            "'#import \"@preview/demo:{version}\"'",
            "'#import \"@preview/demo:{version}\" as demo'",
        ),
    );
    beschreibung(&mit, "demo.toml", DEMO);
    let (k, w) = Katalog::bauen(&Quellen {
        projekt: Some(&projekt),
        beschreibungen: &[eigen, mit],
    });
    assert!(w.is_empty(), "{w:?}");
    let import = |id: &str| match &k.finden(id).unwrap_or_else(|| panic!("{id}")).herkunft {
        Herkunft::Paket { import, .. } => import.clone(),
        _ => panic!("{id}"),
    };
    assert!(import("@preview/demo/mein").ends_with("as demo"));
    assert!(!import("@preview/demo/kasten").ends_with("as demo"));
}
