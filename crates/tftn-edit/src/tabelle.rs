//! Patch-Klasse Tabellenstruktur (Entwurf 4 „Was danach ändern umfasst“ 2, PLAN-M2 AP5).
//!
//! Ein Aufruf `table(…)` wird als Raster gelesen (`tabelle_am`). Die Zellen liegen dort, wo Typst
//! 0.15.1 sie setzt. Nur „reine“ Tabellen werden geändert: Zellen als `[…]`, Ausdruck oder
//! `table.cell(…)` mit `colspan`/`rowspan`. Kopf- und Fußzeilen, Linien, `..`, feste Positionen und
//! Kommentare sperren die Tabelle mit einem Grund („… – im Text bearbeiten“).

use std::ops::Range;
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};
use typst_syntax::ast;
use typst_syntax::{LinkedNode, Side, Source, SyntaxKind};

use crate::{Patch, PatchFehler, aussen_pruefen, fehler};

/// Eine Tabelle als Raster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tabelle {
    pub spalten: usize,
    pub zeilen: usize,
    /// `columns` als Array: Quelltext jeder Spur; `None` bei `columns: N`, `auto` oder ohne `columns`.
    pub spuren: Option<Vec<String>>,
    /// Benannte Argumente außer `columns`, wörtlich (`stroke: 0.5pt`), in Quellreihenfolge.
    pub weitere: Vec<String>,
    /// Lesereihenfolge (nach `zeile`, dann `spalte`); deckt das Raster lückenlos ab.
    pub zellen: Vec<Zelle>,
}

/// Eine Zelle; `zeilen`/`spalten` sind `rowspan`/`colspan` (mindestens 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Zelle {
    pub zeile: usize,
    pub spalte: usize,
    pub zeilen: usize,
    pub spalten: usize,
    pub inhalt: ZellInhalt,
    /// Benannte Argumente von `table.cell` außer `colspan`/`rowspan`, wörtlich.
    pub weitere: Vec<String>,
}

/// Inhalt einer Zelle: Markup zwischen `[` und `]` oder ein anderer Ausdruck (wörtlich).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "art", content = "text", rename_all = "kebab-case")]
pub enum ZellInhalt {
    Markup(String),
    Ausdruck(String),
}

impl ZellInhalt {
    pub fn text(&self) -> &str {
        match self {
            Self::Markup(t) | Self::Ausdruck(t) => t,
        }
    }

    pub fn ist_leer(&self) -> bool {
        matches!(self, Self::Markup(t) if t.is_empty())
    }
}

/// Wie `columns` im Original stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpaltenForm {
    Fehlt,
    Auto,
    Zahl,
    Spuren,
}

/// Die Schreibweise des Aufrufs im Original (für das Neuschreiben).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Form {
    /// Quelltext vor `(`, z. B. `table`.
    pub(crate) name: String,
    pub(crate) mehrzeilig: bool,
    /// Einzug der Argumente bzw. der schließenden Klammer.
    pub(crate) einzug: String,
    pub(crate) schluss: String,
    pub(crate) spalten: SpaltenForm,
    /// Stelle von `columns` unter den benannten Argumenten.
    pub(crate) spalten_stelle: Option<usize>,
}

/// Eine gelesene Tabelle in der Quelle.
#[derive(Debug, Clone)]
pub struct TabellenStelle {
    /// Bytebereich des Aufrufs, vom Namen `table` bis zur letzten Klammer.
    pub aufruf: Range<usize>,
    pub tabelle: Tabelle,
    pub(crate) form: Form,
    /// Je Zelle in `tabelle.zellen`: Bereich des Inhalts (innerhalb `[…]` bzw. der Ausdruck);
    /// `None` für eine Lücke, die nur im Modell steht.
    pub(crate) inhaltsorte: Vec<Option<Range<usize>>>,
}

impl Tabelle {
    /// Index der Zelle, die das Feld (`zeile`, `spalte`) bedeckt.
    pub fn zelle_bei(&self, zeile: usize, spalte: usize) -> Option<usize> {
        self.zellen.iter().position(|z| {
            (z.zeile..z.zeile + z.zeilen).contains(&zeile)
                && (z.spalte..z.spalte + z.spalten).contains(&spalte)
        })
    }
}

