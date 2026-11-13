//! Dokumenttypen in der App (PLAN-M2 AP8): Typquellen (eigener Ordner im Tresor vor dem App-Bündel),
//! Liste für den Dialog „Neues Projekt“, Vorschaubild, Anlegen. Reine Funktionen ohne Tauri; die
//! Befehle in `lib.rs` holen nur die Ordner und reichen weiter.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tftn_core::{ProjektWorld, kompilieren};
pub use tftn_manifest::typen::{Herkunft, Typquelle};
use tftn_sprachen::tr;

/// Ein Typ im Dialog „Neues Projekt“.
#[derive(Debug, Serialize)]
pub struct TypJs {
    pub name: String,
    pub version: String,
    pub titel: String,
    pub beschreibung: Option<String>,
    pub braucht: Vec<String>,
    pub herkunft: Herkunft,
}

#[derive(Debug, Serialize)]
pub struct TypenJs {
    pub typen: Vec<TypJs>,
    pub warnungen: Vec<String>,
    /// `<Einstellungsordner>/typen` (auch wenn es ihn noch nicht gibt)
    pub eigener_ordner: String,
    /// Vorgabe für den Elternordner: Eltern des offenen Projekts
    pub vorgabe_ordner: Option<String>,
}

/// Eigener Ordner (`<einstellungsordner>/typen`, nur wenn es ihn gibt) vor dem Bündel.
pub fn quellen(buendel: &Path, einstellungsordner: &Path) -> Vec<Typquelle> {
    let eigen = einstellungsordner.join("typen");
    let mut q = Vec::new();
    if eigen.is_dir() {
        q.push(Typquelle {
            ordner: eigen,
            herkunft: Herkunft::Eigen,
        });
    }
    q.push(Typquelle {
        ordner: buendel.to_path_buf(),
        herkunft: Herkunft::Tftn,
    });
    q
}

/// Ordner der mitgelieferten Typen: im Entwicklungsbau `types/` im Repo, sonst im App-Bündel.
pub fn buendel_ordner(resource_dir: Option<PathBuf>) -> PathBuf {
    match resource_dir {
        Some(r) if !cfg!(debug_assertions) => r.join("types"),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../types"),
    }
}

/// Unter Windows reservierte Gerätenamen (auch mit Endung).
const RESERVIERT: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Projektname: ein einzelner Ordnername, auf Mac und Windows gültig – nicht leer, ohne `/`, `\`,
/// `<>:"|?*` und Steuerzeichen, nicht mit `.` beginnend, nicht mit `.` endend, kein reservierter
/// Windows-Name (sonst landete etwa `C:Artikel` außerhalb des gewählten Ordners).
pub fn name_pruefen(name: &str) -> Result<(), String> {
    match namensfehler(name) {
        None => Ok(()),
        Some(f) => Err(f.projektname()),
    }
}

/// Was an einem Projektnamen nicht stimmt (siehe `name_pruefen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Namensfehler {
    Fehlt,
    Trenner,
    Zeichen,
    PunktVorn,
    PunktHinten,
    Reserviert,
}

impl Namensfehler {
    /// Meldung im Dialog „Neues Projekt“.
    fn projektname(self) -> String {
        match self {
            Self::Fehlt => tr!("Projektname fehlt"),
            Self::Trenner => tr!("Projektname darf kein / oder \\ enthalten"),
            Self::Zeichen => tr!("Projektname darf keines der Zeichen < > : \" | ? * enthalten"),
            Self::PunktVorn => tr!("Projektname darf nicht mit einem Punkt beginnen"),
            Self::PunktHinten => tr!("Projektname darf nicht mit einem Punkt enden"),
            Self::Reserviert => tr!("Projektname ist unter Windows reserviert"),
        }
    }

    /// Meldung beim Benennen des Entwurfs.
    fn name(self) -> String {
        match self {
            Self::Fehlt => tr!("Der Name fehlt"),
            Self::Trenner => tr!("Der Name darf kein / oder \\ enthalten"),
            Self::Zeichen => tr!("Der Name darf keines der Zeichen < > : \" | ? * enthalten"),
            Self::PunktVorn => tr!("Der Name darf nicht mit einem Punkt beginnen"),
            Self::PunktHinten => tr!("Der Name darf nicht mit einem Punkt enden"),
            Self::Reserviert => tr!("Der Name ist unter Windows reserviert"),
        }
    }
}

fn namensfehler(name: &str) -> Option<Namensfehler> {
    let n = name.trim();
    let stamm = n.split('.').next().unwrap_or("").to_lowercase();
    if n.is_empty() {
        Some(Namensfehler::Fehlt)
    } else if n.contains('/') || n.contains('\\') {
        Some(Namensfehler::Trenner)
    } else if n.chars().any(|c| "<>:\"|?*".contains(c) || c.is_control()) {
        Some(Namensfehler::Zeichen)
    } else if n.starts_with('.') {
        Some(Namensfehler::PunktVorn)
    } else if n.ends_with('.') {
        Some(Namensfehler::PunktHinten)
    } else if RESERVIERT.contains(&stamm.as_str()) {
        Some(Namensfehler::Reserviert)
    } else {
        None
    }
}

