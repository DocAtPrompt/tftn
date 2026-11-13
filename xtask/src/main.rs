//! Hilfsaufgaben für die Entwicklung: `cargo xtask <aufgabe>`.
//!
//! `messen` führt die Latenzmessung (PLAN-M1 4.4) im Release-Bau aus, siehe `crates/tftn-messen`.
//!
//! `typst-holen [VERSION]` lädt die offiziellen typst-CLI-Archive (macOS arm64, Windows x64),
//! prüft sie gegen `xtask/typst-cli.sha256` und legt die Programme unter
//! `_lokal/typst-cli/<version>/` ab (KONZEPT 4.8). Nutzt `curl` und `tar` des Systems
//! (beide in macOS und Windows 10/11 enthalten); jeder Download hat eine Frist.
//!
//! `referenzen [--uebernehmen PFAD…]` – visuelle Regression, siehe `referenzen.rs`.
//! `ci` und `pruefsummen [DATEI]` – CI-Jobs lokal bzw. Plattformvergleich, siehe `ci.rs`.
//! `typstyle [--check] [PFAD…]` – Typst-Quellen formatieren, siehe `typstyle.rs`.
//! `holer-bauen [--release]` – `tftn-holer` bauen und als Sidecar unter `target/sidecar/` ablegen
//! (PLAN-M2 AP9); prüft, dass das Programm keinen Prüfserver kennt.
//! `netzfreiheit` – kein HTTP-Client außer in `tftn-holer`, siehe `ci.rs`.
//! `sprachen [DATEI…]` – `languages/en.toml` an die Texte im Code angleichen, siehe `sprachen.rs`.
//! Jeder gestartete Prozess hat eine Frist (Projektregel Hintergrundprozesse).

mod ci;
mod referenzen;
mod sprachen;
mod typstyle;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const FRIST_SEKUNDEN: &str = "300";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    typst_aus_lokal();
    let ergebnis = match args.first().map(String::as_str) {
        Some("typst-holen") => {
            let nur_host = args.iter().any(|a| a == "--nur-host");
            let version = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with("--"))
                .map(String::as_str);
            typst_holen(version.unwrap_or("0.15.1"), nur_host)
        }
        Some("messen") => ausfuehren(
            Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
                .args(["run", "--quiet", "--release", "--package", "tftn-messen"])
                .current_dir(projektordner()),
        ),
        Some("referenzen") => {
            let rest = &args[1..];
            match rest.first().map(String::as_str) {
                None => referenzen::referenzen(&[]),
                Some("--uebernehmen") if rest.len() > 1 => referenzen::referenzen(&rest[1..]),
                _ => Err("referenzen [--uebernehmen <dokument>[/<datei>]…]".to_string()),
            }
        }
        Some("ci") => ci::ci(),
        Some("pruefsummen") => ci::pruefsummen(args.get(1).map(Path::new)),
        Some("typstyle") => typstyle::typstyle(&args[1..]),
        Some("holer-bauen") => holer_bauen(args.iter().any(|a| a == "--release")).map(|p| {
            println!("{}", p.display());
        }),
        Some("netzfreiheit") => ci::netzfreiheit(),
        Some("sprachen") => sprachen::sprachen(&args[1..]),
        _ => Err(
            "Aufgaben: typst-holen [VERSION] [--nur-host] | messen | referenzen \
                  [--uebernehmen PFAD…] | ci | pruefsummen [DATEI] | typstyle [--check] [PFAD…] \
                  | holer-bauen [--release] | netzfreiheit | sprachen [ÜBERSETZUNGEN.toml…]"
                .to_string(),
        ),
    };
    match ergebnis {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Wie in der CI: die geprüfte typst-CLI aus `_lokal/`, falls vorhanden und `TFTN_TYPST` leer.
fn typst_aus_lokal() {
    if std::env::var_os("TFTN_TYPST").is_some() {
        return;
    }
    let cli = projektordner().join(format!(
        "_lokal/typst-cli/0.15.1/typst-{}{}",
        host_ziel(),
        std::env::consts::EXE_SUFFIX
    ));
    if cli.is_file() {
        // SAFETY: am Anfang von main, noch keine weiteren Fäden.
        unsafe { std::env::set_var("TFTN_TYPST", &cli) };
    }
}

fn cargo() -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.current_dir(projektordner());
    c
}

