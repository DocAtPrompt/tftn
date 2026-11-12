//! Helfer-Katalog (Entwurf 2026-11-04 Abschnitt 5, PLAN-M2 AP1): alle Bausteine eines Projekts in einer
//! gemeinsamen Beschreibung, aus Typst selbst, dem Dokumenttyp und Paketbeschreibungen.

pub mod beschreibung;
pub mod einfuegen;
pub mod ide;
pub mod mathe;
pub mod ort;
pub mod pruefen;
pub mod schnipsel;

use std::collections::BTreeMap;
use tftn_sprachen::tr;

use tftn_manifest::{Modus, Parameter};

pub use beschreibung::{Beschreibung, beschreibungen_lesen, projektpakete};
pub use einfuegen::{Einfuegung, import_fuer, importzeile};
pub use mathe::{Symbol, SymbolGruppe, symbole};
pub use ort::modus_an;
pub use pruefen::{Befund, obere_importe, pruefen};
pub use schnipsel::{Eingesetzt, Feld, Schnipsel, SchnipselFehler, Teil};

/// Woher ein Baustein stammt; bestimmt den Namensraum seiner Id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Herkunft {
    Typst,
    /// `spec` wie `@local/tftn-fachartikel:0.3.0`
    Dokumenttyp {
        spec: String,
    },
    /// `import` ist die fertige Importzeile mit der Version im Projekt.
    Paket {
        name: String,
        version: semver::Version,
        import: String,
        /// Weitere nötige Importe: Paketname und fertige Zeile
        auch: Vec<(String, String)>,
    },
}

/// Ein Baustein im Katalog.
#[derive(Debug, Clone)]
pub struct Eintrag {
    /// `typst/abbildung`, `tftn-fachartikel/hinweis`, `@preview/fletcher/diagramm`
    pub id: String,
    pub titel: String,
    pub gruppe: Option<String>,
    pub herkunft: Herkunft,
    /// Typst-Funktion für Marken, Seitenleiste und Vorschau; `None` bei reiner Markup-Syntax.
    pub funktion: Option<String>,
    pub schnipsel: Schnipsel,
    pub modus: Vec<Modus>,
    pub kuerzel: Option<String>,
    pub parameter: BTreeMap<String, Parameter>,
}

/// `[[baustein]]` in `eingebaut.toml`: wie im Manifest, nur `funktion` optional.
#[derive(serde::Deserialize)]
struct Eingebaut {
    baustein: Vec<EingebautBaustein>,
}

#[derive(serde::Deserialize)]
struct EingebautBaustein {
    id: String,
    titel: String,
    funktion: Option<String>,
    einfuegen: String,
    gruppe: Option<String>,
    kuerzel: Option<String>,
    #[serde(default = "nur_markup")]
    modus: Vec<Modus>,
    #[serde(default)]
    parameter: BTreeMap<String, Parameter>,
}

fn nur_markup() -> Vec<Modus> {
    vec![Modus::Markup]
}

/// Die mitgelieferten Bausteine aus Typst selbst (`eingebaut.toml`).
pub fn eingebaut() -> Vec<Eintrag> {
    eingebaut_lesen(include_str!("../eingebaut.toml"))
}

fn eingebaut_lesen(text: &str) -> Vec<Eintrag> {
    let datei: Eingebaut = toml::from_str(text).expect("eingebaut.toml ist fehlerhaft");
    datei
        .baustein
        .into_iter()
        .map(|b| Eintrag {
            schnipsel: Schnipsel::lesen(&b.einfuegen)
                .unwrap_or_else(|f| panic!("eingebaut.toml, {}: {}", b.id, f.grund)),
            id: format!("typst/{}", b.id),
            titel: b.titel,
            gruppe: b.gruppe,
            herkunft: Herkunft::Typst,
            funktion: b.funktion,
            modus: b.modus,
            kuerzel: b.kuerzel,
            parameter: b.parameter,
        })
        .collect()
}

/// Woraus der Katalog eines Projekts gebaut wird.
pub struct Quellen<'a> {
    pub projekt: Option<&'a tftn_core::Projekt>,
    /// Ordner mit Paketbeschreibungen; der erste hat Vorrang (eigener vor mitgeliefertem).
    pub beschreibungen: &'a [std::path::PathBuf],
}