/// Ordner für einen Projektwechsel: muss sich als Projekt öffnen lassen (kanonischer Pfad).
pub fn wechsel_pruefen(ordner: &Path) -> Result<PathBuf, String> {
    if !ordner.join("tftn.toml").is_file() {
        return Err(tr!(
            "{ordner} ist kein TFTN-Projekt (keine tftn.toml).",
            ordner = ordner.display()
        ));
    }
    tftn_core::Projekt::oeffnen(ordner)
        .map(|p| p.wurzel)
        .map_err(|e| {
            tr!(
                "{ordner} lässt sich nicht öffnen: {fehler}",
                ordner = ordner.display(),
                fehler = e
            )
        })
}

pub fn typen_js(quellen: &[Typquelle], eigener_ordner: &Path, vorgabe: Option<&Path>) -> TypenJs {
    let (liste, warnungen) = tftn_manifest::typen::verfuegbar(quellen);
    TypenJs {
        typen: liste
            .into_iter()
            .map(|t| TypJs {
                name: t.name,
                version: t.version.to_string(),
                titel: t.titel,
                beschreibung: t.beschreibung,
                braucht: t.braucht,
                herkunft: t.herkunft,
            })
            .collect(),
        warnungen,
        eigener_ordner: eigener_ordner.display().to_string(),
        vorgabe_ordner: vorgabe.map(|p| p.display().to_string()),
    }
}

/// Der Typ des leeren Dokuments, mit dem die App startet.
const LEER: &str = "tftn-leer";

/// Der Entwurf: das Dokument ohne Namen. Fehlt er, entsteht er neu, sonst bleibt sein Text.
#[cfg(test)]
pub fn entwurf_sichern(quellen: &[Typquelle], ordner: &Path) -> Result<PathBuf, String> {
    entwurf_sichern_in(quellen, ordner, None)
}

/// Wie `entwurf_sichern`, in der Sprache `sprache` (die der Oberfläche). Ein noch unberührter
/// Entwurf in einer anderen Sprache entsteht neu.
pub fn entwurf_sichern_in(
    quellen: &[Typquelle],
    ordner: &Path,
    sprache: Option<&str>,
) -> Result<PathBuf, String> {
    let jetzt = std::fs::read_to_string(ordner.join("main.typ")).ok();
    let jetzt = jetzt.as_deref().map(str::trim);
    // Ein noch leerer Entwurf der ersten Fassung hat keine Kopf- und Fußzeilen: neu anlegen.
    let veraltet = jetzt == Some("#set text(lang: \"de\", region: \"AT\")");
    let andere_sprache = jetzt.is_some_and(|j| {
        let (gewuenscht, andere) = leer_vorlagen(quellen, sprache);
        gewuenscht.as_deref().is_some_and(|g| g.trim() != j) && andere.iter().any(|a| a.trim() == j)
    });
    if veraltet || andere_sprache || !ordner.join("tftn.toml").is_file() {
        let _ = std::fs::remove_dir_all(ordner);
        tftn_manifest::anlegen::projekt_anlegen_aus_sprache(quellen, LEER, ordner, None, sprache)
            .map_err(|e| e.to_string())?;
        // Schriften erst beim Speichern auf Wunsch; bis dahin die der App.
        let _ = std::fs::remove_dir_all(ordner.join("fonts"));
        let _ = tftn_manifest::anlegen::readme_schriften_abgleichen(ordner);
    }
    Ok(ordner.to_path_buf())
}

/// `main.typ` des leeren Dokuments für `sprache` und die der übrigen Sprachen.
fn leer_vorlagen(quellen: &[Typquelle], sprache: Option<&str>) -> (Option<String>, Vec<String>) {
    let Ok(paket) = tftn_manifest::typen::finden_in(quellen, LEER, None) else {
        return (None, Vec::new());
    };
    let (Ok(m), Some(v)) = (paket.manifest(), paket.toml.template.as_ref()) else {
        return (None, Vec::new());
    };
    let gewaehlt = tftn_manifest::anlegen::vorlage_fuer(m, &v.path, sprache);
    let lesen = |o: &str| std::fs::read_to_string(paket.ordner.join(o).join(&v.entrypoint)).ok();
    let mut alle: Vec<&str> = vec![v.path.as_str()];
    alle.extend(m.vorlagen.values().map(String::as_str));
    let andere = alle
        .into_iter()
        .filter(|o| *o != gewaehlt)
        .filter_map(lesen)
        .collect();
    (lesen(gewaehlt), andere)
}

/// Die Schriftordner aller Dokumenttypen: der Vorrat für Dokumente ohne eingebundene Schriften.
pub fn schriftvorrat(quellen: &[Typquelle]) -> Vec<PathBuf> {
    let mut aus = Vec::new();
    for q in quellen {
        let Ok(eintraege) = std::fs::read_dir(&q.ordner) else {
            continue;
        };
        let mut ordner: Vec<PathBuf> = eintraege
            .filter_map(Result::ok)
            .map(|e| e.path().join("schriften"))
            .filter(|p| p.is_dir())
            .collect();
        ordner.sort();
        aus.extend(ordner);
    }
    aus
}

