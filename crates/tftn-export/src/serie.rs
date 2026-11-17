//! Serien (Serienbrief, Serienrechnung): ein Dokument je Datensatz, über die typst-CLI.
//!
//! Die Datensätze kommen aus einem Ordner mit `.toml`-Dateien (je Datei ein Dokument) oder aus
//! einer `.toml` mit `[[serie]]`-Einträgen. Jeder Datensatz legt sich über die Formulardaten des
//! Projekts (`dokument.toml`): Gruppen werden zusammengeführt, alles andere ersetzt. So stehen
//! Absender, Bank usw. einmal im Projekt und in der Serie nur, was sich ändert.
//!
//! Die Hauptdatei liest ihre Daten mit `toml(sys.inputs.at("daten", default: "dokument.toml"))`;
//! jeder Datensatz kommt als `--input daten=_lokal/serie/<name>.toml` an die CLI. So lässt sich
//! jedes PDF mit seiner `typst compile`-Zeile ohne TFTN nachbauen (Gegenprobe). Die Datensätze
//! bleiben bis zum nächsten Lauf in `_lokal/serie/` liegen (nicht synchronisiert).

use std::path::{Path, PathBuf};
use std::time::Duration;

use tftn_core::{Projekt, ProjektWorld, kompilieren};
use tftn_sprachen::tr;
use toml::{Table, Value};
use typst::syntax::{RootedPath, Source, VirtualPath, VirtualRoot};

use crate::{ExportFehler, TypstCli, ausfuehren, gegenprobe_zeile};

/// Ordner der Datensätze im Projekt (relativ, mit `/`).
pub const ORDNER: &str = "_lokal/serie";
/// Schlüssel der Einträge in einer einzelnen Seriendatei.
pub const EINTRAEGE: &str = "serie";
/// Schlüssel für den Dateinamen des PDFs (ohne `.pdf`); wird nicht an die Vorlage weitergegeben.
pub const DATEI: &str = "datei";
/// Frist je Dokument.
pub const FRIST: Duration = Duration::from_secs(60);

/// Ein Datensatz: Name des PDFs (ohne Endung) und seine Werte (noch ohne Projektdaten).
#[derive(Debug, Clone, PartialEq)]
pub struct Datensatz {
    pub name: String,
    pub werte: Table,
}

/// Was beim Lesen der Quelle herauskommt.
#[derive(Debug, Clone, PartialEq)]
pub struct Quelle {
    pub datensaetze: Vec<Datensatz>,
    /// Hierhin kommen die PDFs: der Ordner selbst bzw. der Ordner der Datei.
    pub zielordner: PathBuf,
}