/// Baut `tftn` im Umfang der Tests (sonst vereinigt Cargo andere Features und baut neu).
fn tftn_bauen() -> Result<PathBuf, String> {
    let mut befehl = cargo();
    // Ohne xtask selbst: Windows kann das laufende Programm nicht überschreiben.
    befehl.args([
        "build",
        "--quiet",
        "--workspace",
        "--bins",
        "--exclude",
        "xtask",
    ]);
    if cfg!(target_os = "linux") {
        befehl.args(["--exclude", "tftn-app"]);
    }
    mit_frist(&mut befehl, 3600)?;
    Ok(projektordner().join(format!("target/debug/tftn{}", std::env::consts::EXE_SUFFIX)))
}

/// Baut `tftn-holer` und legt ihn als `target/sidecar/tftn-holer-<ziel><exe>` ab (Tauri
/// `externalBin` in `src-tauri/tauri.buendel.json`). Danach muss das Programm `--pruefserver` als
/// unbekannte Option ablehnen: Das Feature gibt es nur in Tests (Festlegung 13 des Plans AP9).
fn holer_bauen(release: bool) -> Result<PathBuf, String> {
    let mut befehl = cargo();
    befehl.args(["build", "--quiet", "-p", "tftn-holer"]);
    if release {
        befehl.arg("--release");
    }
    mit_frist(&mut befehl, 3600)?;
    let exe = std::env::consts::EXE_SUFFIX;
    let profil = if release { "release" } else { "debug" };
    let von = projektordner().join(format!("target/{profil}/tftn-holer{exe}"));
    let ordner = projektordner().join("target/sidecar");
    std::fs::create_dir_all(&ordner).map_err(|e| format!("{}: {e}", ordner.display()))?;
    let nach = ordner.join(format!("tftn-holer-{}{exe}", host_ziel()));
    std::fs::copy(&von, &nach)
        .map_err(|e| format!("{} → {}: {e}", von.display(), nach.display()))?;
    // Der Holer beendet sich bei falschem Aufruf sofort (und sonst nach seiner eigenen Frist).
    let aus = Command::new(&nach)
        .args([
            "--pruefserver",
            "http://127.0.0.1:9",
            "index",
            "--zwischenspeicher",
        ])
        .arg(ordner.join("probe"))
        .output()
        .map_err(|e| format!("{}: {e}", nach.display()))?;
    let text = String::from_utf8_lossy(&aus.stdout);
    if aus.status.code() != Some(2) || !text.contains("unbekannte Option --pruefserver") {
        return Err(format!(
            "{} kennt --pruefserver (Exit {:?}: {text}) – Feature pruefserver ist an",
            nach.display(),
            aus.status.code()
        ));
    }
    Ok(nach)
}

