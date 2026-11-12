//! `cargo xtask ci`: die Jobs aus `.github/workflows/ci.yml` lokal, für Arbeit ohne Netz
//! (PLAN-M1 4.6). Statt `npm ci` muss `node_modules/` schon da sein.
//!
//! `cargo xtask pruefsummen [DATEI]`: Korpus mit allen Export-Profilen und `tftn build`
//! erzeugen, SHA-256 je PDF ausgeben. Die CI nutzt das für den Plattformvergleich; `ci` legt
//! das Ergebnis unter `tests/pruefsummen/<ziel>.txt` ab und vergleicht mit den anderen Rechnern.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::{
    cargo, holer_bauen, host_ziel, mit_frist, projektordner, referenzen, sha256, tftn_bauen,
};

/// Linux-Runner haben kein WebKitGTK; dort ohne App (wie in der CI).
fn ohne_app() -> &'static [&'static str] {
    if cfg!(target_os = "linux") {
        &["--exclude", "tftn-app"]
    } else {
        &[]
    }
}

fn npm() -> Command {
    // npm ist unter Windows eine .cmd-Datei, die Command nicht direkt startet.
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "npm"]);
        c
    } else {
        Command::new("npm")
    }
}

pub fn ci() -> Result<(), String> {
    let wurzel = projektordner();
    if !wurzel.join("node_modules").is_dir() {
        return Err("node_modules/ fehlt: einmal mit Netz `npm ci` ausführen".into());
    }
    let mut ergebnisse: Vec<(&str, Result<(), String>)> = Vec::new();
    let mut schritt = |name: &'static str, lauf: &mut dyn FnMut() -> Result<(), String>| {
        println!("\n== {name}");
        let e = lauf();
        if let Err(text) = &e {
            eprintln!("{text}");
        }
        ergebnisse.push((name, e));
    };

    schritt("Oberfläche bauen und testen", &mut || {
        mit_frist(npm().args(["run", "build"]).current_dir(&wurzel), 300)?;
        mit_frist(npm().arg("test").current_dir(&wurzel), 300)
    });
    schritt("cargo fmt --check", &mut || {
        mit_frist(cargo().args(["fmt", "--all", "--check"]), 120)
    });
    schritt("clippy -D warnings", &mut || {
        mit_frist(
            cargo()
                .args(["clippy", "--workspace", "--all-targets"])
                .args(ohne_app())
                .args(["--", "-D", "warnings"]),
            1800,
        )
    });
    schritt("Rust-Tests", &mut || {
        mit_frist(cargo().args(["test", "--workspace"]).args(ohne_app()), 3600)
    });
    schritt("Netzfreiheit", &mut netzfreiheit);
    schritt("tftn-holer ohne Prüfserver", &mut || {
        holer_bauen(false).map(|_| ())
    });
    schritt("Prüfsummen und Plattformvergleich", &mut || {
        let datei = wurzel
            .join("tests/pruefsummen")
            .join(format!("{}.txt", host_ziel()));
        pruefsummen(Some(&datei))?;
        plattformvergleich(&wurzel.join("tests/pruefsummen"))
    });
    if host_ziel() == "aarch64-apple-darwin" {
        schritt("Visuelle Regression (Referenzplattform)", &mut || {
            referenzen::referenzen(&[])
        });
    }

    println!("\n== Zusammenfassung");
    let mut fehler = 0;
    for (name, e) in &ergebnisse {
        println!("{} {name}", if e.is_ok() { "ok    " } else { "FEHLER" });
        fehler += e.is_err() as usize;
    }
    if fehler == 0 {
        Ok(())
    } else {
        Err(format!("{fehler} Schritt(e) fehlgeschlagen"))
    }
}

/// Kein HTTP-Client in App, Kommandozeile und `tftn-pakete` (KONZEPT 4.8); `tftn-holer` nur mit
/// ureq und rustls (Entwurf 8.2, PLAN-M2 AP9).
pub fn netzfreiheit() -> Result<(), String> {
    let pakete: &[&str] = if cfg!(target_os = "linux") {
        &["tftn-cli", "tftn-pakete", "tftn-holer"]
    } else {
        &["tftn-app", "tftn-cli", "tftn-pakete", "tftn-holer"]
    };
    for p in pakete {
        let aus = cargo()
            .args(["tree", "-e", "normal", "-p", p, "--prefix", "none"])
            .output()
            .map_err(|e| e.to_string())?;
        if !aus.status.success() {
            return Err(format!(
                "cargo tree -p {p}: {}",
                String::from_utf8_lossy(&aus.stderr)
            ));
        }
        befund(p, &String::from_utf8_lossy(&aus.stdout))?;
        println!("{p}: ok");
    }
    Ok(())
}

/// Wertet den Abhängigkeitsbaum eines Pakets aus (eine Zeile je Crate, Name zuerst).
fn befund(paket: &str, baum: &str) -> Result<(), String> {
    const HTTP: [&str; 8] = [
        "ureq",
        "reqwest",
        "hyper",
        "isahc",
        "attohttpc",
        "native-tls",
        "rustls",
        "openssl",
    ];
    let namen: BTreeSet<&str> = baum
        .lines()
        .filter_map(|z| z.split_whitespace().next())
        .collect();
    if paket == "tftn-holer" {
        for noetig in ["ureq", "rustls"] {
            if !namen.contains(noetig) {
                return Err(format!("tftn-holer ohne {noetig}"));
            }
        }
        let fremd: Vec<&str> = HTTP
            .iter()
            .copied()
            .filter(|n| !["ureq", "rustls"].contains(n) && namen.contains(n))
            .collect();
        return if fremd.is_empty() {
            Ok(())
        } else {
            Err(format!("tftn-holer mit {fremd:?}"))
        };
    }
    let gefunden: Vec<&str> = HTTP
        .iter()
        .copied()
        .chain(["tftn-holer"])
        .filter(|n| namen.contains(n))
        .collect();
    if gefunden.is_empty() {
        Ok(())
    } else {
        Err(format!("HTTP-Client in {paket}: {gefunden:?}"))
    }
}

