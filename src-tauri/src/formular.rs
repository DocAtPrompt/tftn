//! Kern des Fachs „Dokument“ (PLAN-M2 AP7): Formular laden, Werte setzen, Einträge ändern,
//! Rückgängig und Wiederholen. Reine Funktionen auf dem Projektordner, ohne Tauri; die Befehle in
//! `lib.rs` sperren nur die Sitzung und reichen weiter.
//!
//! Jede Änderung liest `dokument.toml` frisch, vergleicht den `stand`, den die Oberfläche gesehen
//! hat, schreibt atomar und trägt den Schritt in die Historie ein. Feldwerte werden nie
//! protokolliert (`x-tftn-sensibel`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value as Json;
use tftn_core::Projekt;
use tftn_formular::{
    Daten, EintragAktion, Formular, Historie, Kontext, Meldung, ProfilFehler, Schritt,
    alles_pruefen, atomar_schreiben, formular_lesen, sensibel, stand,
};
use tftn_sprachen::tr;

/// Antwort an das Fach (JSON in snake_case).
#[derive(Debug, Serialize)]
pub struct FormularJs {
    pub stand: String,
    /// Name der Datendatei, etwa `dokument.toml`
    pub datei: String,
    /// `None`: Der Dokumenttyp hat kein Formular (oder das Schema verletzt das Profil).
    pub formular: Option<Formular>,
    pub werte: Json,
    pub meldungen: Vec<Meldung>,
    /// Die Datendatei ist kein gültiges TOML: nur lesen, nie schreiben.
    pub dateifehler: Option<String>,
    pub profilfehler: Vec<ProfilFehler>,
    pub sensibel: Option<String>,
    /// Je Feld mit einem Wert (Pfad ohne Eintragsnummer): inkognito für KI-Assistenten?
    pub inkognito: BTreeMap<String, tftn_formular::inkognito::FeldStand>,
    pub rueck: bool,
    pub vor: bool,
    /// Je `format` Vorschläge für das Eingabefeld (`literaturdatei`, `zitierstil`, `bild`).
    pub vorschlaege: BTreeMap<String, Vec<String>>,
    /// Antwort auf eine Änderung, die ungültig war: nichts geschrieben.
    pub abgelehnt: Option<Meldung>,
}

/// Gruppe „Projekt“ im Fach.
#[derive(Debug, Serialize)]
pub struct ProjektJs {
    pub typ: String,
    pub version: String,
    pub typst: String,
    pub pakete: Vec<(String, String)>,
    pub literatur_im_formular: bool,
    /// Neuere Version des Dokumenttyps in Bündel oder eigenem Ordner (PLAN-M2 AP8)
    pub aktualisierung: Option<crate::typen::NeuJs>,
}

pub fn sensibel_warnung() -> String {
    tr!(
        "Dieses Formular enthält personenbezogene Felder, und das Projekt liegt in einem Git-Ordner."
    )
}

/// Zustand des Fachs für ein offenes Projekt.
pub struct FormularSitzung {
    wurzel: PathBuf,
    datei: PathBuf,
    /// `None`: Typ ohne `formular`
    formular: Option<Result<Formular, Vec<ProfilFehler>>>,
    historie: Historie,
    /// Das Projekt nennt einen Dokumenttyp, aber sein Paket oder Manifest fehlt (dann ist auch
    /// unbekannt, welche Felder Personendaten sind).
    paket_fehlt: bool,
}

impl FormularSitzung {
    /// Liest das Schema aus dem vendorten Dokumenttyp (`[tool.tftn].formular`, `daten`).
    pub fn neu(projekt: &Projekt) -> Self {
        let fehlt = || ("dokument.toml".to_string(), None, true);
        let (daten, formular, fehlt) = match tftn_manifest::Typpaket::vom_projekt(projekt) {
            Err(_) => fehlt(),
            Ok(paket) => match paket.manifest() {
                Err(_) => fehlt(),
                Ok(m) => {
                    let daten = m.daten.clone().unwrap_or_else(|| "dokument.toml".into());
                    let formular = m.formular.as_ref().map(|pfad| {
                        let datei = paket.ordner.join(pfad);
                        match std::fs::read_to_string(&datei) {
                            Ok(json) => formular_lesen(&json),
                            Err(e) => Err(vec![ProfilFehler {
                                ort: String::new(),
                                text: tr!("{pfad} nicht lesbar: {fehler}", pfad, fehler = e),
                            }]),
                        }
                    });
                    (daten, formular, false)
                }
            },
        };
        let mut f = Self::mit(projekt.wurzel.clone(), &daten, formular);
        f.paket_fehlt = fehlt && projekt.konfig.typ.is_some();
        f
    }

