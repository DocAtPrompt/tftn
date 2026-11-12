//! Typquellen mit Vorrang und Anlegen aus mehreren Quellen (PLAN-M2 AP8 Task 1).

use std::path::{Path, PathBuf};

use tftn_manifest::anlegen::projekt_anlegen_aus;
use tftn_manifest::typen::{Herkunft, Typquelle, finden_in, verfuegbar};

fn typen() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typen")
}

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let e = e.unwrap();
        let ziel = nach.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            kopieren(&e.path(), &ziel);
        } else {
            std::fs::copy(e.path(), ziel).unwrap();
        }
    }
}

/// Eigener Testordner mit `buendel/` (Kopie von `typen/`) und leerem `eigen/`.
fn ordner(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ap8-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    let buendel = o.join("buendel");
    kopieren(&typen(), &buendel);
    let eigen = o.join("eigen");
    std::fs::create_dir_all(&eigen).unwrap();
    (o, buendel, eigen)
}

fn quellen(eigen: &Path, buendel: &Path) -> Vec<Typquelle> {
    vec![
        Typquelle {
            ordner: eigen.to_path_buf(),
            herkunft: Herkunft::Eigen,
        },
        Typquelle {
            ordner: buendel.to_path_buf(),
            herkunft: Herkunft::Tftn,
        },
    ]
}

/// Ersetzt in `datei` genau einmal `alt` durch `neu`.
fn ersetzen(datei: &Path, alt: &str, neu: &str) {
    let t = std::fs::read_to_string(datei).unwrap();
    assert!(t.contains(alt), "{alt} fehlt in {}", datei.display());
    std::fs::write(datei, t.replacen(alt, neu, 1)).unwrap();
}

/// Eigener Typ `mein-typ` als Kopie des Fachartikels.
fn mein_typ(eigen: &Path) -> PathBuf {
    let t = eigen.join("mein-typ");
    kopieren(&typen().join("tftn-fachartikel"), &t);
    ersetzen(
        &t.join("typst.toml"),
        r#"name = "tftn-fachartikel""#,
        r#"name = "mein-typ""#,
    );
    t
}

#[test]
fn liste_aus_buendel() {
    let (_, buendel, _) = ordner("liste");
    let (liste, warnungen) = verfuegbar(&[Typquelle {
        ordner: buendel,
        herkunft: Herkunft::Tftn,
    }]);
    assert!(warnungen.is_empty(), "{warnungen:?}");
    assert_eq!(liste.len(), 2, "{liste:?}");
    let t = &liste[0];
    assert_eq!(t.name, "tftn-fachartikel");
    assert_eq!(t.version.to_string(), "0.2.0");
    assert_eq!(t.titel, "Fachartikel");
    assert_eq!(t.beschreibung.as_deref(), Some("Fachartikel (TFTN)"));
    assert!(
        t.braucht.iter().any(|b| b.contains("Literatur")),
        "{:?}",
        t.braucht
    );
    assert!(
        liste[1].braucht.is_empty(),
        "das leere Dokument braucht nichts"
    );
    assert_eq!(t.herkunft, Herkunft::Tftn);
}

#[test]
fn eigener_typ_hat_vorrang() {
    let (_, buendel, eigen) = ordner("vorrang");
    let t = eigen.join("tftn-fachartikel");
    kopieren(&typen().join("tftn-fachartikel"), &t);
    ersetzen(
        &t.join("typst.toml"),
        r#"version = "0.2.0""#,
        r#"version = "0.1.5""#,
    );
    ersetzen(
        &t.join("typst.toml"),
        r#"titel = "Fachartikel""#,
        r#"titel = "Mein Artikel""#,
    );
    let (liste, warnungen) = verfuegbar(&quellen(&eigen, &buendel));
    assert_eq!(liste.len(), 2, "{liste:?}");
    assert_eq!(liste[0].titel, "Mein Artikel");
    assert_eq!(liste[0].version.to_string(), "0.1.5");
    assert_eq!(liste[0].herkunft, Herkunft::Eigen);
    assert_eq!(
        warnungen,
        ["tftn-fachartikel im eigenen Ordner überdeckt tftn-fachartikel 0.2.0 aus TFTN"]
    );
}

#[test]
fn kaputter_eigener_typ_ist_warnung() {
    let (_, buendel, eigen) = ordner("kaputt");
    std::fs::create_dir_all(eigen.join("x")).unwrap();
    std::fs::write(eigen.join("x/typst.toml"), "kaputt").unwrap();
    let (liste, warnungen) = verfuegbar(&quellen(&eigen, &buendel));
    assert_eq!(liste.len(), 2);
    assert_eq!(liste[0].name, "tftn-fachartikel");
    assert_eq!(warnungen.len(), 1, "{warnungen:?}");
    assert!(warnungen[0].contains('x'), "{warnungen:?}");
}

