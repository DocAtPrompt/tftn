//! `tftn neu --sprache`: die Vorlage in einer anderen Sprache.

use std::path::Path;

#[test]
fn neu_auf_englisch() {
    let ziel = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-neu-en");
    let _ = std::fs::remove_dir_all(&ziel);
    let aus = std::process::Command::new(env!("CARGO_BIN_EXE_tftn"))
        .args(["neu", "--typ", "tftn-fachartikel", "--sprache", "en"])
        .arg(&ziel)
        .output()
        .unwrap();
    assert!(
        aus.status.success(),
        "{}",
        String::from_utf8_lossy(&aus.stderr)
    );
    let daten = std::fs::read_to_string(ziel.join("dokument.toml")).unwrap();
    assert!(daten.contains("sprache = \"en\""), "{daten}");
    let check = std::process::Command::new(env!("CARGO_BIN_EXE_tftn"))
        .arg("check")
        .arg(&ziel)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let _ = std::fs::remove_dir_all(&ziel);
}
