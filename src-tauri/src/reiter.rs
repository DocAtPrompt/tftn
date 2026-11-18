//! Reiter im Editor: Neben der Hauptdatei lassen sich Kapiteldateien (`.typ`) des Projekts öffnen.
//!
//! Die Sitzung führt die aktive Datei in `datei`, `spiegel` und `auf_platte` wie bisher; so
//! arbeiten Hervorhebung, Fach „Hier“, Rechtschreibung, Zählen und Klicksprung ohne Umbau auf der
//! Datei des aktiven Reiters. Die übrigen offenen Dateien liegen geparkt daneben, jede mit ihrem
//! Spiegel. Jeder Typst-Lauf bekommt alle offenen Quellen (ungespeichert, aus dem Editor); die
//! Vorschau zeigt also immer das ganze Dokument.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;
use tftn_core::spiegel::{Spiegel, pruefsumme, zeilenenden_lf};
use tftn_sprachen::tr;
use typst::syntax::Source;

use crate::{Sitzung, Zustand, kompilieren_lassen, vorschau_von};

/// Eine offene, gerade nicht angezeigte Datei.
pub struct Geparkt {
    pub datei: PathBuf,
    pub spiegel: Spiegel,
    /// Prüfsumme des zuletzt gespeicherten bzw. geladenen Stands.
    pub auf_platte: u64,
}

impl Geparkt {
    pub fn ungespeichert(&self) -> bool {
        self.spiegel.pruefsumme() != self.auf_platte
    }
}

#[derive(Serialize, Debug)]
pub struct DateiJs {
    /// Pfad relativ zum Projekt, mit `/`; die Hauptdatei wie in `tftn.toml`.
    pub datei: String,
    pub haupt: bool,
    pub text: String,
    pub rev: u64,
    pub ungespeichert: bool,
}

/// Pfad relativ zum Projekt, mit `/` auf allen Systemen.
pub fn relativ(wurzel: &Path, pfad: &Path) -> String {
    pfad.strip_prefix(wurzel)
        .unwrap_or(pfad)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Prüft eine Datei, die als Reiter aufgehen soll: im Projekt, eine `.typ`-Datei, nicht unter
/// `packages/`, `_lokal/` oder in versteckten Ordnern. Liefert den kanonischen Pfad.
pub fn pfad_pruefen(wurzel: &Path, relativ: &str) -> Result<PathBuf, String> {
    let fehlt = || {
        tr!(
            "{datei} ist keine Kapiteldatei dieses Dokuments.",
            datei = relativ
        )
    };
    let pfad = wurzel.join(relativ).canonicalize().map_err(|_| fehlt())?;
    let innen = pfad.strip_prefix(wurzel).map_err(|_| fehlt())?;
    let gesperrt = innen.components().any(|t| {
        let t = t.as_os_str().to_string_lossy();
        t.starts_with('.') || t == "packages" || t == "_lokal" || t == "_renders"
    });
    if gesperrt || !pfad.is_file() || pfad.extension().is_none_or(|e| e != "typ") {
        return Err(fehlt());
    }
    Ok(pfad)
}

/// Sind alle offenen Dateien gespeichert? (Export, Archiv, Serie nehmen die Platte.)
pub fn alles_gespeichert(s: &Sitzung) -> bool {
    s.spiegel.pruefsumme() == s.auf_platte && s.geparkt.iter().all(|g| !g.ungespeichert())
}

/// Ist der aktive Reiter die Hauptdatei?
pub fn haupt_aktiv(s: &Sitzung) -> bool {
    s.datei == s.projekt.haupt()
        || s.datei.canonicalize().ok() == s.projekt.haupt().canonicalize().ok()
}

/// Der Spiegel der Hauptdatei, ob aktiv oder geparkt.
pub fn haupt_spiegel(s: &Sitzung) -> &Spiegel {
    if haupt_aktiv(s) {
        return &s.spiegel;
    }
    let haupt = s
        .projekt
        .haupt()
        .canonicalize()
        .unwrap_or_else(|_| s.projekt.haupt());
    s.geparkt
        .iter()
        .find(|g| g.datei == haupt)
        .map(|g| &g.spiegel)
        .unwrap_or(&s.spiegel)
}

/// Der Spiegel einer offenen Datei (aktiv oder geparkt), sonst `None`.
pub fn offener_spiegel<'a>(s: &'a Sitzung, pfad: &Path) -> Option<&'a Spiegel> {
    let pfad = pfad.canonicalize().ok()?;
    if s.datei.canonicalize().ok().as_ref() == Some(&pfad) {
        return Some(&s.spiegel);
    }
    s.geparkt
        .iter()
        .find(|g| g.datei == pfad)
        .map(|g| &g.spiegel)
}

/// Für Befehle, die in die Hauptdatei schreiben (Seite, Dokumenttyp): nur bei ihrem Reiter.
pub fn nur_haupt(s: &Sitzung) -> Result<(), String> {
    if haupt_aktiv(s) {
        Ok(())
    } else {
        Err(tr!(
            "Das steht in der Hauptdatei. Bitte zuerst ihren Reiter wählen."
        ))
    }
}