/// Bindet die Schriften ein, die das Dokument benutzt; übersetzt es dazu einmal.
pub fn schriften_nachreichen(ziel: &Path) -> Result<(), String> {
    let p = tftn_core::Projekt::oeffnen(ziel).map_err(|e| e.to_string())?;
    let k = kompilieren(&ProjektWorld::neu(&p).map_err(|e| e.to_string())?);
    let doc = k
        .dokument
        .ok_or_else(|| tr!("Das Dokument übersetzt noch nicht; Schriften nicht eingebunden."))?;
    tftn_core::schriften::einbinden(&p, &doc).map_err(|e| e.to_string())?;
    tftn_manifest::anlegen::readme_schriften_abgleichen(ziel).map_err(|e| e.to_string())
}

/// Macht aus dem Entwurf `<dokumente>/<name>`.
pub fn entwurf_benennen(
    entwurf: &Path,
    dokumente: &Path,
    name: &str,
    schriften: bool,
) -> Result<PathBuf, String> {
    if let Some(f) = namensfehler(name) {
        return Err(f.name());
    }
    std::fs::create_dir_all(dokumente).map_err(|e| format!("{}: {e}", dokumente.display()))?;
    let ziel = dokumente.join(name.trim());
    if ziel.exists() {
        return Err(tr!(
            "„{name}“ gibt es in {ordner} schon.",
            name = name.trim(),
            ordner = dokumente.display()
        ));
    }
    if tftn_pakete::umbenennen(entwurf, &ziel).is_err() {
        // Anderes Laufwerk oder gesperrt: kopieren, dann erst den Entwurf löschen.
        if let Err(e) = kopieren(entwurf, &ziel) {
            let _ = std::fs::remove_dir_all(&ziel);
            return Err(format!("{}: {e}", ziel.display()));
        }
        let _ = std::fs::remove_dir_all(entwurf);
    }
    let _ = std::fs::remove_file(ziel.join(".tftn-name"));
    if schriften {
        schriften_nachreichen(&ziel).map_err(|e| {
            tr!(
                "Gespeichert, aber Schriften nicht eingebunden: {fehler}",
                fehler = e
            )
        })?;
    }
    Ok(ziel)
}

pub(crate) fn kopieren(von: &Path, nach: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(nach)?;
    for e in std::fs::read_dir(von)? {
        let e = e?;
        let ziel = nach.join(e.file_name());
        if e.file_type()?.is_dir() {
            kopieren(&e.path(), &ziel)?;
        } else {
            std::fs::copy(e.path(), ziel)?;
        }
    }
    Ok(())
}

/// Legt `<eltern>/<name>` aus dem Typ `typ` an und gibt den Pfad zurück.
pub fn anlegen(
    quellen: &[Typquelle],
    typ: &str,
    eltern: &Path,
    name: &str,
    schriften: bool,
    sprache: Option<&str>,
) -> Result<PathBuf, String> {
    name_pruefen(name)?;
    std::fs::create_dir_all(eltern).map_err(|e| format!("{}: {e}", eltern.display()))?;
    let ziel = eltern.join(name.trim());
    let wurzel =
        tftn_manifest::anlegen::projekt_anlegen_aus_sprache(quellen, typ, &ziel, None, sprache)
            .map(|a| a.projekt.wurzel)
            .map_err(|e| e.to_string())?;
    // Zuerst alle weg, dann auf Wunsch nur die benutzten.
    let _ = std::fs::remove_dir_all(wurzel.join("fonts"));
    if schriften {
        schriften_nachreichen(&wurzel)?;
    } else {
        let _ = tftn_manifest::anlegen::readme_schriften_abgleichen(&wurzel);
    }
    Ok(wurzel)
}

/// Seite 1 des Typs als PNG (40 ppi): anlegen im Temp-Ordner, übersetzen, rastern, aufräumen.
pub fn vorschaubild(
    quellen: &[Typquelle],
    name: &str,
    sprache: Option<&str>,
) -> Result<Vec<u8>, String> {
    static ZAEHLER: AtomicU64 = AtomicU64::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "tftn-vorschau-{}-{}",
        std::process::id(),
        ZAEHLER.fetch_add(1, Ordering::Relaxed)
    ));
    let ergebnis = (|| {
        let a = tftn_manifest::anlegen::projekt_anlegen_aus_sprache(
            quellen,
            name,
            &tmp,
            Some(0),
            sprache,
        )
        .map_err(|e| e.to_string())?;
        let world = ProjektWorld::neu(&a.projekt).map_err(|e| e.to_string())?;
        let doc = kompilieren(&world)
            .dokument
            .ok_or_else(|| tr!("Die Vorlage übersetzt nicht"))?;
        let seite = doc
            .pages()
            .first()
            .ok_or_else(|| tr!("Die Vorlage hat keine Seite"))?;
        Ok(tftn_core::vorschau::rastern(seite, 40.0))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    ergebnis
}

/// Base64 (Standardalphabet, mit Auffüllung) für `data:`-URLs.
pub fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut aus = String::with_capacity(b.len().div_ceil(3) * 4);
    for block in b.chunks(3) {
        let n = (u32::from(block[0]) << 16)
            | (u32::from(*block.get(1).unwrap_or(&0)) << 8)
            | u32::from(*block.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= block.len() {
                aus.push(A[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                aus.push('=');
            }
        }
    }
    aus
}

/// PNG als `data:`-URL.
pub fn png_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64(png))
}

