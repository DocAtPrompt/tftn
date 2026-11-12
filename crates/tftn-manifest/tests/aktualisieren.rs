//! Dokumenttyp aktualisieren, Probekopie (PLAN-M2 AP8 Task 2).

use std::path::{Path, PathBuf};

use tftn_core::{Projekt, ProjektWorld, kompilieren, seitenzeilen};
use tftn_manifest::aktualisieren::{
    neuere_version, probekopie, stellen_umschreiben, typ_aktualisieren,
};
use tftn_manifest::anlegen::projekt_anlegen;
use tftn_manifest::typen::{Herkunft, Typquelle};

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn buendel() -> Vec<Typquelle> {
    vec![Typquelle {
        ordner: wurzel().join("typen"),
        herkunft: Herkunft::Tftn,
    }]
}

fn ordner(name: &str) -> PathBuf {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ap8-akt-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    o
}

/// Kopie des Testartikels (Fachartikel 0.1.0) unter `<ordner>/projekt`.
fn testartikel(name: &str) -> Projekt {
    let o = ordner(name);
    let p = Projekt::oeffnen(&wurzel().join("tests/korpus/testartikel")).unwrap();
    probekopie(&p, &o.join("projekt")).unwrap()
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

fn lesen(p: &Projekt, datei: &str) -> String {
    std::fs::read_to_string(p.wurzel.join(datei)).unwrap()
}

#[test]
fn neuere_version_erkannt() {
    let alt = testartikel("neuer");
    let n = neuere_version(&alt, &buendel()).unwrap();
    assert_eq!(n.version.to_string(), "0.2.0");
    let o = ordner("neuer-neu");
    let neu = projekt_anlegen(
        &wurzel().join("typen"),
        "tftn-fachartikel",
        &o.join("p"),
        Some(0),
    )
    .unwrap()
    .projekt;
    assert!(neuere_version(&neu, &buendel()).is_none());
    // Eigener Fachartikel 0.1.5 hat Vorrang: für 0.1.0 ist er neuer, für 0.2.0 nicht.
    let eigen = o.join("eigen");
    kopieren(
        &wurzel().join("typen/tftn-fachartikel"),
        &eigen.join("tftn-fachartikel"),
    );
    let toml = eigen.join("tftn-fachartikel/typst.toml");
    let t = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(
        &toml,
        t.replace(r#"version = "0.2.0""#, r#"version = "0.1.5""#),
    )
    .unwrap();
    let mit_eigen = [
        Typquelle {
            ordner: eigen,
            herkunft: Herkunft::Eigen,
        },
        buendel().remove(0),
    ];
    let n = neuere_version(&alt, &mit_eigen).unwrap();
    assert_eq!(n.version.to_string(), "0.1.5");
    assert_eq!(n.herkunft, Herkunft::Eigen);
    assert!(neuere_version(&neu, &mit_eigen).is_none());
}

#[test]
fn aktualisieren_vendort_und_stellt_um() {
    let p = testartikel("vendoren");
    let tftn_vorher = lesen(&p, "tftn.toml");
    let main_vorher = lesen(&p, "main.typ");
    let neu = neuere_version(&p, &buendel()).unwrap();
    let a = typ_aktualisieren(&p, &neu, &buendel()).unwrap();
    assert_eq!(a.alt, "@local/tftn-fachartikel:0.1.0");
    assert_eq!(a.neu, "@local/tftn-fachartikel:0.2.0");
    assert!(
        p.wurzel
            .join("packages/local/tftn-fachartikel/0.2.0/lib.typ")
            .is_file()
    );
    assert!(
        p.wurzel
            .join("packages/local/tftn-fachartikel/0.1.0/lib.typ")
            .is_file()
    );
    assert_eq!(
        lesen(&p, "tftn.toml"),
        tftn_vorher.replace("tftn-fachartikel:0.1.0", "tftn-fachartikel:0.2.0")
    );
    assert_eq!(a.stellen.len(), 1, "{:?}", a.stellen);
    let s = &a.stellen[0];
    assert_eq!(s.datei, "main.typ");
    assert_eq!(&main_vorher[s.von..s.bis], "@local/tftn-fachartikel:0.1.0");
    assert_eq!(lesen(&p, "main.typ"), main_vorher);
}

#[test]
fn probe_uebersetzt() {
    let p = testartikel("probe");
    let neu = neuere_version(&p, &buendel()).unwrap();
    let a = typ_aktualisieren(&p, &neu, &buendel()).unwrap();
    stellen_umschreiben(&p.wurzel, &a, |_| true).unwrap();
    assert!(lesen(&p, "main.typ").contains("@local/tftn-fachartikel:0.2.0"));
    let p = Projekt::oeffnen(&p.wurzel).unwrap();
    let e = kompilieren(&ProjektWorld::neu(&p).unwrap());
    let fehler: Vec<_> = e.meldungen.iter().filter(|m| m.fehler).collect();
    assert!(fehler.is_empty(), "{fehler:?}");
    let text = seitenzeilen(&e.dokument.unwrap()).concat().join("\n");
    assert!(text.contains("Literatur"), "Verzeichnis aus main.typ fehlt");
}

#[test]
fn importstellen_varianten() {
    let p = testartikel("varianten");
    let main = lesen(&p, "main.typ").replace(
        r#"#import "@local/tftn-fachartikel:0.1.0": *"#,
        "#import \"@local/tftn-fachartikel:0.1.0\": fachartikel, hinweis\n// \"@local/tftn-fachartikel:0.1.0\"",
    );
    std::fs::write(p.wurzel.join("main.typ"), &main).unwrap();
    std::fs::create_dir_all(p.wurzel.join("kapitel")).unwrap();
    std::fs::write(
        p.wurzel.join("kapitel/a.typ"),
        "#import \"@local/tftn-fachartikel:0.1.0\": hinweis\n#hinweis[a]\n",
    )
    .unwrap();
    let neu = neuere_version(&p, &buendel()).unwrap();
    let a = typ_aktualisieren(&p, &neu, &buendel()).unwrap();
    let mut dateien: Vec<&str> = a.stellen.iter().map(|s| s.datei.as_str()).collect();
    dateien.sort();
    assert_eq!(dateien, ["kapitel/a.typ", "main.typ"]);
    stellen_umschreiben(&p.wurzel, &a, |d| d != "main.typ").unwrap();
    assert!(lesen(&p, "kapitel/a.typ").contains("tftn-fachartikel:0.2.0"));
    assert_eq!(lesen(&p, "main.typ"), main, "main.typ ausgenommen");

    // Ohne Import: keine Stelle, kein Fehler.
    let q = testartikel("ohne-import");
    let ohne = lesen(&q, "main.typ").replace(r#"#import "@local/tftn-fachartikel:0.1.0": *"#, "");
    std::fs::write(q.wurzel.join("main.typ"), ohne).unwrap();
    let a = typ_aktualisieren(&q, &neuere_version(&q, &buendel()).unwrap(), &buendel()).unwrap();
    assert!(a.stellen.is_empty());
}

#[test]
fn probekopie_ohne_erzeugtes() {
    let o = ordner("kopie");
    let quelle = o.join("quelle");
    let p = Projekt::oeffnen(&wurzel().join("tests/korpus/testartikel")).unwrap();
    let p = probekopie(&p, &quelle).unwrap();
    for d in ["_renders/x.pdf", ".git/HEAD", "export/a.pdf"] {
        let pfad = p.wurzel.join(d);
        std::fs::create_dir_all(pfad.parent().unwrap()).unwrap();
        std::fs::write(pfad, "x").unwrap();
    }
    let k = probekopie(&p, &o.join("ziel")).unwrap();
    for d in ["_renders", ".git", "export"] {
        assert!(!k.wurzel.join(d).exists(), "{d} kopiert");
    }
    for d in [
        "fonts",
        "packages/local/tftn-fachartikel/0.1.0",
        "dokument.toml",
        "main.typ",
    ] {
        assert!(k.wurzel.join(d).exists(), "{d} fehlt");
    }
}

#[test]
fn neue_schriften_kopiert() {
    let o = ordner("schriften");
    let p = testartikel("schriften-projekt");
    let eigen = o.join("eigen");
    let t = eigen.join("tftn-fachartikel");
    kopieren(&wurzel().join("typen/tftn-fachartikel"), &t);
    std::fs::write(t.join("schriften/Neu.otf"), "neue schrift").unwrap();
    std::fs::write(t.join("schriften/LIZENZEN.txt"), "geändert").unwrap();
    let toml = t.join("typst.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(
        &toml,
        text.replace(
            r#"  "schriften/LIZENZEN.txt","#,
            "  \"schriften/LIZENZEN.txt\",\n  \"schriften/Neu.otf\",",
        ),
    )
    .unwrap();
    let lizenz_vorher = lesen(&p, "fonts/LIZENZEN.txt");
    let quellen = [
        Typquelle {
            ordner: eigen,
            herkunft: Herkunft::Eigen,
        },
        buendel().remove(0),
    ];
    let neu = neuere_version(&p, &quellen).unwrap();
    let a = typ_aktualisieren(&p, &neu, &quellen).unwrap();
    assert_eq!(a.schriften, ["Neu.otf"]);
    assert_eq!(lesen(&p, "fonts/Neu.otf"), "neue schrift");
    assert_eq!(lesen(&p, "fonts/LIZENZEN.txt"), lizenz_vorher);
}

/// Abschlussprüfung AP10: Scheitert eine Datei, bleibt auch die andere beim alten Stand.
#[cfg(unix)]
#[test]
fn umschreiben_alles_oder_nichts() {
    use std::os::unix::fs::PermissionsExt;
    use tftn_manifest::aktualisieren::{Aktualisierung, importstellen, stellen_umschreiben};
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join("umschreiben-alles");
    let _ = std::fs::set_permissions(o.join("kapitel"), std::fs::Permissions::from_mode(0o755));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(o.join("kapitel")).unwrap();
    let alt = "@preview/vg:0.1.0";
    let text = format!("#import \"{alt}\"\n");
    std::fs::write(o.join("a.typ"), &text).unwrap();
    std::fs::write(o.join("kapitel/b.typ"), &text).unwrap();
    let a = Aktualisierung {
        alt: alt.into(),
        neu: "@preview/vg:0.2.0".into(),
        stellen: importstellen(&o, alt).unwrap(),
        schriften: Vec::new(),
    };
    assert_eq!(a.stellen.len(), 2);
    let b = o.join("kapitel/b.typ");
    std::fs::set_permissions(&b, std::fs::Permissions::from_mode(0o444)).unwrap();
    std::fs::set_permissions(o.join("kapitel"), std::fs::Permissions::from_mode(0o555)).unwrap();
    let e = stellen_umschreiben(&o, &a, |_| true);
    std::fs::set_permissions(o.join("kapitel"), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(e.is_err());
    assert_eq!(std::fs::read_to_string(o.join("a.typ")).unwrap(), text);
    assert_eq!(std::fs::read_to_string(&b).unwrap(), text);
    let reste: Vec<_> = std::fs::read_dir(&o)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("tmp"))
        .collect();
    assert!(reste.is_empty(), "{reste:?}");
}

/// Abschlussprüfung AP10: Der Normalfall schreibt alle Dateien und lässt keine Temp-Dateien liegen.
#[test]
fn umschreiben_alle_dateien() {
    use tftn_manifest::aktualisieren::{Aktualisierung, importstellen, stellen_umschreiben};
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join("umschreiben-gut");
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(o.join("kapitel")).unwrap();
    let alt = "@preview/vg:0.1.0";
    std::fs::write(o.join("a.typ"), format!("#import \"{alt}\"\n")).unwrap();
    std::fs::write(o.join("kapitel/b.typ"), format!("#import \"{alt}\"\n")).unwrap();
    let a = Aktualisierung {
        alt: alt.into(),
        neu: "@preview/vg:0.2.0".into(),
        stellen: importstellen(&o, alt).unwrap(),
        schriften: Vec::new(),
    };
    stellen_umschreiben(&o, &a, |_| true).unwrap();
    for d in ["a.typ", "kapitel/b.typ"] {
        assert!(
            std::fs::read_to_string(o.join(d))
                .unwrap()
                .contains("0.2.0"),
            "{d}"
        );
    }
    assert_eq!(std::fs::read_dir(o.join("kapitel")).unwrap().count(), 1);
}