/// Liest die Datensätze aus einem Ordner (`*.toml`, alphabetisch) oder einer Datei
/// (`[[serie]]`). Namen: Feld `datei`, sonst der Dateiname bzw. `<datei>-001` …
pub fn lesen(quelle: &Path) -> Result<Quelle, String> {
    let mut datensaetze = Vec::new();
    let zielordner;
    if quelle.is_dir() {
        zielordner = quelle.to_path_buf();
        // Ein Projektordner hat auch .toml-Dateien (tftn.toml, dokument.toml) – Datensätze sind das nicht.
        if quelle.join("tftn.toml").is_file() {
            return Err(tr!(
                "{ordner} ist ein TFTN-Projektordner, keine Sammlung von Datensätzen. Die Datensätze gehören in einen eigenen Ordner, etwa „serie“ im Projekt.",
                ordner = quelle.display()
            ));
        }
        let mut dateien: Vec<PathBuf> = std::fs::read_dir(quelle)
            .map_err(|e| format!("{}: {e}", quelle.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        dateien.sort();
        if dateien.is_empty() {
            return Err(tr!(
                "Im Ordner {ordner} liegt keine .toml-Datei.",
                ordner = quelle.display()
            ));
        }
        for d in dateien {
            let stamm = d
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut werte = toml_lesen(&d)?;
            let name = name_nehmen(&mut werte, &stamm, &d)?;
            datensaetze.push(Datensatz { name, werte });
        }
    } else {
        zielordner = quelle
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let tabelle = toml_lesen(quelle)?;
        let Some(Value::Array(eintraege)) = tabelle.get(EINTRAEGE) else {
            return Err(tr!(
                "{datei} hat keine Einträge [[serie]]. Je Dokument ein Abschnitt [[serie]] mit den Feldern, die sich ändern.",
                datei = quelle.display()
            ));
        };
        let stamm = quelle
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "serie".into());
        for (i, e) in eintraege.iter().enumerate() {
            let Value::Table(t) = e else {
                return Err(tr!(
                    "{datei}: Eintrag {nr} ist kein Abschnitt [[serie]].",
                    datei = quelle.display(),
                    nr = i + 1
                ));
            };
            let mut werte = t.clone();
            let name = name_nehmen(&mut werte, &format!("{stamm}-{:03}", i + 1), quelle)?;
            datensaetze.push(Datensatz { name, werte });
        }
        if datensaetze.is_empty() {
            return Err(tr!(
                "{datei} hat keine Einträge [[serie]].",
                datei = quelle.display()
            ));
        }
    }
    let mut namen = std::collections::BTreeSet::new();
    for d in &datensaetze {
        if !namen.insert(d.name.to_lowercase()) {
            return Err(tr!(
                "Der Dateiname „{name}“ kommt in der Serie zweimal vor.",
                name = d.name
            ));
        }
    }
    Ok(Quelle {
        datensaetze,
        zielordner,
    })
}

fn toml_lesen(pfad: &Path) -> Result<Table, String> {
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    text.parse::<Table>()
        .map_err(|e| format!("{}: {}", pfad.display(), e.message()))
}

/// Nimmt `datei` aus den Werten (oder `vorgabe`) und macht daraus einen sicheren Dateinamen.
fn name_nehmen(werte: &mut Table, vorgabe: &str, herkunft: &Path) -> Result<String, String> {
    let roh = match werte.remove(DATEI) {
        Some(Value::String(s)) => s,
        Some(_) => {
            return Err(tr!(
                "{datei}: „datei“ muss Text sein.",
                datei = herkunft.display()
            ));
        }
        None => vorgabe.to_string(),
    };
    let roh = roh.trim();
    let roh = roh
        .strip_suffix(".pdf")
        .or_else(|| roh.strip_suffix(".PDF"))
        .unwrap_or(roh);
    let name: String = roh
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = name.trim_matches(|c| c == '.' || c == ' ').to_string();
    if name.is_empty() {
        return Err(tr!(
            "{datei}: kein brauchbarer Dateiname.",
            datei = herkunft.display()
        ));
    }
    Ok(name)
}

/// Legt `ueber` über `basis`: Tabellen werden zusammengeführt, alles andere ersetzt.
pub fn zusammenfuehren(basis: &Table, ueber: &Table) -> Table {
    let mut aus = basis.clone();
    for (k, v) in ueber {
        match (aus.get_mut(k), v) {
            (Some(Value::Table(alt)), Value::Table(neu)) => {
                *alt = zusammenfuehren(alt, neu);
            }
            _ => {
                aus.insert(k.clone(), v.clone());
            }
        }
    }
    aus
}

/// Kann die Hauptdatei Datensätze annehmen?
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hauptdatei {
    /// liest schon `sys.inputs.at("daten", …)`
    Bereit,
    /// liest `toml("<daten>")` genau einmal: so angepasst geht es (`neu` ist der ganze Text)
    Anpassbar { neu: String },
    /// liest die Formulardaten nicht erkennbar
    Nein,
}

/// Prüft den Text der Hauptdatei; `daten` ist der Name der Datendatei (`dokument.toml`).
pub fn hauptdatei_pruefen(text: &str, daten: &str) -> Hauptdatei {
    if text.contains("sys.inputs.at(\"daten\"") {
        return Hauptdatei::Bereit;
    }
    let alt = format!("toml(\"{daten}\")");
    if text.matches(&alt).count() == 1 {
        let neu = format!("toml(sys.inputs.at(\"daten\", default: \"{daten}\"))");
        return Hauptdatei::Anpassbar {
            neu: text.replacen(&alt, &neu, 1),
        };
    }
    Hauptdatei::Nein
}

/// Name der Datendatei des Projekts (Manifest `daten`, sonst `dokument.toml`).
pub fn datendatei(projekt: &Projekt) -> String {
    tftn_manifest::Typpaket::vom_projekt(projekt)
        .ok()
        .and_then(|p| p.manifest().ok().and_then(|m| m.daten.clone()))
        .unwrap_or_else(|| "dokument.toml".into())
}

/// Ergebnis für ein Dokument der Serie.
#[derive(Debug, Clone)]
pub struct Stueck {
    pub name: String,
    /// das PDF, wenn es entstand
    pub pdf: Option<PathBuf>,
    pub fehler: Option<String>,
    /// die `typst compile`-Zeile (im Projektordner auszuführen)
    pub gegenprobe: String,
}

/// Erzeugt die Serie: je Datensatz Prüfung mit der Vorschau-World, dann die CLI. Fehler eines
/// Datensatzes halten die anderen nicht auf. `fortschritt(fertig, gesamt, name)` nach jedem Stück.
/// Die Hauptdatei muss `Hauptdatei::Bereit` sein.
pub fn erzeugen(
    projekt: &Projekt,
    cli: &TypstCli,
    quelle: &Quelle,
    mut fortschritt: impl FnMut(usize, usize, &str),
) -> Result<Vec<Stueck>, ExportFehler> {
    let daten = datendatei(projekt);
    let haupt_text = std::fs::read_to_string(projekt.haupt())
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    if hauptdatei_pruefen(&haupt_text, &daten) != Hauptdatei::Bereit {
        return Err(ExportFehler::Sonstiges(tr!(
            "Die Hauptdatei liest ihre Daten nicht über sys.inputs; für eine Serie muss dort toml(sys.inputs.at(\"daten\", default: \"{daten}\")) stehen.",
            daten
        )));
    }
    let basis_text = std::fs::read_to_string(projekt.wurzel.join(&daten)).unwrap_or_default();
    let basis: Table = basis_text.parse().map_err(|e: toml::de::Error| {
        ExportFehler::Sonstiges(format!("{daten}: {}", e.message()))
    })?;
    // Fehlende Pakete und Fehler im Projekt selbst einmal vorab.
    crate::vorpruefung(projekt)?;
    let ordner = projekt.wurzel.join(ORDNER);
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    std::fs::create_dir_all(&quelle.zielordner)
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let mut world =
        ProjektWorld::neu(projekt).map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
    let daten_id = RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new(&daten).map_err(|e| ExportFehler::Sonstiges(format!("{daten}: {e:?}")))?,
    )
    .intern();
    let gesamt = quelle.datensaetze.len();
    let mut aus = Vec::new();
    for (i, d) in quelle.datensaetze.iter().enumerate() {
        let werte = zusammenfuehren(&basis, &d.werte);
        let text = toml::to_string(&werte).unwrap_or_default();
        let rel = format!("{ORDNER}/{}.toml", d.name);
        let zusatz = vec!["--input".to_string(), format!("daten={rel}")];
        let pdf = quelle.zielordner.join(format!("{}.pdf", d.name));
        let pdf_text = pdf.to_string_lossy().into_owned();
        let mut stueck = Stueck {
            name: d.name.clone(),
            pdf: None,
            fehler: None,
            gegenprobe: gegenprobe_zeile(projekt, &pdf_text, &zusatz),
        };
        let ergebnis = (|| -> Result<(), String> {
            std::fs::write(projekt.wurzel.join(&rel), &text).map_err(|e| e.to_string())?;
            // Vorab mit der Vorschau-World: genaue Fehlermeldungen, ohne die CLI zu bemühen.
            world.zuruecksetzen();
            world.quelle_setzen(Source::new(daten_id, text.clone()));
            let probe = kompilieren(&world);
            if probe.dokument.is_none() {
                let fehler: Vec<String> = probe
                    .meldungen
                    .iter()
                    .filter(|m| m.fehler)
                    .map(|m| m.to_string())
                    .collect();
                return Err(fehler.join("\n"));
            }
            ausfuehren(projekt, cli, &pdf, &zusatz, FRIST).map_err(|e| e.to_string())?;
            Ok(())
        })();
        match ergebnis {
            Ok(()) => stueck.pdf = Some(pdf),
            Err(e) => stueck.fehler = Some(e),
        }
        aus.push(stueck);
        fortschritt(i + 1, gesamt, &d.name);
    }
    Ok(aus)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordner(name: &str) -> PathBuf {
        let o = std::env::temp_dir().join(format!("tftn-serie-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        o
    }

    #[test]
    fn zusammenfuehren_gruppen_tief_rest_ersetzt() {
        let basis: Table =
            "ust = 20\nempfaenger = [\"A\"]\n[absender]\nname = \"N\"\niban = \"X\"\n"
                .parse()
                .unwrap();
        let ueber: Table = "empfaenger = [\"B\", \"C\"]\n[absender]\niban = \"Y\"\n"
            .parse()
            .unwrap();
        let z = zusammenfuehren(&basis, &ueber);
        assert_eq!(z["ust"].as_integer(), Some(20));
        assert_eq!(z["empfaenger"].as_array().unwrap().len(), 2);
        assert_eq!(z["absender"]["name"].as_str(), Some("N"));
        assert_eq!(z["absender"]["iban"].as_str(), Some("Y"));
    }

    #[test]
    fn ordner_mit_dateien() {
        let o = ordner("dateien");
        std::fs::write(o.join("b-mueller.toml"), "anrede = \"Hallo\"\n").unwrap();
        std::fs::write(o.join("a-huber.toml"), "datei = \"Rechnung Huber.pdf\"\n").unwrap();
        std::fs::write(o.join(".versteckt.toml"), "x = 1\n").unwrap();
        std::fs::write(o.join("notiz.txt"), "x").unwrap();
        let q = lesen(&o).unwrap();
        let namen: Vec<&str> = q.datensaetze.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(namen, ["Rechnung Huber", "b-mueller"]);
        assert!(
            q.datensaetze[0].werte.is_empty(),
            "datei geht nicht an die Vorlage"
        );
        assert_eq!(q.zielordner, o);
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn projektordner_ist_keine_quelle() {
        let o = ordner("projekt");
        std::fs::write(o.join("tftn.toml"), "tftn = 1\n").unwrap();
        std::fs::write(o.join("dokument.toml"), "nummer = \"1\"\n").unwrap();
        assert!(lesen(&o).unwrap_err().contains("Projektordner"));
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn eine_datei_mit_eintraegen() {
        let o = ordner("eintraege");
        let d = o.join("rechnungen.toml");
        std::fs::write(
            &d,
            "[[serie]]\nnummer = \"1\"\n\n[[serie]]\ndatei = \"zwei/../x\"\nnummer = \"2\"\n",
        )
        .unwrap();
        let q = lesen(&d).unwrap();
        let namen: Vec<&str> = q.datensaetze.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(namen, ["rechnungen-001", "zwei-..-x"]);
        assert_eq!(q.zielordner, o);
        std::fs::write(&d, "nummer = \"1\"\n").unwrap();
        assert!(lesen(&d).unwrap_err().contains("[[serie]]"));
        std::fs::write(&d, "[[serie]]\ndatei = \"a\"\n[[serie]]\ndatei = \"A\"\n").unwrap();
        assert!(lesen(&d).unwrap_err().contains("zweimal"));
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn hauptdatei_erkennen_und_anpassen() {
        let alt = "#show: brief.with(daten: toml(\"dokument.toml\"))\nText";
        let Hauptdatei::Anpassbar { neu } = hauptdatei_pruefen(alt, "dokument.toml") else {
            panic!("anpassbar erwartet");
        };
        assert_eq!(
            neu,
            "#show: brief.with(daten: toml(sys.inputs.at(\"daten\", default: \"dokument.toml\")))\nText"
        );
        assert_eq!(
            hauptdatei_pruefen(&neu, "dokument.toml"),
            Hauptdatei::Bereit
        );
        assert_eq!(
            hauptdatei_pruefen("= Nur Text", "dokument.toml"),
            Hauptdatei::Nein
        );
    }
}