// --- Aktualisieren mit Vergleich (Task 5) ------------------------------------------------------

/// Neuere Version des Projekttyps (Gruppe „Projekt“ im Fach „Dokument“).
#[derive(Debug, Serialize)]
pub struct NeuJs {
    pub version: String,
    pub herkunft: Herkunft,
}

#[derive(Debug, Serialize)]
pub struct SeitenbildJs {
    pub seite: usize,
    pub alt: Option<String>,
    pub neu: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VergleichJs {
    pub alt: String,
    pub neu: String,
    pub seiten_alt: usize,
    pub seiten_neu: usize,
    pub geaendert: Vec<usize>,
    pub bilder: Vec<SeitenbildJs>,
    pub fehler: Vec<String>,
    /// Dateien mit einer Importstelle
    pub stellen: Vec<String>,
}

pub fn neue_version_js(projekt: &tftn_core::Projekt, quellen: &[Typquelle]) -> Option<NeuJs> {
    tftn_manifest::aktualisieren::neuere_version(projekt, quellen).map(|t| NeuJs {
        version: t.version.to_string(),
        herkunft: t.herkunft,
    })
}

/// Probe in einer Temp-Kopie: neue Version übernehmen, beide Fassungen übersetzen und vergleichen.
/// Die Kopie wird immer entfernt.
pub fn vergleichen(
    projekt: &tftn_core::Projekt,
    quellen: &[Typquelle],
) -> Result<VergleichJs, String> {
    vergleichen_in(projekt, quellen, &std::env::temp_dir())
}

/// Führt `f` auf einer Temp-Kopie des Projekts unter `basis` aus; die Kopie wird immer entfernt.
pub fn in_probe<T>(
    projekt: &tftn_core::Projekt,
    basis: &Path,
    f: impl FnOnce(&tftn_core::Projekt) -> Result<T, String>,
) -> Result<T, String> {
    static ZAEHLER: AtomicU64 = AtomicU64::new(0);
    let tmp = basis.join(format!(
        "tftn-probe-{}-{}",
        std::process::id(),
        ZAEHLER.fetch_add(1, Ordering::Relaxed)
    ));
    let ergebnis = tftn_manifest::aktualisieren::probekopie(projekt, &tmp)
        .map_err(|e| e.to_string())
        .and_then(|kopie| f(&kopie));
    let _ = std::fs::remove_dir_all(&tmp);
    ergebnis
}

/// Ergebnis des Seitenvergleichs für die Oberfläche.
pub fn vergleich_js(
    alt: String,
    neu: String,
    v: tftn_core::vergleich::Vergleich,
    mut stellen: Vec<String>,
) -> VergleichJs {
    stellen.dedup();
    VergleichJs {
        alt,
        neu,
        seiten_alt: v.seiten_alt,
        seiten_neu: v.seiten_neu,
        geaendert: v.geaendert,
        bilder: v
            .bilder
            .into_iter()
            .map(|(seite, alt, neu)| SeitenbildJs {
                seite,
                alt: alt.as_deref().map(png_url),
                neu: neu.as_deref().map(png_url),
            })
            .collect(),
        fehler: v.fehler,
        stellen,
    }
}

/// Wie `vergleichen`, mit der Probekopie unter `basis`.
pub fn vergleichen_in(
    projekt: &tftn_core::Projekt,
    quellen: &[Typquelle],
    basis: &Path,
) -> Result<VergleichJs, String> {
    use tftn_manifest::aktualisieren::{neuere_version, stellen_umschreiben, typ_aktualisieren};
    let neu = neuere_version(projekt, quellen)
        .ok_or_else(|| tr!("Keine neuere Version des Dokumenttyps verfügbar."))?;
    in_probe(projekt, basis, |kopie| {
        let a = typ_aktualisieren(kopie, &neu, quellen).map_err(|e| e.to_string())?;
        stellen_umschreiben(&kopie.wurzel, &a, |_| true).map_err(|e| e.to_string())?;
        let kopie = tftn_core::Projekt::oeffnen(&kopie.wurzel).map_err(|e| e.to_string())?;
        let v = tftn_core::vergleich::seiten_vergleichen(projekt, &kopie, 72.0, 12)?;
        let stellen = a.stellen.iter().map(|s| s.datei.clone()).collect();
        Ok(vergleich_js(a.alt, a.neu, v, stellen))
    })
}

/// Ersetzt alle `stellen` in `text` durch `neu` in einem Patch (ein Undo-Schritt): von der ersten
/// bis zur letzten Stelle.
pub fn stellen_patch(
    text: &str,
    stellen: &[std::ops::Range<usize>],
    neu: &str,
) -> Option<tftn_edit::Patch> {
    let (erste, letzte) = (stellen.first()?, stellen.last()?);
    let bereich = erste.start..letzte.end;
    let mut aus = text[bereich.clone()].to_string();
    for s in stellen.iter().rev() {
        aus.replace_range(s.start - bereich.start..s.end - bereich.start, neu);
    }
    Some(tftn_edit::Patch { bereich, text: aus })
}

/// Übernimmt die neue Version ins echte Projekt, nachdem der Vergleich ohne Fehler übersetzt hat.
/// Andere `.typ`-Dateien werden direkt umgeschrieben; für die Hauptdatei (`haupt`, relativ) kommt
/// ein Patch auf die Quelle des Editors zurück (`None`, wenn sie den Typ nicht direkt importiert).
pub fn uebernehmen(
    projekt: &tftn_core::Projekt,
    quellen: &[Typquelle],
    haupt: &str,
    quelle: &typst::syntax::Source,
) -> Result<
    (
        tftn_manifest::aktualisieren::Aktualisierung,
        Option<tftn_edit::Patch>,
    ),
    String,
> {
    use tftn_manifest::aktualisieren::{
        neuere_version, stellen_in_text, stellen_umschreiben, typ_aktualisieren,
    };
    let v = vergleichen(projekt, quellen)?;
    if !v.fehler.is_empty() {
        return Err(tr!("Die neue Version übersetzt nicht – siehe Vergleich."));
    }
    let neu = neuere_version(projekt, quellen)
        .ok_or_else(|| tr!("Keine neuere Version des Dokumenttyps verfügbar."))?;
    let a = typ_aktualisieren(projekt, &neu, quellen).map_err(|e| e.to_string())?;
    stellen_umschreiben(&projekt.wurzel, &a, |d| d != haupt).map_err(|e| e.to_string())?;
    let stellen = stellen_in_text(quelle.text(), &a.alt);
    let patch = stellen_patch(quelle.text(), &stellen, &a.neu);
    Ok((a, patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tftn_core::Projekt;

    /// Die Typen aus dem Repo; mit `TFTN_TEST_TYPEN` die eines gebauten Bündels.
    fn typen() -> PathBuf {
        std::env::var_os("TFTN_TEST_TYPEN")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../types"))
    }

    fn ordner(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-app-typen-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        o
    }

    #[test]
    fn entwurf_folgt_der_sprache_solange_unberuehrt() {
        let o = ordner("entwurf-sprache");
        let q = quellen(&typen(), &o);
        let ziel = o.join("entwurf");
        let haupt = |e: &Path| std::fs::read_to_string(e.join("main.typ")).unwrap();
        let e = entwurf_sichern_in(&q, &ziel, Some("de")).unwrap();
        assert!(haupt(&e).contains("lang: \"de\""), "{}", haupt(&e));
        let e = entwurf_sichern_in(&q, &ziel, Some("en")).unwrap();
        assert!(haupt(&e).contains("lang: \"en\""), "{}", haupt(&e));
        // Gleiche Sprache: bleibt, wie er ist.
        std::fs::write(e.join("notiz.txt"), "x").unwrap();
        let e = entwurf_sichern_in(&q, &ziel, Some("en")).unwrap();
        assert!(e.join("notiz.txt").is_file());
        // Mit Text bleibt er auch in einer anderen Sprache.
        std::fs::write(e.join("main.typ"), format!("{}Hello\n", haupt(&e))).unwrap();
        let e = entwurf_sichern_in(&q, &ziel, Some("de")).unwrap();
        assert!(haupt(&e).ends_with("Hello\n"), "{}", haupt(&e));
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn entwurf_ist_leer_und_uebersetzt() {
        let o = ordner("entwurf");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("daten/entwurf")).unwrap();
        assert_eq!(
            std::fs::read_to_string(e.join("main.typ")).unwrap(),
            "#import \"@local/tftn-leer:0.2.1\": *\n#set text(lang: \"de\", region: \"AT\")\n"
        );
        assert!(!e.join("dokument.toml").exists());
        assert!(!e.join("fonts").exists(), "Schriften erst beim Speichern");
        tftn_core::schriftvorrat_setzen(schriftvorrat(&q));
        let p = Projekt::oeffnen(&e).unwrap();
        let w = ProjektWorld::neu(&p).unwrap();
        let k = kompilieren(&w);
        assert!(k.meldungen.iter().all(|m| !m.fehler), "{:?}", k.meldungen);
        // Die Vorgabe im Fach „Seite“ muss in der Liste der Schriften stehen.
        let namen: Vec<String> = typst::World::book(&w)
            .families()
            .map(|(n, _)| n.to_string())
            .collect();
        assert!(namen.iter().any(|n| n == "Libertinus Serif"), "{namen:?}");
        assert!(k.dokument.is_some());
        let (_, warnungen) = tftn_katalog::Katalog::bauen(&tftn_katalog::Quellen {
            projekt: Some(&p),
            beschreibungen: &[],
        });
        assert!(warnungen.is_empty(), "{warnungen:?}");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn alter_leerer_entwurf_wird_ersetzt() {
        let o = ordner("entwurf-alt");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        std::fs::write(
            e.join("main.typ"),
            "#set text(lang: \"de\", region: \"AT\")\n\n",
        )
        .unwrap();
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        assert!(
            std::fs::read_to_string(e.join("main.typ"))
                .unwrap()
                .contains("#import")
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn entwurf_mit_kopf_fuss_und_rand_uebersetzt() {
        use tftn_edit::{SeitenFeld as F, Zeile, anwenden, seite_patch};
        use typst::syntax::Source;
        let o = ordner("seiten");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        tftn_core::schriftvorrat_setzen(schriftvorrat(&q));
        let mut t = std::fs::read_to_string(e.join("main.typ")).unwrap();
        let zeile = |innen: &str, mitte: &str, aussen: &str, erste| Zeile {
            innen: innen.into(),
            mitte: mitte.into(),
            aussen: aussen.into(),
            erste,
        };
        for f in [
            F::Raender {
                oben: 3.0,
                unten: 3.0,
                innen: 2.5,
                aussen: 4.0,
                doppelseitig: true,
            },
            F::Kopf(zeile("{kapitel}", "", "Mein Titel", false)),
            F::Fuss(zeile("", "", "Seite {seite}", true)),
            F::Schrift("Source Sans 3".into()),
            F::Blocksatz(true),
            F::Zeilenabstand(0.8),
            F::Nummerierung(true),
        ] {
            t = anwenden(&t, &seite_patch(&Source::detached(t.clone()), &f).unwrap());
        }
        for n in 1..=3 {
            t.push_str(&format!(
                "\n= Kapitel {n}\n\nEin Absatz#randnotiz[Eine Notiz am Rand] geht weiter.\n\n#pagebreak()\n"
            ));
        }
        std::fs::write(e.join("main.typ"), &t).unwrap();
        let p = Projekt::oeffnen(&e).unwrap();
        let k = kompilieren(&ProjektWorld::neu(&p).unwrap());
        assert!(
            k.meldungen.iter().all(|m| !m.fehler),
            "{:?}\n{t}",
            k.meldungen
        );
        assert!(k.dokument.is_some());
    }

    #[test]
    fn schriften_nachreichen_legt_fonts_an() {
        let o = ordner("nachreichen");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        std::fs::write(e.join("main.typ"), "Ein Satz.\n").unwrap();
        tftn_core::schriftvorrat_setzen(schriftvorrat(&q));
        let ziel = entwurf_benennen(&e, &o.join("Dokumente"), "Ohne", false).unwrap();
        assert!(!ziel.join("fonts").exists());
        schriften_nachreichen(&ziel).unwrap();
        assert!(ziel.join("fonts/LibertinusSerif-Regular.otf").is_file());
        let readme = std::fs::read_to_string(ziel.join("README.md")).unwrap();
        assert_eq!(readme.matches("`fonts/`").count(), 1, "{readme}");
        assert!(
            !ziel.join("fonts/SourceSerif4-Regular.otf").exists(),
            "nur Benutztes"
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn entwurf_bleibt_erhalten() {
        let o = ordner("entwurf-bleibt");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        std::fs::write(e.join("main.typ"), "Mein Text\n").unwrap();
        let e2 = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        assert_eq!(
            std::fs::read_to_string(e2.join("main.typ")).unwrap(),
            "Mein Text\n"
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn entwurf_benennen_verschiebt_nach_dokumente() {
        let o = ordner("benennen");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        std::fs::write(e.join("main.typ"), "Mein Text\n").unwrap();
        let dok = o.join("Dokumente");
        tftn_core::schriftvorrat_setzen(schriftvorrat(&q));
        let ziel = entwurf_benennen(&e, &dok, " Mein Artikel ", true).unwrap();
        assert!(
            ziel.join("fonts/LibertinusSerif-Regular.otf").is_file(),
            "eingebunden"
        );
        assert!(
            !ziel.join("fonts/SourceSans3-Regular.otf").exists(),
            "nur Benutztes"
        );
        assert_eq!(ziel, dok.join("Mein Artikel"));
        assert_eq!(
            std::fs::read_to_string(ziel.join("main.typ")).unwrap(),
            "Mein Text\n"
        );
        assert!(!e.exists());
        // Danach gibt es wieder einen frischen Entwurf.
        let neu = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        assert!(
            std::fs::read_to_string(neu.join("main.typ"))
                .unwrap()
                .starts_with("#import")
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn entwurf_benennen_lehnt_vorhandenen_namen_ab() {
        let o = ordner("benennen-da");
        let q = quellen(&typen(), &o);
        let e = entwurf_sichern(&q, &o.join("entwurf")).unwrap();
        let dok = o.join("Dokumente");
        std::fs::create_dir_all(dok.join("Text")).unwrap();
        let f = entwurf_benennen(&e, &dok, "Text", false).unwrap_err();
        assert!(f.contains("gibt es"), "{f}");
        assert!(e.join("main.typ").is_file(), "der Entwurf bleibt");
        assert!(entwurf_benennen(&e, &dok, "a/b", false).is_err());
        let ohne = entwurf_benennen(&e, &dok, "Ohne", false).unwrap();
        assert!(!ohne.join("fonts").exists(), "nicht eingebunden");
        let readme = std::fs::read_to_string(ohne.join("README.md")).unwrap();
        assert!(!readme.contains("fonts/"), "{readme}");
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn quellen_reihenfolge() {
        let o = ordner("quellen");
        let q = quellen(&typen(), &o);
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].herkunft, Herkunft::Tftn);
        std::fs::create_dir_all(o.join("typen")).unwrap();
        let q = quellen(&typen(), &o);
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].herkunft, Herkunft::Eigen);
        assert_eq!(q[0].ordner, o.join("typen"));
        assert_eq!(q[1].ordner, typen());
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn name_pruefen_faelle() {
        for schlecht in ["", "a/b", "a\\b", "..", ".x", "  "] {
            let e = name_pruefen(schlecht).unwrap_err();
            assert!(e.starts_with("Projektname"), "{schlecht:?}: {e}");
        }
        assert_eq!(name_pruefen("Mein Artikel 2026"), Ok(()));
        // Abschlussprüfung: unter Windows unzulässig bzw. laufwerksrelativ.
        for schlecht in [
            "C:Artikel",
            "a<b",
            "a>b",
            "a\"b",
            "a|b",
            "a?b",
            "a*b",
            "CON",
            "nul",
            "Com1",
            "x.",
            "x\u{7}",
        ] {
            let e = name_pruefen(schlecht).unwrap_err();
            assert!(e.starts_with("Projektname"), "{schlecht:?}: {e}");
        }
    }

    #[test]
    fn base64_bekannte_werte() {
        for (ein, aus) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(ein.as_bytes()), aus);
        }
    }

    #[test]
    fn vorschaubild_ist_png() {
        let vorher = tempreste();
        let q = quellen(&typen(), &ordner("bild"));
        let png = vorschaubild(&q, "tftn-fachartikel", None).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        assert_eq!(tempreste(), vorher, "Temp-Ordner nicht aufgeräumt");
    }

    /// Anzahl der Vorschau-Ordner dieses Prozesses im Temp-Ordner.
    fn tempreste() -> usize {
        let praefix = format!("tftn-vorschau-{}-", std::process::id());
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(&praefix))
            .count()
    }

    #[test]
    fn anlegen_kern() {
        let o = ordner("anlegen");
        let q = quellen(&typen(), &o);
        let p = anlegen(&q, "tftn-fachartikel", &o, "Probe", true, None).unwrap();
        assert_eq!(p, o.join("Probe").canonicalize().unwrap());
        assert!(o.join("Probe/tftn.toml").is_file());
        let main = std::fs::read_to_string(o.join("Probe/main.typ")).unwrap();
        let e = anlegen(&q, "tftn-fachartikel", &o, "Probe", true, None).unwrap_err();
        assert!(e.contains("nicht leer"), "{e}");
        assert_eq!(
            std::fs::read_to_string(o.join("Probe/main.typ")).unwrap(),
            main
        );
        // Ein fehlender Zielordner wird angelegt.
        assert!(
            anlegen(
                &q,
                "tftn-fachartikel",
                &o.join("gibtsnicht"),
                "X",
                true,
                None
            )
            .is_ok()
        );
        assert!(anlegen(&q, "tftn-fachartikel", &o, "a/b", true, None).is_err());
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn typen_js_mit_herkunft() {
        let o = ordner("liste");
        let js = typen_js(&quellen(&typen(), &o), &o.join("typen"), Some(&o));
        assert_eq!(js.typen.len(), 2);
        assert_eq!(js.typen[0].name, "tftn-fachartikel");
        assert_eq!(js.typen[1].name, "tftn-leer");
        assert_eq!(js.typen[0].version, "0.3.0");
        let v = serde_json::to_value(&js).unwrap();
        assert_eq!(v["typen"][0]["herkunft"], "tftn");
        assert!(v.get("eigener_ordner").is_some() && v.get("vorgabe_ordner").is_some());
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Kopie des Testartikels (Fachartikel 0.1.0) in einem eigenen Temp-Ordner.
    fn testartikel(name: &str) -> (tftn_core::Projekt, PathBuf) {
        let o = ordner(name);
        let p = tftn_core::Projekt::oeffnen(&typen().join("../tests/korpus/testartikel")).unwrap();
        let k = tftn_manifest::aktualisieren::probekopie(&p, &o.join("projekt")).unwrap();
        (k, o)
    }

    fn probereste(basis: &Path) -> usize {
        let praefix = format!("tftn-probe-{}-", std::process::id());
        std::fs::read_dir(basis)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(&praefix))
            .count()
    }

    #[test]
    fn vergleich_testartikel_auf_0_2() {
        let (p, o) = testartikel("vergleich");
        let v = vergleichen_in(&p, &quellen(&typen(), &o), &o).unwrap();
        assert!(v.fehler.is_empty(), "{:?}", v.fehler);
        assert_eq!(v.alt, "@local/tftn-fachartikel:0.1.0");
        assert!(v.neu.ends_with("0.3.0"), "{}", v.neu);
        assert_eq!(v.stellen, ["main.typ"]);
        assert_eq!(probereste(&o), 0, "Probekopie nicht entfernt");
        for b in &v.bilder {
            assert!(
                b.alt
                    .as_deref()
                    .unwrap_or("data:image/png;base64,")
                    .starts_with("data:image/png;base64,")
            );
        }
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn uebernehmen_liefert_patch() {
        let (p, o) = testartikel("uebernehmen");
        let main = std::fs::read_to_string(p.wurzel.join("main.typ")).unwrap();
        let quelle = typst::syntax::Source::detached(main.clone());
        let (a, patch) = uebernehmen(&p, &quellen(&typen(), &o), "main.typ", &quelle).unwrap();
        let patch = patch.unwrap();
        assert_eq!(
            &main[patch.bereich.clone()],
            "@local/tftn-fachartikel:0.1.0"
        );
        assert_eq!(patch.text, a.neu);
        let tftn = std::fs::read_to_string(p.wurzel.join("tftn.toml")).unwrap();
        assert!(tftn.contains("tftn-fachartikel:0.3.0"), "{tftn}");
        assert_eq!(
            std::fs::read_to_string(p.wurzel.join("main.typ")).unwrap(),
            main
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Abschlussprüfung: Alle Importstellen der Hauptdatei kommen in einen Patch.
    #[test]
    fn uebernehmen_patcht_alle_stellen() {
        let (p, o) = testartikel("alle-stellen");
        let main = std::fs::read_to_string(p.wurzel.join("main.typ")).unwrap().replace(
            r#"#import "@local/tftn-fachartikel:0.1.0": *"#,
            "#import \"@local/tftn-fachartikel:0.1.0\": fachartikel\n#import \"@local/tftn-fachartikel:0.1.0\": hinweis",
        );
        std::fs::write(p.wurzel.join("main.typ"), &main).unwrap();
        let quelle = typst::syntax::Source::detached(main.clone());
        let (a, patch) = uebernehmen(&p, &quellen(&typen(), &o), "main.typ", &quelle).unwrap();
        let patch = patch.unwrap();
        let mut neu = main.clone();
        neu.replace_range(patch.bereich.clone(), &patch.text);
        assert_eq!(neu, main.replace(&a.alt, &a.neu));
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Abschlussprüfung: Wechsel nur in ein Projekt, das sich öffnen lässt.
    #[test]
    fn wechsel_pruefen_faelle() {
        let o = ordner("wechsel");
        assert!(wechsel_pruefen(&o).is_err(), "ohne tftn.toml");
        std::fs::write(o.join("tftn.toml"), "tftn = 2\n").unwrap();
        assert!(wechsel_pruefen(&o).is_err(), "kaputte tftn.toml");
        let q = quellen(&typen(), &o.join("leer"));
        let p = anlegen(&q, "tftn-fachartikel", &o, "Gut", true, None).unwrap();
        assert_eq!(wechsel_pruefen(&o.join("Gut")).unwrap(), p);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn uebernehmen_gesperrt_bei_fehler() {
        let (p, o) = testartikel("gesperrt");
        // Eigener Fachartikel 0.3.0 mit Syntaxfehler in lib.typ.
        let eigen = o.join("einstellungen/typen/tftn-fachartikel");
        kopieren(&typen().join("tftn-fachartikel"), &eigen);
        let toml = eigen.join("typst.toml");
        let t = std::fs::read_to_string(&toml).unwrap();
        std::fs::write(
            &toml,
            t.replace(r#"version = "0.3.0""#, r#"version = "0.4.0""#),
        )
        .unwrap();
        let lib = std::fs::read_to_string(eigen.join("lib.typ")).unwrap();
        std::fs::write(eigen.join("lib.typ"), format!("{lib}\n#let kaputt = (\n")).unwrap();
        let q = quellen(&typen(), &o.join("einstellungen"));
        let tftn_vorher = std::fs::read_to_string(p.wurzel.join("tftn.toml")).unwrap();
        let v = vergleichen(&p, &q).unwrap();
        assert!(!v.fehler.is_empty());
        let quelle = typst::syntax::Source::detached(
            std::fs::read_to_string(p.wurzel.join("main.typ")).unwrap(),
        );
        let e = uebernehmen(&p, &q, "main.typ", &quelle).unwrap_err();
        assert!(e.contains("übersetzt nicht"), "{e}");
        assert_eq!(
            std::fs::read_to_string(p.wurzel.join("tftn.toml")).unwrap(),
            tftn_vorher
        );
        assert!(
            !p.wurzel
                .join("packages/local/tftn-fachartikel/0.3.0")
                .exists()
        );
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn projekt_meldet_neuere_version() {
        let (p, o) = testartikel("neujs");
        let n = neue_version_js(&p, &quellen(&typen(), &o)).unwrap();
        assert_eq!(n.version, "0.3.0");
        assert_eq!(n.herkunft, Herkunft::Tftn);
        let _ = std::fs::remove_dir_all(&o);
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
}