/// Erzeugt alle Korpus-PDFs und gibt ihre Prüfsummen aus (Format wie `sha256sum`, Pfade relativ
/// zum Projektordner mit `/`, nach Pfad sortiert); mit `datei` auch dorthin geschrieben.
pub fn pruefsummen(datei: Option<&Path>) -> Result<(), String> {
    let wurzel = projektordner();
    let tftn = tftn_bauen()?;
    let mut pdfs = Vec::new();
    for dokument in referenzen::korpus(&wurzel)? {
        let liste = ausgabe(Command::new(&tftn).arg("export").arg(&dokument))?;
        for profil in liste.lines().filter_map(|z| z.split_whitespace().next()) {
            let aus = ausgabe(
                Command::new(&tftn)
                    .arg("export")
                    .arg(&dokument)
                    .args(["--profil", profil]),
            )?;
            pdfs.push(erste_zeile(&aus)?);
        }
        pdfs.push(erste_zeile(&ausgabe(
            Command::new(&tftn).arg("build").arg(&dokument),
        )?)?);
    }
    let mut zeilen = Vec::new();
    for pdf in pdfs {
        // Kanonisch auf beiden Seiten (Windows: `\\?\`-Präfix).
        let pfad = std::fs::canonicalize(&pdf).map_err(|e| format!("{pdf}: {e}"))?;
        let basis = std::fs::canonicalize(&wurzel).map_err(|e| e.to_string())?;
        let relativ = pfad
            .strip_prefix(&basis)
            .map_err(|_| format!("{pdf} liegt nicht im Projektordner"))?;
        let relativ = relativ.to_string_lossy().replace('\\', "/");
        zeilen.push((relativ, sha256(&pfad)?));
    }
    zeilen.sort();
    let text: String = zeilen.iter().map(|(p, h)| format!("{h}  {p}\n")).collect();
    print!("{text}");
    if let Some(datei) = datei {
        if let Some(eltern) = datei.parent() {
            std::fs::create_dir_all(eltern).map_err(|e| e.to_string())?;
        }
        std::fs::write(datei, &text).map_err(|e| e.to_string())?;
        println!("→ {}", datei.display());
    }
    Ok(())
}

/// Alle Dateien in `tests/pruefsummen/` müssen gleich sein (A8 ohne CI).
fn plattformvergleich(ordner: &Path) -> Result<(), String> {
    let mut dateien: Vec<_> = std::fs::read_dir(ordner)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    dateien.sort();
    let erste = std::fs::read_to_string(&dateien[0]).map_err(|e| e.to_string())?;
    let mut gleich = true;
    for d in &dateien[1..] {
        let text = std::fs::read_to_string(d).map_err(|e| e.to_string())?;
        if text.replace("\r\n", "\n") != erste.replace("\r\n", "\n") {
            println!(
                "{} weicht von {} ab",
                d.file_name().unwrap().to_string_lossy(),
                dateien[0].file_name().unwrap().to_string_lossy()
            );
            gleich = false;
        }
    }
    if dateien.len() == 1 {
        println!("Nur ein Rechner in tests/pruefsummen/ – noch nichts zu vergleichen.");
    }
    if gleich {
        Ok(())
    } else {
        Err("Rechner erzeugen verschiedene PDFs".into())
    }
}

fn ausgabe(befehl: &mut Command) -> Result<String, String> {
    let aus = befehl.output().map_err(|e| format!("{befehl:?}: {e}"))?;
    if !aus.status.success() {
        return Err(format!(
            "{befehl:?} endete mit {}:\n{}",
            aus.status,
            String::from_utf8_lossy(&aus.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&aus.stdout).into_owned())
}

fn erste_zeile(text: &str) -> Result<String, String> {
    text.lines()
        .next()
        .map(str::to_string)
        .ok_or_else(|| "keine Ausgabe".into())
}

#[cfg(test)]
mod tests {
    use super::befund;

    const MIT_UREQ: &str = "tftn-holer v0.1.0\nureq v3.5.1\nrustls v0.23.0\nring v0.17.0\n";

    #[test]
    fn befund_netzfreiheit() {
        assert!(befund("tftn-cli", "tftn-cli v0.1.0\ntypst v0.15.1\n").is_ok());
        assert!(befund("tftn-cli", "tftn-cli v0.1.0\nureq v3.5.1\n").is_err());
        assert!(befund("tftn-app", "tftn-app v0.1.0\ntftn-holer v0.1.0\n").is_err());
        assert!(befund("tftn-pakete", "tftn-pakete v0.1.0\nopenssl v0.10.0\n").is_err());
        assert!(befund("tftn-holer", MIT_UREQ).is_ok());
        assert!(befund("tftn-holer", "tftn-holer v0.1.0\nrustls v0.23.0\n").is_err());
        assert!(befund("tftn-holer", "tftn-holer v0.1.0\nureq v3.5.1\n").is_err());
        assert!(befund("tftn-holer", &format!("{MIT_UREQ}native-tls v0.2.0\n")).is_err());
        assert!(befund("tftn-holer", &format!("{MIT_UREQ}reqwest v0.13.0\n")).is_err());
    }
}