/// Ein Parameter mit Widget und den Anzeigenamen für die Oberfläche.
#[derive(Debug, Clone)]
pub struct WidgetEintrag {
    pub parameter: tftn_edit::WidgetParameter,
    /// Titel des Bausteins
    pub baustein: String,
    /// Titel des Parameters (sonst sein Name)
    pub titel: String,
    /// Hilfe-Text aus dem Manifest
    pub hilfe: Option<String>,
    /// Standardwert aus dem Manifest (als Text, zur Anzeige)
    pub standard: Option<String>,
}

/// Alle Bausteine eines Projekts: Typst, dann Dokumenttyp, dann Pakete.
#[derive(Debug, Clone, Default)]
pub struct Katalog {
    eintraege: Vec<Eintrag>,
    holbar: Vec<Holbar>,
    beschreibungen: Vec<Beschreibung>,
}

/// Ein Paket mit Beschreibung, das im Projekt fehlt und sich holen lässt (PLAN-M2 AP10, Festlegung 9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holbar {
    pub paket: String,
    /// `@preview/<name>:<version>`
    pub spec: String,
    pub titel: String,
    /// Titel der Bausteine
    pub bausteine: Vec<String>,
}

impl Katalog {
    /// Baut den Katalog; Fehler in einzelnen Quellen werden zu Warnungen, nie zum Abbruch.
    pub fn bauen(q: &Quellen) -> (Katalog, Vec<String>) {
        let mut eintraege = eingebaut();
        let mut warnungen = Vec::new();
        let mut holbar = Vec::new();
        let mut beschreibungen = Vec::new();
        if let Some(projekt) = q.projekt {
            match tftn_manifest::Typpaket::vom_projekt(projekt)
                .and_then(|p| Ok((p.spec(), p.toml.package.name.clone(), p.manifest()?.clone())))
            {
                Ok((spec, name, manifest)) => {
                    let herkunft = Herkunft::Dokumenttyp { spec };
                    for b in &manifest.baustein {
                        match aus_manifest(b, &name, &herkunft) {
                            Ok(e) => hinzufuegen(&mut eintraege, e, &mut warnungen),
                            Err(w) => warnungen.push(w),
                        }
                    }
                }
                Err(e) => warnungen.push(tr!("Dokumenttyp: {fehler}", fehler = e)),
            }
            beschreibungen = alle_beschreibungen(q.beschreibungen, &mut warnungen);
            pakete_einlesen(projekt, &beschreibungen, &mut eintraege, &mut warnungen);
            holbar = holbare(projekt, &beschreibungen);
        }
        (
            Katalog {
                eintraege,
                holbar,
                beschreibungen,
            },
            warnungen,
        )
    }

    pub fn eintraege(&self) -> &[Eintrag] {
        &self.eintraege
    }

    /// Pakete mit Beschreibung, die im Projekt ganz fehlen.
    pub fn holbar(&self) -> &[Holbar] {
        &self.holbar
    }

    /// Passt eine Beschreibung zu genau dieser Version des Pakets?
    pub fn hat_helfer(&self, name: &str, version: &semver::Version) -> bool {
        self.beschreibungen
            .iter()
            .any(|b| b.paket == name && b.versionen.matches(version))
    }

    /// Gibt es überhaupt eine Beschreibung des Pakets (gleich für welche Version)?
    pub fn hat_beschreibung(&self, name: &str) -> bool {
        self.beschreibungen.iter().any(|b| b.paket == name)
    }

    pub fn finden(&self, id: &str) -> Option<&Eintrag> {
        self.eintraege.iter().find(|e| e.id == id)
    }

    /// Je Baustein mit Funktion und je Parameter ein Widget (ersetzt die Liste aus M1).
    pub fn widgets(&self) -> Vec<WidgetEintrag> {
        self.eintraege
            .iter()
            .filter_map(|e| e.funktion.as_ref().map(|f| (e, f)))
            .flat_map(|(e, funktion)| {
                e.parameter.iter().map(move |(name, p)| WidgetEintrag {
                    parameter: tftn_edit::WidgetParameter {
                        funktion: funktion.clone(),
                        parameter: name.clone(),
                        art: widget_art(p),
                        position: p.position,
                    },
                    baustein: e.titel.clone(),
                    titel: p.titel.clone().unwrap_or_else(|| name.clone()),
                    hilfe: p.hilfe.clone(),
                    standard: p.standard.clone(),
                })
            })
            .collect()
    }
}