    /// Lässt sich nicht sicher sagen, welche Felder Personendaten sind (Schema fehlerhaft, oder das
    /// Paket des Dokumenttyps fehlt, obwohl es Formulardaten gibt)? Dann sperrt Inkognito MCP.
    pub fn unsicher(&self) -> bool {
        matches!(self.formular, Some(Err(_))) || (self.paket_fehlt && self.datei.exists())
    }

    pub fn mit(
        wurzel: PathBuf,
        daten: &str,
        formular: Option<Result<Formular, Vec<ProfilFehler>>>,
    ) -> Self {
        Self {
            datei: wurzel.join(daten),
            wurzel,
            formular,
            historie: Historie::default(),
            paket_fehlt: false,
        }
    }

    /// Pfad der Datendatei (für den Dateiwächter).
    pub fn datei(&self) -> &Path {
        &self.datei
    }

    fn formular(&self) -> Option<&Formular> {
        self.formular.as_ref().and_then(|f| f.as_ref().ok())
    }

    /// Das Formular-Schema, wenn der Dokumenttyp eines hat und es gültig ist.
    pub fn schema(&self) -> Option<&Formular> {
        self.formular()
    }

    /// Text der Datendatei; fehlt sie, ist sie leer (das erste Schreiben legt sie an).
    fn text(&self) -> Result<String, String> {
        match std::fs::read_to_string(&self.datei) {
            Ok(t) => Ok(t),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(tr!(
                "{pfad} nicht lesbar: {fehler}",
                pfad = self.name(),
                fehler = e
            )),
        }
    }

