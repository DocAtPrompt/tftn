//! Serien (Serienbrief, Serienrechnung): ein Dokument je Datensatz, über die typst-CLI.
//!
//! Die Datensätze kommen aus einem Ordner mit `.toml`-Dateien (je Datei ein Dokument), aus
//! einer `.toml` mit `[[serie]]`-Einträgen oder aus einer CSV-Datei (je Zeile ein Dokument; die
//! Spalten gehören zu Feldern des Formulars, siehe `tftn_formular::csv`). Eine CSV wird dabei zu
//! denselben Datensätzen wie `[[serie]]`. Jeder Datensatz legt sich über die Formulardaten des
//! Projekts (`dokument.toml`): Gruppen werden zusammengeführt, alles andere ersetzt. So stehen
//! Absender, Bank usw. einmal im Projekt und in der Serie nur, was sich ändert.
//!
//! Die Hauptdatei liest ihre Daten mit `toml(sys.inputs.at("daten", default: "dokument.toml"))`;
//! jeder Datensatz kommt als `--input daten=_lokal/serie/<name>.toml` an die CLI. So lässt sich
//! jedes PDF mit seiner `typst compile`-Zeile ohne TFTN nachbauen (Gegenprobe). Die Datensätze
//! bleiben bis zum nächsten Lauf in `_lokal/serie/` liegen (nicht synchronisiert).
//!
//! Für einen einzigen Drucklauf fasst ein zweiter CLI-Lauf alle PDFs zu einem zusammen
//! (`_lokal/serie/alle.typ` bindet sie Seite für Seite als Bild ein; typst übernimmt PDF-Seiten
//! als Vektorgrafik, der Text bleibt Text). Beidseitig: Nach einem Dokument mit ungerader
//! Seitenzahl folgt eine leere Seite, damit jedes auf einer Vorderseite beginnt.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tftn_core::{Projekt, ProjektWorld, kompilieren};
use tftn_formular::{Formular, csv};
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
/// Frist für das Sammel-PDF.
pub const FRIST_GESAMT: Duration = Duration::from_secs(300);

/// Was entsteht: je Datensatz ein PDF, ein Sammel-PDF für den Druck, oder beides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ausgabe {
    #[default]
    Einzeln,
    Gesamt,
    Beides,
}

impl Ausgabe {
    pub fn einzeln(self) -> bool {
        self != Ausgabe::Gesamt
    }
    pub fn gesamt(self) -> bool {
        self != Ausgabe::Einzeln
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Optionen {
    pub ausgabe: Ausgabe,
    /// Sammel-PDF: jedes Dokument beginnt auf einer Vorderseite (Leerseite nach ungerader Zahl).
    pub beidseitig: bool,
}

/// Ein Datensatz: Name des PDFs (ohne Endung) und seine Werte (noch ohne Projektdaten).
#[derive(Debug, Clone, PartialEq)]
pub struct Datensatz {
    pub name: String,
    pub werte: Table,
    /// Fehler beim Lesen (eine CSV-Zeile, deren Werte nicht passen): kein PDF, nur die Meldung
    pub fehler: Option<String>,
}

/// Was beim Lesen der Quelle herauskommt.
#[derive(Debug, Clone, PartialEq)]
pub struct Quelle {
    pub datensaetze: Vec<Datensatz>,
    /// Hierhin kommen die PDFs: der Ordner selbst bzw. der Ordner der Datei.
    pub zielordner: PathBuf,
    /// Name des Ordners bzw. der Datei (ohne Endung); daraus der Name des Sammel-PDFs.
    pub stamm: String,
}

impl Quelle {
    /// Dateiname des Sammel-PDFs (ohne `.pdf`).
    pub fn gesamt_name(&self) -> String {
        tr!("{name}-alle", name = self.stamm)
    }
}

/// Liest die Datensätze aus einem Ordner (`*.toml`, alphabetisch) oder einer Datei
/// (`[[serie]]`). Namen: Feld `datei`, sonst der Dateiname bzw. `<datei>-001` …
pub fn lesen(quelle: &Path) -> Result<Quelle, String> {
    let mut datensaetze = Vec::new();
    let zielordner;
    let quell_stamm = quelle
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "serie".into());
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
            datensaetze.push(Datensatz {
                name,
                werte,
                fehler: None,
            });
        }
    } else {
        zielordner = quelle
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let tabelle = toml_lesen(quelle)?;
        datensaetze = eintraege(&tabelle, &quell_stamm, quelle)?;
    }
    fertig(datensaetze, zielordner, &quell_stamm, quelle)
}