/// Eine Änderung der Struktur oder eines Zelltexts. Indizes zählen ab 0; `nach` beim Verschieben
/// ist der Index nach dem Herausnehmen (wie `Vec::remove` und danach `Vec::insert`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
pub enum TabellenAktion {
    ZeileEinfuegen {
        vor: usize,
    },
    SpalteEinfuegen {
        vor: usize,
    },
    ZeileLoeschen {
        zeile: usize,
    },
    SpalteLoeschen {
        spalte: usize,
    },
    ZeileVerschieben {
        von: usize,
        nach: usize,
    },
    SpalteVerschieben {
        von: usize,
        nach: usize,
    },
    Verbinden {
        zeile: usize,
        spalte: usize,
        zeilen: usize,
        spalten: usize,
    },
    Trennen {
        zeile: usize,
        spalte: usize,
    },
    Zelltext {
        zeile: usize,
        spalte: usize,
        inhalt: String,
    },
}

fn ungueltig(t: impl Into<String>) -> PatchFehler {
    PatchFehler::UngueltigerWert(t.into())
}

fn ausserhalb() -> PatchFehler {
    ungueltig(tr!("Index außerhalb der Tabelle"))
}

/// Achse einer Strukturänderung.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Achse {
    Zeile,
    Spalte,
}

impl Zelle {
    /// Anfang und Spanne auf der Achse.
    fn lage(&mut self, a: Achse) -> (&mut usize, &mut usize) {
        match a {
            Achse::Zeile => (&mut self.zeile, &mut self.zeilen),
            Achse::Spalte => (&mut self.spalte, &mut self.spalten),
        }
    }

    fn auf(&self, a: Achse) -> (usize, usize) {
        match a {
            Achse::Zeile => (self.zeile, self.zeilen),
            Achse::Spalte => (self.spalte, self.spalten),
        }
    }

    fn leer(zeile: usize, spalte: usize) -> Self {
        Zelle {
            zeile,
            spalte,
            zeilen: 1,
            spalten: 1,
            inhalt: ZellInhalt::Markup(String::new()),
            weitere: vec![],
        }
    }
}

impl Tabelle {
    /// Wendet `aktion` auf das Modell an. Fehler sind immer `UngueltigerWert`.
    pub fn aendern(&self, aktion: &TabellenAktion) -> Result<Tabelle, PatchFehler> {
        use TabellenAktion as A;
        let mut t = self.clone();
        match aktion {
            A::ZeileEinfuegen { vor } => t.einfuegen(Achse::Zeile, *vor)?,
            A::SpalteEinfuegen { vor } => t.einfuegen(Achse::Spalte, *vor)?,
            A::ZeileLoeschen { zeile } => t.loeschen(Achse::Zeile, *zeile)?,
            A::SpalteLoeschen { spalte } => t.loeschen(Achse::Spalte, *spalte)?,
            A::ZeileVerschieben { von, nach } => t.verschieben(Achse::Zeile, *von, *nach)?,
            A::SpalteVerschieben { von, nach } => t.verschieben(Achse::Spalte, *von, *nach)?,
            A::Verbinden {
                zeile,
                spalte,
                zeilen,
                spalten,
            } => t.verbinden(*zeile, *spalte, *zeilen, *spalten)?,
            A::Trennen { zeile, spalte } => t.trennen(*zeile, *spalte)?,
            A::Zelltext {
                zeile,
                spalte,
                inhalt,
            } => {
                let i = t.zelle_bei(*zeile, *spalte).ok_or_else(ausserhalb)?;
                t.zellen[i].inhalt = ZellInhalt::Markup(inhalt.clone());
            }
        }
        t.zellen.sort_by_key(|z| (z.zeile, z.spalte));
        Ok(t)
    }

    fn anzahl(&self, a: Achse) -> usize {
        match a {
            Achse::Zeile => self.zeilen,
            Achse::Spalte => self.spalten,
        }
    }

    fn anzahl_mut(&mut self, a: Achse) -> &mut usize {
        match a {
            Achse::Zeile => &mut self.zeilen,
            Achse::Spalte => &mut self.spalten,
        }
    }