/// Die Quellen der geparkten Dateien (für den Typst-Lauf).
pub fn geparkte_quellen(s: &Sitzung) -> Vec<Source> {
    s.geparkt
        .iter()
        .map(|g| g.spiegel.quelle().clone())
        .collect()
}

/// Der Reiter mit `ziel` wird aktiv. Eine noch nicht offene Datei wird von der Platte gelesen;
/// die bisher aktive bleibt mit ihrem Stand geparkt.
pub fn aktivieren(s: &mut Sitzung, ziel: PathBuf, id: typst::syntax::FileId) -> Result<(), String> {
    tauschen(
        &mut s.datei,
        &mut s.spiegel,
        &mut s.auf_platte,
        &mut s.geparkt,
        ziel,
        id,
    )
}

/// Kern von `aktivieren`, ohne Sitzung (für Tests).
fn tauschen(
    datei: &mut PathBuf,
    spiegel: &mut Spiegel,
    auf_platte: &mut u64,
    geparkt: &mut Vec<Geparkt>,
    ziel: PathBuf,
    id: typst::syntax::FileId,
) -> Result<(), String> {
    if *datei == ziel {
        return Ok(());
    }
    let neu = match geparkt.iter().position(|g| g.datei == ziel) {
        Some(i) => geparkt.remove(i),
        None => {
            let roh =
                std::fs::read_to_string(&ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
            Geparkt {
                auf_platte: pruefsumme(&zeilenenden_lf(&roh)),
                spiegel: Spiegel::neu(id, &roh),
                datei: ziel,
            }
        }
    };
    geparkt.push(Geparkt {
        datei: std::mem::replace(datei, neu.datei),
        spiegel: std::mem::replace(spiegel, neu.spiegel),
        auf_platte: std::mem::replace(auf_platte, neu.auf_platte),
    });
    Ok(())
}

fn js(s: &Sitzung) -> DateiJs {
    DateiJs {
        datei: relativ(&s.projekt.wurzel, &s.datei),
        haupt: haupt_aktiv(s),
        text: s.spiegel.text().to_string(),
        rev: s.spiegel.rev(),
        ungespeichert: s.spiegel.pruefsumme() != s.auf_platte,
    }
}

/// Reiter `datei` (relativ zum Projekt; leer: die Hauptdatei) aktiv machen und seinen Stand
/// liefern. Der Editor setzt dann den Zustand dieses Reiters ein.
#[tauri::command]
pub fn datei_aktivieren(datei: String, zustand: State<'_, Zustand>) -> Result<DateiJs, String> {
    let vorschau = vorschau_von(&zustand)?;
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let haupt = s
        .projekt
        .haupt()
        .canonicalize()
        .unwrap_or_else(|_| s.projekt.haupt());
    let ziel = if datei.is_empty() || datei == s.projekt.konfig.haupt {
        haupt.clone()
    } else {
        pfad_pruefen(&s.projekt.wurzel, &datei)?
    };
    let id = {
        let v = vorschau.lock().unwrap();
        if ziel == haupt {
            v.world().haupt()
        } else {
            v.world()
                .datei_id(&ziel)
                .ok_or_else(|| tr!("{datei} ist keine Kapiteldatei dieses Dokuments.", datei))?
        }
    };
    aktivieren(s, ziel, id)?;
    kompilieren_lassen(&zustand, s);
    Ok(js(s))
}

/// Reiter `datei` schließen (nicht der aktive, nicht die Hauptdatei). Ungespeichertes nur mit
/// `verwerfen`; dann zeigt die Vorschau wieder die Fassung auf der Platte.
#[tauri::command]
pub fn datei_schliessen(
    datei: String,
    verwerfen: bool,
    zustand: State<'_, Zustand>,
) -> Result<(), String> {
    let mut sitzung = zustand.sitzung.lock().unwrap();
    let s = sitzung
        .as_mut()
        .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
    let wurzel = s.projekt.wurzel.clone();
    let Some(i) = s
        .geparkt
        .iter()
        .position(|g| relativ(&wurzel, &g.datei) == datei)
    else {
        return Ok(());
    };
    if s.geparkt[i].datei
        == s.projekt
            .haupt()
            .canonicalize()
            .unwrap_or_else(|_| s.projekt.haupt())
    {
        return Err(tr!("Die Hauptdatei bleibt offen."));
    }
    if s.geparkt[i].ungespeichert() && !verwerfen {
        return Err(tr!("{datei} ist nicht gespeichert.", datei));
    }
    s.geparkt.remove(i);
    kompilieren_lassen(&zustand, s);
    Ok(())
}

/// Die Kapiteldateien des Projekts zum Öffnen (`.typ` außer der Hauptdatei), sortiert.
#[tauri::command]
pub fn kapiteldateien(zustand: State<'_, Zustand>) -> Result<Vec<String>, String> {
    let (wurzel, haupt) = {
        let sitzung = zustand.sitzung.lock().unwrap();
        let s = sitzung
            .as_ref()
            .ok_or_else(|| tr!("kein Projekt geöffnet"))?;
        (s.projekt.wurzel.clone(), s.projekt.konfig.haupt.clone())
    };
    let mut liste = Vec::new();
    sammeln(&wurzel, &wurzel, 0, &mut liste);
    liste.retain(|d| *d != haupt);
    liste.sort();
    Ok(liste)
}

/// `.typ`-Dateien unter `ordner`, höchstens vier Ebenen tief, ohne gesperrte Ordner und die
/// Hauptdatei des Projekts.
fn sammeln(wurzel: &Path, ordner: &Path, tiefe: usize, aus: &mut Vec<String>) {
    if tiefe > 4 || aus.len() > 500 {
        return;
    }
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return;
    };
    for e in eintraege.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.')
            || ["packages", "_lokal", "_renders", "export"].contains(&name.as_str())
        {
            continue;
        }
        if p.is_dir() {
            sammeln(wurzel, &p, tiefe + 1, aus);
        } else if p.extension().is_some_and(|x| x == "typ") {
            aus.push(relativ(wurzel, &p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordner(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-reiter-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        for d in ["kapitel", "packages/preview/x", ".fassungen", "_lokal"] {
            std::fs::create_dir_all(o.join(d)).unwrap();
        }
        for f in [
            "main.typ",
            "kapitel/eins.typ",
            "kapitel/notizen.txt",
            "packages/preview/x/lib.typ",
            ".fassungen/alt.typ",
            "_lokal/probe.typ",
        ] {
            std::fs::write(o.join(f), "x").unwrap();
        }
        o.canonicalize().unwrap()
    }

    #[test]
    fn nur_typ_dateien_im_projekt() {
        let o = ordner("pruefen");
        assert!(pfad_pruefen(&o, "kapitel/eins.typ").is_ok());
        for falsch in [
            "kapitel/notizen.txt",
            "packages/preview/x/lib.typ",
            ".fassungen/alt.typ",
            "_lokal/probe.typ",
            "../ausserhalb.typ",
            "gibt-es-nicht.typ",
        ] {
            assert!(pfad_pruefen(&o, falsch).is_err(), "{falsch}");
        }
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn tauschen_behaelt_den_stand_jedes_reiters() {
        use tftn_core::spiegel::Aenderung;
        let o = ordner("tauschen");
        std::fs::write(o.join("kapitel/eins.typ"), "Kapitel\n").unwrap();
        let haupt = o.join("main.typ");
        let kapitel = o.join("kapitel/eins.typ");
        let id = Source::detached("").id();
        let mut datei = haupt.clone();
        let mut spiegel = Spiegel::neu(id, "x");
        let mut auf_platte = spiegel.pruefsumme();
        let mut geparkt = Vec::new();
        // In der Hauptdatei getippt (ungespeichert), dann das Kapitel geöffnet.
        let rev = spiegel
            .anwenden(
                0,
                &[Aenderung {
                    von: 1,
                    bis: 1,
                    text: "y".into(),
                }],
            )
            .unwrap();
        tauschen(
            &mut datei,
            &mut spiegel,
            &mut auf_platte,
            &mut geparkt,
            kapitel.clone(),
            id,
        )
        .unwrap();
        assert_eq!(datei, kapitel);
        assert_eq!(spiegel.text(), "Kapitel\n");
        assert_eq!(
            spiegel.pruefsumme(),
            auf_platte,
            "frisch geladen: gespeichert"
        );
        assert_eq!(geparkt.len(), 1);
        assert!(geparkt[0].ungespeichert());
        // Zurück: derselbe Stand, dieselbe Revision; das Kapitel liegt jetzt geparkt.
        tauschen(
            &mut datei,
            &mut spiegel,
            &mut auf_platte,
            &mut geparkt,
            haupt.clone(),
            id,
        )
        .unwrap();
        assert_eq!(datei, haupt);
        assert_eq!(spiegel.text(), "xy");
        assert_eq!(spiegel.rev(), rev);
        assert_eq!(geparkt.len(), 1);
        assert_eq!(geparkt[0].datei, kapitel);
        // Derselbe Reiter noch einmal: nichts passiert.
        tauschen(
            &mut datei,
            &mut spiegel,
            &mut auf_platte,
            &mut geparkt,
            haupt.clone(),
            id,
        )
        .unwrap();
        assert_eq!(geparkt.len(), 1);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn kapitel_zum_oeffnen() {
        let o = ordner("sammeln");
        let mut liste = Vec::new();
        sammeln(&o, &o, 0, &mut liste);
        liste.sort();
        assert_eq!(liste, vec!["kapitel/eins.typ", "main.typ"]);
        assert_eq!(relativ(&o, &o.join("kapitel/eins.typ")), "kapitel/eins.typ");
        let _ = std::fs::remove_dir_all(&o);
    }
}
