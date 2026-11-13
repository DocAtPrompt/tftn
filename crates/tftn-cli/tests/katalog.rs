//! `tftn katalog` (PLAN-M2 AP1, Task 6).

use std::path::{Path, PathBuf};

use tftn_manifest::anlegen::projekt_anlegen;

const ZEITSTEMPEL: i64 = 1_793_404_800; // 2026-10-31T00:00:00Z

fn tmp_projekt(name: &str) -> PathBuf {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ordner);
    let typen = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types");
    projekt_anlegen(&typen, "tftn-fachartikel", &ordner, Some(ZEITSTEMPEL)).unwrap();
    ordner
}

#[test]
fn katalog_listet_fachartikel() {
    let ordner = tmp_projekt("cli-katalog");
    let aus = std::process::Command::new(env!("CARGO_BIN_EXE_tftn"))
        .arg("katalog")
        .arg(&ordner)
        .output()
        .unwrap();
    assert!(
        aus.status.success(),
        "{}",
        String::from_utf8_lossy(&aus.stderr)
    );
    let text = String::from_utf8(aus.stdout).unwrap();
    assert!(
        text.lines().any(|z| z == "typst/tabelle\tTabelle\ttypst"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|z| z == "tftn-fachartikel/hinweis\tHinweiskasten\tdokumenttyp"),
        "{text}"
    );
}