    /// Fügt vor `vor` eine Zeile bzw. Spalte ein; eine Zelle, die über die Grenze reicht, wächst mit.
    fn einfuegen(&mut self, a: Achse, vor: usize) -> Result<(), PatchFehler> {
        if vor > self.anzahl(a) {
            return Err(ausserhalb());
        }
        let mut quer = Vec::new(); // Felder der anderen Achse, die eine wachsende Zelle bedeckt
        for z in &mut self.zellen {
            let (anfang, spanne) = z.auf(a);
            let andere = match a {
                Achse::Zeile => z.spalte..z.spalte + z.spalten,
                Achse::Spalte => z.zeile..z.zeile + z.zeilen,
            };
            let (anf, span) = z.lage(a);
            if anfang >= vor {
                *anf += 1;
            } else if anfang + spanne > vor {
                *span += 1;
                quer.extend(andere);
            }
        }
        let andere_anzahl = match a {
            Achse::Zeile => self.spalten,
            Achse::Spalte => self.zeilen,
        };
        for k in (0..andere_anzahl).filter(|k| !quer.contains(k)) {
            self.zellen.push(match a {
                Achse::Zeile => Zelle::leer(vor, k),
                Achse::Spalte => Zelle::leer(k, vor),
            });
        }
        if a == Achse::Spalte
            && let Some(s) = &mut self.spuren
        {
            s.insert(vor, "auto".into());
        }
        *self.anzahl_mut(a) += 1;
        Ok(())
    }

    /// Löscht eine Zeile bzw. Spalte; verbundene Zellen werden kleiner und behalten den Inhalt.
    fn loeschen(&mut self, a: Achse, i: usize) -> Result<(), PatchFehler> {
        if i >= self.anzahl(a) {
            return Err(ausserhalb());
        }
        if self.anzahl(a) == 1 {
            return Err(ungueltig(match a {
                Achse::Zeile => tr!("die letzte Zeile bleibt"),
                Achse::Spalte => tr!("die letzte Spalte bleibt"),
            }));
        }
        self.zellen.retain(|z| z.auf(a) != (i, 1));
        for z in &mut self.zellen {
            let (anfang, spanne) = z.auf(a);
            let (anf, span) = z.lage(a);
            if anfang > i {
                *anf -= 1;
            } else if anfang + spanne > i {
                *span -= 1;
            }
        }
        if a == Achse::Spalte
            && let Some(s) = &mut self.spuren
        {
            s.remove(i);
        }
        *self.anzahl_mut(a) -= 1;
        Ok(())
    }

    /// Verschiebt eine Zeile bzw. Spalte, wenn keine verbundene Zelle den Bereich kreuzt.
    fn verschieben(&mut self, a: Achse, von: usize, nach: usize) -> Result<(), PatchFehler> {
        let n = self.anzahl(a);
        if von >= n || nach >= n {
            return Err(ausserhalb());
        }
        let (lo, hi) = (von.min(nach), von.max(nach));
        if self.zellen.iter().any(|z| {
            let (anfang, spanne) = z.auf(a);
            spanne > 1 && anfang <= hi && anfang + spanne > lo
        }) {
            return Err(ungueltig(tr!("verbundene Zellen im Weg – erst trennen")));
        }
        let mut folge: Vec<usize> = (0..n).collect();
        let x = folge.remove(von);
        folge.insert(nach, x);
        // neue Stelle je alter Stelle
        let mut neu = vec![0; n];
        for (stelle, alt) in folge.iter().enumerate() {
            neu[*alt] = stelle;
        }
        for z in &mut self.zellen {
            let (anf, _) = z.lage(a);
            *anf = neu[*anf];
        }
        if a == Achse::Spalte
            && let Some(s) = &mut self.spuren
        {
            let x = s.remove(von);
            s.insert(nach, x);
        }
        Ok(())
    }