/// Widget-Art eines Manifest-Parameters mit seinen Zusatzschlüsseln (KONZEPT 5.4).
pub fn widget_art(p: &Parameter) -> tftn_edit::WidgetArt {
    use tftn_edit::{Farbraum, WidgetArt};
    match p.widget.as_str() {
        "text" => WidgetArt::Text {
            mehrzeilig: p.mehrzeilig,
        },
        "zahl" => WidgetArt::Zahl {
            min: p.min,
            max: p.max,
            ganzzahl: p.ganzzahl,
        },
        "laenge" => WidgetArt::Laenge {
            einheiten: p.einheiten.clone(),
        },
        "prozent" => WidgetArt::Prozent {
            min: p.min,
            max: p.max,
        },
        "schalter" => WidgetArt::Schalter,
        "wahl" => WidgetArt::Wahl {
            werte: p.werte.clone(),
        },
        "farbe" => WidgetArt::Farbe {
            raum: match p.raum.as_deref() {
                Some("cmyk") => Farbraum::Cmyk,
                _ => Farbraum::Rgb,
            },
        },
        "datum" => WidgetArt::Datum,
        "label" => WidgetArt::Label,
        "datei" => WidgetArt::Datei {
            endungen: p.endungen.clone(),
        },
        "punkt" => WidgetArt::Punkt,
        "liste" => {
            let element = Parameter {
                widget: p.element.clone().unwrap_or_else(|| "text".into()),
                element: None,
                ..p.clone()
            };
            WidgetArt::Liste {
                element: Box::new(widget_art(&element)),
            }
        }
        "inhalt" => WidgetArt::Inhalt { rumpf: p.rumpf },
        "tabelle" => WidgetArt::Tabelle,
        andere => WidgetArt::Unbekannt(andere.into()),
    }
}

/// Fügt `e` an, außer seine Id ist schon vergeben: dann bleibt der erste, mit Warnung.
fn hinzufuegen(eintraege: &mut Vec<Eintrag>, e: Eintrag, warnungen: &mut Vec<String>) {
    if eintraege.iter().any(|x| x.id == e.id) {
        let quelle = match &e.herkunft {
            Herkunft::Paket { version, .. } => tr!(" (Version {version})", version),
            _ => String::new(),
        };
        warnungen.push(tr!(
            "{id} doppelt{quelle}, der erste bleibt",
            id = e.id,
            quelle
        ));
    } else {
        eintraege.push(e);
    }
}

/// Eintrag aus einem Manifest- oder Beschreibungs-Baustein; Id `<namensraum>/<id>`.
fn aus_manifest(
    b: &tftn_manifest::Baustein,
    namensraum: &str,
    herkunft: &Herkunft,
) -> Result<Eintrag, String> {
    let schnipsel = Schnipsel::lesen(&b.einfuegen).map_err(|f| {
        tr!(
            "{id}: {grund} (Stelle {stelle})",
            id = b.id,
            grund = f.grund,
            stelle = f.stelle
        )
    })?;
    Ok(Eintrag {
        id: format!("{namensraum}/{}", b.id),
        titel: b.titel.clone(),
        gruppe: b.gruppe.clone(),
        herkunft: herkunft.clone(),
        funktion: Some(b.funktion.clone()),
        schnipsel,
        modus: b.modus.clone(),
        kuerzel: b.kuerzel.clone(),
        parameter: b.parameter.clone(),
    })
}

fn alle_beschreibungen(
    ordner: &[std::path::PathBuf],
    warnungen: &mut Vec<String>,
) -> Vec<Beschreibung> {
    let mut alle = Vec::new();
    for o in ordner {
        let (b, w) = beschreibungen_lesen(o);
        alle.extend(b);
        warnungen.extend(w);
    }
    alle
}

