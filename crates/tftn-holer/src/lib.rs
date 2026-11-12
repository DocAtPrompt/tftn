//! tftn-holer (PLAN-M2 AP9, Entwurf 8.2): holt den Paketindex und Pakete samt allen Paketen, die
//! sie importieren, aus dem Typst Universe in den Zwischenspeicher.
//!
//! Netz gibt es in TFTN nur hier. Jede Adresse wird vor dem Verbinden geprüft: nur
//! `https://packages.typst.org` und für „Nach Updates suchen“ genau zwei feste Adressen bei
//! `https://api.github.com` (`RELEASES`), für „Literatur aus DOI“ je eine feste Abfrage bei Crossref
//! und DataCite (`doi_adressen`), keine Weiterleitung, kein Proxy. Geprüft und entpackt wird über
//! `tftn-pakete`; ins Projekt übernimmt erst die App.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::Read;
use std::time::Duration;
use tftn_sprachen::tr;

use tftn_pakete::{Grenzen, Index, PaketFehler, PaketSpec, Zwischenspeicher, importe, sha256_hex};

/// Die einzige Gegenstelle im ausgelieferten Programm.
pub const UNIVERSE: &str = "https://packages.typst.org";

/// Für „Nach Updates suchen“: GitHub, und dort nur die neuesten Releases von typst und TFTN.
pub const GITHUB: &str = "https://api.github.com";
pub const RELEASES: [(&str, &str); 2] = [
    ("typst", "/repos/typst/typst/releases/latest"),
    ("TFTN", "/repos/DocAtPrompt/tftn/releases/latest"),
];

/// Für „Literatur aus DOI“: BibTeX zu einer DOI, ohne Weiterleitung (freigegeben 2026-11-11).
pub const CROSSREF: &str = "https://api.crossref.org";
pub const DATACITE: &str = "https://api.datacite.org";
const CROSSREF_VOR: &str = "/works/";
const CROSSREF_NACH: &str = "/transform/application/x-bibtex";
const DATACITE_VOR: &str = "/dois/application/x-bibtex/";

/// Höchstgröße des Index (Festlegung 8).
pub const INDEX_MAX: u64 = 64 << 20;

/// Wohin der Holer sich verbinden darf.
#[derive(Debug, Clone)]
pub enum Erlaubnis {
    Universe,
    /// Lokaler Prüfserver `http://127.0.0.1:<port>`, nur in Tests (Feature `pruefserver`)
    #[cfg(feature = "pruefserver")]
    Pruefserver(String),
}

impl Erlaubnis {
    pub fn basis(&self) -> &str {
        match self {
            Erlaubnis::Universe => UNIVERSE,
            #[cfg(feature = "pruefserver")]
            Erlaubnis::Pruefserver(b) => b,
        }
    }

    /// Wo die Releases stehen; im Test derselbe Prüfserver.
    pub fn github(&self) -> &str {
        match self {
            Erlaubnis::Universe => GITHUB,
            #[cfg(feature = "pruefserver")]
            Erlaubnis::Pruefserver(b) => b,
        }
    }

    pub fn crossref(&self) -> &str {
        match self {
            Erlaubnis::Universe => CROSSREF,
            #[cfg(feature = "pruefserver")]
            Erlaubnis::Pruefserver(b) => b,
        }
    }

    pub fn datacite(&self) -> &str {
        match self {
            Erlaubnis::Universe => DATACITE,
            #[cfg(feature = "pruefserver")]
            Erlaubnis::Pruefserver(b) => b,
        }
    }
}

#[derive(Debug)]
pub enum HolerFehler {
    Adresse(String),
    Weiterleitung(String),
    NichtGefunden(String),
    Status(u16, String),
    Netz(String),
    ZuGross(String),
    Paket(PaketFehler),
}

impl fmt::Display for HolerFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adresse(t)
            | Self::Weiterleitung(t)
            | Self::NichtGefunden(t)
            | Self::Netz(t)
            | Self::ZuGross(t) => f.write_str(t),
            Self::Status(s, url) => f.write_str(&tr!("{url}: Antwort {status}", url, status = s)),
            Self::Paket(p) => write!(f, "{p}"),
        }
    }
}

impl std::error::Error for HolerFehler {}

impl From<PaketFehler> for HolerFehler {
    fn from(p: PaketFehler) -> Self {
        HolerFehler::Paket(p)
    }
}

