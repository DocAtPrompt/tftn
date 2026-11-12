//! Paketangabe `@preview/<name>:<x.y.z>` (Festlegung 8 des Plans AP9).

use std::fmt;
use tftn_sprachen::tr;

use crate::PaketFehler;

/// Ein Universe-Paket in genau einer Version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaketSpec {
    pub name: String,
    pub version: semver::Version,
}

/// Höchstlänge eines Paketnamens.
const NAME_MAX: usize = 64;

impl PaketSpec {
    /// Liest `@preview/<name>:<x.y.z>`.
    pub fn lesen(text: &str) -> Result<PaketSpec, PaketFehler> {
        let falsch = || {
            PaketFehler::Angabe(tr!(
                "„{text}“ ist keine Paketangabe der Form @preview/<name>:<x.y.z>",
                text
            ))
        };
        let rest = text.strip_prefix("@preview/").ok_or_else(falsch)?;
        let (name, version) = rest.split_once(':').ok_or_else(falsch)?;
        PaketSpec::neu(name, version)
    }

    /// Prüft Name (`^[a-z0-9][a-z0-9-]*$`, höchstens 64 Zeichen) und Version (streng `x.y.z`).
    pub fn neu(name: &str, version: &str) -> Result<PaketSpec, PaketFehler> {
        let name_ok = !name.is_empty()
            && name.len() <= NAME_MAX
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !name.starts_with('-');
        if !name_ok {
            return Err(PaketFehler::Angabe(tr!(
                "Paketname „{name}“ ist ungültig (nur a–z, 0–9 und -, höchstens {max} Zeichen)",
                name,
                max = NAME_MAX
            )));
        }
        let v = semver::Version::parse(version)
            .ok()
            .filter(|v| v.pre.is_empty() && v.build.is_empty())
            .ok_or_else(|| {
                PaketFehler::Angabe(tr!(
                    "Version „{version}“ von {name} ist ungültig (erwartet x.y.z)",
                    version,
                    name
                ))
            })?;
        Ok(PaketSpec {
            name: name.to_string(),
            version: v,
        })
    }
}

impl fmt::Display for PaketSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@preview/{}:{}", self.name, self.version)
    }
}