    fn verbinden(
        &mut self,
        zeile: usize,
        spalte: usize,
        zeilen: usize,
        spalten: usize,
    ) -> Result<(), PatchFehler> {
        if zeilen == 0
            || spalten == 0
            || zeile + zeilen > self.zeilen
            || spalte + spalten > self.spalten
        {
            return Err(ausserhalb());
        }
        let schneidet = |z: &Zelle| {
            z.zeile < zeile + zeilen
                && z.zeile + z.zeilen > zeile
                && z.spalte < spalte + spalten
                && z.spalte + z.spalten > spalte
        };
        let drin = |z: &Zelle| {
            z.zeile >= zeile
                && z.zeile + z.zeilen <= zeile + zeilen
                && z.spalte >= spalte
                && z.spalte + z.spalten <= spalte + spalten
        };
        if self.zellen.iter().any(|z| schneidet(z) && !drin(z)) {
            return Err(ungueltig(tr!("Bereich schneidet verbundene Zellen")));
        }
        let innen: Vec<&Zelle> = self.zellen.iter().filter(|z| drin(z)).collect();
        if innen.len() < 2 {
            return Err(ungueltig(tr!("Bereich ist nur eine Zelle")));
        }
        let voll: Vec<&ZellInhalt> = innen
            .iter()
            .map(|z| &z.inhalt)
            .filter(|i| !i.ist_leer())
            .collect();
        let inhalt = match voll.as_slice() {
            [] => ZellInhalt::Markup(String::new()),
            [eins] => (*eins).clone(),
            viele => {
                if viele.iter().any(|i| matches!(i, ZellInhalt::Ausdruck(_))) {
                    return Err(ungueltig(tr!(
                        "mehrere Zellen mit berechnetem Inhalt – erst im Text zusammenführen"
                    )));
                }
                ZellInhalt::Markup(viele.iter().map(|i| i.text()).collect::<Vec<_>>().join(" "))
            }
        };
        let weitere = innen
            .iter()
            .find(|z| z.zeile == zeile && z.spalte == spalte)
            .map(|z| z.weitere.clone())
            .unwrap_or_default();
        self.zellen.retain(|z| !drin(z));
        self.zellen.push(Zelle {
            zeile,
            spalte,
            zeilen,
            spalten,
            inhalt,
            weitere,
        });
        Ok(())
    }

    fn trennen(&mut self, zeile: usize, spalte: usize) -> Result<(), PatchFehler> {
        let i = self.zelle_bei(zeile, spalte).ok_or_else(ausserhalb)?;
        let z = self.zellen[i].clone();
        if z.zeilen == 1 && z.spalten == 1 {
            return Err(ungueltig(tr!("Zelle ist nicht verbunden")));
        }
        self.zellen[i].zeilen = 1;
        self.zellen[i].spalten = 1;
        for zz in z.zeile..z.zeile + z.zeilen {
            for ss in z.spalte..z.spalte + z.spalten {
                if (zz, ss) != (z.zeile, z.spalte) {
                    self.zellen.push(Zelle::leer(zz, ss));
                }
            }
        }
        Ok(())
    }
}

/// Obergrenzen fürs Raster (Abschlussprüfung AP5): darüber würde das Modell riesig, und die
/// Tabelle bearbeitet man im Text.
const MAX_SPALTEN: usize = 1_000;
const MAX_FELDER: usize = 100_000;

fn zu_gross() -> String {
    tr!("Tabelle zu groß für das Raster – im Text bearbeiten")
}

fn berechnet() -> String {
    tr!("berechnet – im Text bearbeiten")
}
fn zellfehler() -> String {
    tr!("Tabelle mit Fehler in den Zellen – im Text bearbeiten")
}
fn kommentar() -> String {
    tr!("Kommentar in der Tabelle – im Text bearbeiten")
}

/// Der innerste Aufruf `table(…)` um `byte`: `None`, wenn es keinen gibt; `Some(Err(grund))`, wenn
/// die Tabelle nicht rein ist.
pub fn tabelle_am(quelle: &Source, byte: usize) -> Option<Result<TabellenStelle, String>> {
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before));
    while let Some(k) = knoten {
        if k.kind() == SyntaxKind::FuncCall
            && k.children()
                .next()
                .is_some_and(|c| c.kind() == SyntaxKind::Ident && c.get().leaf_text() == "table")
        {
            return Some(lesen(quelle, &k));
        }
        knoten = k.parent().cloned();
    }
    None
}

/// Eine Zelle, wie sie in der Quelle steht (vor der Platzierung).
struct Roh {
    zeilen: usize,
    spalten: usize,
    inhalt: ZellInhalt,
    weitere: Vec<String>,
    ort: Range<usize>,
}

