//! Importe eines Pakets und Zwischenspeicher (PLAN-M2 AP9 Task 2).

mod hilfe;

use std::path::Path;

use hilfe::*;
use tftn_pakete::{Grenzen, PaketSpec, Zwischenspeicher, importe, sha256_hex};

fn s(text: &str) -> PaketSpec {
    PaketSpec::lesen(text).unwrap()
}

fn schreiben(datei: &Path, text: &str) {
    std::fs::create_dir_all(datei.parent().unwrap()).unwrap();
    std::fs::write(datei, text).unwrap();
}

/// Archiv eines Testpakets, das die angegebenen Pakete importiert.
fn paket(name: &str, importiert: &[&str]) -> Vec<u8> {
    let lib: String = importiert
        .iter()
        .map(|i| format!("#import \"{i}\": *\n"))
        .collect();
    archiv(&[
        typst_toml(name, "0.1.0"),
        datei("lib.typ", &format!("{lib}#let probe(x) = x\n")),
    ])
}

#[test]
fn importe_erkannt() {
    let o = testordner("importe");
    schreiben(
        &o.join("lib.typ"),
        "#import \"@preview/tftn-probe-basis:0.1.0\": *\n// \"@preview/y:1.0.0\"\n#let s = \"@preview/z:1.0.0\"\n#import \"@local/q:0.1.0\"\n",
    );
    schreiben(
        &o.join("src/a.typ"),
        "#{\n  include \"@preview/x:1.0.0\"\n}\n#import \"@preview/Falsch:1\"\n#import \"teil.typ\"\n",
    );
    let (gefunden, warnungen) = importe(&o).unwrap();
    assert_eq!(
        gefunden.into_iter().collect::<Vec<_>>(),
        vec![s("@preview/tftn-probe-basis:0.1.0"), s("@preview/x:1.0.0")]
    );
    assert!(
        warnungen.iter().any(|w| w.contains("@local/q:0.1.0")),
        "{warnungen:?}"
    );
    assert!(
        warnungen.iter().any(|w| w.contains("@preview/Falsch:1")),
        "{warnungen:?}"
    );
}

#[test]
fn ablegen_und_finden() {
    let zs = Zwischenspeicher {
        wurzel: testordner("ablegen"),
    };
    let spec = s("@preview/tftn-probe:0.1.0");
    assert_eq!(zs.ordner(&spec), None);
    let daten = paket("tftn-probe", &[]);
    let sha = zs.ablegen(&spec, &daten, &Grenzen::default()).unwrap();
    assert_eq!(sha, sha256_hex(&daten));
    let ordner = zs.ordner(&spec).unwrap();
    assert!(ordner.join("lib.typ").is_file());
    assert_eq!(zs.sha256(&spec).as_deref(), Some(sha.as_str()));
    assert_eq!(
        zs.index_pfad(),
        zs.wurzel.join("index.json"),
        "Index liegt in der Wurzel"
    );

    // Ohne .sha256 gilt der Eintrag als nicht vorhanden und wird ersetzt.
    std::fs::remove_file(zs.wurzel.join("preview/tftn-probe/0.1.0.sha256")).unwrap();
    std::fs::write(ordner.join("rest.txt"), "alt").unwrap();
    assert_eq!(zs.ordner(&spec), None);
    assert_eq!(zs.sha256(&spec), None);
    zs.ablegen(&spec, &daten, &Grenzen::default()).unwrap();
    assert!(!ordner.join("rest.txt").exists());
    assert_eq!(
        inhalt(&zs.wurzel.join("preview/tftn-probe")),
        vec!["0.1.0".to_string(), "0.1.0.sha256".to_string()]
    );
}

#[test]
fn ablegen_fehler_laesst_nichts() {
    let zs = Zwischenspeicher {
        wurzel: testordner("ablegen-fehler"),
    };
    let spec = s("@preview/tftn-probe:0.1.0");
    let boese = archiv(&[typst_toml("tftn-probe", "0.1.0"), datei("../x", "x")]);
    assert!(zs.ablegen(&spec, &boese, &Grenzen::default()).is_err());
    assert_eq!(
        inhalt(&zs.wurzel.join("preview/tftn-probe")),
        Vec::<String>::new()
    );
    assert_eq!(zs.ordner(&spec), None);
}

#[test]
fn abschluss_rekursiv() {
    let zs = Zwischenspeicher {
        wurzel: testordner("abschluss"),
    };
    let g = Grenzen::default();
    let probe = s("@preview/tftn-probe:0.1.0");
    let basis = s("@preview/tftn-probe-basis:0.1.0");
    zs.ablegen(
        &probe,
        &paket("tftn-probe", &["@preview/tftn-probe-basis:0.1.0"]),
        &g,
    )
    .unwrap();
    let (da, fehlt) = zs.abschluss(&probe).unwrap();
    assert_eq!(
        da.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>(),
        vec![probe.clone()]
    );
    assert_eq!(fehlt, vec![basis.clone()]);

    zs.ablegen(&basis, &paket("tftn-probe-basis", &[]), &g)
        .unwrap();
    let (da, fehlt) = zs.abschluss(&probe).unwrap();
    assert!(fehlt.is_empty());
    assert_eq!(
        da.iter().map(|(s, _)| s.clone()).collect::<Vec<_>>(),
        vec![probe.clone(), basis.clone()]
    );
    assert!(da.iter().all(|(_, sha)| sha.len() == 64));

    // Kreis a ↔ b endet
    let a = s("@preview/kreis-a:0.1.0");
    let b = s("@preview/kreis-b:0.1.0");
    zs.ablegen(&a, &paket("kreis-a", &["@preview/kreis-b:0.1.0"]), &g)
        .unwrap();
    zs.ablegen(&b, &paket("kreis-b", &["@preview/kreis-a:0.1.0"]), &g)
        .unwrap();
    let (da, fehlt) = zs.abschluss(&a).unwrap();
    assert_eq!(da.len(), 2);
    assert!(fehlt.is_empty());
}

#[test]
fn projektimporte_ohne_pakete() {
    let o = testordner("projektimporte");
    let schreiben = |datei: &str, text: &str| {
        let p = o.join(datei);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    schreiben(
        "main.typ",
        "#import \"@preview/a:1.0.0\"\n#include \"kapitel/b.typ\"\n",
    );
    schreiben("kapitel/b.typ", "#import \"@preview/b:1.0.0\": x\n");
    schreiben(
        "packages/preview/a/1.0.0/lib.typ",
        "#import \"@preview/c:1.0.0\"\n",
    );
    schreiben("_renders/z.typ", "#import \"@preview/d:1.0.0\"\n");
    schreiben(".git/z.typ", "#import \"@preview/e:1.0.0\"\n");
    let namen: Vec<String> = tftn_pakete::projektimporte(&o)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(namen, ["a", "b"]);
}