/// Startet `befehl` und bricht ihn nach `sekunden` ab.
fn mit_frist(befehl: &mut Command, sekunden: u64) -> Result<(), String> {
    let mut kind = befehl.spawn().map_err(|e| format!("{befehl:?}: {e}"))?;
    let ende = Instant::now() + Duration::from_secs(sekunden);
    loop {
        if let Some(status) = kind.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("{befehl:?} endete mit {status}"))
            };
        }
        if Instant::now() > ende {
            let _ = kind.kill();
            let _ = kind.wait();
            return Err(format!("{befehl:?} nach {sekunden} s abgebrochen"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn projektordner() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

struct Eintrag {
    ziel: String,
    sha256: String,
    datei: String,
}

fn eintraege(version: &str) -> Result<Vec<Eintrag>, String> {
    let liste = std::fs::read_to_string(projektordner().join("xtask/typst-cli.sha256"))
        .map_err(|e| format!("xtask/typst-cli.sha256: {e}"))?;
    let gefunden: Vec<Eintrag> = liste
        .lines()
        .filter(|z| !z.starts_with('#') && !z.trim().is_empty())
        .filter_map(|z| {
            let teile: Vec<&str> = z.split_whitespace().collect();
            (teile.len() == 4 && teile[0] == version).then(|| Eintrag {
                ziel: teile[1].into(),
                sha256: teile[2].into(),
                datei: teile[3].into(),
            })
        })
        .collect();
    if gefunden.is_empty() {
        return Err(format!(
            "Keine Prüfsummen für typst {version} in xtask/typst-cli.sha256"
        ));
    }
    Ok(gefunden)
}

/// Zielplattform dieses Rechners in der Schreibweise der typst-Release-Archive.
fn host_ziel() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", _) => "x86_64-apple-darwin",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("windows", _) => "x86_64-pc-windows-msvc",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        _ => "x86_64-unknown-linux-musl",
    }
}

fn typst_holen(version: &str, nur_host: bool) -> Result<(), String> {
    let ablage = projektordner().join("_lokal/typst-cli").join(version);
    let arbeit = ablage.join("download");
    std::fs::create_dir_all(&arbeit).map_err(|e| e.to_string())?;

    let liste: Vec<Eintrag> = eintraege(version)?
        .into_iter()
        .filter(|e| !nur_host || e.ziel == host_ziel())
        .collect();
    if liste.is_empty() {
        return Err(format!(
            "Keine Prüfsumme für {} in xtask/typst-cli.sha256",
            host_ziel()
        ));
    }
    for e in liste {
        let url = format!(
            "https://github.com/typst/typst/releases/download/v{version}/{}",
            e.datei
        );
        let archiv = arbeit.join(&e.datei);
        if !archiv.exists() || sha256(&archiv)? != e.sha256 {
            println!("lade {url}");
            ausfuehren(
                Command::new("curl")
                    .args(["--fail", "--location", "--silent", "--show-error"])
                    .args(["--max-time", FRIST_SEKUNDEN, "--output"])
                    .arg(&archiv)
                    .arg(&url),
            )?;
        }
        let ist = sha256(&archiv)?;
        if ist != e.sha256 {
            let _ = std::fs::remove_file(&archiv);
            return Err(format!(
                "{}: SHA-256 {ist} ≠ erwartet {}",
                e.datei, e.sha256
            ));
        }
        println!("geprüft  {}  {}", e.sha256, e.datei);

        // Archiv enthält typst-<ziel>/typst[.exe] und LICENSE.
        ausfuehren(
            Command::new("tar")
                .arg("-xf")
                .arg(&archiv)
                .arg("-C")
                .arg(&arbeit),
        )?;
        let ordner = arbeit.join(format!("typst-{}", e.ziel));
        let endung = if e.ziel.contains("windows") {
            ".exe"
        } else {
            ""
        };
        let ziel = ablage.join(format!("typst-{}{endung}", e.ziel));
        std::fs::copy(ordner.join(format!("typst{endung}")), &ziel).map_err(|e| e.to_string())?;
        std::fs::copy(ordner.join("LICENSE"), ablage.join("LICENSE")).map_err(|e| e.to_string())?;
        println!("abgelegt {}", ziel.display());
    }
    std::fs::write(
        ablage.join("HERKUNFT.txt"),
        format!(
            "Offizielle typst-CLI {version} von https://github.com/typst/typst/releases/tag/v{version}\n\
             Geprüft gegen xtask/typst-cli.sha256 durch `cargo xtask typst-holen`.\n\
             Lizenz: Apache-2.0 (LICENSE). Wiederbeschaffbar, daher in _lokal/ (nicht gesichert).\n"
        ),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn sha256(pfad: &Path) -> Result<String, String> {
    let daten = std::fs::read(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    Ok(Sha256::digest(&daten)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn ausfuehren(befehl: &mut Command) -> Result<(), String> {
    let status = befehl.status().map_err(|e| format!("{befehl:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{befehl:?} endete mit {status}"))
    }
}