#[test]
fn finden_in_mit_version() {
    let (_, buendel, eigen) = ordner("finden");
    let k = eigen.join("tftn-kern");
    kopieren(&typen().join("tftn-kern"), &k);
    ersetzen(
        &k.join("typst.toml"),
        r#"version = "0.1.0""#,
        r#"version = "0.2.0""#,
    );
    let q = quellen(&eigen, &buendel);
    let p = finden_in(&q, "tftn-kern", Some("0.1.0")).unwrap();
    assert!(p.ordner.starts_with(&buendel), "{}", p.ordner.display());
    let p = finden_in(&q, "tftn-kern", None).unwrap();
    assert!(p.ordner.starts_with(&eigen));
    assert!(finden_in(&q, "tftn-kern", Some("9.9.9")).is_err());
}

#[test]
fn anlegen_aus_eigenem_typ() {
    let (o, buendel, eigen) = ordner("anlegen");
    mein_typ(&eigen);
    let ziel = o.join("projekt");
    let a = projekt_anlegen_aus(&quellen(&eigen, &buendel), "mein-typ", &ziel, Some(0)).unwrap();
    assert_eq!(
        a.projekt.konfig.typ.as_deref(),
        Some("@local/mein-typ:0.2.0")
    );
    assert!(
        ziel.join("packages/local/tftn-kern/0.1.0/lib.typ")
            .is_file()
    );
    assert!(ziel.join("packages/local/mein-typ/0.2.0/lib.typ").is_file());
}

#[test]
fn fehler_raeumt_auf() {
    let (o, buendel, eigen) = ordner("aufraeumen");
    let t = mein_typ(&eigen);
    ersetzen(
        &t.join("typst.toml"),
        r#"  "schriften/LIZENZEN.txt","#,
        "  \"schriften/LIZENZEN.txt\",\n  \"schriften/Fehlt.otf\",",
    );
    let ziel = o.join("neu");
    assert!(projekt_anlegen_aus(&quellen(&eigen, &buendel), "mein-typ", &ziel, Some(0)).is_err());
    assert!(!ziel.exists());
    // Ein leerer Ordner, den es schon gab, bleibt (leer) stehen.
    std::fs::create_dir_all(&ziel).unwrap();
    assert!(projekt_anlegen_aus(&quellen(&eigen, &buendel), "mein-typ", &ziel, Some(0)).is_err());
    assert!(ziel.is_dir());
    assert_eq!(std::fs::read_dir(&ziel).unwrap().count(), 0);
}

#[test]
fn abhaengigkeit_fehlt_raeumt_auf() {
    let (o, buendel, eigen) = ordner("abhaengigkeit");
    let t = mein_typ(&eigen);
    let lib = std::fs::read_to_string(t.join("lib.typ")).unwrap();
    std::fs::write(
        t.join("lib.typ"),
        format!("#import \"@local/gibtsnicht:1.0.0\": *\n{lib}"),
    )
    .unwrap();
    let ziel = o.join("neu");
    let e = projekt_anlegen_aus(&quellen(&eigen, &buendel), "mein-typ", &ziel, Some(0))
        .unwrap_err()
        .to_string();
    assert!(e.contains("gibtsnicht"), "{e}");
    assert!(!ziel.exists());
}

#[test]
fn nicht_leer_wird_abgelehnt() {
    let (o, buendel, eigen) = ordner("nichtleer");
    let ziel = o.join("voll");
    std::fs::create_dir_all(&ziel).unwrap();
    std::fs::write(ziel.join("wichtig.txt"), "bleibt").unwrap();
    assert!(
        projekt_anlegen_aus(
            &quellen(&eigen, &buendel),
            "tftn-fachartikel",
            &ziel,
            Some(0)
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(ziel.join("wichtig.txt")).unwrap(),
        "bleibt"
    );
}

/// Abschlussprüfung: Ordnername ≠ Paketname – Liste und Anlage meinen denselben Typ.
#[test]
fn ordnername_weicht_ab() {
    let (o, buendel, eigen) = ordner("ordnername");
    let t = eigen.join("mein-fa");
    kopieren(&typen().join("tftn-fachartikel"), &t);
    ersetzen(
        &t.join("typst.toml"),
        r#"version = "0.2.0""#,
        r#"version = "0.3.0""#,
    );
    let q = quellen(&eigen, &buendel);
    let (liste, _) = verfuegbar(&q);
    assert_eq!(liste[0].version.to_string(), "0.3.0");
    let p = finden_in(&q, "tftn-fachartikel", None).unwrap();
    assert_eq!(p.toml.package.version, "0.3.0");
    assert!(p.ordner.starts_with(&eigen));
    let a = projekt_anlegen_aus(&q, "tftn-fachartikel", &o.join("p"), Some(0)).unwrap();
    assert_eq!(
        a.projekt.konfig.typ.as_deref(),
        Some("@local/tftn-fachartikel:0.3.0")
    );
}