fn lesen(quelle: &Source, aufruf: &LinkedNode) -> Result<TabellenStelle, String> {
    let text = quelle.text();
    if fehler(aufruf.get()) > 0 {
        return Err(tr!("Tabelle mit Syntaxfehler"));
    }
    let args = aufruf
        .children()
        .find(|c| c.kind() == SyntaxKind::Args)
        .ok_or_else(|| tr!("Tabelle mit Syntaxfehler"))?;

    let mut spalten = 1;
    let mut spuren = None;
    let mut spalten_form = SpaltenForm::Fehlt;
    let mut spalten_stelle = None;
    let mut weitere = Vec::new();
    let mut roh: Vec<Roh> = Vec::new();
    let mut erstes_argument: Option<usize> = None;

    for c in args.children() {
        match c.kind() {
            SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => continue,
            SyntaxKind::LineComment | SyntaxKind::BlockComment => return Err(kommentar()),
            k if k.is_trivia() => continue,
            _ => {}
        }
        erstes_argument.get_or_insert(c.offset());
        match c.kind() {
            SyntaxKind::Named => {
                if !roh.is_empty() {
                    return Err(tr!(
                        "benanntes Argument hinter den Zellen – im Text bearbeiten"
                    ));
                }
                let named = c.cast::<ast::Named>().ok_or_else(berechnet)?;
                if named.name().as_str() == "columns" {
                    // Der Wert wird neu geschrieben: ein Kommentar darin ginge verloren.
                    if hat_kommentar(&c) {
                        return Err(kommentar());
                    }
                    let wert = c
                        .children()
                        .filter(|w| !w.kind().is_trivia())
                        .last()
                        .ok_or_else(berechnet)?;
                    (spalten, spuren, spalten_form) = spalten_lesen(text, &wert)?;
                    spalten_stelle = Some(weitere.len());
                } else {
                    weitere.push(text[c.range()].to_string());
                }
            }
            SyntaxKind::Spread => return Err(tr!("Tabelle mit `..` – im Text bearbeiten")),
            SyntaxKind::ContentBlock => roh.push(Roh {
                zeilen: 1,
                spalten: 1,
                inhalt: ZellInhalt::Markup(innen(text, &c).to_string()),
                weitere: vec![],
                ort: innen_bereich(&c),
            }),
            SyntaxKind::FuncCall if tabellen_glied(&c).is_some() => {
                let (glied, kurz) = tabellen_glied(&c).unwrap_or_default();
                match glied.as_str() {
                    "cell" if kurz => roh.push(zelle_lesen(text, &c)?),
                    "header" | "footer" | "hline" | "vline" => {
                        return Err(tr!("Tabelle mit table.{glied} – im Text bearbeiten", glied));
                    }
                    "cell" => return Err(berechnet()),
                    _ => roh.push(ausdruck(text, &c)),
                }
            }
            // Eine Variable oder ein Feld kann eine verbundene Zelle oder eine Linie sein; ihre
            // Lage im Raster ist ohne Auswerten nicht bekannt.
            SyntaxKind::Ident | SyntaxKind::FieldAccess => return Err(berechnet()),
            _ => roh.push(ausdruck(text, &c)),
        }
    }

    let (zellen, inhaltsorte, zeilen) = platzieren(spalten, roh)?;
    let start = aufruf.offset();
    let name = text[start..args.offset()].to_string();
    let aufruf_text = &text[aufruf.range()];
    let mehrzeilig = aufruf_text.contains('\n');
    let schluss = einzug_der_zeile(text, start);
    let einzug = match erstes_argument {
        Some(o) if text[args.offset()..o].contains('\n') => einzug_der_zeile(text, o),
        _ => format!("{schluss}  "),
    };
    Ok(TabellenStelle {
        aufruf: aufruf.range(),
        tabelle: Tabelle {
            spalten,
            zeilen,
            spuren,
            weitere,
            zellen,
        },
        form: Form {
            name,
            mehrzeilig,
            einzug,
            schluss,
            spalten: spalten_form,
            spalten_stelle,
        },
        inhaltsorte,
    })
}

