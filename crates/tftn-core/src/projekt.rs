//! Ein TFTN-Projekt: Ordner mit `tftn.toml` (KONZEPT 5.7).

use std::fmt;
use std::path::{Path, PathBuf};
use tftn_sprachen::tr;

use serde::Deserialize;

static VORRAT: std::sync::RwLock<Vec<PathBuf>> = std::sync::RwLock::new(Vec::new());

/// Der Vorrat an Schriftordnern (die der App bzw. der Dokumenttypen).
pub fn schriftvorrat() -> Vec<PathBuf> {
    VORRAT.read().map(|v| v.clone()).unwrap_or_default()
}

/// Schriftordner für Projekte ohne eingebundene Schriften (die der App bzw. der Dokumenttypen).
pub fn schriftvorrat_setzen(ordner: Vec<PathBuf>) {
    if let Ok(mut v) = VORRAT.write() {
        *v = ordner;
    }
}

/// Inhalt von `tftn.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct TftnToml {
    /// Formatversion der Projektdatei.
    pub tftn: u32,
    /// Typst-Version, mit der das Projekt zuletzt abgenommen wurde.
    pub typst: String,
    /// Hauptdatei relativ zum Projektordner.
    #[serde(default = "standard_haupt")]
    pub haupt: String,
    /// Fester Zeitpunkt für `SOURCE_DATE_EPOCH` und `World::today()`.
    pub zeitstempel: toml::value::Datetime,
    /// Dokumenttyp als `@local/<name>:<version>`, vendort unter `packages/local/`.
    pub typ: Option<String>,
    /// Prüfsummen der geholten Universe-Pakete: `"@preview/<name>:<version>" = "sha256:<hex>"`
    /// (PLAN-M2 AP9).
    #[serde(default)]
    pub pakete: std::collections::BTreeMap<String, String>,
}

fn standard_haupt() -> String {
    "main.typ".into()
}

/// Ein geöffnetes Projekt.
#[derive(Debug, Clone)]
pub struct Projekt {
    /// Projektordner, kanonisiert (entspricht `--root .` der Gegenprobe).
    pub wurzel: PathBuf,
    pub konfig: TftnToml,
}

#[derive(Debug)]
pub enum ProjektFehler {
    Lesen(PathBuf, std::io::Error),
    Format(PathBuf, String),
}

impl fmt::Display for ProjektFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lesen(p, e) => f.write_str(&tr!(
                "{datei} lässt sich nicht lesen: {fehler}",
                datei = p.display(),
                fehler = e
            )),
            Self::Format(p, e) => f.write_str(&tr!(
                "{datei} ist fehlerhaft: {fehler}",
                datei = p.display(),
                fehler = e
            )),
        }
    }
}

impl std::error::Error for ProjektFehler {}

impl Projekt {
    /// Öffnet den Projektordner und liest `tftn.toml`.
    pub fn oeffnen(ordner: &Path) -> Result<Self, ProjektFehler> {
        let wurzel = ordner
            .canonicalize()
            .map_err(|e| ProjektFehler::Lesen(ordner.to_path_buf(), e))?;
        let pfad = wurzel.join("tftn.toml");
        let text =
            std::fs::read_to_string(&pfad).map_err(|e| ProjektFehler::Lesen(pfad.clone(), e))?;
        let konfig: TftnToml = toml::from_str(&text)
            .map_err(|e| ProjektFehler::Format(pfad.clone(), e.to_string()))?;
        if konfig.tftn != 1 {
            return Err(ProjektFehler::Format(
                pfad,
                tr!(
                    "tftn = {version} wird nicht unterstützt (erwartet 1)",
                    version = konfig.tftn
                ),
            ));
        }
        sekunden_seit_1970(&konfig.zeitstempel).map_err(|e| ProjektFehler::Format(pfad, e))?;
        Ok(Self { wurzel, konfig })
    }

    /// Pfad der Hauptdatei.
    pub fn haupt(&self) -> PathBuf {
        self.wurzel.join(&self.konfig.haupt)
    }

    /// Wo die Schriften herkommen: zuerst `fonts/` im Projekt (eingebunden), dahinter der Vorrat
    /// der App (`schriftvorrat_setzen`). So stimmt die Vorschau auch mit einer neu gewählten Schrift,
    /// bevor das Speichern sie einbindet.
    pub fn schriftordner(&self) -> Vec<PathBuf> {
        let eigen = self.schriften();
        let mut aus = if eigen.is_dir() {
            vec![eigen]
        } else {
            Vec::new()
        };
        aus.extend(schriftvorrat());
        aus
    }

