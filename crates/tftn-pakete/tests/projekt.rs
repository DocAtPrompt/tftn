//! Übernahme ins Projekt mit Prüfsummen in `tftn.toml` (PLAN-M2 AP9 Task 3).

mod hilfe;

use std::path::{Path, PathBuf};

use hilfe::*;
use tftn_pakete::{
    Grenzen, PaketFehler, PaketSpec, Zwischenspeicher, benutzer, entfernen, im_projekt, readme,
    uebernehmen,
};

fn s(text: &str) -> PaketSpec {
    PaketSpec::lesen(text).unwrap()
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

/// Kopie des Testartikels und ein Zwischenspeicher mit tftn-probe → tftn-probe-basis.
fn aufbau(name: &str) -> (PathBuf, Zwischenspeicher, Vec<(PaketSpec, String)>) {
    let o = testordner(name);
    let projekt = o.join("projekt");
    kopieren(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel"),
        &projekt,
    );
    let _ = std::fs::remove_dir_all(projekt.join("packages/preview"));
    let zs = Zwischenspeicher {
        wurzel: o.join("zs"),
    };
    let g = Grenzen::default();
    let probe = s("@preview/tftn-probe:0.1.0");
    let basis = s("@preview/tftn-probe-basis:0.1.0");
    zs.ablegen(
        &probe,
        &archiv(&[
            typst_toml("tftn-probe", "0.1.0"),
            datei(
                "lib.typ",
                "#import \"@preview/tftn-probe-basis:0.1.0\": *\n#let probe(x) = x\n",
            ),
            datei("README.md", "# Probe\n"),
        ]),
        &g,
    )
    .unwrap();
    zs.ablegen(
        &basis,
        &archiv(&[
            typst_toml("tftn-probe-basis", "0.1.0"),
            datei("lib.typ", "#let basis = 1\n"),
        ]),
        &g,
    )
    .unwrap();
    let (pakete, fehlt) = zs.abschluss(&probe).unwrap();
    assert!(fehlt.is_empty());
    (projekt, zs, pakete)
}

#[test]
fn uebernehmen_kopiert_und_vermerkt() {
    let (projekt, zs, pakete) = aufbau("uebernehmen");
    let vorher = std::fs::read_to_string(projekt.join("tftn.toml")).unwrap();
    let u = uebernehmen(&projekt, &zs, &pakete).unwrap();
    assert_eq!(u.kopiert.len(), 2);
    assert!(u.schon_da.is_empty());
    assert!(
        projekt
            .join("packages/preview/tftn-probe/0.1.0/lib.typ")
            .is_file()
    );
    assert!(
        projekt
            .join("packages/preview/tftn-probe-basis/0.1.0/lib.typ")
            .is_file()
    );
    let nachher = std::fs::read_to_string(projekt.join("tftn.toml")).unwrap();
    assert!(nachher.starts_with(&vorher), "{nachher}");
    for (spec, sha) in &pakete {
        assert!(
            nachher.contains(&format!("\"{spec}\" = \"sha256:{sha}\"")),
            "{nachher}"
        );
    }
    let p = tftn_core::Projekt::oeffnen(&projekt).unwrap();
    assert_eq!(p.konfig.pakete.len(), 2);
    assert_eq!(
        p.konfig.pakete["@preview/tftn-probe:0.1.0"],
        format!("sha256:{}", pakete[0].1)
    );
    assert!(
        std::fs::read_dir(projekt.join("packages/preview/tftn-probe"))
            .unwrap()
            .all(|e| !e.unwrap().file_name().to_string_lossy().starts_with('.'))
    );
}

#[test]
fn uebernehmen_prueft_pruefsumme() {
    let (projekt, zs, pakete) = aufbau("pruefsumme");
    let mut text = std::fs::read_to_string(projekt.join("tftn.toml")).unwrap();
    text.push_str(&format!(
        "\n[pakete]\n\"@preview/tftn-probe-basis:0.1.0\" = \"sha256:{}\"\n",
        "0".repeat(64)
    ));
    std::fs::write(projekt.join("tftn.toml"), &text).unwrap();
    let fehler = uebernehmen(&projekt, &zs, &pakete).unwrap_err();
    assert!(
        matches!(fehler, PaketFehler::Pruefsumme { .. }),
        "{fehler:?}"
    );
    let meldung = fehler.to_string();
    assert!(meldung.contains(&"0".repeat(64)), "{meldung}");
    assert!(meldung.contains(&pakete[1].1), "{meldung}");
    assert!(!projekt.join("packages/preview").exists());
    assert_eq!(
        std::fs::read_to_string(projekt.join("tftn.toml")).unwrap(),
        text
    );
}

#[test]
fn vorhandenes_bleibt() {
    let (projekt, zs, pakete) = aufbau("vorhanden");
    let eigen = projekt.join("packages/preview/tftn-probe/0.1.0");
    std::fs::create_dir_all(&eigen).unwrap();
    std::fs::write(eigen.join("lib.typ"), "eigen").unwrap();
    let u = uebernehmen(&projekt, &zs, &pakete).unwrap();
    assert_eq!(u.schon_da, vec![s("@preview/tftn-probe:0.1.0")]);
    assert_eq!(u.kopiert, vec![s("@preview/tftn-probe-basis:0.1.0")]);
    assert_eq!(
        std::fs::read_to_string(eigen.join("lib.typ")).unwrap(),
        "eigen"
    );
}

#[test]
fn kommentare_und_crlf_bleiben() {
    let (projekt, zs, pakete) = aufbau("crlf");
    let text = "# Projekt\r\ntftn = 1\r\ntypst = \"0.15.1\" # Version\r\ntyp = \"@local/tftn-fachartikel:0.1.0\"\r\nhaupt = \"main.typ\"\r\nzeitstempel = 2026-10-31T00:00:00Z\r\n";
    std::fs::write(projekt.join("tftn.toml"), text).unwrap();
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let nachher = std::fs::read_to_string(projekt.join("tftn.toml")).unwrap();
    assert!(nachher.starts_with(text), "{nachher:?}");
    assert!(!nachher.replace("\r\n", "").contains('\n'), "{nachher:?}");
    assert!(nachher.contains("[pakete]"));
    // Nichts Neues zu vermerken: Datei bleibt byte-gleich.
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    assert_eq!(
        std::fs::read_to_string(projekt.join("tftn.toml")).unwrap(),
        nachher
    );
}

#[test]
fn im_projekt_liste() {
    let (projekt, zs, pakete) = aufbau("liste");
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let hand = projekt.join("packages/preview/von-hand/1.0.0");
    std::fs::create_dir_all(&hand).unwrap();
    std::fs::write(
        hand.join("typst.toml"),
        "[package]\nname = \"von-hand\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let liste = im_projekt(&projekt);
    let kurz: Vec<_> = liste
        .iter()
        .map(|p| (p.spec.as_str(), p.pruefsumme.is_some(), p.readme))
        .collect();
    assert_eq!(
        kurz,
        vec![
            ("@preview/tftn-probe:0.1.0", true, true),
            ("@preview/tftn-probe-basis:0.1.0", true, false),
            ("@preview/von-hand:1.0.0", false, false),
        ]
    );
    assert_eq!(liste[0].name, "tftn-probe");
    assert_eq!(liste[0].version, "0.1.0");
    assert_eq!(
        readme(&projekt, &zs, &s("@preview/tftn-probe:0.1.0")).as_deref(),
        Some("# Probe\n")
    );
    assert_eq!(readme(&projekt, &zs, &s("@preview/von-hand:1.0.0")), None);
}

/// Abschlussprüfung: Lässt sich `tftn.toml` nicht schreiben, bleiben keine Paketordner ohne
/// Prüfsumme im Projekt (sonst ließe sich die Prüfsumme über die Oberfläche nie nachtragen).
#[cfg(unix)]
#[test]
fn schreibfehler_baut_zurueck() {
    use std::os::unix::fs::PermissionsExt;
    let (projekt, zs, pakete) = aufbau("schreibfehler");
    let preview = projekt.join("packages/preview");
    std::fs::create_dir_all(&preview).unwrap();
    let vorher = std::fs::read(projekt.join("tftn.toml")).unwrap();
    std::fs::set_permissions(&projekt, std::fs::Permissions::from_mode(0o555)).unwrap();
    let ergebnis = uebernehmen(&projekt, &zs, &pakete);
    std::fs::set_permissions(&projekt, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(ergebnis.is_err());
    assert_eq!(inhalt(&preview), Vec::<String>::new());
    assert_eq!(std::fs::read(projekt.join("tftn.toml")).unwrap(), vorher);
    assert!(
        inhalt(&projekt).iter().all(|n| !n.ends_with(".tmp")),
        "{:?}",
        inhalt(&projekt)
    );
}

// --- Entfernen (PLAN-M2 AP10, Festlegung 11) ---------------------------------------------------

fn ordner(projekt: &Path, name: &str) -> PathBuf {
    projekt.join("packages/preview").join(name).join("0.1.0")
}

fn schreiben(projekt: &Path, datei: &str, text: &str) {
    let p = projekt.join(datei);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn stand(projekt: &Path) -> (String, Vec<String>) {
    let namen = im_projekt(projekt).into_iter().map(|p| p.spec).collect();
    (
        std::fs::read_to_string(projekt.join("tftn.toml")).unwrap(),
        namen,
    )
}

#[test]
fn entfernen_ohne_benutzer() {
    let (projekt, zs, pakete) = aufbau("entfernen-ok");
    let toml = projekt.join("tftn.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(
        &toml,
        format!("# Kommentar oben\n{text}").replace('\n', "\r\n"),
    )
    .unwrap();
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let weg = entfernen(&projekt, &s("@preview/tftn-probe:0.1.0")).unwrap();
    assert_eq!(
        weg,
        vec![
            s("@preview/tftn-probe:0.1.0"),
            s("@preview/tftn-probe-basis:0.1.0")
        ]
    );
    assert!(!ordner(&projekt, "tftn-probe").exists());
    assert!(!projekt.join("packages/preview/tftn-probe").exists());
    assert!(!ordner(&projekt, "tftn-probe-basis").exists());
    let text = std::fs::read_to_string(&toml).unwrap();
    assert!(!text.contains("tftn-probe"), "{text}");
    assert!(text.starts_with("# Kommentar oben\r\n"), "{text}");
    assert!(!text.replace("\r\n", "").contains('\n'), "{text:?}");
}

/// Review Focus 5: Solange jemand importiert, ändert sich nichts.
#[test]
fn entfernen_abgelehnt_wenn_benutzt() {
    let (projekt, zs, pakete) = aufbau("entfernen-benutzt");
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    schreiben(
        &projekt,
        "kapitel/a.typ",
        "#import \"@preview/tftn-probe:0.1.0\": *\n",
    );
    let vorher = stand(&projekt);
    let f = entfernen(&projekt, &s("@preview/tftn-probe:0.1.0")).unwrap_err();
    assert!(f.to_string().contains("kapitel/a.typ"), "{f}");
    assert_eq!(stand(&projekt), vorher);
    let f = entfernen(&projekt, &s("@preview/tftn-probe-basis:0.1.0")).unwrap_err();
    assert!(f.to_string().contains("@preview/tftn-probe:0.1.0"), "{f}");
    assert_eq!(stand(&projekt), vorher);
    assert_eq!(
        benutzer(&projekt, &s("@preview/tftn-probe:0.1.0")),
        vec!["kapitel/a.typ".to_string()]
    );
}

#[test]
fn entfernen_laesst_geteilte_abhaengigkeit() {
    let (projekt, zs, mut pakete) = aufbau("entfernen-geteilt");
    let zweit = s("@preview/tftn-zweit:0.1.0");
    zs.ablegen(
        &zweit,
        &archiv(&[
            typst_toml("tftn-zweit", "0.1.0"),
            datei(
                "lib.typ",
                "#import \"@preview/tftn-probe-basis:0.1.0\": *\n",
            ),
        ]),
        &Grenzen::default(),
    )
    .unwrap();
    pakete.push((zweit.clone(), zs.sha256(&zweit).unwrap()));
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let weg = entfernen(&projekt, &s("@preview/tftn-probe:0.1.0")).unwrap();
    assert_eq!(weg, vec![s("@preview/tftn-probe:0.1.0")]);
    assert!(ordner(&projekt, "tftn-probe-basis").exists());
    assert!(
        std::fs::read_to_string(projekt.join("tftn.toml"))
            .unwrap()
            .contains("tftn-probe-basis")
    );
}

#[test]
fn entfernen_ohne_eintrag() {
    let (projekt, zs, pakete) = aufbau("entfernen-ohne-eintrag");
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let toml = projekt.join("tftn.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    let ohne: String = text
        .lines()
        .filter(|z| !z.contains("@preview/tftn-probe:0.1.0"))
        .map(|z| format!("{z}\n"))
        .collect();
    std::fs::write(&toml, ohne).unwrap();
    entfernen(&projekt, &s("@preview/tftn-probe:0.1.0")).unwrap();
    assert!(!ordner(&projekt, "tftn-probe").exists());
}

#[test]
fn entfernen_fehlendes_paket() {
    let (projekt, _, _) = aufbau("entfernen-fehlt");
    assert!(entfernen(&projekt, &s("@preview/gibtsnicht:1.0.0")).is_err());
}

/// `pakete = {}` (Inline-Tabelle, von Hand geschrieben): Übernehmen und Entfernen gehen.
#[test]
fn pakete_als_inline_tabelle() {
    let (projekt, zs, pakete) = aufbau("inline-pakete");
    let toml = projekt.join("tftn.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    assert!(!text.contains("[pakete]"), "{text}");
    std::fs::write(&toml, format!("pakete = {{}}\n{text}")).unwrap();
    uebernehmen(&projekt, &zs, &pakete).unwrap();
    let p = tftn_core::Projekt::oeffnen(&projekt).unwrap();
    assert_eq!(p.konfig.pakete.len(), 2);
    entfernen(&projekt, &s("@preview/tftn-probe:0.1.0")).unwrap();
    let p = tftn_core::Projekt::oeffnen(&projekt).unwrap();
    assert!(p.konfig.pakete.is_empty());
}

/// Abschlussprüfung AP9: Zwei Fenster fügen zugleich Pakete hinzu; kein Eintrag in `[pakete]` geht
/// verloren (Lesen–Ändern–Schreiben von `tftn.toml` ist gesperrt).
#[test]
fn gleichzeitig_uebernehmen_verliert_nichts() {
    for runde in 0..30 {
        let (projekt, zs, pakete) = aufbau(&format!("gleichzeitig-{runde}"));
        let zs = std::sync::Arc::new(zs);
        let faeden: Vec<_> = pakete
            .into_iter()
            .map(|p| {
                let (projekt, zs) = (projekt.clone(), zs.clone());
                std::thread::spawn(move || uebernehmen(&projekt, &zs, &[p]).unwrap())
            })
            .collect();
        for f in faeden {
            f.join().unwrap();
        }
        let p = tftn_core::Projekt::oeffnen(&projekt).unwrap();
        assert_eq!(p.konfig.pakete.len(), 2, "Runde {runde}");
    }
}