    fn name(&self) -> String {
        self.datei
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn laden(&mut self, stile: &[&str]) -> FormularJs {
        let k = Kontext {
            wurzel: Some(&self.wurzel),
            stile,
        };
        let (text, lesefehler) = match self.text() {
            Ok(t) => (t, None),
            Err(e) => (String::new(), Some(e)),
        };
        self.historie.abgleichen(&text);
        let (werte, dateifehler) = match (&lesefehler, Daten::lesen(&text)) {
            (Some(e), _) => (Json::Object(Default::default()), Some(e.clone())),
            (None, Ok(d)) => (d.werte(), None),
            (None, Err(e)) => (
                Json::Object(Default::default()),
                Some(tr!(
                    "{datei} ist fehlerhaft ({fehler}) – mit „Als Text bearbeiten“ beheben",
                    datei = self.name(),
                    fehler = e
                )),
            ),
        };
        let formular = self.formular().cloned();
        let meldungen = match (&formular, &dateifehler) {
            (Some(f), None) => alles_pruefen(f, &werte, &k),
            _ => Vec::new(),
        };
        let (rueck, vor) = self.historie.kann();
        FormularJs {
            stand: stand(&text),
            datei: self.name(),
            sensibel: formular
                .as_ref()
                .filter(|f| sensibel(f) && im_git(&self.wurzel))
                .map(|_| sensibel_warnung()),
            inkognito: formular
                .as_ref()
                .map(|f| {
                    tftn_formular::inkognito::feld_staende(
                        f,
                        &crate::inkognito::markierungen(&self.wurzel),
                    )
                })
                .unwrap_or_default(),
            formular,
            werte,
            meldungen,
            dateifehler,
            profilfehler: match &self.formular {
                Some(Err(f)) => f.clone(),
                _ => Vec::new(),
            },
            rueck,
            vor,
            vorschlaege: vorschlaege(&self.wurzel, stile),
            abgelehnt: None,
        }
    }

    /// Der Text der Datei, wenn er noch der Stand ist, den die Oberfläche gesehen hat; sonst
    /// „veraltet“.
    fn text_im_stand(&self, stand_ui: &str) -> Result<String, String> {
        let text = self.text()?;
        if stand(&text) != stand_ui {
            return Err("veraltet".into());
        }
        Ok(text)
    }

    /// Liest frisch, prüft den Stand, wendet `aenderung` an und schreibt. Ein abgelehnter Wert
    /// kommt als `abgelehnt` mit `pfad` zurück; geschrieben wird dann nichts.
    fn aendern(
        &mut self,
        stand_ui: &str,
        pfad: Vec<Schritt>,
        stile: &[&str],
        aenderung: impl FnOnce(&mut Daten, &Formular, &Kontext) -> Result<(), String>,
    ) -> Result<FormularJs, String> {
        let text = self.text_im_stand(stand_ui)?;
        let formular = self
            .formular()
            .cloned()
            .ok_or_else(|| tr!("Dieser Dokumenttyp hat kein Formular"))?;
        let mut daten = Daten::lesen(&text).map_err(|e| {
            tr!(
                "{datei} ist fehlerhaft ({fehler}) – mit „Als Text bearbeiten“ beheben",
                datei = self.name(),
                fehler = e
            )
        })?;
        let k = Kontext {
            wurzel: Some(&self.wurzel),
            stile,
        };
        if let Err(text) = aenderung(&mut daten, &formular, &k) {
            let mut js = self.laden(stile);
            js.abgelehnt = Some(Meldung {
                pfad,
                text,
                fehler: true,
            });
            return Ok(js);
        }
        let neu = daten.text();
        if neu != text {
            atomar_schreiben(&self.datei, &neu).map_err(|e| {
                tr!(
                    "{datei} nicht geschrieben: {fehler}",
                    datei = self.name(),
                    fehler = e
                )
            })?;
            self.historie.geschrieben(text, &neu);
        }
        Ok(self.laden(stile))
    }

    pub fn setzen(
        &mut self,
        stand_ui: &str,
        pfad: Vec<Schritt>,
        wert: Option<Json>,
        stile: &[&str],
    ) -> Result<FormularJs, String> {
        let p = pfad.clone();
        self.aendern(stand_ui, pfad, stile, |d, f, k| d.setzen(f, &p, wert, k))
    }

    pub fn eintrag(
        &mut self,
        stand_ui: &str,
        pfad: Vec<Schritt>,
        aktion: EintragAktion,
        stile: &[&str],
    ) -> Result<FormularJs, String> {
        let p = pfad.clone();
        self.aendern(stand_ui, pfad, stile, |d, f, _| d.eintrag(f, &p, aktion))
    }

    /// Mehrere Werte auf einmal (MCP `formular_setzen`): `werte` wie in `dokument.toml`, Gruppen als
    /// Objekte, Eintragslisten als Arrays von Objekten (ersetzen die Liste), `null` entfernt. Liest
    /// frisch, prüft alles und schreibt einmal (ein Rückgängig-Schritt); bei einem Fehler nichts.
    pub fn mehrere_setzen(
        &mut self,
        werte: &serde_json::Map<String, Json>,
        stile: &[&str],
    ) -> Result<FormularJs, String> {
        let jetzt = stand(&self.text()?);
        let js = self.aendern(&jetzt, Vec::new(), stile, |d, f, k| {
            anwenden(d, f, &f.felder, &[], werte, k)
        })?;
        match js.abgelehnt {
            Some(m) => Err(m.text),
            None => Ok(js),
        }
    }

    /// Stand und Text der Datendatei für „Als Text bearbeiten“.
    pub fn text_lesen(&self) -> Result<(String, String), String> {
        let text = self.text()?;
        Ok((stand(&text), text))
    }

    /// Schreibt den ganzen Text („Als Text bearbeiten“), wenn er gültiges TOML ist und `stand_ui`
    /// noch gilt; ein Rückgängig-Schritt wie jede Änderung im Formular. Was das Formular am Inhalt
    /// beanstandet, zeigt es danach als Meldung – geschrieben wird trotzdem, der Text ist der Wille.
    pub fn text_setzen(
        &mut self,
        stand_ui: &str,
        neu: &str,
        stile: &[&str],
    ) -> Result<FormularJs, String> {
        let text = self.text_im_stand(stand_ui)?;
        Daten::lesen(neu).map_err(|e| {
            tr!(
                "{datei} nicht übernommen: {fehler}",
                datei = self.name(),
                fehler = e
            )
        })?;
        if neu != text {
            atomar_schreiben(&self.datei, neu).map_err(|e| {
                tr!(
                    "{datei} nicht geschrieben: {fehler}",
                    datei = self.name(),
                    fehler = e
                )
            })?;
            self.historie.geschrieben(text, neu);
        }
        Ok(self.laden(stile))
    }

    pub fn rueckgaengig(&mut self, stand_ui: &str, stile: &[&str]) -> Result<FormularJs, String> {
        self.zurueck_oder_vor(stand_ui, stile, true)
    }

    pub fn wiederholen(&mut self, stand_ui: &str, stile: &[&str]) -> Result<FormularJs, String> {
        self.zurueck_oder_vor(stand_ui, stile, false)
    }

    fn zurueck_oder_vor(
        &mut self,
        stand_ui: &str,
        stile: &[&str],
        zurueck: bool,
    ) -> Result<FormularJs, String> {
        let text = self.text_im_stand(stand_ui)?;
        // Die Historie stellt ihre Stapel vorab um; scheitert das Schreiben, gilt der alte Stand.
        let vorher = self.historie.clone();
        let ziel = if zurueck {
            self.historie.rueckgaengig(&text)
        } else {
            self.historie.wiederholen(&text)
        };
        if let Some(ziel) = ziel {
            atomar_schreiben(&self.datei, &ziel).map_err(|e| {
                self.historie = vorher;
                tr!(
                    "{datei} nicht geschrieben: {fehler}",
                    datei = self.name(),
                    fehler = e
                )
            })?;
        }
        Ok(self.laden(stile))
    }

    /// Hat das Formular die Gruppe `literatur` (Fachartikel ab 0.2.0)?
    pub fn literatur_im_formular(&self) -> bool {
        self.formular()
            .is_some_and(|f| f.felder.iter().any(|f| f.name == "literatur"))
    }
}

/// `pfad` lesbar für Meldungen an den Assistenten: `autoren[0].name`.
fn pfad_text(pfad: &[Schritt]) -> String {
    let mut t = String::new();
    for s in pfad {
        match s {
            Schritt::Name(n) if t.is_empty() => t.push_str(n),
            Schritt::Name(n) => t.push_str(&format!(".{n}")),
            Schritt::Index(i) => t.push_str(&format!("[{i}]")),
        }
    }
    t
}

/// Wendet `werte` auf die Felder `felder` unter `pfad` an (für `mehrere_setzen`).
fn anwenden(
    d: &mut Daten,
    f: &Formular,
    felder: &[tftn_formular::Feld],
    pfad: &[Schritt],
    werte: &serde_json::Map<String, Json>,
    k: &Kontext,
) -> Result<(), String> {
    use tftn_formular::Art;
    for (name, wert) in werte {
        let mut p = pfad.to_vec();
        p.push(Schritt::Name(name.clone()));
        let ort = pfad_text(&p);
        let Some(feld) = felder.iter().find(|x| &x.name == name) else {
            let bekannt: Vec<&str> = felder.iter().map(|x| x.name.as_str()).collect();
            return Err(format!(
                "{ort}: unbekanntes Feld; bekannt sind {}",
                bekannt.join(", ")
            ));
        };
        let fehler = |e: String| format!("{ort}: {e}");
        match (&feld.art, wert) {
            (Art::Gruppe { felder: unter }, Json::Object(m)) => {
                anwenden(d, f, unter, &p, m, k)?;
            }
            (Art::Gruppe { .. }, _) => {
                return Err(fehler(
                    "erwartet ein Objekt mit den Feldern der Gruppe".into(),
                ));
            }
            (Art::Eintraege { felder: unter, .. }, Json::Array(liste)) => {
                let vorher = wert_an(&d.werte(), &p)
                    .and_then(Json::as_array)
                    .map_or(0, Vec::len);
                for _ in vorher..liste.len() {
                    d.eintrag(f, &p, EintragAktion::Hinzufuegen)
                        .map_err(fehler)?;
                }
                for i in (liste.len()..vorher).rev() {
                    d.eintrag(f, &p, EintragAktion::Entfernen { index: i })
                        .map_err(fehler)?;
                }
                for (i, eintrag) in liste.iter().enumerate() {
                    let Json::Object(m) = eintrag else {
                        return Err(fehler(format!("Eintrag {i} ist kein Objekt")));
                    };
                    let mut pi = p.clone();
                    pi.push(Schritt::Index(i));
                    // Ersetzen: Was der Eintrag nicht nennt, fällt weg.
                    let mut voll = m.clone();
                    for u in unter {
                        let blatt = !matches!(u.art, Art::Gruppe { .. } | Art::Eintraege { .. });
                        if blatt && !voll.contains_key(&u.name) {
                            voll.insert(u.name.clone(), Json::Null);
                        }
                    }
                    anwenden(d, f, unter, &pi, &voll, k)?;
                }
            }
            (Art::Eintraege { .. }, _) => {
                return Err(fehler("erwartet ein Array von Objekten (Einträgen)".into()));
            }
            (_, Json::Null) => d.setzen(f, &p, None, k).map_err(fehler)?,
            _ => d.setzen(f, &p, Some(wert.clone()), k).map_err(fehler)?,
        }
    }
    Ok(())
}

/// Wert unter `pfad` in den Formularwerten.
fn wert_an<'a>(werte: &'a Json, pfad: &[Schritt]) -> Option<&'a Json> {
    pfad.iter().try_fold(werte, |w, s| match s {
        Schritt::Name(n) => w.get(n),
        Schritt::Index(i) => w.get(i),
    })
}