    /// Ordner mit den Projektschriften (`--font-path fonts`).
    pub fn schriften(&self) -> PathBuf {
        self.wurzel.join("fonts")
    }

    /// Ordner mit den vendorten Paketen (`--package-path packages`).
    pub fn pakete(&self) -> PathBuf {
        self.wurzel.join("packages")
    }

    /// Wert für `SOURCE_DATE_EPOCH`.
    pub fn epoche(&self) -> i64 {
        sekunden_seit_1970(&self.konfig.zeitstempel).expect("beim Öffnen geprüft")
    }
}

/// Rechnet einen TOML-Zeitpunkt mit Datum, Uhrzeit und Zeitzone in Unix-Sekunden um.
fn sekunden_seit_1970(dt: &toml::value::Datetime) -> Result<i64, String> {
    let (Some(datum), Some(zeit), Some(zone)) = (dt.date, dt.time, dt.offset) else {
        return Err(format!(
            "zeitstempel = {dt} braucht Datum, Uhrzeit und Zeitzone, z. B. 2026-10-31T00:00:00Z"
        ));
    };
    let tage = tage_seit_1970(
        i64::from(datum.year),
        i64::from(datum.month),
        i64::from(datum.day),
    );
    let sekunden =
        i64::from(zeit.hour) * 3600 + i64::from(zeit.minute) * 60 + i64::from(zeit.second);
    let versatz = match zone {
        toml::value::Offset::Z => 0,
        toml::value::Offset::Custom { minutes } => i64::from(minutes) * 60,
    };
    Ok(tage * 86_400 + sekunden - versatz)
}

/// Tage seit 1970-01-01 im gregorianischen Kalender (Verfahren nach H. Hinnant).
fn tage_seit_1970(jahr: i64, monat: i64, tag: i64) -> i64 {
    let j = if monat <= 2 { jahr - 1 } else { jahr };
    let era = j.div_euclid(400);
    let jde = j - era * 400;
    let m = (monat + 9) % 12;
    let tdj = (153 * m + 2) / 5 + tag - 1;
    let tde = jde * 365 + jde / 4 - jde / 100 + tdj;
    era * 146_097 + tde - 719_468
}

/// Unix-Sekunden → `2026-10-31T12:00:00Z` (Verfahren nach H. Hinnant).
pub fn iso_utc(sekunden: i64) -> String {
    let tage = sekunden.div_euclid(86_400);
    let rest = sekunden.rem_euclid(86_400);
    let z = tage + 719_468;
    let era = z.div_euclid(146_097);
    let tde = z - era * 146_097;
    let jde = (tde - tde / 1460 + tde / 36_524 - tde / 146_096) / 365;
    let tdj = tde - (365 * jde + jde / 4 - jde / 100);
    let m = (5 * tdj + 2) / 153;
    let tag = tdj - (153 * m + 2) / 5 + 1;
    let monat = if m < 10 { m + 3 } else { m - 9 };
    let jahr = jde + era * 400 + i64::from(monat <= 2);
    format!(
        "{jahr:04}-{monat:02}-{tag:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoche(s: &str) -> i64 {
        let dt: toml::value::Datetime = s.parse().unwrap();
        sekunden_seit_1970(&dt).unwrap()
    }

    #[test]
    fn epoche_wie_date_kommando() {
        assert_eq!(epoche("1970-01-01T00:00:00Z"), 0);
        assert_eq!(epoche("2026-10-31T00:00:00Z"), 1_793_404_800);
        assert_eq!(epoche("2000-02-29T12:34:56Z"), 951_827_696);
        assert_eq!(epoche("2026-10-31T01:00:00+01:00"), 1_793_404_800);
    }

    #[test]
    fn zeitstempel_ohne_zone_ist_fehler() {
        let dt: toml::value::Datetime = "2026-10-31T00:00:00".parse().unwrap();
        assert!(sekunden_seit_1970(&dt).is_err());
    }

    #[test]
    fn iso_utc_bekannte_werte() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_793_404_800), "2026-10-31T00:00:00Z");
        assert_eq!(iso_utc(951_827_696), "2000-02-29T12:34:56Z");
    }
}
