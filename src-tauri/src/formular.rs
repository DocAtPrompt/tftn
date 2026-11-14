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
    pub rueck: bool,
    pub vor: bool,
    /// Je `format` Vorschläge für das Eingabefeld (`literaturdatei`, `zitierstil`).
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
}

impl FormularSitzung {
    /// Liest das Schema aus dem vendorten Dokumenttyp (`[tool.tftn].formular`, `daten`).
    pub fn neu(projekt: &Projekt) -> Self {
        let (daten, formular) = match tftn_manifest::Typpaket::vom_projekt(projekt) {
            Err(_) => ("dokument.toml".to_string(), None),
            Ok(paket) => match paket.manifest() {
                Err(_) => ("dokument.toml".to_string(), None),
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
                    (daten, formular)
                }
            },
        };
        Self::mit(projekt.wurzel.clone(), &daten, formular)
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
        }
    }

    /// Pfad der Datendatei (für den Dateiwächter).
    pub fn datei(&self) -> &Path {
        &self.datei
    }

    fn formular(&self) -> Option<&Formular> {
        self.formular.as_ref().and_then(|f| f.as_ref().ok())
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
                    "{datei} ist fehlerhaft ({fehler}) – bitte im Texteditor beheben",
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

    /// Liest frisch, prüft den Stand, wendet `aenderung` an und schreibt. Ein abgelehnter Wert
    /// kommt als `abgelehnt` mit `pfad` zurück; geschrieben wird dann nichts.
    fn aendern(
        &mut self,
        stand_ui: &str,
        pfad: Vec<Schritt>,
        stile: &[&str],
        aenderung: impl FnOnce(&mut Daten, &Formular, &Kontext) -> Result<(), String>,
    ) -> Result<FormularJs, String> {
        let text = self.text()?;
        if stand(&text) != stand_ui {
            return Err("veraltet".into());
        }
        let formular = self
            .formular()
            .cloned()
            .ok_or_else(|| tr!("Dieser Dokumenttyp hat kein Formular"))?;
        let mut daten = Daten::lesen(&text).map_err(|e| {
            tr!(
                "{datei} ist fehlerhaft ({fehler}) – bitte im Texteditor beheben",
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
        let text = self.text()?;
        if stand(&text) != stand_ui {
            return Err("veraltet".into());
        }
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

/// Liegt `ordner` in einem Git-Arbeitsbaum (ein Vorfahr hat `.git`)?
fn im_git(ordner: &Path) -> bool {
    ordner.ancestors().any(|o| o.join(".git").exists())
}

/// Vorschläge je Format: Literaturdateien und `.csl`-Dateien im Projekt (zwei Ebenen tief, ohne
/// `packages/` und versteckte bzw. `_`-Ordner), dazu die eingebauten Stile.
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
    BTreeMap::from([
        ("literaturdatei".to_string(), literatur),
        ("zitierstil".to_string(), stil),
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
        ] {
            std::fs::write(o.join(d), "").unwrap();
        }
        let v = vorschlaege(&o, STILE);
        assert_eq!(v["literaturdatei"], ["a.bib", "b/c.yaml"]);
        assert!(v["zitierstil"].contains(&"ieee".to_string()));
        assert!(v["zitierstil"].contains(&"eigen.csl".to_string()));
        let _ = std::fs::remove_dir_all(&o);
    }
}