/// Je Paket ohne Version im Projekt die erste Beschreibung mit `titel` und `empfohlen`.
fn holbare(projekt: &tftn_core::Projekt, alle: &[Beschreibung]) -> Vec<Holbar> {
    let da = projektpakete(&projekt.wurzel);
    let direkt = tftn_pakete::projektimporte(&projekt.wurzel);
    let mut aus: Vec<Holbar> = Vec::new();
    for b in alle {
        let (Some(titel), Some(empfohlen)) = (&b.titel, &b.empfohlen) else {
            continue;
        };
        // Eine Version außerhalb des Bereichs zählt nur, wenn das Projekt sie selbst importiert;
        // als Abhängigkeit eines anderen Pakets (fletcher bringt ein altes cetz mit) stört sie nicht.
        let passend = da
            .iter()
            .any(|(n, v)| *n == b.paket && b.versionen.matches(v));
        if passend
            || direkt.iter().any(|s| s.name == b.paket)
            || aus.iter().any(|h| h.paket == b.paket)
        {
            continue;
        }
        aus.push(Holbar {
            paket: b.paket.clone(),
            spec: format!("@preview/{}:{empfohlen}", b.paket),
            titel: titel.clone(),
            bausteine: b.baustein.iter().map(|x| x.titel.clone()).collect(),
        });
    }
    aus
}

/// Die `auch`-Zeilen von `b` für `version`: die Version, die das Paket selbst importiert, sonst die
/// höchste im Projekt. `Err` nennt das fehlende Paket.
fn auch_zeilen(
    projekt: &tftn_core::Projekt,
    b: &Beschreibung,
    version: &semver::Version,
    pakete: &[(String, semver::Version)],
) -> Result<Vec<(String, String)>, String> {
    let ordner = projekt
        .pakete()
        .join("preview")
        .join(&b.paket)
        .join(version.to_string());
    let importe = tftn_pakete::importe(&ordner)
        .map(|(i, _)| i)
        .unwrap_or_default();
    b.auch
        .iter()
        .map(|name| {
            let genutzt = importe
                .iter()
                .filter(|s| s.name == *name)
                .map(|s| s.version.clone())
                .max();
            let v = genutzt
                .filter(|v| pakete.iter().any(|(n, x)| n == name && x == v))
                .or_else(|| {
                    pakete
                        .iter()
                        .filter(|(n, _)| n == name)
                        .map(|(_, x)| x.clone())
                        .max()
                })
                .ok_or_else(|| {
                    tr!(
                        "{paket} braucht {name}, das im Projekt fehlt",
                        paket = b.paket,
                        name
                    )
                })?;
            Ok((name.clone(), format!("#import \"@preview/{name}:{v}\"")))
        })
        .collect()
}