/// Prüft die Namen auf Doppelte und macht die Quelle fertig.
fn fertig(
    datensaetze: Vec<Datensatz>,
    zielordner: PathBuf,
    quell_stamm: &str,
    quelle: &Path,
) -> Result<Quelle, String> {
    let mut namen = std::collections::BTreeSet::new();
    for d in &datensaetze {
        if !namen.insert(d.name.to_lowercase()) {
            return Err(tr!(
                "Der Dateiname „{name}“ kommt in der Serie zweimal vor.",
                name = d.name
            ));
        }
    }
    // Sauberer Name auch für einen Ordner wie „Serie: Nov“.
    let stamm = name_nehmen(&mut Table::new(), quell_stamm, quelle)?;
    Ok(Quelle {
        datensaetze,
        zielordner,
        stamm,
    })
}

/// Die Einträge `[[serie]]` einer Tabelle als Datensätze (Namen `<stamm>-001` …, sonst `datei`).
fn eintraege(tabelle: &Table, stamm: &str, quelle: &Path) -> Result<Vec<Datensatz>, String> {
    let Some(Value::Array(eintraege)) = tabelle.get(EINTRAEGE) else {
        return Err(tr!(
            "{datei} hat keine Einträge [[serie]]. Je Dokument ein Abschnitt [[serie]] mit den Feldern, die sich ändern.",
            datei = quelle.display()
        ));
    };
    let mut datensaetze = Vec::new();
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
        datensaetze.push(Datensatz {
            name,
            werte,
            fehler: None,
        });
    }
    if datensaetze.is_empty() {
        return Err(tr!(
            "{datei} hat keine Einträge [[serie]].",
            datei = quelle.display()
        ));
    }
    Ok(datensaetze)
}

/// Ist die Quelle eine CSV-Datei (`.csv`, `.tsv`)?
pub fn ist_csv(quelle: &Path) -> bool {
    quelle
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv") || e.eq_ignore_ascii_case("tsv"))
}