/// DOI aus Eingaben wie `10.1038/x`, `doi:10.1038/x` oder `https://doi.org/10.1038/x`:
/// `10.` + mindestens vier Ziffern (Unterteilung mit `.` erlaubt) + `/` + nicht leerer Rest ohne
/// Leer- und Steuerzeichen.
pub fn doi_lesen(eingabe: &str) -> Option<String> {
    let mut t = eingabe.trim();
    for vor in [
        "https://doi.org/",
        "http://doi.org/",
        "https://www.doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if let Some(r) = t.strip_prefix(vor) {
            t = r;
        }
    }
    if t.len() > 4 && t[..4].eq_ignore_ascii_case("doi:") {
        t = t[4..].trim_start();
    }
    let (praefix, rest) = t.split_once('/')?;
    let registrant = praefix.strip_prefix("10.")?;
    let ziffern = registrant.split('.').next().unwrap_or("");
    let ok = ziffern.len() >= 4
        && registrant.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && !rest.is_empty()
        && !rest.chars().any(|c| c.is_whitespace() || c.is_control());
    ok.then(|| t.to_string())
}

/// Prozentkodiert bis auf `A–Z a–z 0–9 - . _ ~ /`.
fn doi_pfad(doi: &str) -> String {
    let mut aus = String::new();
    for b in doi.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            aus.push(b as char);
        } else {
            aus.push_str(&format!("%{b:02X}"));
        }
    }
    aus
}

/// Ein kodierter DOI-Pfad, wie `doi_pfad` ihn baut.
fn doi_pfad_gueltig(p: &str) -> bool {
    let b = p.as_bytes();
    let kodiert_ok = b.iter().enumerate().all(|(i, &c)| {
        c.is_ascii_alphanumeric()
            || b"-._~/".contains(&c)
            || (c == b'%'
                && b.get(i + 1..i + 3)
                    .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)))
    });
    kodiert_ok && p.starts_with("10.") && p.contains('/') && !p.split('/').any(|t| t == "..")
}

/// Die zwei Abfragen zu einer DOI: Crossref, dann DataCite.
pub fn doi_adressen(doi: &str, e: &Erlaubnis) -> [String; 2] {
    let p = doi_pfad(doi);
    [
        format!("{}{CROSSREF_VOR}{p}{CROSSREF_NACH}", e.crossref()),
        format!("{}{DATACITE_VOR}{p}", e.datacite()),
    ]
}

/// BibTeX zu einer DOI: Crossref, und nur wenn Crossref sie nicht kennt, DataCite.
pub fn doi_holen(a: &dyn Abruf, e: &Erlaubnis, doi: &str) -> Result<String, HolerFehler> {
    let [cr, dc] = doi_adressen(doi, e);
    let daten = match a.holen(&cr, 1 << 20) {
        Err(HolerFehler::NichtGefunden(_)) => match a.holen(&dc, 1 << 20) {
            Err(HolerFehler::NichtGefunden(_)) => {
                return Err(HolerFehler::NichtGefunden(tr!(
                    "Die DOI {doi} kennen weder Crossref noch DataCite.",
                    doi
                )));
            }
            r => r?,
        },
        r => r?,
    };
    let text = String::from_utf8(daten)
        .map_err(|_| HolerFehler::Netz(tr!("Antwort zu {doi} ist kein UTF-8", doi)))?;
    if !text.trim_start().starts_with('@') {
        return Err(HolerFehler::Netz(tr!(
            "Antwort zu {doi} ist kein BibTeX-Eintrag",
            doi
        )));
    }
    Ok(text)
}

/// Erlaubt ist genau `<basis>/<pfad>` mit einem Pfad aus `a–z 0–9 . _ - /` ohne `..`, dazu die
/// zwei Release-Adressen aus `RELEASES` wörtlich und die DOI-Abfragen aus `doi_adressen`.
pub fn adresse_pruefen(url: &str, e: &Erlaubnis) -> Result<(), HolerFehler> {
    if RELEASES
        .iter()
        .any(|(_, pfad)| url == format!("{}{pfad}", e.github()))
    {
        return Ok(());
    }
    let crossref = url
        .strip_prefix(e.crossref())
        .and_then(|r| r.strip_prefix(CROSSREF_VOR))
        .and_then(|r| r.strip_suffix(CROSSREF_NACH));
    let datacite = url
        .strip_prefix(e.datacite())
        .and_then(|r| r.strip_prefix(DATACITE_VOR));
    if crossref.or(datacite).is_some_and(doi_pfad_gueltig) {
        return Ok(());
    }
    let basis = e.basis();
    let ok = url.strip_prefix(basis).is_some_and(|pfad| {
        pfad.len() > 1
            && pfad.starts_with('/')
            && !pfad.split('/').any(|t| t == "..")
            && pfad
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-/".contains(&b))
    });
    if ok {
        Ok(())
    } else {
        Err(HolerFehler::Adresse(tr!(
            "Adresse {url} abgelehnt: TFTN holt nur von {basis}",
            url,
            basis
        )))
    }
}