/// `columns`: fehlend/`auto` → 1, ganzzahliges Literal ≥ 1, oder ein Array-Literal (Spuren).
fn spalten_lesen(
    text: &str,
    wert: &LinkedNode,
) -> Result<(usize, Option<Vec<String>>, SpaltenForm), String> {
    match wert.kind() {
        SyntaxKind::Auto => Ok((1, None, SpaltenForm::Auto)),
        SyntaxKind::Int => match wert.cast::<ast::Int>().map(|i| i.get()) {
            Some(n) if n as u64 > MAX_SPALTEN as u64 => Err(zu_gross()),
            Some(n) if n >= 1 => Ok((n as usize, None, SpaltenForm::Zahl)),
            _ => Err(berechnet()),
        },
        SyntaxKind::Array => {
            let mut spuren = Vec::new();
            for e in wert.children() {
                match e.kind() {
                    SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => {}
                    SyntaxKind::LineComment | SyntaxKind::BlockComment => {
                        return Err(kommentar());
                    }
                    k if k.is_trivia() => {}
                    SyntaxKind::Spread => return Err(berechnet()),
                    _ => spuren.push(text[e.range()].to_string()),
                }
            }
            if spuren.is_empty() {
                return Err(berechnet());
            }
            if spuren.len() > MAX_SPALTEN {
                return Err(zu_gross());
            }
            Ok((spuren.len(), Some(spuren), SpaltenForm::Spuren))
        }
        _ => Err(berechnet()),
    }
}

/// `table.<glied>(…)` bzw. `….table.<glied>(…)` (etwa `std.table.header`): der Name des Glieds
/// und ob der Aufruf genau `table.<glied>` heißt; sonst `None`.
fn tabellen_glied(aufruf: &LinkedNode) -> Option<(String, bool)> {
    let callee = aufruf.children().next()?;
    if callee.kind() != SyntaxKind::FieldAccess {
        return None;
    }
    let teile: Vec<LinkedNode> = callee.children().collect();
    let [ziel, .., glied] = teile.as_slice() else {
        return None;
    };
    if glied.kind() != SyntaxKind::Ident {
        return None;
    }
    let kurz = match ziel.kind() {
        SyntaxKind::Ident if ziel.get().leaf_text() == "table" => true,
        SyntaxKind::FieldAccess
            if ziel.children().last().is_some_and(|l| {
                l.kind() == SyntaxKind::Ident && l.get().leaf_text() == "table"
            }) =>
        {
            false
        }
        _ => return None,
    };
    Some((glied.get().leaf_text().to_string(), kurz))
}

/// Ein Kommentar direkt in diesem Knoten (etwa zwischen `colspan:` und dem Wert).
fn hat_kommentar(knoten: &LinkedNode) -> bool {
    knoten
        .children()
        .any(|c| matches!(c.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment))
}

fn zelle_lesen(text: &str, aufruf: &LinkedNode) -> Result<Roh, String> {
    let args = aufruf
        .children()
        .find(|c| c.kind() == SyntaxKind::Args)
        .ok_or_else(zellfehler)?;
    let mut zeilen = 1;
    let mut spalten = 1;
    let mut weitere = Vec::new();
    let mut rumpf: Option<LinkedNode> = None;
    for c in args.children() {
        match c.kind() {
            SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma => {}
            SyntaxKind::LineComment | SyntaxKind::BlockComment => return Err(kommentar()),
            k if k.is_trivia() => {}
            SyntaxKind::Spread => return Err(tr!("Tabelle mit `..` – im Text bearbeiten")),
            SyntaxKind::Named => {
                let named = c.cast::<ast::Named>().ok_or_else(berechnet)?;
                let name = named.name().as_str().to_string();
                match name.as_str() {
                    "colspan" | "rowspan" => {
                        if hat_kommentar(&c) {
                            return Err(kommentar());
                        }
                        let n = match named.expr() {
                            ast::Expr::Int(i) if i.get() as u64 > MAX_FELDER as u64 => {
                                return Err(zu_gross());
                            }
                            ast::Expr::Int(i) if i.get() >= 1 => i.get() as usize,
                            _ => return Err(berechnet()),
                        };
                        if name == "colspan" {
                            spalten = n;
                        } else {
                            zeilen = n;
                        }
                    }
                    "x" | "y" => {
                        return Err(tr!("Zelle mit fester Position – im Text bearbeiten"));
                    }
                    _ => weitere.push(text[c.range()].to_string()),
                }
            }
            _ => {
                // Genau ein Inhalt, als Rumpf `[…]` oder als positionales Argument.
                if rumpf.is_some() {
                    return Err(zellfehler());
                }
                rumpf = Some(c);
            }
        }
    }
    let rumpf = rumpf.ok_or_else(zellfehler)?;
    let (inhalt, ort) = if rumpf.kind() == SyntaxKind::ContentBlock {
        (
            ZellInhalt::Markup(innen(text, &rumpf).to_string()),
            innen_bereich(&rumpf),
        )
    } else {
        (
            ZellInhalt::Ausdruck(text[rumpf.range()].to_string()),
            rumpf.range(),
        )
    };
    Ok(Roh {
        zeilen,
        spalten,
        inhalt,
        weitere,
        ort,
    })
}