/// Das Formular des Dokumenttyps im Projekt: Ziel der Spalten einer CSV.
pub fn formular(projekt: &Projekt) -> Result<Formular, String> {
    let paket = tftn_manifest::Typpaket::vom_projekt(projekt).map_err(|e| e.to_string())?;
    let m = paket.manifest().map_err(|e| e.to_string())?;
    let pfad = m.formular.as_ref().ok_or_else(|| {
        tr!("Der Dokumenttyp hat kein Formular; die Spalten einer CSV lassen sich keinen Feldern zuordnen.")
    })?;
    let datei = paket.ordner.join(pfad);
    let json = std::fs::read_to_string(&datei).map_err(|e| format!("{}: {e}", datei.display()))?;
    tftn_formular::formular_lesen(&json).map_err(|f| {
        f.iter()
            .map(|f| format!("{} {}", f.ort, f.text))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// Die eigene Zuordnung einer CSV aus `tftn.toml` des Projekts.
pub fn zuordnung(projekt: &Projekt, quelle: &Path) -> csv::Zuordnung {
    let text = std::fs::read_to_string(projekt.wurzel.join("tftn.toml")).unwrap_or_default();
    csv::zuordnung_lesen(&text, &csv::dateischluessel(quelle))
}

/// Liest eine CSV: je Zeile ein Datensatz wie ein Eintrag `[[serie]]`, dazu der Bericht für den
/// Dialog (Zuordnung, Vorschau, Fehler). Eine Zeile mit Fehlern wird ein Datensatz mit `fehler`.
pub fn csv_lesen(
    quelle: &Path,
    formular: &Formular,
    eigene: &csv::Zuordnung,
) -> Result<(Quelle, csv::Bericht), String> {
    let bytes = std::fs::read(quelle).map_err(|e| format!("{}: {e}", quelle.display()))?;
    let tabelle = csv::lesen(&bytes).map_err(|e| format!("{}: {e}", quelle.display()))?;
    let bericht = csv::auswerten(&tabelle, formular, eigene, false);
    if bericht.saetze.is_empty() {
        return Err(tr!(
            "{datei} hat unter der Kopfzeile keine Zeilen.",
            datei = quelle.display()
        ));
    }
    let toml_text = csv::serie_toml(&bericht.saetze);
    let werte: Table = toml_text
        .parse()
        .map_err(|e: toml::de::Error| format!("{}: {}", quelle.display(), e.message()))?;
    let stamm = quelle
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "serie".into());
    let mut datensaetze = eintraege(&werte, &stamm, quelle)?;
    for (d, s) in datensaetze.iter_mut().zip(&bericht.saetze) {
        if !s.fehler.is_empty() {
            d.fehler = Some(
                s.fehler
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
    let zielordner = quelle
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    Ok((fertig(datensaetze, zielordner, &stamm, quelle)?, bericht))
}

/// Liest die Quelle einer Serie im Projekt: Ordner oder Datei mit TOML, oder eine CSV (dann mit
/// Formular und eigener Zuordnung des Projekts, und dem Bericht).
pub fn quelle_lesen(
    projekt: &Projekt,
    quelle: &Path,
) -> Result<(Quelle, Option<csv::Bericht>), String> {
    if ist_csv(quelle) && !quelle.is_dir() {
        let f = formular(projekt)?;
        let (q, b) = csv_lesen(quelle, &f, &zuordnung(projekt, quelle))?;
        Ok((q, Some(b)))
    } else {
        Ok((lesen(quelle)?, None))
    }
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
    /// Seitenzahl (aus der Vorprüfung)
    pub seiten: usize,
}

/// Ergebnis einer Serie.
#[derive(Debug, Clone)]
pub struct Serie {
    pub stuecke: Vec<Stueck>,
    /// das Sammel-PDF, wenn verlangt (und mindestens ein Dokument entstand)
    pub gesamt: Option<Stueck>,
}

/// Der Typst-Text des Sammel-PDFs: `(pfad relativ zu alle.typ, seiten)` je Dokument.
pub fn gesamt_quelltext(stuecke: &[(String, usize)], beidseitig: bool) -> String {
    let liste: String = stuecke
        .iter()
        .map(|(p, n)| {
            format!(
                "  (\"{}\", {n}),\n",
                p.replace('\\', "\\\\").replace('"', "\\\"")
            )
        })
        .collect();
    format!(
        "// Von TFTN erzeugt: alle PDFs der Serie hintereinander, für einen Drucklauf.\n\
         #set page(width: auto, height: auto, margin: 0pt)\n\
         #let beidseitig = {beidseitig}\n\
         #let stuecke = (\n{liste})\n\
         #for (datei, seiten) in stuecke {{\n\
         \x20 for i in range(1, seiten + 1) {{ page(image(datei, page: i)) }}\n\
         \x20 // Beidseitig: Leerseite in der Größe der letzten, damit das nächste vorne beginnt.\n\
         \x20 if beidseitig and calc.odd(seiten) {{ page(hide(image(datei, page: seiten))) }}\n\
         }}\n"
    )
}

/// Erzeugt die Serie: je Datensatz Prüfung mit der Vorschau-World, dann die CLI. Fehler eines
/// Datensatzes halten die anderen nicht auf. `fortschritt(fertig, gesamt, name)` nach jedem Stück.
/// Mit `Ausgabe::Gesamt`/`Beides` danach das Sammel-PDF (ohne die fehlerhaften Datensätze).
/// Die Hauptdatei muss `Hauptdatei::Bereit` sein.
pub fn erzeugen(
    projekt: &Projekt,
    cli: &TypstCli,
    quelle: &Quelle,
    optionen: Optionen,
    mut fortschritt: impl FnMut(usize, usize, &str),
) -> Result<Serie, ExportFehler> {
    let gesamt_name = quelle.gesamt_name();
    if optionen.ausgabe.gesamt()
        && quelle
            .datensaetze
            .iter()
            .any(|d| d.name.eq_ignore_ascii_case(&gesamt_name))
    {
        return Err(ExportFehler::Sonstiges(tr!(
            "Ein Datensatz heißt wie das Sammel-PDF („{name}“); bitte umbenennen.",
            name = gesamt_name
        )));
    }
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
    std::fs::create_dir_all(ordner.join("pdf"))
        .map_err(|e| ExportFehler::Sonstiges(e.to_string()))?;
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
        // Nur fürs Sammel-PDF: die Einzelnen im Projekt (typst liest nur unter --root).
        let pdf = if optionen.ausgabe.einzeln() {
            quelle.zielordner.join(format!("{}.pdf", d.name))
        } else {
            ordner.join("pdf").join(format!("{}.pdf", d.name))
        };
        let pdf_text = pdf.to_string_lossy().into_owned();
        let mut stueck = Stueck {
            name: d.name.clone(),
            pdf: None,
            fehler: None,
            gegenprobe: gegenprobe_zeile(projekt, &pdf_text, &zusatz),
            seiten: 0,
        };
        let ergebnis = (|| -> Result<usize, String> {
            if let Some(f) = &d.fehler {
                return Err(f.clone());
            }
            std::fs::write(projekt.wurzel.join(&rel), &text).map_err(|e| e.to_string())?;
            // Vorab mit der Vorschau-World: genaue Fehlermeldungen, ohne die CLI zu bemühen.
            world.zuruecksetzen();
            world.quelle_setzen(Source::new(daten_id, text.clone()));
            let probe = kompilieren(&world);
            let Some(dokument) = probe.dokument else {
                let fehler: Vec<String> = probe
                    .meldungen
                    .iter()
                    .filter(|m| m.fehler)
                    .map(|m| m.to_string())
                    .collect();
                return Err(fehler.join("\n"));
            };
            ausfuehren(projekt, cli, &pdf, &zusatz, FRIST).map_err(|e| e.to_string())?;
            Ok(dokument.pages().len())
        })();
        match ergebnis {
            Ok(seiten) => {
                stueck.pdf = Some(pdf);
                stueck.seiten = seiten;
            }
            Err(e) => stueck.fehler = Some(e),
        }
        aus.push(stueck);
        fortschritt(i + 1, gesamt, &d.name);
    }
    let gesamt_stueck = if optionen.ausgabe.gesamt() && aus.iter().any(|s| s.pdf.is_some()) {
        Some(gesamt_erzeugen(
            projekt,
            cli,
            &aus,
            &quelle.zielordner.join(format!("{gesamt_name}.pdf")),
            optionen.beidseitig,
            &gesamt_name,
        ))
    } else {
        None
    };
    Ok(Serie {
        stuecke: aus,
        gesamt: gesamt_stueck,
    })
}

/// Das Sammel-PDF: die fertigen PDFs (nach `_lokal/serie/pdf/` kopiert, falls sie woanders
/// liegen) über `_lokal/serie/alle.typ` mit einem zweiten CLI-Lauf.
fn gesamt_erzeugen(
    projekt: &Projekt,
    cli: &TypstCli,
    stuecke: &[Stueck],
    ziel: &Path,
    beidseitig: bool,
    name: &str,
) -> Stueck {
    let ordner = projekt.wurzel.join(ORDNER);
    let haupt = format!("{ORDNER}/alle.typ");
    let mut p = projekt.clone();
    p.konfig.haupt = haupt.clone();
    let mut stueck = Stueck {
        name: name.to_string(),
        pdf: None,
        fehler: None,
        gegenprobe: gegenprobe_zeile(&p, &ziel.to_string_lossy(), &[]),
        seiten: 0,
    };
    let ergebnis = (|| -> Result<usize, String> {
        let mut liste = Vec::new();
        for s in stuecke {
            let Some(pdf) = &s.pdf else { continue };
            let im_projekt = ordner.join("pdf").join(format!("{}.pdf", s.name));
            if *pdf != im_projekt {
                std::fs::copy(pdf, &im_projekt).map_err(|e| format!("{}: {e}", pdf.display()))?;
            }
            liste.push((format!("pdf/{}.pdf", s.name), s.seiten));
        }
        let seiten: usize = liste
            .iter()
            .map(|(_, n)| if beidseitig { n + n % 2 } else { *n })
            .sum();
        std::fs::write(
            projekt.wurzel.join(&haupt),
            gesamt_quelltext(&liste, beidseitig),
        )
        .map_err(|e| e.to_string())?;
        ausfuehren(&p, cli, ziel, &[], FRIST_GESAMT).map_err(|e| e.to_string())?;
        Ok(seiten)
    })();
    match ergebnis {
        Ok(seiten) => {
            stueck.pdf = Some(ziel.to_path_buf());
            stueck.seiten = seiten;
        }
        Err(e) => stueck.fehler = Some(e),
    }
    stueck
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
    fn sammel_quelltext_und_name() {
        let t = gesamt_quelltext(&[("pdf/a \"b\".pdf".into(), 3)], true);
        assert!(t.contains(r#"("pdf/a \"b\".pdf", 3),"#), "{t}");
        assert!(t.contains("#let beidseitig = true"), "{t}");
        let o = ordner("sammel");
        let d = o.join("november.toml");
        std::fs::write(&d, "[[serie]]\nnummer = \"1\"\n").unwrap();
        assert_eq!(lesen(&d).unwrap().gesamt_name(), "november-alle");
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
