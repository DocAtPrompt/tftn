//! Universe-Pakete ohne Netz (PLAN-M2 AP9, Entwurf 8.2): Paketangabe, Archiv prüfen und entpacken,
//! Importe, Zwischenspeicher, Index und Übernahme ins Projekt. Holen über das Netz kann nur
//! `tftn-holer`; dieses Crate nutzen App und Holer gemeinsam.

pub mod archiv;
pub mod importe;
pub mod index;
pub mod projekt;
pub mod spec;
pub mod zwischenspeicher;

use std::fmt;
use std::path::PathBuf;
use tftn_sprachen::tr;

pub use archiv::{Grenzen, entpacken, sha256_hex};
pub use importe::{importe, projektimporte};
pub use index::{Index, IndexEintrag};
pub use projekt::{ProjektPaket, Uebernahme, benutzer, entfernen, im_projekt, readme, uebernehmen};
pub use spec::PaketSpec;
pub use zwischenspeicher::Zwischenspeicher;

#[derive(Debug)]
pub enum PaketFehler {
    /// Ungültige Paketangabe
    Angabe(String),
    /// Archiv abgelehnt (Pfad, Art, Größe, typst.toml …)
    Archiv(String),
    /// Prüfsumme weicht von der in `tftn.toml` ab
    Pruefsumme {
        spec: String,
        vermerkt: String,
        gefunden: String,
    },
    Io(PathBuf, std::io::Error),
    /// Index nicht lesbar
    Index(String),
    /// Projektdatei oder Paketordner im Projekt
    Projekt(String),
}

impl fmt::Display for PaketFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Angabe(t) | Self::Archiv(t) | Self::Index(t) | Self::Projekt(t) => f.write_str(t),
            Self::Pruefsumme {
                spec,
                vermerkt,
                gefunden,
            } => f.write_str(&tr!(
                "{spec}: Prüfsumme weicht ab – im Projekt vermerkt {vermerkt}, geholt {gefunden}. Abgebrochen, nichts übernommen.",
                spec,
                vermerkt,
                gefunden
            )),
            Self::Io(p, e) => write!(f, "{}: {e}", p.display()),
        }
    }
}

impl std::error::Error for PaketFehler {}

/// Wiederholt `f` bei `PermissionDenied`, höchstens `versuche`-mal mit `pause` dazwischen. Unter
/// Windows hält der Virenschutz bzw. der Sync eine frisch geschriebene Datei oft kurz offen; dann
/// scheitert ein Umbenennen mit „Zugriff verweigert“ (Abschlussprüfung AP9).
pub fn wiederholen<T>(
    versuche: u32,
    pause: std::time::Duration,
    mut f: impl FnMut() -> std::io::Result<T>,
) -> std::io::Result<T> {
    let mut n = 1;
    loop {
        match f() {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && n < versuche => {
                n += 1;
                std::thread::sleep(pause);
            }
            r => return r,
        }
    }
}

/// `std::fs::rename` mit `wiederholen` (10 × 100 ms).
pub fn umbenennen(von: &std::path::Path, nach: &std::path::Path) -> std::io::Result<()> {
    wiederholen(10, std::time::Duration::from_millis(100), || {
        std::fs::rename(von, nach)
    })
}

#[cfg(test)]
mod tests {
    use super::wiederholen;
    use std::io::{Error, ErrorKind};
    use std::time::Duration;

    #[test]
    fn wiederholen_bei_zugriff_verweigert() {
        let mut n = 0;
        let r = wiederholen(10, Duration::ZERO, || {
            n += 1;
            if n < 3 {
                Err(Error::from(ErrorKind::PermissionDenied))
            } else {
                Ok(n)
            }
        });
        assert_eq!(r.unwrap(), 3);

        let mut n = 0;
        let r: std::io::Result<()> = wiederholen(10, Duration::ZERO, || {
            n += 1;
            Err(Error::from(ErrorKind::NotFound))
        });
        assert!(r.is_err());
        assert_eq!(n, 1, "andere Fehler nicht wiederholen");

        let mut n = 0;
        let r: std::io::Result<()> = wiederholen(3, Duration::ZERO, || {
            n += 1;
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert_eq!(r.unwrap_err().kind(), ErrorKind::PermissionDenied);
        assert_eq!(n, 3);
    }
}