/// Liegt `ordner` in einem Git-Arbeitsbaum (ein Vorfahr hat `.git`)?
fn im_git(ordner: &Path) -> bool {
    ordner.ancestors().any(|o| o.join(".git").exists())
}

/// Vorschläge je Format: Literaturdateien, `.csl`-Dateien und Bilder im Projekt (zwei Ebenen tief,
/// ohne `packages/` und versteckte bzw. `_`-Ordner), dazu die eingebauten Stile.
pub fn vorschlaege(wurzel: &Path, stile: &[&str]) -> BTreeMap<String, Vec<String>> {
    let mut dateien = Vec::new();
    sammeln(wurzel, "", 2, &mut dateien);
    dateien.sort();
    let endet = |d: &String, e: &[&str]| {
        let k = d.to_lowercase();
        e.iter().any(|e| k.ends_with(e))
    };
    let literatur: Vec<String> = dateien
        .iter()
        .filter(|d| endet(d, &[".bib", ".yaml", ".yml"]))
        .cloned()
        .collect();
    let mut stil: Vec<String> = stile.iter().map(|s| s.to_string()).collect();
    stil.extend(dateien.iter().filter(|d| endet(d, &[".csl"])).cloned());
    let endungen: Vec<String> = tftn_formular::BILD_ENDUNGEN
        .iter()
        .map(|e| format!(".{e}"))
        .collect();
    let endungen: Vec<&str> = endungen.iter().map(String::as_str).collect();
    let bilder: Vec<String> = dateien
        .iter()
        .filter(|d| endet(d, &endungen))
        .cloned()
        .collect();
    BTreeMap::from([
        ("literaturdatei".to_string(), literatur),
        ("zitierstil".to_string(), stil),
        ("bild".to_string(), bilder),
    ])
}