/// Eine Anfrage; `max` begrenzt die gelesenen Bytes.
pub trait Abruf {
    fn holen(&self, url: &str, max: u64) -> Result<Vec<u8>, HolerFehler>;
}

/// HTTP über ureq mit rustls: HTTPS nur, keine Weiterleitung, kein Proxy, mit Zeitgrenzen.
pub struct Netz {
    agent: ureq::Agent,
    erlaubnis: Erlaubnis,
}

impl Netz {
    pub fn neu(erlaubnis: Erlaubnis) -> Netz {
        let https_only = matches!(erlaubnis, Erlaubnis::Universe);
        let config = ureq::Agent::config_builder()
            .https_only(https_only)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .proxy(None)
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(120)))
            .user_agent(concat!("tftn-holer/", env!("CARGO_PKG_VERSION")))
            .build();
        Netz {
            agent: config.into(),
            erlaubnis,
        }
    }
}

impl Abruf for Netz {
    fn holen(&self, url: &str, max: u64) -> Result<Vec<u8>, HolerFehler> {
        adresse_pruefen(url, &self.erlaubnis)?;
        let antwort = self
            .agent
            .get(url)
            .call()
            .map_err(|e| HolerFehler::Netz(format!("{url}: {e}")))?;
        let status = antwort.status().as_u16();
        if (300..400).contains(&status) {
            let ziel = antwort
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("?")
                .to_string();
            return Err(HolerFehler::Weiterleitung(tr!(
                "Weiterleitung von {url} nach {ziel} abgelehnt",
                url,
                ziel
            )));
        }
        if status == 404 {
            return Err(HolerFehler::NichtGefunden(tr!("{url} nicht gefunden", url)));
        }
        if status != 200 {
            return Err(HolerFehler::Status(status, url.to_string()));
        }
        let mut daten = Vec::new();
        antwort
            .into_body()
            .into_reader()
            .take(max + 1)
            .read_to_end(&mut daten)
            .map_err(|e| HolerFehler::Netz(format!("{url}: {e}")))?;
        if daten.len() as u64 > max {
            return Err(HolerFehler::ZuGross(tr!(
                "Antwort von {url} ist zu groß (mehr als {max} Bytes)",
                url,
                max
            )));
        }
        Ok(daten)
    }
}

/// Die neuesten Versionen (Release-Tags) von typst und TFTN; was fehlt, steht in `fehler`.
#[derive(Debug, Default)]
pub struct Versionen {
    pub typst: Option<String>,
    pub tftn: Option<String>,
    pub fehler: Vec<String>,
}

pub fn versionen_holen(a: &dyn Abruf, e: &Erlaubnis) -> Versionen {
    let mut v = Versionen::default();
    for (name, pfad) in RELEASES {
        let url = format!("{}{pfad}", e.github());
        let tag = match a.holen(&url, 1 << 20) {
            Ok(daten) => serde_json::from_slice::<serde_json::Value>(&daten)
                .ok()
                .and_then(|j| j["tag_name"].as_str().map(str::to_string))
                .ok_or_else(|| format!("{name}: {}", tr!("Antwort ohne Versionsnummer"))),
            Err(HolerFehler::NichtGefunden(_)) => Err(format!(
                "{name}: {}",
                tr!("keine öffentlich veröffentlichte Version gefunden")
            )),
            Err(f) => Err(format!("{name}: {f}")),
        };
        match (tag, name) {
            (Ok(t), "typst") => v.typst = Some(t),
            (Ok(t), _) => v.tftn = Some(t),
            (Err(f), _) => v.fehler.push(f),
        }
    }
    v
}

