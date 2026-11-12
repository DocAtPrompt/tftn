//! Archiv eines Universe-Pakets prüfen und entpacken (Entwurf 8.2).
//!
//! Entpackt wird Eintrag für Eintrag von Hand, nicht mit `Entry::unpack_in`: Nur Dateien und
//! Ordner sind erlaubt, Pfade nur aus gewöhnlichen Teilen. Gezählt wird beim Schreiben, nicht nach
//! der Angabe im Kopf.

use std::cell::Cell;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tftn_sprachen::tr;

use sha2::{Digest, Sha256};
use tar::EntryType;

use crate::{PaketFehler, PaketSpec};

/// Obergrenzen beim Holen (Festlegung 8).
#[derive(Debug, Clone)]
pub struct Grenzen {
    /// Bytes des komprimierten Archivs
    pub archiv: usize,
    /// Bytes aller entpackten Dateien zusammen
    pub entpackt: u64,
    /// Einträge im Archiv
    pub eintraege: usize,
}

impl Default for Grenzen {
    fn default() -> Self {
        Grenzen {
            archiv: 64 << 20,
            entpackt: 256 << 20,
            eintraege: 20_000,
        }
    }
}

/// SHA-256 als Kleinbuchstaben-Hex.
pub fn sha256_hex(daten: &[u8]) -> String {
    Sha256::digest(daten)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Prüft das `.tar.gz` und entpackt es in `ziel` (muss leer sein oder fehlen). Gibt die SHA-256
/// des Archivs zurück. Bei einem Fehler ist `ziel` danach entfernt bzw. wieder leer.
pub fn entpacken(
    daten: &[u8],
    spec: &PaketSpec,
    ziel: &Path,
    g: &Grenzen,
) -> Result<String, PaketFehler> {
    let gab_es = ziel.exists();
    if gab_es {
        let leer = std::fs::read_dir(ziel)
            .map_err(|e| PaketFehler::Io(ziel.to_path_buf(), e))?
            .next()
            .is_none();
        if !leer {
            return Err(PaketFehler::Archiv(tr!(
                "{ordner} ist nicht leer",
                ordner = ziel.display()
            )));
        }
    }
    let ergebnis = (|| {
        if daten.len() > g.archiv {
            return Err(archiv(
                spec,
                tr!("Archiv zu groß ({bytes} Bytes)", bytes = daten.len()),
            ));
        }
        std::fs::create_dir_all(ziel).map_err(|e| PaketFehler::Io(ziel.to_path_buf(), e))?;
        auspacken(daten, spec, ziel, g)?;
        typst_toml_pruefen(spec, ziel)?;
        Ok(sha256_hex(daten))
    })();
    if ergebnis.is_err() {
        let _ = std::fs::remove_dir_all(ziel);
        if gab_es {
            let _ = std::fs::create_dir(ziel);
        }
    }
    ergebnis
}

fn archiv(spec: &PaketSpec, text: String) -> PaketFehler {
    PaketFehler::Archiv(tr!("{spec}: {text}", spec, text))
}

/// Begrenzt den entpackten Datenstrom insgesamt. `tar` liest Erweiterungsköpfe (GNU-LongName,
/// PAX) selbst ganz in den Speicher und überspringt Inhalte von Ordnern; die Zählung in
/// `auspacken` sieht beides nicht (Abschlussprüfung AP9).
struct Begrenzt<R> {
    innen: R,
    rest: u64,
    ueberschritten: Rc<Cell<bool>>,
}

impl<R: Read> Read for Begrenzt<R> {
    fn read(&mut self, puffer: &mut [u8]) -> std::io::Result<usize> {
        let n = self.innen.read(puffer)?;
        if n as u64 > self.rest {
            self.ueberschritten.set(true);
            return Err(std::io::Error::other("entpackt zu groß"));
        }
        self.rest -= n as u64;
        Ok(n)
    }
}

fn auspacken(daten: &[u8], spec: &PaketSpec, ziel: &Path, g: &Grenzen) -> Result<(), PaketFehler> {
    let ueberschritten = Rc::new(Cell::new(false));
    // Spielraum für Köpfe: 1 KiB je Eintrag und 1 MiB für Füllbytes am Ende.
    let gesamt = g.entpackt + g.eintraege as u64 * 1024 + (1 << 20);
    let lesefehler = |e: std::io::Error| {
        if ueberschritten.get() {
            archiv(
                spec,
                tr!(
                    "entpackt zu groß (mehr als {gesamt} Bytes samt Köpfen)",
                    gesamt
                ),
            )
        } else {
            archiv(spec, tr!("Archiv nicht lesbar: {fehler}", fehler = e))
        }
    };
    let mut tar = tar::Archive::new(Begrenzt {
        innen: flate2::read::GzDecoder::new(daten),
        rest: gesamt,
        ueberschritten: ueberschritten.clone(),
    });
    let mut eintraege = 0usize;
    let mut geschrieben = 0u64;
    for eintrag in tar.entries().map_err(lesefehler)? {
        let mut eintrag = eintrag.map_err(lesefehler)?;
        let art = eintrag.header().entry_type();
        // Pax-Köpfe tragen nur Metadaten (etwa einen Kommentar); `tar` wertet sie selbst aus.
        if matches!(art, EntryType::XGlobalHeader | EntryType::XHeader) {
            continue;
        }
        eintraege += 1;
        if eintraege > g.eintraege {
            return Err(archiv(
                spec,
                tr!("zu viele Einträge (mehr als {max})", max = g.eintraege),
            ));
        }
        let roh = eintrag.path_bytes().into_owned();
        let relativ = pfad_pruefen(&roh).map_err(|t| archiv(spec, t))?;
        match art {
            EntryType::Directory => {
                let o = ziel.join(&relativ);
                std::fs::create_dir_all(&o).map_err(|e| PaketFehler::Io(o, e))?;
            }
            EntryType::Regular | EntryType::Continuous => {
                if relativ.as_os_str().is_empty() {
                    return Err(archiv(spec, tr!("Pfad ohne Namen")));
                }
                let datei = ziel.join(&relativ);
                if let Some(eltern) = datei.parent() {
                    std::fs::create_dir_all(eltern)
                        .map_err(|e| PaketFehler::Io(eltern.to_path_buf(), e))?;
                }
                let mut aus =
                    std::fs::File::create(&datei).map_err(|e| PaketFehler::Io(datei.clone(), e))?;
                let rest = g.entpackt - geschrieben;
                let mut puffer = [0u8; 64 * 1024];
                let mut begrenzt = (&mut eintrag).take(rest + 1);
                loop {
                    let n = begrenzt.read(&mut puffer).map_err(lesefehler)?;
                    if n == 0 {
                        break;
                    }
                    geschrieben += n as u64;
                    if geschrieben > g.entpackt {
                        return Err(archiv(
                            spec,
                            tr!(
                                "entpackt zu groß (mehr als {bytes} Bytes)",
                                bytes = g.entpackt
                            ),
                        ));
                    }
                    aus.write_all(&puffer[..n])
                        .map_err(|e| PaketFehler::Io(datei.clone(), e))?;
                }
            }
            EntryType::Symlink | EntryType::Link => {
                return Err(archiv(
                    spec,
                    tr!(
                        "Verknüpfung „{pfad}“ ist nicht erlaubt",
                        pfad = String::from_utf8_lossy(&roh)
                    ),
                ));
            }
            andere => {
                return Err(archiv(
                    spec,
                    tr!(
                        "Eintragsart {art} bei „{pfad}“ ist nicht erlaubt",
                        art = format!("{andere:?}"),
                        pfad = String::from_utf8_lossy(&roh)
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Nur gewöhnliche Teile, getrennt durch `/`; `.` und leere Teile werden übergangen.
fn pfad_pruefen(roh: &[u8]) -> Result<PathBuf, String> {
    let text = std::str::from_utf8(roh).map_err(|_| tr!("Pfad ist kein UTF-8"))?;
    let falsch = || tr!("Pfad „{text}“ ist nicht erlaubt", text);
    if text.starts_with('/') || text.contains('\0') {
        return Err(falsch());
    }
    let mut pfad = PathBuf::new();
    for teil in text.split('/') {
        match teil {
            "" | "." => {}
            ".." => return Err(falsch()),
            t if t.contains('\\') || t.contains(':') => return Err(falsch()),
            t => pfad.push(t),
        }
    }
    Ok(pfad)
}

/// `typst.toml` vorhanden, `[package]` `name` und `version` wie verlangt.
fn typst_toml_pruefen(spec: &PaketSpec, ziel: &Path) -> Result<(), PaketFehler> {
    let pfad = ziel.join("typst.toml");
    let text = std::fs::read_to_string(&pfad).map_err(|_| archiv(spec, tr!("typst.toml fehlt")))?;
    let wert: toml::Table = toml::from_str(&text)
        .map_err(|e| archiv(spec, tr!("typst.toml fehlerhaft: {fehler}", fehler = e)))?;
    let paket = wert.get("package").and_then(|p| p.as_table());
    let feld = |n: &str| paket.and_then(|p| p.get(n)).and_then(|v| v.as_str());
    match feld("name") {
        Some(n) if n == spec.name => {}
        n => {
            return Err(archiv(
                spec,
                tr!(
                    "Name in typst.toml ist {ist}, erwartet „{soll}“",
                    ist = format!("{n:?}"),
                    soll = spec.name
                ),
            ));
        }
    }
    match feld("version").map(semver::Version::parse) {
        Some(Ok(v)) if v == spec.version => Ok(()),
        _ => Err(archiv(
            spec,
            tr!(
                "Version in typst.toml ist {ist}, erwartet „{soll}“",
                ist = format!("{:?}", feld("version")),
                soll = spec.version
            ),
        )),
    }
}