fn sammeln(ordner: &Path, praefix: &str, tiefe: usize, aus: &mut Vec<String>) {
    let Ok(eintraege) = std::fs::read_dir(ordner) else {
        return;
    };
    for e in eintraege.filter_map(Result::ok) {
        let name = e.file_name().to_string_lossy().into_owned();
        let pfad = format!("{praefix}{name}");
        let Ok(art) = e.file_type() else { continue };
        if art.is_dir() {
            let ueberspringen =
                name.starts_with('.') || name.starts_with('_') || name == "packages";
            if tiefe > 1 && !ueberspringen {
                sammeln(&e.path(), &format!("{pfad}/"), tiefe - 1, aus);
            }
        } else if art.is_file() {
            aus.push(pfad);
        }
    }
}

/// Gruppe „Projekt“: Dokumenttyp und Version, typst-Version, Pakete im Projekt.
pub fn projekt_js(
    projekt: &Projekt,
    formular: &FormularSitzung,
    quellen: &[tftn_manifest::typen::Typquelle],
) -> ProjektJs {
    let typ = projekt.konfig.typ.clone().unwrap_or_default();
    let ohne = typ.strip_prefix("@local/").unwrap_or(&typ);
    let (name, version) = ohne.split_once(':').unwrap_or((ohne, ""));
    ProjektJs {
        typ: name.to_string(),
        version: version.to_string(),
        typst: projekt.konfig.typst.clone(),
        pakete: tftn_katalog::projektpakete(&projekt.pakete())
            .into_iter()
            .map(|(n, v)| (n, v.to_string()))
            .collect(),
        literatur_im_formular: formular.literatur_im_formular(),
        aktualisierung: crate::typen::neue_version_js(projekt, quellen),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;
    use tftn_formular::Schritt;

    const STILE: &[&str] = &["american-psychological-association", "ieee"];

    fn buendel() -> Vec<tftn_manifest::typen::Typquelle> {
        vec![tftn_manifest::typen::Typquelle {
            ordner: Path::new(env!("CARGO_MANIFEST_DIR")).join("../types"),
            herkunft: tftn_manifest::typen::Herkunft::Tftn,
        }]
    }

    /// Neues Projekt aus `types/` (Fachartikel 0.3.0) in einem eigenen Temp-Ordner.
    fn projekt(name: &str) -> Projekt {
        let ordner =
            std::env::temp_dir().join(format!("tftn-app-formular-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ordner);
        tftn_manifest::anlegen::projekt_anlegen(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../types"),
            "tftn-fachartikel",
            &ordner,
            Some(1_793_404_800),
        )
        .unwrap()
        .projekt
    }

    fn titel() -> Vec<Schritt> {
        vec![Schritt::Name("titel".into())]
    }

    fn lesen(p: &Projekt) -> String {
        std::fs::read_to_string(p.wurzel.join("dokument.toml")).unwrap()
    }

    #[test]
    fn laden_ohne_fehler() {
        let p = projekt("laden");
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        assert!(js.formular.is_some());
        assert!(js.meldungen.is_empty(), "{:?}", js.meldungen);
        assert!(js.profilfehler.is_empty());
        assert!(js.dateifehler.is_none());
        assert!(!js.rueck && !js.vor);
        assert_eq!(js.datei, "dokument.toml");
        assert_eq!(js.werte["titel"], "Titel des Artikels");
    }

    #[test]
    fn setzen_schreibt_und_historie() {
        let p = projekt("setzen");
        let mut f = FormularSitzung::neu(&p);
        let vorher = lesen(&p);
        let js = f.laden(STILE);
        let js = f
            .setzen(&js.stand, titel(), Some(json!("Neu")), STILE)
            .unwrap();
        assert!(js.abgelehnt.is_none());
        assert!(lesen(&p).contains(r#"titel = "Neu""#));
        assert!(js.rueck);
        let js = f.rueckgaengig(&js.stand, STILE).unwrap();
        assert_eq!(lesen(&p), vorher);
        assert!(!js.rueck && js.vor);
        let js = f.wiederholen(&js.stand, STILE).unwrap();
        assert!(lesen(&p).contains(r#"titel = "Neu""#));
        assert!(js.rueck);
    }

    /// Scheitert das Zurückschreiben (Sperre, keine Rechte), bleibt die Historie, wie sie war.
    #[cfg(unix)]
    #[test]
    fn historie_bleibt_wenn_schreiben_scheitert() {
        use std::os::unix::fs::PermissionsExt;
        let p = projekt("schreibfehler");
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        let js = f
            .setzen(&js.stand, titel(), Some(json!("Neu")), STILE)
            .unwrap();
        let rechte =
            |m| std::fs::set_permissions(&p.wurzel, std::fs::Permissions::from_mode(m)).unwrap();
        rechte(0o555);
        let fehler = f.rueckgaengig(&js.stand, STILE);
        rechte(0o755);
        assert!(fehler.is_err());
        let js = f.laden(STILE);
        assert!(js.rueck, "Historie nach Schreibfehler leer");
        f.rueckgaengig(&js.stand, STILE).unwrap();
        assert!(!lesen(&p).contains("Neu"));
    }

    #[test]
    fn setzen_veraltet() {
        let p = projekt("veraltet");
        let mut f = FormularSitzung::neu(&p);
        let alt = f.laden(STILE).stand;
        std::fs::write(p.wurzel.join("dokument.toml"), lesen(&p) + "# fremd\n").unwrap();
        let davor = lesen(&p);
        assert_eq!(
            f.setzen(&alt, titel(), Some(json!("Neu")), STILE).err(),
            Some("veraltet".into())
        );
        assert_eq!(lesen(&p), davor);
    }

    #[test]
    fn historie_nach_fremder_aenderung_leer() {
        let p = projekt("fremd");
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        f.setzen(&js.stand, titel(), Some(json!("Neu")), STILE)
            .unwrap();
        std::fs::write(
            p.wurzel.join("dokument.toml"),
            lesen(&p).replace("Neu", "Fremd"),
        )
        .unwrap();
        let js = f.laden(STILE);
        assert!(!js.rueck);
        let js = f.rueckgaengig(&js.stand, STILE).unwrap();
        assert!(lesen(&p).contains("Fremd"));
        assert!(!js.rueck);
    }

    /// „Als Text bearbeiten“: schreibt den ganzen Text, Rückgängig gilt auch dafür.
    #[test]
    fn text_setzen_schreibt_und_ist_rueckgaengig() {
        let p = projekt("text");
        let mut f = FormularSitzung::neu(&p);
        let vorher = lesen(&p);
        let (stand_ui, text) = f.text_lesen().unwrap();
        assert_eq!(text, vorher);
        let neu = text.replace("Titel des Artikels", "Von Hand");
        let js = f.text_setzen(&stand_ui, &neu, STILE).unwrap();
        assert_eq!(lesen(&p), neu);
        assert_eq!(js.werte["titel"], "Von Hand");
        assert!(js.rueck);
        f.rueckgaengig(&js.stand, STILE).unwrap();
        assert_eq!(lesen(&p), vorher);
    }

    /// Kaputtes TOML oder ein veralteter Stand: nichts geschrieben, eine Meldung mit Zeile.
    #[test]
    fn text_setzen_lehnt_ab() {
        let p = projekt("text-ab");
        let mut f = FormularSitzung::neu(&p);
        let vorher = lesen(&p);
        let (stand_ui, _) = f.text_lesen().unwrap();
        let fehler = f.text_setzen(&stand_ui, "titel = ", STILE).unwrap_err();
        assert!(fehler.contains("Zeile 1"), "{fehler}");
        assert_eq!(lesen(&p), vorher);
        assert_eq!(
            f.text_setzen("alt", "titel = \"x\"\n", STILE).err(),
            Some("veraltet".into())
        );
        assert_eq!(lesen(&p), vorher);
    }

    /// Eine kaputte Datei lässt sich als Text reparieren.
    #[test]
    fn text_setzen_repariert_kaputte_datei() {
        let p = projekt("text-heil");
        std::fs::write(p.wurzel.join("dokument.toml"), "titel = ").unwrap();
        let mut f = FormularSitzung::neu(&p);
        let (stand_ui, text) = f.text_lesen().unwrap();
        assert_eq!(text, "titel = ");
        let js = f
            .text_setzen(&stand_ui, "titel = \"Heil\"\n", STILE)
            .unwrap();
        assert!(js.dateifehler.is_none());
        assert_eq!(js.werte["titel"], "Heil");
    }

    #[test]
    fn kaputte_datei_nur_lesen() {
        let p = projekt("kaputt");
        std::fs::write(p.wurzel.join("dokument.toml"), "titel = ").unwrap();
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        let fehler = js.dateifehler.clone().unwrap();
        assert!(fehler.contains("Zeile 1"), "{fehler}");
        assert!(
            f.setzen(&js.stand, titel(), Some(json!("Neu")), STILE)
                .is_err()
        );
        assert_eq!(lesen(&p), "titel = ");
    }

    #[test]
    fn ungueltig_abgelehnt() {
        let p = projekt("ungueltig");
        let mut f = FormularSitzung::neu(&p);
        let vorher = lesen(&p);
        let js = f.laden(STILE);
        let js = f
            .setzen(
                &js.stand,
                vec![Schritt::Name("datum".into())],
                Some(json!("x")),
                STILE,
            )
            .unwrap();
        let a = js.abgelehnt.unwrap();
        assert_eq!(a.text, "kein gültiges Datum");
        assert_eq!(a.pfad, vec![Schritt::Name("datum".into())]);
        assert_eq!(lesen(&p), vorher);
        assert!(!js.rueck);
    }

    #[test]
    fn eintrag_hinzufuegen() {
        let p = projekt("eintrag");
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        let js = f
            .eintrag(
                &js.stand,
                vec![Schritt::Name("autoren".into())],
                tftn_formular::EintragAktion::Hinzufuegen,
                STILE,
            )
            .unwrap();
        assert_eq!(js.werte["autoren"].as_array().unwrap().len(), 2);
        // Der neue Autor hat einen leeren Namen: nur ein Hinweis.
        assert!(js.meldungen.iter().all(|m| !m.fehler), "{:?}", js.meldungen);
        assert_eq!(js.meldungen.len(), 1);
    }

    #[test]
    fn testartikel_ohne_literaturfelder() {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel");
        let p = Projekt::oeffnen(&wurzel).unwrap();
        let mut f = FormularSitzung::neu(&p);
        let js = f.laden(STILE);
        let formular = js.formular.unwrap();
        assert!(formular.felder.iter().all(|f| f.name != "literatur"));
        let info = projekt_js(&p, &f, &buendel());
        assert!(!info.literatur_im_formular);
        assert_eq!(info.typ, "tftn-fachartikel");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.typst, "0.15.1");
        let neu = projekt("info");
        assert!(projekt_js(&neu, &FormularSitzung::neu(&neu), &buendel()).literatur_im_formular);
        assert!(
            info.aktualisierung.is_some(),
            "Testartikel 0.1.0: 0.3.0 verfügbar"
        );
    }

    #[test]
    fn sensibel_im_git_ordner() {
        let o = std::env::temp_dir().join(format!("tftn-app-sensibel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(o.join("projekt")).unwrap();
        let formular = tftn_formular::formular_lesen(
            r#"{ "type": "object", "properties": { "sv": { "type": "string", "x-tftn-sensibel": true } } }"#,
        )
        .unwrap();
        let mut f = FormularSitzung::mit(o.join("projekt"), "dokument.toml", Some(Ok(formular)));
        assert!(f.laden(STILE).sensibel.is_none());
        std::fs::create_dir_all(o.join(".git")).unwrap();
        assert!(f.laden(STILE).sensibel.is_some());
        let _ = std::fs::remove_dir_all(&o);
    }

    /// Die JSON-Form, die ui/src/formular.ts erwartet (Felder in kebab-case, Antwort in snake_case).
    #[test]
    fn json_form_wie_oberflaeche() {
        let p = projekt("json");
        let mut f = FormularSitzung::neu(&p);
        let js = serde_json::to_value(f.laden(STILE)).unwrap();
        for k in [
            "stand",
            "datei",
            "formular",
            "werte",
            "meldungen",
            "dateifehler",
            "profilfehler",
            "sensibel",
            "rueck",
            "vor",
            "vorschlaege",
            "abgelehnt",
        ] {
            assert!(js.get(k).is_some(), "{k} fehlt");
        }
        let titel = &js["formular"]["felder"][0];
        assert_eq!(titel["name"], "titel");
        assert_eq!(titel["art"]["art"], "text");
        assert_eq!(titel["art"]["widget"], "einzeilig");
        assert!(titel["art"].get("min-laenge").is_some());
        let autoren = &js["formular"]["felder"][2];
        assert_eq!(autoren["art"]["art"], "eintraege");
        let a: tftn_formular::EintragAktion =
            serde_json::from_value(json!({ "art": "verschieben", "index": 1, "nach": 0 })).unwrap();
        assert_eq!(
            a,
            tftn_formular::EintragAktion::Verschieben { index: 1, nach: 0 }
        );
        let info = serde_json::to_value(projekt_js(&p, &f, &buendel())).unwrap();
        assert!(info.get("literatur_im_formular").is_some());
    }

    #[test]
    fn vorschlaege_literatur() {
        let o = std::env::temp_dir().join(format!("tftn-app-vorschlaege-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(o.join("b")).unwrap();
        std::fs::create_dir_all(o.join("packages/x")).unwrap();
        for d in [
            "a.bib",
            "b/c.yaml",
            "x.txt",
            "packages/x/p.bib",
            "eigen.csl",
            "b/logo.PNG",
            "packages/x/bild.png",
        ] {
            std::fs::write(o.join(d), "").unwrap();
        }
        let v = vorschlaege(&o, STILE);
        assert_eq!(v["literaturdatei"], ["a.bib", "b/c.yaml"]);
        assert_eq!(v["bild"], ["b/logo.PNG"]);
        assert!(v["zitierstil"].contains(&"ieee".to_string()));
        assert!(v["zitierstil"].contains(&"eigen.csl".to_string()));
        let _ = std::fs::remove_dir_all(&o);
    }

    fn werte(j: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        j.as_object().unwrap().clone()
    }

    #[test]
    fn mehrere_setzen_mit_gruppe_und_eintraegen() {
        let p = projekt("mehrere");
        let mut f = FormularSitzung::neu(&p);
        let js = f
            .mehrere_setzen(
                &werte(json!({
                    "titel": "Ein Testartikel",
                    "datum": "2026-11-16",
                    "schlagworte": ["eins", "zwei", "drei"],
                    "autoren": [
                        { "name": "Erika Example", "institution": ["Institut A"] },
                        { "name": "Max Sample" },
                    ],
                    "literatur": { "stil": "ieee" },
                })),
                STILE,
            )
            .unwrap();
        assert!(js.abgelehnt.is_none());
        let w = &js.werte;
        assert_eq!(w["titel"], "Ein Testartikel");
        assert_eq!(w["datum"], "2026-11-16");
        assert_eq!(w["schlagworte"], json!(["eins", "zwei", "drei"]));
        assert_eq!(w["autoren"].as_array().unwrap().len(), 2);
        assert_eq!(w["autoren"][0]["name"], "Erika Example");
        assert_eq!(w["autoren"][0]["institution"], json!(["Institut A"]));
        // Was beim Eintrag fehlt, fällt weg (auch „korrespondierend“ der Vorlage)
        assert!(w["autoren"][0].get("korrespondierend").is_none());
        assert_eq!(w["autoren"][1]["name"], "Max Sample");
        assert_eq!(w["literatur"]["stil"], "ieee");
        assert_eq!(
            w["literatur"]["dateien"],
            json!(["literatur.bib"]),
            "Rest der Gruppe bleibt"
        );
        assert!(js.rueck, "ein Rückgängig-Schritt");
        let js = f.rueckgaengig(&js.stand, STILE).unwrap();
        assert_eq!(
            js.werte["titel"], "Titel des Artikels",
            "alles in einem Schritt zurück"
        );
        // Weniger Einträge als vorher
        let js = f
            .mehrere_setzen(&werte(json!({ "autoren": [{ "name": "Allein" }] })), STILE)
            .unwrap();
        assert_eq!(js.werte["autoren"], json!([{ "name": "Allein" }]));
    }

    #[test]
    fn mehrere_setzen_schreibt_bei_fehler_nichts() {
        let p = projekt("mehrere-fehler");
        let mut f = FormularSitzung::neu(&p);
        let vorher = lesen(&p);
        let e = f
            .mehrere_setzen(&werte(json!({ "titel": "Neu", "gibtsnicht": 1 })), STILE)
            .unwrap_err();
        assert!(e.contains("gibtsnicht") && e.contains("titel"), "{e}");
        let e = f
            .mehrere_setzen(&werte(json!({ "titel": "Neu", "datum": "gestern" })), STILE)
            .unwrap_err();
        assert!(e.contains("datum"), "{e}");
        let e = f
            .mehrere_setzen(&werte(json!({ "autoren": [] })), STILE)
            .unwrap_err();
        assert!(e.contains("autoren"), "{e}");
        assert_eq!(lesen(&p), vorher);
        // `null` entfernt ein Feld
        let js = f
            .mehrere_setzen(&werte(json!({ "untertitel": "U" })), STILE)
            .unwrap();
        assert_eq!(js.werte["untertitel"], "U");
        let js = f
            .mehrere_setzen(&werte(json!({ "untertitel": null })), STILE)
            .unwrap();
        assert!(js.werte.get("untertitel").is_none());
    }
}
