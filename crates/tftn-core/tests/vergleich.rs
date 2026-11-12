//! Seitenvergleich zweier Fassungen (PLAN-M2 AP8 Task 3).

use std::path::{Path, PathBuf};

use tftn_core::Projekt;
use tftn_core::vergleich::seiten_vergleichen;

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        if name.to_string_lossy().starts_with('_') {
            continue;
        }
        let ziel = nach.join(&name);
        if e.file_type().unwrap().is_dir() {
            kopieren(&e.path(), &ziel);
        } else {
            std::fs::copy(e.path(), ziel).unwrap();
        }
    }
}

/// Zwei Kopien des Testartikels.
fn paar(name: &str) -> (Projekt, Projekt, PathBuf) {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ap8-vergleich-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    let quelle = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    kopieren(&quelle, &o.join("alt"));
    kopieren(&quelle, &o.join("neu"));
    (
        Projekt::oeffnen(&o.join("alt")).unwrap(),
        Projekt::oeffnen(&o.join("neu")).unwrap(),
        o.join("neu"),
    )
}

fn aendern(datei: &Path, alt: &str, neu: &str) {
    let t = std::fs::read_to_string(datei).unwrap();
    assert!(t.contains(alt));
    std::fs::write(datei, t.replacen(alt, neu, 1)).unwrap();
}

#[test]
fn gleiche_projekte_ohne_aenderung() {
    let (a, b, _) = paar("gleich");
    let v = seiten_vergleichen(&a, &b, 72.0, 12).unwrap();
    assert!(v.fehler.is_empty(), "{:?}", v.fehler);
    assert_eq!((v.seiten_alt, v.seiten_neu), (3, 3));
    assert!(v.geaendert.is_empty(), "{:?}", v.geaendert);
    assert!(v.bilder.is_empty());
}

#[test]
fn titel_aendert_seite_1() {
    let (a, b, neu) = paar("titel");
    aendern(
        &neu.join("dokument.toml"),
        "Über die Reproduzierbarkeit",
        "Zur Reproduzierbarkeit",
    );
    let v = seiten_vergleichen(&a, &b, 72.0, 12).unwrap();
    assert_eq!(v.geaendert, [0]);
    assert_eq!(v.bilder.len(), 1);
    let (seite, alt, neu) = &v.bilder[0];
    assert_eq!(*seite, 0);
    assert!(alt.as_ref().unwrap().starts_with(b"\x89PNG"));
    assert!(neu.as_ref().unwrap().starts_with(b"\x89PNG"));
    assert_ne!(alt, neu);
}

#[test]
fn neue_seite() {
    let (a, b, neu) = paar("seite");
    let main = neu.join("main.typ");
    let t = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{t}\n#pagebreak()\nNeue letzte Seite.\n")).unwrap();
    let v = seiten_vergleichen(&a, &b, 72.0, 12).unwrap();
    assert_eq!(v.seiten_neu, v.seiten_alt + 1);
    let letzte = v.seiten_neu - 1;
    assert!(v.geaendert.contains(&letzte), "{:?}", v.geaendert);
    let (_, alt, neu) = v.bilder.iter().find(|(s, _, _)| *s == letzte).unwrap();
    assert!(alt.is_none());
    assert!(neu.is_some());
}

#[test]
fn bilder_hoechstens() {
    let (a, b, neu) = paar("max");
    // Schrift größer: alle Seiten ändern sich, Bilder nur für die erste.
    aendern(
        &neu.join("main.typ"),
        "= Einleitung",
        "#set text(size: 12pt)\n= Einleitung",
    );
    let v = seiten_vergleichen(&a, &b, 72.0, 1).unwrap();
    assert!(v.geaendert.len() > 1, "{:?}", v.geaendert);
    assert_eq!(v.bilder.len(), 1);
}

#[test]
fn vergleich_mit_fehlern() {
    let (a, b, neu) = paar("fehler");
    let main = neu.join("main.typ");
    let t = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{t}\n#undefiniert\n")).unwrap();
    let v = seiten_vergleichen(&a, &b, 72.0, 12).unwrap();
    assert!(!v.fehler.is_empty());
    assert!(v.geaendert.is_empty());
    // Scheitert die alte Fassung, ist das ein Fehler des Vergleichs.
    assert!(seiten_vergleichen(&b, &a, 72.0, 12).is_err());
}
