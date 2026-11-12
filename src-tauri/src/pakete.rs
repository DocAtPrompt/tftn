//! Universe-Pakete in der App (PLAN-M2 AP9): Holer finden und mit Frist starten, Sperre „ganz
//! offline“, Suche im Index, Hinzufügen aus dem Zwischenspeicher. Reine Funktionen ohne Tauri; die
//! Befehle in `lib.rs` holen nur Ordner und Sitzung und reichen weiter. Netz gibt es hier nicht: Nur
//! der Holer (eigenes Programm) verbindet sich.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tftn_core::Projekt;
use tftn_einstellungen::Rechner;
use tftn_pakete::{Index, PaketSpec, Uebernahme, Zwischenspeicher, uebernehmen};
use tftn_sprachen::tr;

/// So lange wartet die App auf den Holer (er selbst beendet sich nach 600 s).
pub const HOLER_FRIST: Duration = Duration::from_secs(660);

/// Höchstens so viele Treffer zeigt die Suche (Festlegung 6).
pub const TREFFER_MAX: usize = 50;

/// „Ganz offline“: kein Start des Holers (Entwurf 8.2).
pub fn darf_holen(rechner: &Rechner) -> Result<(), String> {
    if rechner.ganz_offline {
        Err(tr!(
            "„Ganz offline“ ist eingeschaltet: TFTN holt nichts aus dem Netz. Nutzbar sind nur Pakete im Zwischenspeicher."
        ))
    } else {
        Ok(())
    }
}