fn ausdruck(text: &str, knoten: &LinkedNode) -> Roh {
    Roh {
        zeilen: 1,
        spalten: 1,
        inhalt: ZellInhalt::Ausdruck(text[knoten.range()].to_string()),
        weitere: vec![],
        ort: knoten.range(),
    }
}

fn innen_bereich(block: &LinkedNode) -> Range<usize> {
    let r = block.range();
    r.start + 1..r.end - 1
}

fn innen<'a>(text: &'a str, block: &LinkedNode) -> &'a str {
    &text[innen_bereich(block)]
}

/// Leerraum am Anfang der Zeile, in der `pos` liegt.
fn einzug_der_zeile(text: &str, pos: usize) -> String {
    let anfang = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    text[anfang..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// Platziert die Zellen wie Typst 0.15.1 und füllt freie Felder mit leeren Zellen auf.
///
/// Regel aus `typst-library-0.15.1/src/layout/grid/resolve.rs`: `resolve_cell_position` (Fall
/// Auto/Auto) setzt eine Zelle an die erste freie Stelle ab dem Zähler `auto_index`; danach steigt
/// der Zähler um `colspan`, bei voller Breite um `colspan * rowspan`. Ragt `colspan` über den
/// rechten Rand („cell's colspan would cause it to exceed the available column(s)“) oder überlappt
/// die Zelle eine andere, meldet Typst einen Fehler.
#[allow(clippy::type_complexity)]
fn platzieren(
    spalten: usize,
    roh: Vec<Roh>,
) -> Result<(Vec<Zelle>, Vec<Option<Range<usize>>>, usize), String> {
    let mut belegt: Vec<bool> = Vec::new();
    let frei = |b: &Vec<bool>, i: usize| !b.get(i).copied().unwrap_or(false);
    let mut zaehler = 0usize;
    let mut zellen: Vec<(Zelle, Option<Range<usize>>)> = Vec::new();
    for r in roh {
        let mut i = zaehler;
        while !frei(&belegt, i) {
            i += 1;
        }
        let (zeile, spalte) = (i / spalten, i % spalten);
        if (zeile + r.zeilen).saturating_mul(spalten) > MAX_FELDER {
            return Err(zu_gross());
        }
        if r.spalten > spalten - spalte {
            return Err(zellfehler());
        }
        for z in zeile..zeile + r.zeilen {
            for s in spalte..spalte + r.spalten {
                let f = z * spalten + s;
                if !frei(&belegt, f) {
                    return Err(zellfehler());
                }
                if belegt.len() <= f {
                    belegt.resize((f / spalten + 1) * spalten, false);
                }
                belegt[f] = true;
            }
        }
        zaehler = if r.spalten == spalten {
            i + r.spalten * r.zeilen
        } else {
            i + r.spalten
        };
        zellen.push((
            Zelle {
                zeile,
                spalte,
                zeilen: r.zeilen,
                spalten: r.spalten,
                inhalt: r.inhalt,
                weitere: r.weitere,
            },
            Some(r.ort),
        ));
    }
    let zeilen = belegt.len().div_ceil(spalten);
    for f in 0..zeilen * spalten {
        if frei(&belegt, f) {
            zellen.push((
                Zelle {
                    zeile: f / spalten,
                    spalte: f % spalten,
                    zeilen: 1,
                    spalten: 1,
                    inhalt: ZellInhalt::Markup(String::new()),
                    weitere: vec![],
                },
                None,
            ));
        }
    }
    zellen.sort_by_key(|(z, _)| (z.zeile, z.spalte));
    let (zellen, orte) = zellen.into_iter().unzip();
    Ok((zellen, orte, zeilen))
}

/// Patch für `aktion` an der Tabelle `stelle`; geprüft nach (a)–(c).
///
/// `Zelltext` in einer Zelle, die in der Quelle steht, ersetzt nur deren Inhalt (bei einem
/// Ausdruck den ganzen Ausdruck durch `[…]`). Jede andere Aktion schreibt den ganzen Aufruf in der
/// festen Form neu: benannte Argumente je eine Zeile, dann eine Zeile je Rasterzeile; ein
/// einzeiliger Aufruf bleibt einzeilig.
pub fn tabelle_patch(
    quelle: &Source,
    stelle: &TabellenStelle,
    aktion: &TabellenAktion,
) -> Result<Patch, PatchFehler> {
    // Die Stelle muss zu diesem Text gehören (sonst zeigen ihre Bereiche ins Leere).
    match tabelle_am(quelle, stelle.aufruf.start) {
        Some(Ok(s)) if s.aufruf == stelle.aufruf && s.tabelle == stelle.tabelle => {}
        _ => {
            return Err(PatchFehler::Gesperrt(tr!(
                "Die Tabelle hat sich inzwischen geändert"
            )));
        }
    }
    let erwartet = stelle.tabelle.aendern(aktion)?;
    let minimal = match aktion {
        TabellenAktion::Zelltext {
            zeile,
            spalte,
            inhalt,
        } => stelle.tabelle.zelle_bei(*zeile, *spalte).and_then(|i| {
            let ort = stelle.inhaltsorte.get(i).cloned().flatten()?;
            Some(Patch {
                bereich: ort,
                text: match stelle.tabelle.zellen[i].inhalt {
                    ZellInhalt::Markup(_) => inhalt.clone(),
                    ZellInhalt::Ausdruck(_) => format!("[{inhalt}]"),
                },
            })
        }),
        _ => None,
    };
    let patch = minimal.unwrap_or_else(|| Patch {
        bereich: stelle.aufruf.clone(),
        text: schreiben(&stelle.form, &erwartet),
    });

    let neu = aussen_pruefen(quelle, &patch)?;
    let ende = stelle.aufruf.end + patch.text.len() + patch.bereich.start - patch.bereich.end;
    match tabelle_am(&neu, stelle.aufruf.start) {
        Some(Ok(s)) if s.aufruf == (stelle.aufruf.start..ende) && s.tabelle == erwartet => {
            Ok(patch)
        }
        _ => Err(PatchFehler::Invariante(
            "(b) Tabelle danach nicht wie erwartet",
        )),
    }
}

/// Der Aufruf in der festen Form.
fn schreiben(form: &Form, t: &Tabelle) -> String {
    let columns = match (&t.spuren, form.spalten) {
        (Some(s), _) if s.len() == 1 => Some(format!("columns: ({},)", s[0])),
        (Some(s), _) => Some(format!("columns: ({})", s.join(", "))),
        (None, SpaltenForm::Auto) if t.spalten == 1 => Some("columns: auto".into()),
        (None, SpaltenForm::Fehlt) if t.spalten == 1 => None,
        (None, _) => Some(format!("columns: {}", t.spalten)),
    };
    let mut benannt = t.weitere.clone();
    if let Some(c) = columns {
        let stelle = form.spalten_stelle.unwrap_or(0).min(benannt.len());
        benannt.insert(stelle, c);
    }
    let mut zeilen: Vec<Vec<String>> = vec![Vec::new(); t.zeilen];
    for z in &t.zellen {
        zeilen[z.zeile].push(zelle_schreiben(z));
    }
    let zeilen: Vec<String> = zeilen
        .into_iter()
        .filter(|z| !z.is_empty())
        .map(|z| z.join(", "))
        .collect();
    let mut aus = format!("{}(", form.name);
    if form.mehrzeilig {
        aus.push('\n');
        for b in benannt.iter().chain(&zeilen) {
            aus.push_str(&format!("{}{b},\n", form.einzug));
        }
        aus.push_str(&form.schluss);
    } else {
        aus.push_str(
            &benannt
                .into_iter()
                .chain(zeilen)
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    aus.push(')');
    aus
}

fn zelle_schreiben(z: &Zelle) -> String {
    let mut args = z.weitere.clone();
    if z.spalten > 1 {
        args.push(format!("colspan: {}", z.spalten));
    }
    if z.zeilen > 1 {
        args.push(format!("rowspan: {}", z.zeilen));
    }
    match (&z.inhalt, args.is_empty()) {
        (ZellInhalt::Markup(m), true) => format!("[{m}]"),
        (ZellInhalt::Ausdruck(a), true) => a.clone(),
        (ZellInhalt::Markup(m), false) => format!("table.cell({})[{m}]", args.join(", ")),
        (ZellInhalt::Ausdruck(a), false) => format!("table.cell({}, {a})", args.join(", ")),
    }
}
