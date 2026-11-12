//! Das Beispielprojekt `demo/` muss sich ohne Fehler und Warnungen übersetzen lassen.

use std::path::Path;

#[test]
fn demo_uebersetzt_ohne_meldungen() {
    let demo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../demo");
    let aus = std::process::Command::new(env!("CARGO_BIN_EXE_tftn"))
        .arg("check")
        .arg(&demo)
        .output()
        .unwrap();
    let fehler = String::from_utf8_lossy(&aus.stderr);
    assert!(aus.status.success(), "{fehler}");
    assert!(fehler.trim().is_empty(), "Meldungen: {fehler}");
    assert!(String::from_utf8_lossy(&aus.stdout).starts_with("in Ordnung"));
}
