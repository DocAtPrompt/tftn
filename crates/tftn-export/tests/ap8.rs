//! Abnahme AP8 / A9 (PLAN-M1): Export-Profil `archiv` ergibt PDF/A-2b laut veraPDF; die
//! angezeigte Gegenprobe-Zeile, in der Shell ausgeführt, ergibt dieselbe Datei; „Abnehmen“ setzt
//! den Zeitstempel, legt nach `export/` ab und schreibt eine Notiz mit stimmender Prüfsumme.
//!
//! Braucht typst in der Projektversion (TFTN_TYPST oder PATH) und `verapdf` auf dem PATH.
//! `TFTN_OHNE_VERAPDF=1` überspringt nur die veraPDF-Prüfung (CI-Runner ohne veraPDF, PLAN-M1 4.6).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use tftn_core::Projekt;
use tftn_export::TypstCli;
use tftn_export::profil::profil_exportieren;

fn kopie(name: &str) -> PathBuf {
    let ziel = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&ziel);
    let korpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    kopieren(&korpus, &ziel);
    ziel
}

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap().to_string_lossy().starts_with('_') {
            continue;
        }
        let ziel = nach.join(p.file_name().unwrap());
        if p.is_dir() {
            kopieren(&p, &ziel)
        } else {
            std::fs::copy(&p, &ziel).unwrap();
        }
    }
}

/// veraPDF im maschinenlesbaren Format; `true`, wenn konform (oder ausdrücklich übersprungen).
fn verapdf(pdf: &Path) -> bool {
    if std::env::var_os("TFTN_OHNE_VERAPDF").is_some() {
        println!("veraPDF übersprungen (TFTN_OHNE_VERAPDF)");
        return true;
    }
    // Nacheinander: Beim ersten Start nach der Installation legt veraPDF seine Konfiguration an;
    // zwei gleichzeitige Starts brechen dann ab („validator.xml must be a creatable or readable
    // file“, CI-Lauf 42352881646, vermutlich auch 41725511197).
    static NACHEINANDER: Mutex<()> = Mutex::new(());
    let _sperre = NACHEINANDER.lock().unwrap_or_else(|e| e.into_inner());
    let aus = Command::new("verapdf")
        .args(["--flavour", "2b", "--format", "mrr"])
        .arg(pdf)
        .output()
        .expect("verapdf nicht gefunden – siehe README (brew install verapdf / verapdf.org)");
    let bericht = String::from_utf8_lossy(&aus.stdout);
    let konform = bericht.contains("isCompliant=\"true\"");
    if !konform {
        // Auch Exit-Code und stderr: In der CI kam einmal gar kein Bericht (Lauf 41725511197).
        println!("veraPDF: {}\n{bericht}", aus.status);
        println!("stderr:\n{}", String::from_utf8_lossy(&aus.stderr));
    }
    konform
}

/// Führt die Gegenprobe-Zeile in der Shell aus, mit der CLI aus `cli` statt „typst“ vom PATH.
fn gegenprobe_ausfuehren(ordner: &Path, zeile: &str, cli: &TypstCli) {
    // PowerShell braucht für ein Programm in Anführungszeichen den Aufrufoperator `&`.
    let aufruf = if cfg!(windows) {
        format!(" & '{}' ", cli.pfad.display())
    } else {
        format!(" '{}' ", cli.pfad.display())
    };
    let zeile = zeile.replacen(" typst ", &aufruf, 1);
    let status = if cfg!(windows) {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &zeile])
            .current_dir(ordner)
            .status()
    } else {
        Command::new("sh")
            .args(["-c", &zeile])
            .current_dir(ordner)
            .status()
    }
    .unwrap();
    assert!(status.success(), "Gegenprobe-Zeile fehlgeschlagen: {zeile}");
}

#[test]
fn archiv_ist_pdfa_2b_und_gleich_der_gegenprobe() {
    let ordner = kopie("ap8-probe");
    let projekt = Projekt::oeffnen(&ordner).unwrap();
    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");

    let e = profil_exportieren(&projekt, &cli, "archiv", false).unwrap();
    assert!(e.relativ.starts_with("_renders/"), "{}", e.relativ);
    println!("Gegenprobe: {}", e.gegenprobe);
    assert!(verapdf(&e.pfad), "veraPDF: nicht PDF/A-2b-konform");

    // Gegenprobe von Hand: andere Zieldatei, sonst dieselbe Zeile
    let hand = e.gegenprobe.replace(&e.relativ, "_renders/hand.pdf");
    gegenprobe_ausfuehren(&ordner, &hand, &cli);
    assert_eq!(
        std::fs::read(&e.pfad).unwrap(),
        std::fs::read(ordner.join("_renders/hand.pdf")).unwrap()
    );
}

#[test]
fn abnehmen_setzt_zeitstempel_und_schreibt_notiz() {
    let ordner = kopie("ap8-abnehmen");
    let projekt = Projekt::oeffnen(&ordner).unwrap();
    let cli = TypstCli::finden(&projekt.konfig.typst).unwrap();
    let vorher = projekt.epoche();

    let e = profil_exportieren(&projekt, &cli, "archiv", true).unwrap();
    let nachher = Projekt::oeffnen(&ordner).unwrap();
    assert!(nachher.epoche() > vorher, "Zeitstempel nicht aktualisiert");
    assert!(e.relativ.starts_with("export/"), "{}", e.relativ);
    let toml = std::fs::read_to_string(ordner.join("tftn.toml")).unwrap();
    assert!(
        toml.contains("typ = \"@local/tftn-fachartikel:0.1.0\""),
        "andere Zeilen müssen bleiben"
    );

    let notiz = std::fs::read_to_string(e.pfad.with_extension("gegenprobe.txt")).unwrap();
    let sha = e.sha256.clone().unwrap();
    assert!(notiz.contains(&sha) && notiz.contains(&e.gegenprobe));
    assert!(verapdf(&e.pfad));

    // Die Zeile aus der Notiz ergibt dieselbe Prüfsumme.
    let zeile = notiz.lines().find(|z| z.contains("typst compile")).unwrap();
    let hand = zeile.replace(&e.relativ, "_renders/aus-notiz.pdf");
    std::fs::create_dir_all(ordner.join("_renders")).unwrap();
    gegenprobe_ausfuehren(&ordner, &hand, &cli);
    assert_eq!(
        std::fs::read(&e.pfad).unwrap(),
        std::fs::read(ordner.join("_renders/aus-notiz.pdf")).unwrap()
    );
}
