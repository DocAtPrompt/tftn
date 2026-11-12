//! `tftn typ-pruefen` (PLAN-M2 AP10, Entwurf 5): Jeder Baustein wird in einem leeren Dokument des
//! Typs übersetzt. Geprüft wird, dass das Schnipsel übersetzt, dass es die genannte Funktion aufruft
//! und dass sie die benannten Parameter kennt. Eine World für alle Bausteine, damit Pakete nur
//! einmal ausgewertet werden.

use tftn_core::{Meldung, Projekt, ProjektWorld, kompilieren};
use tftn_edit::{Ort, Patch, WidgetArt, WidgetParameter, angebote_im_bereich, anwenden};
use tftn_manifest::{Modus, Parameter};
use tftn_sprachen::tr;
use typst_syntax::ast;
use typst_syntax::{LinkedNode, Source};

use crate::{Eintrag, Herkunft, Katalog, import_fuer, widget_art};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Befund {
    pub id: String,
    pub grund: String,
}

/// Alle `#import` der obersten Ebene, je eine Zeile.
pub fn obere_importe(text: &str) -> String {
    let quelle = Source::detached(text);
    LinkedNode::new(quelle.root())
        .children()
        .filter(|k| k.get().cast::<ast::ModuleImport>().is_some())
        .map(|k| format!("#{}\n", &text[k.range()]))
        .collect()
}

/// Prüft jeden Baustein. `Err` nur, wenn die World nicht entsteht oder die Hauptdatei fehlt.
pub fn pruefen(projekt: &Projekt, katalog: &Katalog) -> Result<Vec<Befund>, String> {
    let haupt = projekt.wurzel.join(&projekt.konfig.haupt);
    let text = std::fs::read_to_string(&haupt).map_err(|e| format!("{}: {e}", haupt.display()))?;
    let kopf = obere_importe(&text);
    let mut world = ProjektWorld::neu(projekt).map_err(|e| e.to_string())?;
    let mut befunde = Vec::new();
    for e in katalog.eintraege() {
        for grund in eintrag_pruefen(&mut world, &kopf, e) {
            befunde.push(Befund {
                id: e.id.clone(),
                grund,
            });
        }
    }
    Ok(befunde)
}

fn uebersetzen(world: &mut ProjektWorld, text: String) -> Vec<Meldung> {
    world.quelle_setzen(Source::new(world.haupt(), text));
    let e = kompilieren(world);
    e.meldungen.into_iter().filter(|m| m.fehler).collect()
}

fn eintrag_pruefen(world: &mut ProjektWorld, kopf: &str, e: &Eintrag) -> Vec<String> {
    let mut oben = kopf.to_string();
    if let Some(f) = import_fuer(&Source::detached(oben.clone()), e) {
        oben.insert_str(f.byte, &f.text);
    }
    if !oben.is_empty() && !oben.ends_with('\n') {
        oben.push('\n');
    }
    // Ohne Auswahl, denn eine Auswahl ginge in jedem Modus anders ins Schnipsel ein.
    let eingesetzt = e.schnipsel.einsetzen("").text;
    let rumpf = match e.modus.first() {
        Some(Modus::Mathe) => format!("$ {eingesetzt} $"),
        Some(Modus::Code) => format!("#{{ {eingesetzt} }}"),
        _ => eingesetzt,
    };
    let text = format!("{oben}{rumpf}");

    let fehler: Vec<String> = uebersetzen(world, text.clone())
        .into_iter()
        // Platzhalter-Dateien wie `bilder/bild.png` gibt es im leeren Dokument nicht.
        .filter(|m| !m.text.contains("file not found"))
        .map(|m| m.text)
        .collect();
    if !fehler.is_empty() {
        return vec![fehler.join("; ")];
    }
    let Some(funktion) = &e.funktion else {
        return Vec::new();
    };
    let probe = WidgetParameter {
        funktion: funktion.clone(),
        parameter: "_".into(),
        art: WidgetArt::Text { mehrzeilig: false },
        position: None,
    };
    let quelle = Source::detached(text.clone());
    if angebote_im_bereich(&quelle, oben.len()..text.len(), &[probe]).is_empty() {
        // Eingebaute Bausteine dürfen Kurzsyntax einfügen (``` statt `raw`); die Funktion gibt es.
        return if matches!(e.herkunft, Herkunft::Typst) {
            Vec::new()
        } else {
            vec![tr!("Das Schnipsel ruft {funktion} nicht auf.", funktion)]
        };
    }
    parameter_pruefen(world, &text, oben.len(), funktion, e)
}

/// Ergänzt alle benannten Parameter, die das Schnipsel nicht setzt, mit einem Beispielwert in einem
/// Lauf. Gezählt wird nur `unexpected argument: <name>`; dann ein neuer Lauf ohne diesen Parameter.
fn parameter_pruefen(
    world: &mut ProjektWorld,
    text: &str,
    anfang: usize,
    funktion: &str,
    e: &Eintrag,
) -> Vec<String> {
    let mut namen: Vec<&String> = e
        .parameter
        .iter()
        .filter(|(_, p)| zu_pruefen(p))
        .map(|(n, _)| n)
        .collect();
    let mut gruende = Vec::new();
    while !namen.is_empty() {
        let mut probe = text.to_string();
        for name in &namen {
            let p = &e.parameter[*name];
            let wp = WidgetParameter {
                funktion: funktion.to_string(),
                parameter: (*name).clone(),
                art: widget_art(p),
                position: None,
            };
            let quelle = Source::detached(probe.clone());
            let a = angebote_im_bereich(&quelle, anfang..probe.len(), &[wp]);
            if let Some(Ort::Einfuegen { pos, vorne, hinten }) = a.first().map(|a| &a.ort) {
                let patch = Patch {
                    bereich: *pos..*pos,
                    text: format!("{vorne}{name}: {}{hinten}", beispielwert(p)),
                };
                probe = anwenden(&probe, &patch);
            }
        }
        let unbekannt = uebersetzen(world, probe)
            .iter()
            .find_map(|m| unbekanntes_argument(&m.text));
        let Some(unbekannt) = unbekannt else {
            break;
        };
        let vorher = namen.len();
        namen.retain(|n| **n != unbekannt);
        if namen.len() == vorher {
            // Stammt nicht von einem ergänzten Parameter.
            break;
        }
        gruende.push(format!("unexpected argument: {unbekannt}"));
    }
    gruende
}

fn zu_pruefen(p: &Parameter) -> bool {
    p.position.is_none() && !p.rumpf && p.widget != "tabelle"
}

fn unbekanntes_argument(text: &str) -> Option<String> {
    let rest = text.split("unexpected argument: ").nth(1)?;
    let ende = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(rest.len());
    Some(rest[..ende].to_string())
}

fn beispielwert(p: &Parameter) -> String {
    if let Some(s) = &p.standard {
        return s.clone();
    }
    match p.widget.as_str() {
        "zahl" => "1".into(),
        "laenge" => "1pt".into(),
        "prozent" => "50%".into(),
        "schalter" => "true".into(),
        "wahl" => p
            .werte
            .first()
            .map_or_else(|| "\"x\"".into(), |w| format!("{w:?}")),
        "farbe" => "rgb(\"#000000\")".into(),
        "datum" => "datetime(year: 2026, month: 1, day: 1)".into(),
        "label" => "<x>".into(),
        "punkt" => "(0, 0)".into(),
        "liste" => "()".into(),
        "inhalt" => "[x]".into(),
        _ => "\"x\"".into(),
    }
}
