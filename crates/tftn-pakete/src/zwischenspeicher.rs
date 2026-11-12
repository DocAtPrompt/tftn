//! Zwischenspeicher am Cache-Ort des Systems (Festlegung 2 des Plans AP9):
//!
//! ```text
//! <wurzel>/index.json                         zuletzt geholter Index
//! <wurzel>/preview/<name>/<version>/          entpackt und geprüft
//! <wurzel>/preview/<name>/<version>.sha256    SHA-256 des Archivs; erst sie macht den Eintrag gültig
//! ```

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use crate::{Grenzen, PaketFehler, PaketSpec, entpacken, importe};

/// Höchstens so viele Pakete in einem Abschluss (Festlegung 4).
pub const ABSCHLUSS_MAX: usize = 100;

#[derive(Debug, Clone)]
pub struct Zwischenspeicher {
    pub wurzel: PathBuf,
}

fn io(p: &Path) -> impl Fn(std::io::Error) -> PaketFehler + '_ {
    move |e| PaketFehler::Io(p.to_path_buf(), e)
}

impl Zwischenspeicher {
    pub fn index_pfad(&self) -> PathBuf {
        self.wurzel.join("index.json")
    }

    fn basis(&self, s: &PaketSpec) -> PathBuf {
        self.wurzel.join("preview").join(&s.name)
    }

    fn eintrag(&self, s: &PaketSpec) -> PathBuf {
        self.basis(s).join(s.version.to_string())
    }

    fn sha_datei(&self, s: &PaketSpec) -> PathBuf {
        self.basis(s).join(format!("{}.sha256", s.version))
    }

    /// Ordner des Pakets, nur wenn der Eintrag gültig ist (mit `.sha256`).
    pub fn ordner(&self, s: &PaketSpec) -> Option<PathBuf> {
        self.sha256(s)?;
        let o = self.eintrag(s);
        o.is_dir().then_some(o)
    }

    /// SHA-256 des Archivs, aus dem der Eintrag stammt.
    pub fn sha256(&self, s: &PaketSpec) -> Option<String> {
        let text = std::fs::read_to_string(self.sha_datei(s)).ok()?;
        let sha = text.trim();
        (sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit()) && self.eintrag(s).is_dir())
            .then(|| sha.to_ascii_lowercase())
    }

    /// Prüft und entpackt `archiv` neben dem Ziel, benennt dann um und schreibt zuletzt `.sha256`.
    /// Ein vorhandener Eintrag wird ersetzt.
    pub fn ablegen(
        &self,
        s: &PaketSpec,
        archiv: &[u8],
        g: &Grenzen,
    ) -> Result<String, PaketFehler> {
        let basis = self.basis(s);
        std::fs::create_dir_all(&basis).map_err(io(&basis))?;
        let temp = basis.join(format!(".tmp-{}-{}", s.version, std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let sha = entpacken(archiv, s, &temp, g)?;
        let ergebnis = (|| {
            let sha_datei = self.sha_datei(s);
            if sha_datei.exists() {
                std::fs::remove_file(&sha_datei).map_err(io(&sha_datei))?;
            }
            let ziel = self.eintrag(s);
            if ziel.exists() {
                std::fs::remove_dir_all(&ziel).map_err(io(&ziel))?;
            }
            crate::umbenennen(&temp, &ziel).map_err(io(&ziel))?;
            let neu = basis.join(format!(".tmp-{}.sha256", s.version));
            std::fs::write(&neu, format!("{sha}\n")).map_err(io(&neu))?;
            crate::umbenennen(&neu, &sha_datei).map_err(io(&sha_datei))
        })();
        if ergebnis.is_err() {
            let _ = std::fs::remove_dir_all(&temp);
        }
        ergebnis.map(|()| sha)
    }

    /// Abschluss über die Importe: (vorhanden mit SHA, fehlend), jeweils sortiert. Fehlende Pakete
    /// können selbst weitere importieren; das zeigt sich erst, wenn sie geholt sind.
    #[allow(clippy::type_complexity)]
    pub fn abschluss(
        &self,
        s: &PaketSpec,
    ) -> Result<(Vec<(PaketSpec, String)>, Vec<PaketSpec>), PaketFehler> {
        let mut gesehen = BTreeSet::new();
        let mut schlange = VecDeque::from([s.clone()]);
        let mut da = Vec::new();
        let mut fehlt = Vec::new();
        while let Some(p) = schlange.pop_front() {
            if !gesehen.insert(p.clone()) {
                continue;
            }
            if gesehen.len() > ABSCHLUSS_MAX {
                return Err(PaketFehler::Archiv(tr!(
                    "{spec} braucht mehr als {max} Pakete – abgebrochen",
                    spec = s,
                    max = ABSCHLUSS_MAX
                )));
            }
            match (self.ordner(&p), self.sha256(&p)) {
                (Some(o), Some(sha)) => {
                    let (weitere, _) = importe(&o)?;
                    schlange.extend(weitere);
                    da.push((p, sha));
                }
                _ => fehlt.push(p),
            }
        }
        da.sort();
        fehlt.sort();
        Ok((da, fehlt))
    }
}