/// `TFTN_HOLER`, sonst `tftn-holer` neben dem laufenden Programm (Sidecar bzw. `target/debug`).
pub fn holer_finden() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("TFTN_HOLER") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(tr!(
                "TFTN_HOLER zeigt auf {pfad}, dort liegt kein Programm",
                pfad = p.display()
            ))
        };
    }
    let neben = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .map(|o| o.join(format!("tftn-holer{}", std::env::consts::EXE_SUFFIX)));
    match neben {
        Some(p) if p.is_file() => Ok(p),
        _ => Err(tr!(
            "Das Hilfsprogramm tftn-holer fehlt neben TFTN (Entwicklung: cargo build -p tftn-holer oder TFTN_HOLER setzen)."
        )),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexZahl {
    pub pakete: usize,
    pub uebersprungen: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeholtJs {
    pub spec: String,
    pub sha256: String,
    #[allow(dead_code)]
    pub neu: bool,
}

/// Die JSON-Zeile des Holers (PLAN-M2 AP9 Task 4).
#[derive(Debug, Deserialize)]
pub struct HolerAntwort {
    pub ok: bool,
    pub fehler: Option<String>,
    pub index: Option<IndexZahl>,
    #[serde(default)]
    pub pakete: Vec<GeholtJs>,
    #[serde(default)]
    pub warnungen: Vec<String>,
    pub versionen: Option<VersionenJs>,
    /// `tftn-holer doi`: die erkannte DOI und das BibTeX dazu.
    pub doi: Option<String>,
    pub bibtex: Option<String>,
}

/// Antwort von `tftn-holer versionen`: die neuesten Release-Tags.
#[derive(Debug, Default, Deserialize)]
pub struct VersionenJs {
    pub typst: Option<String>,
    pub tftn: Option<String>,
    #[serde(default)]
    pub fehler: Vec<String>,
}

/// Wo neue TFTN-Versionen zum Herunterladen liegen.
pub const TFTN_RELEASES: &str = "https://github.com/DocAtPrompt/tftn/releases/latest";

#[derive(Debug, Serialize, PartialEq)]
pub struct UpdatesJs {
    pub tftn_aktuell: String,
    pub tftn_neu: Option<String>,
    pub tftn_link: &'static str,
    pub typst_aktuell: String,
    pub typst_neu: Option<String>,
    pub hinweise: Vec<String>,
}

/// `tag` (etwa `v0.16.0`), wenn es eine höhere Version als `aktuell` ist.
fn neuer(tag: Option<&str>, aktuell: &str) -> Option<String> {
    let t = tag?.trim().trim_start_matches('v');
    let neu = semver::Version::parse(t).ok()?;
    let alt = semver::Version::parse(aktuell).ok()?;
    (neu > alt).then(|| t.to_string())
}

pub fn updates_js(v: &VersionenJs, tftn: &str, typst: &str) -> UpdatesJs {
    UpdatesJs {
        tftn_aktuell: tftn.into(),
        tftn_neu: neuer(v.tftn.as_deref(), tftn),
        tftn_link: TFTN_RELEASES,
        typst_aktuell: typst.into(),
        typst_neu: neuer(v.typst.as_deref(), typst),
        hinweise: v.fehler.clone(),
    }
}

/// Erste Zeile von stdout; `Err` mit der Meldung des Holers, wenn er nicht `ok` meldet.
pub fn antwort_lesen(stdout: &[u8], status: Option<i32>) -> Result<HolerAntwort, String> {
    let text = String::from_utf8_lossy(stdout);
    let zeile = text.lines().next().unwrap_or("");
    let Ok(a) = serde_json::from_str::<HolerAntwort>(zeile) else {
        return Err(tr!(
            "Der Holer hat keine Antwort gegeben (Exit {status}).",
            status = format!("{status:?}")
        ));
    };
    if a.ok {
        return Ok(a);
    }
    let fehler = a.fehler.unwrap_or_else(|| tr!("unbekannter Fehler"));
    if status == Some(2) {
        Err(tr!("falscher Aufruf des Holers: {fehler}", fehler))
    } else {
        Err(fehler)
    }
}

/// Setzt `laeuft` zurück, wenn der Lauf endet (auch bei Fehlern).
struct Lauf<'a>(&'a AtomicBool);

impl Drop for Lauf<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Startet den Holer, liest seine Antwort und beendet ihn nach `frist`. Läuft schon einer, wird
/// nichts gestartet.
pub fn holer_aufrufen(
    programm: &Path,
    argumente: &[String],
    frist: Duration,
    laeuft: &AtomicBool,
) -> Result<HolerAntwort, String> {
    if laeuft.swap(true, Ordering::SeqCst) {
        return Err(tr!("Es wird schon geholt – bitte warten."));
    }
    let _lauf = Lauf(laeuft);
    let mut befehl = Command::new(programm);
    befehl
        .args(argumente)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Der Holer meldet in der Sprache der Oberfläche; er liest die Sprachdatei selbst.
    if let Some(pfad) = tftn_sprachen::aktuell().and_then(|k| k.pfad.clone()) {
        befehl.env("TFTN_SPRACHDATEI", pfad);
    }
    // Unter Windows kein Konsolenfenster für das Konsolenprogramm (Abschlussprüfung AP9):
    // CREATE_NO_WINDOW.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        befehl.creation_flags(0x0800_0000);
    }
    let mut kind = befehl.spawn().map_err(|e| {
        tr!(
            "tftn-holer ({pfad}) startet nicht: {fehler}",
            pfad = programm.display(),
            fehler = e
        )
    })?;
    let mut stdout = kind.stdout.take().expect("stdout ist umgeleitet");
    let leser = std::thread::spawn(move || {
        let mut daten = Vec::new();
        let _ = stdout.read_to_end(&mut daten);
        daten
    });
    let start = Instant::now();
    let status = loop {
        match kind.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None if start.elapsed() > frist => {
                let _ = kind.kill();
                let _ = kind.wait();
                return Err(tr!(
                    "Frist von {sekunden} s abgelaufen – der Holer wurde beendet.",
                    sekunden = frist.as_secs()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    let daten = leser.join().unwrap_or_default();
    antwort_lesen(&daten, status.code())
}

/// `--erwartet <spec>=sha256:<hex>` für jede Prüfsumme aus `[pakete]` in `tftn.toml`.
pub fn erwartet_argumente(projekt: &Projekt) -> Vec<String> {
    projekt
        .konfig
        .pakete
        .iter()
        .flat_map(|(s, sha)| ["--erwartet".to_string(), format!("{s}={sha}")])
        .collect()
}

/// Liegt `spec` samt allen Importen gültig im Zwischenspeicher?
fn vollstaendig(zs: &Zwischenspeicher, spec: &PaketSpec) -> bool {
    zs.abschluss(spec).is_ok_and(|(_, fehlt)| fehlt.is_empty())
}

/// Pakete mit ihrer SHA-256, geprüft im Zwischenspeicher.
pub type Geholte = Vec<(PaketSpec, String)>;

/// Paket samt Abhängigkeiten bereitstellen: aus dem Zwischenspeicher, sonst über `holer` (nur ohne
/// „ganz offline“). Warnungen kommen vom Holer.
pub fn holen_kern(
    zs: &Zwischenspeicher,
    rechner: &Rechner,
    spec: &PaketSpec,
    holer: impl Fn() -> Result<HolerAntwort, String>,
) -> Result<(Geholte, Vec<String>), String> {
    let (da, fehlt) = zs.abschluss(spec).map_err(|e| e.to_string())?;
    if fehlt.is_empty() {
        return Ok((da, Vec::new()));
    }
    darf_holen(rechner).map_err(|e| {
        let namen: Vec<String> = fehlt.iter().map(ToString::to_string).collect();
        tr!(
            "{fehler} Es fehlt: {namen}.",
            fehler = e,
            namen = namen.join(", ")
        )
    })?;
    let a = holer()?;
    let pakete = a
        .pakete
        .iter()
        .map(|g| Ok((PaketSpec::lesen(&g.spec)?, g.sha256.clone())))
        .collect::<Result<Vec<_>, tftn_pakete::PaketFehler>>()
        .map_err(|e| e.to_string())?;
    Ok((pakete, a.warnungen))
}

/// Schritte 2–3 von `paket_hinzufuegen`: bereitstellen, dann ins Projekt übernehmen.
pub fn hinzufuegen_kern(
    projekt: &Projekt,
    zs: &Zwischenspeicher,
    rechner: &Rechner,
    spec: &PaketSpec,
    holer: impl Fn() -> Result<HolerAntwort, String>,
) -> Result<(Uebernahme, Vec<String>), String> {
    let (pakete, warnungen) = holen_kern(zs, rechner, spec, holer)?;
    let u = uebernehmen(&projekt.wurzel, zs, &pakete).map_err(|e| e.to_string())?;
    Ok((u, warnungen))
}

/// Die neue Paketangabe aus den geholten Paketen.
fn neue_angabe<'a>(
    pakete: &'a [(PaketSpec, String)],
    alt: &PaketSpec,
) -> Result<&'a PaketSpec, String> {
    pakete
        .iter()
        .map(|(s, _)| s)
        .find(|s| s.name == alt.name && s.version != alt.version)
        .ok_or_else(|| tr!("Keine neue Version von {paket} geholt.", paket = alt.name))
}

/// Probe des Versionswechsels in einer Temp-Kopie: neue Version übernehmen, Importe umschreiben,
/// beide Fassungen übersetzen und vergleichen (PLAN-M2 AP10, Festlegung 10).
pub fn wechsel_vergleichen_in(
    projekt: &Projekt,
    zs: &Zwischenspeicher,
    pakete: &[(PaketSpec, String)],
    alt: &PaketSpec,
    basis: &Path,
) -> Result<crate::typen::VergleichJs, String> {
    use tftn_manifest::aktualisieren::{Aktualisierung, importstellen, stellen_umschreiben};
    let neu = neue_angabe(pakete, alt)?;
    crate::typen::in_probe(projekt, basis, |kopie| {
        uebernehmen(&kopie.wurzel, zs, pakete).map_err(|e| e.to_string())?;
        let a = Aktualisierung {
            alt: alt.to_string(),
            neu: neu.to_string(),
            stellen: importstellen(&kopie.wurzel, &alt.to_string()).map_err(|e| e.to_string())?,
            schriften: Vec::new(),
        };
        stellen_umschreiben(&kopie.wurzel, &a, |_| true).map_err(|e| e.to_string())?;
        let kopie = Projekt::oeffnen(&kopie.wurzel).map_err(|e| e.to_string())?;
        let v = tftn_core::vergleich::seiten_vergleichen(projekt, &kopie, 72.0, 12)?;
        let stellen = a.stellen.iter().map(|s| s.datei.clone()).collect();
        Ok(crate::typen::vergleich_js(a.alt, a.neu, v, stellen))
    })
}

/// Übernimmt die neue Version, nachdem die Probe ohne Fehler übersetzt hat. Nebendateien werden
/// umgeschrieben; für die Hauptdatei kommt ein Patch zurück (ein Undo-Schritt). Die alte Version
/// bleibt liegen, damit Undo nicht ins Leere läuft. `rev`/`aktuell`: Stand der Anfrage und des Editors.
#[allow(clippy::too_many_arguments)]
pub fn wechsel_uebernehmen(
    projekt: &Projekt,
    zs: &Zwischenspeicher,
    pakete: &[(PaketSpec, String)],
    alt: &PaketSpec,
    haupt: &str,
    quelle: &typst::syntax::Source,
    aktuell: &dyn Fn() -> u64,
    rev: u64,
) -> Result<Option<tftn_edit::Patch>, String> {
    use tftn_manifest::aktualisieren::{
        Aktualisierung, importstellen, stellen_in_text, stellen_umschreiben,
    };
    crate::helfer::rev_pruefen(aktuell(), rev)?;
    let neu = neue_angabe(pakete, alt)?;
    let v = wechsel_vergleichen_in(projekt, zs, pakete, alt, &std::env::temp_dir())?;
    if !v.fehler.is_empty() {
        return Err(tr!(
            "{neu} übersetzt nicht – siehe Vergleich. Nichts geändert.",
            neu
        ));
    }
    // Die Probe dauert Sekunden; wer inzwischen getippt hat, bekommt nichts Halbes.
    crate::helfer::rev_pruefen(aktuell(), rev)?;
    uebernehmen(&projekt.wurzel, zs, pakete).map_err(|e| e.to_string())?;
    let a = Aktualisierung {
        alt: alt.to_string(),
        neu: neu.to_string(),
        stellen: importstellen(&projekt.wurzel, &alt.to_string()).map_err(|e| e.to_string())?,
        schriften: Vec::new(),
    };
    stellen_umschreiben(&projekt.wurzel, &a, |d| d != haupt).map_err(|e| e.to_string())?;
    let stellen = stellen_in_text(quelle.text(), &a.alt);
    Ok(crate::typen::stellen_patch(quelle.text(), &stellen, &a.neu))
}

/// Ein Treffer der Suche im Fach „Pakete“.
#[derive(Debug, Serialize)]
pub struct TrefferJs {
    pub name: String,
    pub version: String,
    pub beschreibung: Option<String>,
    pub lizenz: Option<String>,
    pub im_projekt: bool,
    /// samt allen Importen
    pub im_zwischenspeicher: bool,
    /// eine Paketbeschreibung des Katalogs passt
    pub helfer: bool,
}

pub fn treffer_js(
    index: &Index,
    anfrage: &str,
    projekt: &Projekt,
    zs: &Zwischenspeicher,
    helfer: &dyn Fn(&str, &semver::Version) -> bool,
) -> Vec<TrefferJs> {
    let typst = semver::Version::parse(&projekt.konfig.typst)
        .unwrap_or_else(|_| semver::Version::new(0, 15, 1));
    index
        .suchen(anfrage, &typst, TREFFER_MAX)
        .into_iter()
        .map(|e| {
            let spec = PaketSpec {
                name: e.name.clone(),
                version: e.version.clone(),
            };
            TrefferJs {
                im_projekt: projekt
                    .wurzel
                    .join("packages/preview")
                    .join(&e.name)
                    .join(e.version.to_string())
                    .is_dir(),
                im_zwischenspeicher: vollstaendig(zs, &spec),
                helfer: helfer(&e.name, &e.version),
                name: e.name,
                version: e.version.to_string(),
                beschreibung: e.description,
                lizenz: e.license,
            }
        })
        .collect()
}

/// Unix-Sekunden → `JJJJ-MM-TT` (UTC; Verfahren nach H. Hinnant).
pub fn datum(sekunden: i64) -> String {
    let z = sekunden.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tag = doy - (153 * mp + 2) / 5 + 1;
    let monat = if mp < 10 { mp + 3 } else { mp - 9 };
    let jahr = yoe + era * 400 + i64::from(monat <= 2);
    format!("{jahr:04}-{monat:02}-{tag:02}")
}

/// Eigener Ordner (nur wenn es ihn gibt) vor dem Bündel (PLAN-M2 AP10, Festlegung 8).
pub fn beschreibungs_ordner(buendel: &Path, einstellungsordner: &Path) -> Vec<PathBuf> {
    let eigen = einstellungsordner.join(tftn_einstellungen::BESCHREIBUNGEN);
    let mut aus = Vec::new();
    if eigen.is_dir() {
        aus.push(eigen);
    }
    aus.push(buendel.to_path_buf());
    aus
}

/// Mitgelieferte Beschreibungen: im Entwicklungsbau `pakete/` im Repo, sonst im App-Bündel.
pub fn beschreibungen_buendel(resource_dir: Option<PathBuf>) -> PathBuf {
    match resource_dir {
        Some(r) if !cfg!(debug_assertions) => r.join("beschreibungen"),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../pakete"),
    }
}

/// Einträge fürs Bausteinmenü: je holbarem Paket Id und Titel (Festlegung 9).
pub fn holbar_eintraege(k: &tftn_katalog::Katalog) -> Vec<(String, String)> {
    k.holbar()
        .iter()
        .map(|h| {
            (
                format!("holen/{}", h.spec),
                tr!("{titel} (Paket holen)", titel = h.titel),
            )
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct HolbarJs {
    pub paket: String,
    pub spec: String,
    pub titel: String,
    pub bausteine: Vec<String>,
    /// samt allen Importen
    pub im_zwischenspeicher: bool,
}

pub fn holbar_js(k: &tftn_katalog::Katalog, zs: &Zwischenspeicher) -> Vec<HolbarJs> {
    k.holbar()
        .iter()
        .map(|h| HolbarJs {
            paket: h.paket.clone(),
            spec: h.spec.clone(),
            titel: h.titel.clone(),
            bausteine: h.bausteine.clone(),
            im_zwischenspeicher: PaketSpec::lesen(&h.spec).is_ok_and(|s| vollstaendig(zs, &s)),
        })
        .collect()
}

/// Ein Paket im Projekt für das Fach „Pakete“.
#[derive(Debug, Clone, Serialize)]
pub struct ProjektPaketJs {
    #[serde(flatten)]
    pub paket: tftn_pakete::ProjektPaket,
    /// Eine Beschreibung passt zu dieser Version.
    pub helfer: bool,
    /// Es gibt eine Beschreibung, gleich für welche Version.
    pub beschreibung: bool,
    /// Eine `.typ`-Datei des Projekts oder ein anderes Paket importiert es.
    pub benutzt: bool,
    /// Eine `.typ`-Datei des Projekts importiert es direkt.
    pub direkt: bool,
    /// Höhere passende Version laut Index; nur bei direktem Import im Projekt.
    pub neuer: Option<String>,
}

pub fn im_projekt_js(
    wurzel: &Path,
    k: &tftn_katalog::Katalog,
    index: Option<&Index>,
    typst: &semver::Version,
) -> Vec<ProjektPaketJs> {
    let liste = tftn_pakete::im_projekt(wurzel);
    let direkt = tftn_pakete::projektimporte(wurzel);
    let mut von_paketen = std::collections::BTreeSet::new();
    for p in &liste {
        let ordner = wurzel
            .join("packages/preview")
            .join(&p.name)
            .join(&p.version);
        if let Ok((i, _)) = tftn_pakete::importe(&ordner) {
            von_paketen.extend(i);
        }
    }
    liste
        .into_iter()
        .map(|paket| {
            let spec = PaketSpec::lesen(&paket.spec).ok();
            let version = spec.as_ref().map(|s| s.version.clone());
            let ist_direkt = spec.as_ref().is_some_and(|s| direkt.contains(s));
            ProjektPaketJs {
                helfer: version
                    .as_ref()
                    .is_some_and(|v| k.hat_helfer(&paket.name, v)),
                beschreibung: k.hat_beschreibung(&paket.name),
                direkt: ist_direkt,
                benutzt: ist_direkt || spec.as_ref().is_some_and(|s| von_paketen.contains(s)),
                neuer: index
                    .filter(|_| ist_direkt)
                    .and_then(|i| i.neueste(&paket.name, typst))
                    .filter(|e| version.as_ref().is_some_and(|v| e.version > *v))
                    .map(|e| e.version.to_string()),
                paket,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    use tftn_pakete::Grenzen;

    fn testordner(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-ap9-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        o
    }

    fn kopieren(von: &Path, nach: &Path) {
        std::fs::create_dir_all(nach).unwrap();
        for e in std::fs::read_dir(von).unwrap() {
            let e = e.unwrap();
            let ziel = nach.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                kopieren(&e.path(), &ziel);
            } else {
                std::fs::copy(e.path(), ziel).unwrap();
            }
        }
    }

    /// .tar.gz aus (Pfad, Inhalt).
    fn archiv(dateien: &[(&str, &str)]) -> Vec<u8> {
        let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        for (pfad, inhalt) in dateien {
            let mut h = tar::Header::new_gnu();
            h.set_path(pfad).unwrap();
            h.set_size(inhalt.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            tar.append(&h, inhalt.as_bytes()).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    fn toml(name: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n")
    }

    fn probe() -> PaketSpec {
        PaketSpec::lesen("@preview/tftn-probe:0.1.0").unwrap()
    }

    /// Projekt (Kopie des Testartikels) und Zwischenspeicher; mit `basis` liegt der Abschluss vollständig.
    fn aufbau(name: &str, basis: bool) -> (Projekt, Zwischenspeicher) {
        let o = testordner(name);
        kopieren(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel"),
            &o.join("projekt"),
        );
        let zs = Zwischenspeicher {
            wurzel: o.join("zs"),
        };
        let g = Grenzen::default();
        zs.ablegen(
            &probe(),
            &archiv(&[
                ("typst.toml", &toml("tftn-probe")),
                (
                    "lib.typ",
                    "#import \"@preview/tftn-probe-basis:0.1.0\": *\n",
                ),
            ]),
            &g,
        )
        .unwrap();
        if basis {
            zs.ablegen(
                &PaketSpec::lesen("@preview/tftn-probe-basis:0.1.0").unwrap(),
                &archiv(&[("typst.toml", &toml("tftn-probe-basis")), ("lib.typ", "")]),
                &g,
            )
            .unwrap();
        }
        (Projekt::oeffnen(&o.join("projekt")).unwrap(), zs)
    }

    fn offline() -> Rechner {
        Rechner {
            ordner: None,
            ganz_offline: true,
            sprache: None,
        }
    }

    #[test]
    fn updates_vergleichen() {
        let v = VersionenJs {
            typst: Some("v0.16.0".into()),
            tftn: Some("v0.1.0".into()),
            fehler: vec![],
        };
        let u = updates_js(&v, "0.1.0", "0.15.1");
        assert_eq!(u.typst_neu.as_deref(), Some("0.16.0"));
        assert_eq!(u.tftn_neu, None, "gleich ist nicht neuer");
        let v = VersionenJs {
            typst: Some("v0.15.0".into()),
            tftn: Some("kaputt".into()),
            fehler: vec!["TFTN: x".into()],
        };
        let u = updates_js(&v, "0.1.0", "0.15.1");
        assert_eq!(
            (u.typst_neu, u.tftn_neu),
            (None, None),
            "älter oder unlesbar"
        );
        assert_eq!(u.hinweise, ["TFTN: x"]);
    }

    #[test]
    fn versionen_antwort_lesen() {
        let a = antwort_lesen(
            br#"{"ok": true, "versionen": {"typst": "v0.15.1", "tftn": null, "fehler": ["TFTN: weg"]}}"#,
            Some(0),
        )
        .unwrap();
        let v = a.versionen.unwrap();
        assert_eq!(v.typst.as_deref(), Some("v0.15.1"));
        assert_eq!(v.fehler.len(), 1);
    }

    #[test]
    fn offline_sperrt() {
        let e = darf_holen(&offline()).unwrap_err();
        assert!(e.to_lowercase().contains("ganz offline"), "{e}");
        assert!(darf_holen(&Rechner::default()).is_ok());
    }

    #[test]
    fn offline_aus_zwischenspeicher() {
        let (projekt, zs) = aufbau("offline-voll", true);
        let aufrufe = Cell::new(0);
        let holer = || {
            aufrufe.set(aufrufe.get() + 1);
            Err::<HolerAntwort, String>("darf nicht laufen".into())
        };
        let (u, _) = hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), holer).unwrap();
        assert_eq!(u.kopiert.len(), 2);
        assert_eq!(aufrufe.get(), 0);
        assert!(
            projekt
                .wurzel
                .join("packages/preview/tftn-probe-basis/0.1.0")
                .is_dir()
        );

        let (projekt, zs) = aufbau("offline-halb", false);
        let e = hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), holer).unwrap_err();
        assert!(e.to_lowercase().contains("ganz offline"), "{e}");
        assert!(e.contains("tftn-probe-basis"), "{e}");
        assert_eq!(aufrufe.get(), 0);
        assert!(!projekt.wurzel.join("packages/preview").exists());
    }

    #[test]
    fn mit_holer_wird_uebernommen() {
        let (projekt, zs) = aufbau("mit-holer", false);
        let basis = PaketSpec::lesen("@preview/tftn-probe-basis:0.1.0").unwrap();
        let holer = || {
            // Wie der echte Holer: legt ab und meldet beide.
            let sha = zs
                .ablegen(
                    &basis,
                    &archiv(&[("typst.toml", &toml("tftn-probe-basis")), ("lib.typ", "")]),
                    &Grenzen::default(),
                )
                .unwrap();
            antwort_lesen(
                format!(
                    r#"{{"ok":true,"pakete":[{{"spec":"@preview/tftn-probe:0.1.0","sha256":"{}","neu":false}},{{"spec":"@preview/tftn-probe-basis:0.1.0","sha256":"{sha}","neu":true}}],"warnungen":["w"]}}"#,
                    zs.sha256(&probe()).unwrap()
                )
                .as_bytes(),
                Some(0),
            )
        };
        let (u, warnungen) =
            hinzufuegen_kern(&projekt, &zs, &Rechner::default(), &probe(), holer).unwrap();
        assert_eq!(u.kopiert.len(), 2);
        assert_eq!(warnungen, vec!["w".to_string()]);
    }

    #[test]
    fn fehler_laesst_projekt_unberuehrt() {
        let (projekt, zs) = aufbau("fehler", false);
        let vorher = std::fs::read(projekt.wurzel.join("tftn.toml")).unwrap();
        let holer = || antwort_lesen(br#"{"ok":false,"fehler":"Paket nicht gefunden"}"#, Some(1));
        let e = hinzufuegen_kern(&projekt, &zs, &Rechner::default(), &probe(), holer).unwrap_err();
        assert!(e.contains("nicht gefunden"), "{e}");
        assert!(!projekt.wurzel.join("packages/preview").exists());
        assert_eq!(
            std::fs::read(projekt.wurzel.join("tftn.toml")).unwrap(),
            vorher
        );
    }

    #[test]
    fn antwort_lesen_faelle() {
        let a = antwort_lesen(
            br#"{"ok":true,"index":{"pakete":3,"uebersprungen":1}}"#,
            Some(0),
        )
        .unwrap();
        assert_eq!(a.index.unwrap().pakete, 3);
        assert_eq!(
            antwort_lesen(br#"{"ok":false,"fehler":"kaputt"}"#, Some(1)).unwrap_err(),
            "kaputt"
        );
        let e = antwort_lesen(b"", Some(1)).unwrap_err();
        assert!(e.contains("Holer"), "{e}");
        let e = antwort_lesen(br#"{"ok":false,"fehler":"kein Befehl"}"#, Some(2)).unwrap_err();
        assert!(e.contains("falscher Aufruf"), "{e}");
    }

    #[test]
    fn nur_ein_lauf() {
        let laeuft = AtomicBool::new(true);
        let e = holer_aufrufen(
            Path::new("/gibt/es/nicht"),
            &[],
            Duration::from_secs(1),
            &laeuft,
        )
        .unwrap_err();
        assert!(e.contains("schon geholt"), "{e}");
        assert!(laeuft.load(Ordering::SeqCst), "fremder Lauf bleibt gesetzt");

        let laeuft = AtomicBool::new(false);
        assert!(
            holer_aufrufen(
                Path::new("/gibt/es/nicht"),
                &[],
                Duration::from_secs(1),
                &laeuft
            )
            .is_err()
        );
        assert!(!laeuft.load(Ordering::SeqCst), "nach Fehler zurückgesetzt");
    }

    #[cfg(unix)]
    #[test]
    fn frist_beendet() {
        let laeuft = AtomicBool::new(false);
        let start = Instant::now();
        let e = holer_aufrufen(
            Path::new("/bin/sleep"),
            &["30".to_string()],
            Duration::from_secs(1),
            &laeuft,
        )
        .unwrap_err();
        assert!(e.contains("Frist"), "{e}");
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(!laeuft.load(Ordering::SeqCst));
    }

    #[test]
    fn erwartet_aus_tftn_toml() {
        let (projekt, _) = aufbau("erwartet", true);
        let pfad = projekt.wurzel.join("tftn.toml");
        let mut text = std::fs::read_to_string(&pfad).unwrap();
        text.push_str("\n[pakete]\n\"@preview/b:1.0.0\" = \"sha256:bb\"\n\"@preview/a:1.0.0\" = \"sha256:aa\"\n");
        std::fs::write(&pfad, text).unwrap();
        let projekt = Projekt::oeffnen(&projekt.wurzel).unwrap();
        assert_eq!(
            erwartet_argumente(&projekt),
            vec![
                "--erwartet",
                "@preview/a:1.0.0=sha256:aa",
                "--erwartet",
                "@preview/b:1.0.0=sha256:bb"
            ]
        );
    }

    #[test]
    fn treffer_marken() {
        let (projekt, zs) = aufbau("treffer", false);
        let index = Index::lesen(
            br#"[{"name":"tftn-probe","version":"0.1.0","description":"Probe","license":"MIT-0"},
                 {"name":"tftn-probe-basis","version":"0.1.0"}]"#,
        )
        .unwrap();
        let t = treffer_js(&index, "tftn-probe", &projekt, &zs, &|n, _| {
            n == "tftn-probe"
        });
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].name, "tftn-probe");
        assert_eq!(t[0].lizenz.as_deref(), Some("MIT-0"));
        assert!(!t[0].im_zwischenspeicher, "Abschluss unvollständig");
        assert!(!t[0].im_projekt);
        assert!(t[0].helfer);
        assert!(!t[1].helfer);

        let (projekt, zs) = aufbau("treffer-voll", true);
        hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), || Err("nein".into())).unwrap();
        let t = treffer_js(&index, "tftn-probe", &projekt, &zs, &|_, _| false);
        assert!(t[0].im_zwischenspeicher);
        assert!(t[0].im_projekt);
    }

    #[test]
    fn beschreibungs_ordner_reihenfolge() {
        let basis = std::env::temp_dir().join(format!("tftn-b-ordner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&basis);
        std::fs::create_dir_all(&basis).unwrap();
        let buendel = Path::new("/buendel");
        assert_eq!(
            beschreibungs_ordner(buendel, &basis),
            vec![buendel.to_path_buf()]
        );
        std::fs::create_dir_all(basis.join("beschreibungen")).unwrap();
        assert_eq!(
            beschreibungs_ordner(buendel, &basis),
            vec![basis.join("beschreibungen"), buendel.to_path_buf()]
        );
        std::fs::remove_dir_all(&basis).unwrap();
    }

    fn demo_beschreibung(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-b-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        std::fs::write(
            o.join("demo.toml"),
            "paket = \"tftn-probe\"\nversionen = \"0.1\"\nempfohlen = \"0.1.0\"\ntitel = \"Probe\"\nimport = '#import \"@preview/tftn-probe:{version}\"'\n\n[[baustein]]\nid = \"b\"\nfunktion = \"tftn-probe.b\"\ntitel = \"B\"\neinfuegen = \"#tftn-probe.b()\"\n",
        )
        .unwrap();
        o
    }

    #[test]
    fn eintraege_holen_js() {
        let (projekt, zs) = aufbau("holen-js", true);
        let ordner = demo_beschreibung("holen-js");
        let bauen = |p: &Projekt| {
            tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
                projekt: Some(p),
                beschreibungen: std::slice::from_ref(&ordner),
            })
            .0
        };
        let k = bauen(&projekt);
        assert_eq!(
            holbar_eintraege(&k),
            vec![(
                "holen/@preview/tftn-probe:0.1.0".to_string(),
                "Probe (Paket holen)".to_string()
            )]
        );
        let h = holbar_js(&k, &zs);
        assert_eq!(h[0].spec, "@preview/tftn-probe:0.1.0");
        assert_eq!(h[0].bausteine, vec!["B".to_string()]);
        assert!(h[0].im_zwischenspeicher);
        hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), || Err("nein".into())).unwrap();
        let k = bauen(&projekt);
        assert!(holbar_eintraege(&k).is_empty());
        std::fs::remove_dir_all(&ordner).unwrap();
    }

    #[test]
    fn treffer_mit_helfer() {
        let (projekt, zs) = aufbau("treffer-helfer", true);
        hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), || Err("nein".into())).unwrap();
        let ordner = demo_beschreibung("treffer-helfer");
        let (k, _) = tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
            projekt: Some(&projekt),
            beschreibungen: std::slice::from_ref(&ordner),
        });
        let index = Index::lesen(
            br#"[{"name":"tftn-probe","version":"0.1.0"},{"name":"tftn-probe","version":"0.2.0"}]"#,
        )
        .unwrap();
        let t = treffer_js(&index, "tftn-probe", &projekt, &zs, &|n, v| {
            k.hat_helfer(n, v)
        });
        assert_eq!(t[0].version, "0.2.0");
        assert!(!t[0].helfer, "außerhalb des Bereichs");
        let j = im_projekt_js(&projekt.wurzel, &k, None, &TYPST);
        let p = j.iter().find(|p| p.paket.name == "tftn-probe").unwrap();
        assert!(p.helfer && p.beschreibung);
        let b = j
            .iter()
            .find(|p| p.paket.name == "tftn-probe-basis")
            .unwrap();
        assert!(!b.helfer && !b.beschreibung);
        std::fs::remove_dir_all(&ordner).unwrap();
    }

    const TYPST: semver::Version = semver::Version::new(0, 15, 1);

    fn vg_ablegen(zs: &Zwischenspeicher, version: &str, koerper: &str) -> (PaketSpec, String) {
        let spec = PaketSpec::neu("vg", version).unwrap();
        let toml = format!(
            "[package]\nname = \"vg\"\nversion = \"{version}\"\nentrypoint = \"lib.typ\"\n"
        );
        let lib = format!("#let probe(x) = {koerper}\n");
        zs.ablegen(
            &spec,
            &archiv(&[("typst.toml", &toml), ("lib.typ", &lib)]),
            &Grenzen::default(),
        )
        .unwrap();
        let sha = zs.sha256(&spec).unwrap();
        (spec, sha)
    }

    const IMPORT_ALT: &str = "@preview/vg:0.1.0";

    /// Projekt mit vg 0.1.0: zwei Importe in der Hauptdatei, einer in `kapitel/a.typ`.
    fn wechsel_aufbau(
        name: &str,
        koerper_neu: &str,
    ) -> (
        Projekt,
        Zwischenspeicher,
        Vec<(PaketSpec, String)>,
        PaketSpec,
    ) {
        let (projekt, zs) = aufbau(name, false);
        let alt = vg_ablegen(&zs, "0.1.0", "[Alt #x]");
        let neu = vg_ablegen(&zs, "0.2.0", koerper_neu);
        uebernehmen(&projekt.wurzel, &zs, std::slice::from_ref(&alt)).unwrap();
        let haupt = projekt.wurzel.join("main.typ");
        let mut text = std::fs::read_to_string(&haupt).unwrap();
        text.push_str(&format!(
            "\n#import \"{IMPORT_ALT}\": probe\n#import \"{IMPORT_ALT}\" as zweit\n#probe[Text]\n"
        ));
        std::fs::write(&haupt, text).unwrap();
        std::fs::create_dir_all(projekt.wurzel.join("kapitel")).unwrap();
        std::fs::write(
            projekt.wurzel.join("kapitel/a.typ"),
            format!("#import \"{IMPORT_ALT}\": probe\n"),
        )
        .unwrap();
        let projekt = Projekt::oeffnen(&projekt.wurzel).unwrap();
        (projekt, zs, vec![neu], alt.0)
    }

    /// Alles, was ein Wechsel anfassen dürfte.
    fn wechsel_stand(p: &Projekt) -> (String, String, String, Vec<String>) {
        let lies = |d: &str| std::fs::read_to_string(p.wurzel.join(d)).unwrap();
        (
            lies("tftn.toml"),
            lies("main.typ"),
            lies("kapitel/a.typ"),
            tftn_pakete::im_projekt(&p.wurzel)
                .into_iter()
                .map(|x| x.spec)
                .collect(),
        )
    }

    fn haupt_quelle(p: &Projekt) -> typst::syntax::Source {
        typst::syntax::Source::detached(std::fs::read_to_string(p.wurzel.join("main.typ")).unwrap())
    }

    #[test]
    fn wechsel_probe() {
        let (p, zs, neu, alt) = wechsel_aufbau("wechsel-probe", "[Neu #x]");
        let vorher = wechsel_stand(&p);
        let basis = testordner("wechsel-probe-basis");
        let v = wechsel_vergleichen_in(&p, &zs, &neu, &alt, &basis).unwrap();
        assert_eq!(v.fehler, Vec::<String>::new());
        assert_eq!(v.alt, IMPORT_ALT);
        assert_eq!(v.neu, "@preview/vg:0.2.0");
        assert!(!v.geaendert.is_empty());
        assert_eq!(wechsel_stand(&p), vorher);
        assert_eq!(std::fs::read_dir(&basis).unwrap().count(), 0);
    }

    #[test]
    fn wechsel_uebernehmen_ok() {
        let (p, zs, neu, alt) = wechsel_aufbau("wechsel-ok", "[Neu #x]");
        let q = haupt_quelle(&p);
        let patch = wechsel_uebernehmen(&p, &zs, &neu, &alt, "main.typ", &q, &|| 7, 7)
            .unwrap()
            .expect("Patch");
        let text = tftn_edit::anwenden(q.text(), &patch);
        assert_eq!(text.matches("@preview/vg:0.2.0").count(), 2);
        assert!(!text.contains(IMPORT_ALT));
        // Die Hauptdatei auf der Platte schreibt nur der Editor.
        assert!(
            std::fs::read_to_string(p.wurzel.join("main.typ"))
                .unwrap()
                .contains(IMPORT_ALT)
        );
        assert!(
            std::fs::read_to_string(p.wurzel.join("kapitel/a.typ"))
                .unwrap()
                .contains("@preview/vg:0.2.0")
        );
        let toml = std::fs::read_to_string(p.wurzel.join("tftn.toml")).unwrap();
        assert!(
            toml.contains(IMPORT_ALT) && toml.contains("@preview/vg:0.2.0"),
            "{toml}"
        );
        assert!(p.wurzel.join("packages/preview/vg/0.1.0").is_dir());
        assert!(p.wurzel.join("packages/preview/vg/0.2.0").is_dir());
    }

    /// Review Focus 4.
    #[test]
    fn wechsel_fehler_aendert_nichts() {
        let (p, zs, neu, alt) = wechsel_aufbau("wechsel-fehler", "nichtda");
        let vorher = wechsel_stand(&p);
        let q = haupt_quelle(&p);
        let f = wechsel_uebernehmen(&p, &zs, &neu, &alt, "main.typ", &q, &|| 1, 1).unwrap_err();
        assert!(f.contains("übersetzt nicht"), "{f}");
        assert_eq!(wechsel_stand(&p), vorher);
    }

    /// Review Focus 4.
    #[test]
    fn wechsel_veraltet_aendert_nichts() {
        let (p, zs, neu, alt) = wechsel_aufbau("wechsel-veraltet", "[Neu #x]");
        let vorher = wechsel_stand(&p);
        let q = haupt_quelle(&p);
        let f = wechsel_uebernehmen(&p, &zs, &neu, &alt, "main.typ", &q, &|| 2, 1).unwrap_err();
        assert_eq!(f, "veraltet");
        assert_eq!(wechsel_stand(&p), vorher);
    }

    /// Abschlussprüfung AP10: Die Probe dauert Sekunden; getippt wird auch währenddessen.
    #[test]
    fn wechsel_getippt_waehrend_probe() {
        let (p, zs, neu, alt) = wechsel_aufbau("wechsel-getippt", "[Neu #x]");
        let vorher = wechsel_stand(&p);
        let q = haupt_quelle(&p);
        let aufrufe = Cell::new(0);
        let aktuell = || {
            aufrufe.set(aufrufe.get() + 1);
            if aufrufe.get() == 1 { 1 } else { 2 }
        };
        let f = wechsel_uebernehmen(&p, &zs, &neu, &alt, "main.typ", &q, &aktuell, 1).unwrap_err();
        assert_eq!(f, "veraltet");
        assert_eq!(wechsel_stand(&p), vorher);
    }

    #[test]
    fn neuer_nur_bei_direktem_import() {
        let (projekt, zs) = aufbau("neuer", true);
        hinzufuegen_kern(&projekt, &zs, &offline(), &probe(), || Err("nein".into())).unwrap();
        std::fs::write(
            projekt.wurzel.join("kapitel.typ"),
            "#import \"@preview/tftn-probe:0.1.0\"\n",
        )
        .unwrap();
        let index = Index::lesen(br#"[{"name":"tftn-probe","version":"0.2.0"},{"name":"tftn-probe-basis","version":"0.2.0"}]"#).unwrap();
        let j = im_projekt_js(
            &projekt.wurzel,
            &tftn_katalog::Katalog::default(),
            Some(&index),
            &TYPST,
        );
        let finde = |n: &str| j.iter().find(|p| p.paket.name == n).unwrap();
        assert_eq!(finde("tftn-probe").neuer.as_deref(), Some("0.2.0"));
        assert!(finde("tftn-probe").benutzt);
        assert_eq!(finde("tftn-probe-basis").neuer, None);
        assert!(
            finde("tftn-probe-basis").benutzt,
            "von tftn-probe importiert"
        );
    }

    #[test]
    fn datum_aus_sekunden() {
        assert_eq!(datum(0), "1970-01-01");
        assert_eq!(datum(1_793_836_800), "2026-11-05");
        assert_eq!(datum(951_782_400), "2000-02-29");
    }
}