/// Je Universe-Paket im Projekt die erste passende Beschreibung in Ordner-Reihenfolge.
fn pakete_einlesen(
    projekt: &tftn_core::Projekt,
    alle: &[Beschreibung],
    eintraege: &mut Vec<Eintrag>,
    warnungen: &mut Vec<String>,
) {
    // Neueste Version zuerst: Liegen mehrere im Projekt, zählt je Paket nur sie.
    let mut pakete = projektpakete(&projekt.wurzel);
    pakete.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    let vorhanden = pakete.clone();
    let mut beschrieben: Vec<String> = Vec::new();
    for (name, version) in pakete {
        if beschrieben.contains(&name) {
            continue;
        }
        let passend: Vec<&Beschreibung> = alle
            .iter()
            .filter(|b| b.paket == name && b.versionen.matches(&version))
            .collect();
        if passend.is_empty() {
            continue;
        }
        beschrieben.push(name.clone());
        // Spätere Dateien ergänzen; je Id gewinnt die erste (Entwurf 5).
        let mut quelle_der_id: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();
        for b in passend {
            let auch = match auch_zeilen(projekt, b, &version, &vorhanden) {
                Ok(z) => z,
                Err(w) => {
                    warnungen.push(format!("{}: {w}", b.datei.display()));
                    continue;
                }
            };
            let herkunft = Herkunft::Paket {
                name: name.clone(),
                version: version.clone(),
                import: b.import.replace("{version}", &version.to_string()),
                auch,
            };
            for baustein in &b.baustein {
                match aus_manifest(baustein, &format!("@preview/{name}"), &herkunft) {
                    Ok(e) => match quelle_der_id.get(&e.id) {
                        Some(erste) if *erste != b.datei => warnungen.push(tr!(
                            "{erste} überdeckt {zweite} bei {id}",
                            erste = erste.display(),
                            zweite = b.datei.display(),
                            id = e.id
                        )),
                        _ => {
                            quelle_der_id.insert(e.id.clone(), b.datei.clone());
                            hinzufuegen(eintraege, e, warnungen);
                        }
                    },
                    Err(w) => warnungen.push(format!("{}: {w}", b.datei.display())),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebaute_ids_eindeutig_und_vollstaendig() {
        let e = eingebaut();
        let ids: Vec<_> = e.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), 36);
        assert!(ids.contains(&"typst/mathe-bruch"));
        assert!(ids.contains(&"typst/abbildung") && ids.contains(&"typst/inhaltsverzeichnis"));
        let mut sortiert = ids.clone();
        sortiert.sort();
        sortiert.dedup();
        assert_eq!(sortiert.len(), ids.len());
    }

    /// PLAN-M2 AP6 Task 2: Parameter der eingebauten Bausteine.
    #[test]
    fn eingebaute_parameter() {
        let k = Katalog {
            eintraege: eingebaut(),
            ..Default::default()
        };
        let w = k.widgets();
        let finde = |f: &str, p: &str| {
            w.iter()
                .find(|w| w.parameter.funktion == f && w.parameter.parameter == p)
                .unwrap_or_else(|| panic!("{f}.{p} fehlt"))
        };
        for (f, p, art, position, titel) in [
            ("figure", "caption", "inhalt", None, "Unterschrift"),
            ("image", "pfad", "datei", Some(0), "Datei"),
            ("image", "width", "prozent", None, "Breite"),
            ("footnote", "text", "inhalt", None, "Text"),
            ("quote", "text", "inhalt", None, "Text"),
            ("quote", "attribution", "inhalt", None, "Quelle"),
            ("quote", "block", "schalter", None, "Als Block"),
            ("link", "ziel", "text", Some(0), "Ziel"),
            ("link", "text", "inhalt", None, "Text"),
            ("raw", "code", "text", Some(0), "Code"),
            ("raw", "lang", "text", None, "Sprache"),
            ("raw", "block", "schalter", None, "Als Block"),
            ("outline", "title", "inhalt", None, "Titel"),
            ("outline", "depth", "zahl", None, "Tiefe"),
            ("strong", "text", "inhalt", None, "Text"),
            ("emph", "text", "inhalt", None, "Text"),
        ] {
            let e = finde(f, p);
            assert_eq!(e.parameter.art.name(), art, "{f}.{p}");
            assert_eq!(e.parameter.position, position, "{f}.{p}");
            assert_eq!(e.titel, titel, "{f}.{p}");
        }
        for (f, p) in [
            ("footnote", "text"),
            ("quote", "text"),
            ("link", "text"),
            ("strong", "text"),
            ("emph", "text"),
        ] {
            assert_eq!(
                finde(f, p).parameter.art,
                tftn_edit::WidgetArt::Inhalt { rumpf: true },
                "{f}.{p}"
            );
        }
        assert_eq!(
            finde("raw", "code").parameter.art,
            tftn_edit::WidgetArt::Text { mehrzeilig: true }
        );
        assert_eq!(
            finde("image", "width").parameter.art,
            tftn_edit::WidgetArt::Prozent {
                min: Some(1.0),
                max: Some(100.0)
            }
        );
        assert_eq!(
            finde("outline", "depth").parameter.art,
            tftn_edit::WidgetArt::Zahl {
                min: Some(1.0),
                max: Some(6.0),
                ganzzahl: true
            }
        );
        let tabelle = w.iter().any(|w| w.parameter.funktion == "table");
        assert!(!tabelle, "table bekommt das Raster, keinen Parameter");
    }

    #[test]
    fn eingebaut_liest_modus() {
        let e = eingebaut_lesen(
            r#"
[[baustein]]
id = "a"
titel = "A"
einfuegen = "a"
modus = ["markup", "code"]

[[baustein]]
id = "b"
titel = "B"
einfuegen = "b"
"#,
        );
        assert_eq!(e[0].modus, vec![Modus::Markup, Modus::Code]);
        assert_eq!(e[1].modus, vec![Modus::Markup]);
    }

    #[test]
    fn eingebaute_schnipsel_sind_fehlerfreies_markup() {
        for e in eingebaut() {
            for auswahl in ["", "Text mit *fett*"] {
                let text = e.schnipsel.einsetzen(auswahl).text;
                let text = match e.modus[0] {
                    Modus::Markup => text,
                    Modus::Code => format!("#{{ {text} }}"),
                    Modus::Mathe => format!("$ {text} $"),
                };
                let baum = typst_syntax::parse(&text);
                assert!(!baum.diagnosis().either(), "{}: {text:?}", e.id);
            }
        }
    }
}
