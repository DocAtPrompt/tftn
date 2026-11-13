//! `tftn typ-pruefen` (PLAN-M2 AP10, Task 3).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn fall(name: &str) -> (PathBuf, PathBuf) {
    let basis = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("typ-pruefen-{name}"));
    let _ = std::fs::remove_dir_all(&basis);
    let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types");
    projekt_anlegen(
        &typen,
        "tftn-fachartikel",
        &basis.join("p"),
        Some(ZEITSTEMPEL),
    )
    .unwrap();
    let paket = basis.join("p/packages/preview/demo/0.1.3");
    std::fs::create_dir_all(&paket).unwrap();
    std::fs::write(
        paket.join("typst.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.3\"\nentrypoint = \"lib.typ\"\n",
    )
    .unwrap();
    std::fs::write(paket.join("lib.typ"), "#let k(body) = body\n").unwrap();
    std::fs::create_dir_all(basis.join("b")).unwrap();
    (basis.join("p"), basis.join("b"))
}

fn lauf(projekt: &Path, b: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tftn"))
        .arg("typ-pruefen")
        .arg(projekt)
        .arg("--beschreibungen")
        .arg(b)
        .output()
        .unwrap()
}

fn beschreibung(funktion: &str) -> String {
    format!(
        "paket = \"demo\"\nversionen = \"0.1\"\nimport = '#import \"@preview/demo:{{version}}\"'\n\n[[baustein]]\nid = \"b\"\ntitel = \"B\"\nfunktion = \"{funktion}\"\neinfuegen = \"#{funktion}[x]\"\n"
    )
}

#[test]
fn typ_pruefen_exit() {
    let (p, b) = fall("gut");
    std::fs::write(b.join("d.toml"), beschreibung("demo.k")).unwrap();
    let a = lauf(&p, &b);
    let raus = String::from_utf8_lossy(&a.stdout);
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    assert!(raus.starts_with("in Ordnung: "), "{raus}");
    assert!(raus.contains("Bausteine"), "{raus}");

    let (p, b) = fall("befund");
    std::fs::write(b.join("d.toml"), beschreibung("demo.nichtda")).unwrap();
    let a = lauf(&p, &b);
    assert_eq!(a.status.code(), Some(1));
    let fehler = String::from_utf8_lossy(&a.stderr);
    assert!(fehler.contains("@preview/demo/b: "), "{fehler}");

    let (p, b) = fall("kaputt");
    std::fs::write(b.join("d.toml"), "paket = ").unwrap();
    let a = lauf(&p, &b);
    assert_eq!(a.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&a.stderr).contains("d.toml"));
}
