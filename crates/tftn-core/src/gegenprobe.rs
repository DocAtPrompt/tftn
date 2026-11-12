//! Die Gegenprobe-Zeile (KONZEPT 2.1): der typst-Aufruf, der ohne TFTN dasselbe Ergebnis liefert.

use crate::Projekt;

/// Argumente der Gegenprobe-Zeile ohne Programmname, relativ zum Projektordner.
pub fn gegenprobe_argumente(projekt: &Projekt, ziel: &str, zusatz: &[String]) -> Vec<String> {
    let mut a: Vec<String> = [
        "compile",
        "--root",
        ".",
        "--package-path",
        "packages",
        "--package-cache-path",
        "packages",
        "--ignore-system-fonts",
        "--ignore-embedded-fonts",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for ordner in projekt.schriftordner() {
        a.push("--font-path".into());
        a.push(match ordner.strip_prefix(&projekt.wurzel) {
            Ok(relativ) => relativ.to_string_lossy().replace('\\', "/"),
            Err(_) => ordner.to_string_lossy().into_owned(),
        });
    }
    a.extend(zusatz.iter().cloned());
    a.push(projekt.konfig.haupt.clone());
    a.push(ziel.to_string());
    a
}

/// Die Gegenprobe-Zeile zum Abtippen, für die Shell des laufenden Systems.
pub fn gegenprobe_zeile(projekt: &Projekt, ziel: &str, zusatz: &[String]) -> String {
    let mut liste = gegenprobe_argumente(projekt, ziel, zusatz);
    // Mit eingebundenen Schriften genügt `fonts`; die Ordner der App gehören nicht in die Zeile.
    if projekt.schriften().is_dir() {
        let mut i = 0;
        while i + 1 < liste.len() {
            if liste[i] == "--font-path" && liste[i + 1] != "fonts" {
                liste.drain(i..i + 2);
            } else {
                i += 1;
            }
        }
    }
    let argumente = liste
        .iter()
        .map(|a| shell_wort(a))
        .collect::<Vec<_>>()
        .join(" ");
    if cfg!(windows) {
        format!(
            "$env:SOURCE_DATE_EPOCH={}; typst {argumente}",
            projekt.epoche()
        )
    } else {
        format!("SOURCE_DATE_EPOCH={} typst {argumente}", projekt.epoche())
    }
}

fn shell_wort(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./{}".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}