/// Holt `preview/index.json`; geschrieben wird nur ein lesbarer Index (Temp-Datei, dann umbenannt).
pub fn index_holen(
    a: &dyn Abruf,
    e: &Erlaubnis,
    zs: &Zwischenspeicher,
) -> Result<Index, HolerFehler> {
    let url = format!("{}/preview/index.json", e.basis());
    let daten = a.holen(&url, INDEX_MAX)?;
    let index = Index::lesen(&daten)?;
    let ziel = zs.index_pfad();
    let io = |p: &std::path::Path, f: std::io::Error| PaketFehler::Io(p.to_path_buf(), f);
    std::fs::create_dir_all(&zs.wurzel).map_err(|f| io(&zs.wurzel, f))?;
    let temp = zs
        .wurzel
        .join(format!(".tmp-index-{}.json", std::process::id()));
    std::fs::write(&temp, &daten).map_err(|f| io(&temp, f))?;
    tftn_pakete::umbenennen(&temp, &ziel).map_err(|f| {
        let _ = std::fs::remove_file(&temp);
        io(&ziel, f)
    })?;
    Ok(index)
}

#[derive(Debug, Clone)]
pub struct Geholt {
    pub spec: PaketSpec,
    pub sha256: String,
    /// Aus dem Netz geholt (sonst aus dem Zwischenspeicher)
    pub neu: bool,
}

fn erwartet_hex(erwartet: &BTreeMap<PaketSpec, String>, s: &PaketSpec) -> Option<String> {
    erwartet.get(s).map(|v| {
        v.strip_prefix("sha256:")
            .unwrap_or(v)
            .trim()
            .to_ascii_lowercase()
    })
}

fn pruefen(
    erwartet: &BTreeMap<PaketSpec, String>,
    s: &PaketSpec,
    sha: &str,
) -> Result<(), HolerFehler> {
    match erwartet_hex(erwartet, s) {
        Some(v) if v != sha => Err(PaketFehler::Pruefsumme {
            spec: s.to_string(),
            vermerkt: format!("sha256:{v}"),
            gefunden: format!("sha256:{sha}"),
        }
        .into()),
        _ => Ok(()),
    }
}

/// Holt `spec` und alle Pakete, die es importiert (rekursiv, höchstens 100). Was gültig im
/// Zwischenspeicher liegt, wird nicht erneut geholt. Jede SHA-256 wird gegen `erwartet` geprüft,
/// bevor ein Paket abgelegt wird.
pub fn paket_holen(
    a: &dyn Abruf,
    e: &Erlaubnis,
    zs: &Zwischenspeicher,
    spec: &PaketSpec,
    erwartet: &BTreeMap<PaketSpec, String>,
) -> Result<(Vec<Geholt>, Vec<String>), HolerFehler> {
    let g = Grenzen::default();
    let mut gesehen = BTreeSet::new();
    let mut schlange = VecDeque::from([spec.clone()]);
    let mut geholt = Vec::new();
    let mut warnungen = Vec::new();
    while let Some(p) = schlange.pop_front() {
        if !gesehen.insert(p.clone()) {
            continue;
        }
        if gesehen.len() > tftn_pakete::zwischenspeicher::ABSCHLUSS_MAX {
            return Err(PaketFehler::Archiv(tr!(
                "{spec} braucht mehr als {max} Pakete – abgebrochen",
                spec,
                max = tftn_pakete::zwischenspeicher::ABSCHLUSS_MAX
            ))
            .into());
        }
        let (sha, neu) = match (zs.ordner(&p), zs.sha256(&p)) {
            (Some(_), Some(sha)) => {
                pruefen(erwartet, &p, &sha)?;
                (sha, false)
            }
            _ => {
                let url = format!("{}/preview/{}-{}.tar.gz", e.basis(), p.name, p.version);
                let daten = a.holen(&url, g.archiv as u64).map_err(|f| match f {
                    HolerFehler::NichtGefunden(_) => HolerFehler::NichtGefunden(tr!(
                        "Paket {paket} nicht gefunden ({url})",
                        paket = p,
                        url
                    )),
                    andere => andere,
                })?;
                pruefen(erwartet, &p, &sha256_hex(&daten))?;
                (zs.ablegen(&p, &daten, &g)?, true)
            }
        };
        let ordner = zs.ordner(&p).expect("eben abgelegt bzw. vorhanden");
        let (weitere, w) = importe(&ordner)?;
        warnungen.extend(
            w.into_iter()
                .map(|t| tr!("{paket}: {text}", paket = p, text = t)),
        );
        schlange.extend(weitere);
        geholt.push(Geholt {
            spec: p,
            sha256: sha,
            neu,
        });
    }
    geholt.sort_by(|x, y| x.spec.cmp(&y.spec));
    Ok((geholt, warnungen))
}
