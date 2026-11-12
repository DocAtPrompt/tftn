//! Widgets: Literale über den Syntaxbaum ändern (KONZEPT 4.5, PLAN-M1 AP6).
//!
//! Am Cursor wird der umschließende Aufruf eines deklarierten Bausteins gesucht (z. B.
//! `hinweis(farbe: …)`). Ist das Argument ein reines Literal, berechnet TFTN einen Patch, der genau
//! den Bytebereich des alten Literals ersetzt; fehlt das Argument, wird es ergänzt. Jeder Patch wird
//! vor der Rückgabe geprüft:
//! (a) außerhalb des Bereichs ändert sich kein Byte,
//! (b) der Syntaxbaum ist bis auf das Literal (bzw. das ergänzte Argument) gleich,
//! (c) es entsteht kein Fehlerknoten.
//! Ausdrücke, die keine Literale sind, werden angezeigt, aber nicht geändert.
//!
//! Seit PLAN-M2 AP4 gehen alle Widget-Arten aus KONZEPT 5.4 diesen Weg (`literal`: Werte lesen und
//! schreiben; `wert_patch`), dazu die Patch-Klasse Inhalt (`caption: […]`, Rumpf `[…]`) und die
//! Markup-Helfer (`markup`). Seit AP5 dazu die Patch-Klasse Tabellenstruktur (`tabelle`): Zeilen
//! und Spalten, verbundene Zellen und Zelltext eines `table(…)`. Die Bedienung in der Oberfläche
//! kommt mit AP6.

pub mod gliederung;
pub mod literal;
pub mod markup;
pub mod rechtschreibung;
pub mod seite;
pub mod tabelle;
pub mod zaehlen;

use std::ops::Range;
use tftn_sprachen::tr;

pub use gliederung::{Ueberschrift, abschnitt_verschieben, gliederung};
pub use literal::{
    Farbe, Farbraum, Wert, WidgetArt, ZAHL_GRENZE, farbe_lesen, farbe_literal, hex_lesen, lesen,
    schreiben,
};
pub use markup::{
    EBENE_MAX, MarkupAktion, MarkupStelle, formel_einfuegen, formel_patch, formel_text,
    markup_patch, markup_stellen, zitat_patch,
};
pub use seite::{Raender, SeitenFeld, SeitenStand, Zeile, seite_lesen, seite_patch};
pub use tabelle::{
    Tabelle, TabellenAktion, TabellenStelle, ZellInhalt, Zelle, tabelle_am, tabelle_patch,
};
pub use zaehlen::{Zahlen, zaehlen};

use serde::Serialize;
use typst_syntax::{LinkedNode, Side, Source, SyntaxKind, SyntaxNode};

/// Ein Parameter eines Bausteins, der ein Widget bekommt (aus dem Manifest).
#[derive(Debug, Clone)]
pub struct WidgetParameter {
    /// Name der Typst-Funktion, z. B. `hinweis`.
    pub funktion: String,
    /// Name des benannten Arguments, z. B. `farbe`.
    pub parameter: String,
    /// Widget-Art aus dem Manifest samt Zusatzschlüsseln.
    pub art: WidgetArt,
    /// Positionaler Parameter: Index unter den positionalen Argumenten (`image("a.png")` → 0);
    /// `None` für ein benanntes Argument bzw. den Rumpf.
    pub position: Option<usize>,
}

/// Wo ein Wert steht oder ergänzt würde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
pub enum Ort {
    /// Das Argument steht da; `bereich` ist der Bytebereich seines Werts.
    Ersetzen { bereich: Range<usize> },
    /// Das Argument fehlt; Einfügen bei `pos` als `vorne` + `name: wert` + `hinten`.
    Einfuegen {
        pos: usize,
        vorne: String,
        hinten: String,
    },
    /// Der Rumpf `[…]` fehlt; er wird bei `pos` (Ende des Aufrufs) angehängt.
    Anhaengen { pos: usize },
}

/// Ein Widget, das am Cursor angeboten wird.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Angebot {
    pub funktion: String,
    pub parameter: String,
    /// Name der Widget-Art (`WidgetArt::name`).
    pub widget: String,
    /// Die Widget-Art samt Zusatzschlüsseln; nur mit ihr wird geschrieben.
    #[serde(skip)]
    pub art: WidgetArt,
    /// Wie `WidgetParameter::position` (für das Wiederlesen in der Prüfung).
    #[serde(skip)]
    pub position: Option<usize>,
    /// Bytebereich des ganzen Aufrufs (zur Wiedererkennung).
    pub aufruf: Range<usize>,
    pub ort: Ort,
    /// Aktueller Wert, falls er ein lesbares Literal ist.
    pub wert: Option<Wert>,
    /// Quelltext des aktuellen Werts (auch wenn nicht änderbar).
    pub quelltext: Option<String>,
    /// `None`: änderbar; sonst der Grund, warum nicht.
    pub gesperrt: Option<String>,
}

/// Ein Patch auf die Quelle in Bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Patch {
    pub bereich: Range<usize>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchFehler {
    Gesperrt(String),
    UngueltigerWert(String),
    /// Eine Invariante (a)–(c) ist verletzt; der Patch wird verworfen.
    Invariante(&'static str),
}

impl std::fmt::Display for PatchFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gesperrt(g) => f.write_str(&tr!("nicht änderbar: {grund}", grund = g)),
            Self::UngueltigerWert(w) => f.write_str(&tr!("ungültiger Wert: {grund}", grund = w)),
            Self::Invariante(i) => write!(f, "Patch verworfen, Invariante verletzt: {i}"),
        }
    }
}

impl std::error::Error for PatchFehler {}

/// Alle Widgets am Cursor (Byte): die des innersten passenden Aufrufs.
pub fn angebote(quelle: &Source, byte: usize, parameter: &[WidgetParameter]) -> Vec<Angebot> {
    ebenen(quelle, byte, parameter)
        .into_iter()
        .next()
        .map(|e| e.angebote)
        .unwrap_or_default()
}

/// Ein umschließender Aufruf eines Bausteins mit seinen Widgets (PLAN-M2 AP6, Fach „Hier“).
#[derive(Debug, Clone, PartialEq)]
pub struct Ebene {
    pub funktion: String,
    pub aufruf: Range<usize>,
    pub angebote: Vec<Angebot>,
}

/// Alle umschließenden Aufrufe mit mindestens einem Parameter in `parameter`, innerster zuerst.
pub fn ebenen(quelle: &Source, byte: usize, parameter: &[WidgetParameter]) -> Vec<Ebene> {
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before));
    let mut aus = Vec::new();
    while let Some(k) = knoten {
        if let Some(e) = ebene(quelle, &k, parameter) {
            aus.push(e);
        }
        knoten = k.parent().cloned();
    }
    aus
}

/// Alle Angebote an Aufrufen, deren Bereich `bereich` schneidet, in Quellreihenfolge (Marken B).
pub fn angebote_im_bereich(
    quelle: &Source,
    bereich: Range<usize>,
    parameter: &[WidgetParameter],
) -> Vec<Angebot> {
    fn gehen(
        quelle: &Source,
        k: &LinkedNode,
        bereich: &Range<usize>,
        parameter: &[WidgetParameter],
        aus: &mut Vec<Angebot>,
    ) {
        let r = k.range();
        if r.start >= bereich.end || r.end <= bereich.start {
            return;
        }
        if let Some(e) = ebene(quelle, k, parameter) {
            aus.extend(e.angebote);
        }
        for c in k.children() {
            gehen(quelle, &c, bereich, parameter, aus);
        }
    }
    let mut aus = Vec::new();
    gehen(
        quelle,
        &LinkedNode::new(quelle.root()),
        &bereich,
        parameter,
        &mut aus,
    );
    aus
}

fn ebene(quelle: &Source, k: &LinkedNode, parameter: &[WidgetParameter]) -> Option<Ebene> {
    if k.kind() != SyntaxKind::FuncCall {
        return None;
    }
    let name = aufgerufene_funktion(k)?;
    let angebote: Vec<Angebot> = parameter
        .iter()
        .filter(|p| funktion_passt(&name, &p.funktion))
        .map(|p| angebot(quelle, k, p))
        .collect();
    let funktion = angebote.first()?.funktion.clone();
    Some(Ebene {
        funktion,
        aufruf: k.range(),
        angebote,
    })
}

/// Positionales Argument Nr. `n` (ohne benannte Argumente und ohne Inhaltsblöcke hinter der
/// Klammer). Fehlt es, ist es gesperrt: Einfügen an einer Position wäre zu fehleranfällig.
fn positional_setzen(quelle: &Source, a: &mut Angebot, args: &LinkedNode, n: usize) {
    let mut in_klammern = false;
    let mut i = 0;
    for c in args.children() {
        match c.kind() {
            SyntaxKind::LeftParen => in_klammern = true,
            SyntaxKind::RightParen => break,
            SyntaxKind::Comma | SyntaxKind::Named => {}
            k if k.is_trivia() => {}
            _ if !in_klammern => {}
            SyntaxKind::Spread => {
                a.gesperrt = Some(tr!("berechnet – im Text bearbeiten"));
                return;
            }
            _ => {
                if i == n {
                    if matches!(a.art, WidgetArt::Inhalt { .. })
                        && c.kind() == SyntaxKind::ContentBlock
                    {
                        let art = a.art.clone();
                        inhalt_setzen(quelle, a, &art, &c);
                        return;
                    }
                    a.quelltext = Some(quelle.text()[c.range()].to_string());
                    a.ort = Ort::Ersetzen { bereich: c.range() };
                    match lesen(&a.art, c.get()) {
                        Some(w) => a.wert = Some(w),
                        None => {
                            a.gesperrt
                                .get_or_insert_with(|| tr!("berechnet – im Text bearbeiten"));
                        }
                    }
                    return;
                }
                i += 1;
            }
        }
    }
    a.gesperrt = Some(tr!("fehlt – im Text ergänzen"));
}

/// Pfad des Aufrufs: `hinweis`, `modul.hinweis`, `cetz.draw.line`. Ist die Wurzel kein Bezeichner
/// (`f(x).g()`), nur das letzte Segment.
fn aufgerufene_funktion(aufruf: &LinkedNode) -> Option<String> {
    pfad(&aufruf.children().next()?)
}

fn pfad(k: &LinkedNode) -> Option<String> {
    match k.kind() {
        SyntaxKind::Ident => Some(k.get().leaf_text().to_string()),
        SyntaxKind::FieldAccess => {
            let feld = k
                .children()
                .filter(|c| c.kind() == SyntaxKind::Ident)
                .last()?;
            let feld = feld.get().leaf_text().to_string();
            Some(match k.children().next().and_then(|z| pfad(&z)) {
                Some(vorne) => format!("{vorne}.{feld}"),
                None => feld,
            })
        }
        _ => None,
    }
}

/// Passt der Aufrufpfad auf einen `funktion`-Namen des Katalogs? Einer der beiden muss an einer
/// Punktgrenze auf den anderen enden; ein Aufruf ohne Punkt passt nur auf denselben Namen, sonst
/// fiele Typsts eigenes `line` unter `cetz.draw.line`.
pub fn funktion_passt(aufruf: &str, funktion: &str) -> bool {
    fn endet(lang: &str, kurz: &str) -> bool {
        lang == kurz
            || lang
                .strip_suffix(kurz)
                .is_some_and(|vorne| vorne.ends_with('.'))
    }
    if !aufruf.contains('.') {
        return aufruf == funktion;
    }
    endet(aufruf, funktion) || endet(funktion, aufruf)
}

fn angebot(quelle: &Source, aufruf: &LinkedNode, p: &WidgetParameter) -> Angebot {
    let mut a = Angebot {
        funktion: p.funktion.clone(),
        parameter: p.parameter.clone(),
        widget: p.art.name().to_string(),
        art: p.art.clone(),
        position: p.position,
        aufruf: aufruf.range(),
        ort: Ort::Einfuegen {
            pos: 0,
            vorne: String::new(),
            hinten: String::new(),
        },
        wert: None,
        quelltext: None,
        gesperrt: None,
    };
    if let WidgetArt::Unbekannt(n) = &p.art {
        a.gesperrt = Some(tr!("unbekannte Widget-Art {n}", n));
    }
    let Some(args) = aufruf.children().find(|c| c.kind() == SyntaxKind::Args) else {
        a.gesperrt = Some(tr!("Aufruf ohne Argumente"));
        return a;
    };
    if let Some(n) = p.position {
        positional_setzen(quelle, &mut a, &args, n);
        return a;
    }
    // Rumpf: der letzte Inhaltsblock der Argumente (`#f(…)[…]`, `#f[…]`); fehlt er, anhängen.
    if matches!(p.art, WidgetArt::Inhalt { rumpf: true }) {
        match args
            .children()
            .filter(|c| c.kind() == SyntaxKind::ContentBlock)
            .last()
        {
            Some(block) => inhalt_setzen(quelle, &mut a, &p.art, &block),
            None => {
                a.ort = Ort::Anhaengen {
                    pos: aufruf.range().end,
                };
                // Steht der Inhalt schon positional da (Variable, String), wäre ein angehängter
                // Rumpf ein zweites Argument.
                let positional = args.children().any(|c| {
                    !c.kind().is_trivia()
                        && !matches!(
                            c.kind(),
                            SyntaxKind::LeftParen
                                | SyntaxKind::RightParen
                                | SyntaxKind::Comma
                                | SyntaxKind::Named
                        )
                });
                if positional {
                    a.gesperrt
                        .get_or_insert_with(|| tr!("berechnet – im Text bearbeiten"));
                }
            }
        }
        return a;
    }
    // Benanntes Argument suchen
    for arg in args.children().filter(|c| c.kind() == SyntaxKind::Named) {
        let name = arg.children().find(|c| c.kind() == SyntaxKind::Ident);
        if name.is_some_and(|n| n.get().leaf_text() == p.parameter.as_str()) {
            let Some(wert) = arg.children().filter(|c| !c.kind().is_trivia()).last() else {
                break;
            };
            if p.art == WidgetArt::Tabelle {
                tabelle_setzen(quelle, &mut a, &wert);
                return a;
            }
            if matches!(p.art, WidgetArt::Inhalt { .. }) && wert.kind() == SyntaxKind::ContentBlock
            {
                inhalt_setzen(quelle, &mut a, &p.art, &wert);
                return a;
            }
            a.quelltext = Some(quelle.text()[wert.range()].to_string());
            a.ort = Ort::Ersetzen {
                bereich: wert.range(),
            };
            match lesen(&p.art, wert.get()) {
                Some(w) => a.wert = Some(w),
                None => {
                    a.gesperrt
                        .get_or_insert_with(|| tr!("berechnet – im Text bearbeiten"));
                }
            }
            return a;
        }
    }
    // Fehlt: nach "(" einfügen oder Klammern ergänzen.
    if p.art == WidgetArt::Tabelle {
        a.gesperrt = Some(tr!("Tabelle fehlt – im Text ergänzen"));
    }
    let kinder: Vec<LinkedNode> = args.children().collect();
    match kinder
        .iter()
        .position(|c| c.kind() == SyntaxKind::LeftParen)
    {
        Some(i) => {
            let weitere = kinder[i + 1..]
                .iter()
                .take_while(|c| c.kind() != SyntaxKind::RightParen)
                .any(|c| !c.kind().is_trivia());
            a.ort = Ort::Einfuegen {
                pos: kinder[i].range().end,
                vorne: String::new(),
                hinten: if weitere { ", ".into() } else { String::new() },
            };
        }
        None => {
            a.ort = Ort::Einfuegen {
                pos: args.offset(),
                vorne: "(".into(),
                hinten: ")".into(),
            };
        }
    }
    a
}

/// Tabelle als Wert (PLAN-M2 AP5): der Ort ist der ganze Aufruf `table(…)`; geändert wird über
/// das Raster (`tabelle_am`, `tabelle_patch`), nicht über `wert_patch`.
fn tabelle_setzen(quelle: &Source, a: &mut Angebot, wert: &LinkedNode) {
    a.quelltext = Some(quelle.text()[wert.range()].to_string());
    a.ort = Ort::Ersetzen {
        bereich: wert.range(),
    };
    let ist_tabelle = wert.kind() == SyntaxKind::FuncCall
        && wert
            .children()
            .next()
            .is_some_and(|c| c.kind() == SyntaxKind::Ident && c.get().leaf_text() == "table");
    a.gesperrt = match ist_tabelle
        .then(|| tabelle_am(quelle, wert.offset()))
        .flatten()
    {
        Some(Ok(_)) => None,
        Some(Err(grund)) => Some(grund),
        None => Some(tr!("berechnet – im Text bearbeiten")),
    };
}

/// Inhaltsblock als Ort: Bereich **innerhalb** der Klammern (Entwurf 4, Patch-Klasse Inhalt).
fn inhalt_setzen(quelle: &Source, a: &mut Angebot, art: &WidgetArt, block: &LinkedNode) {
    let r = block.range();
    let innen = r.start + 1..r.end - 1;
    a.quelltext = Some(quelle.text()[r].to_string());
    a.ort = Ort::Ersetzen { bereich: innen };
    match lesen(art, block.get()) {
        Some(w) => a.wert = Some(w),
        None => {
            a.gesperrt
                .get_or_insert_with(|| tr!("berechnet – im Text bearbeiten"));
        }
    }
}

/// Patch für eine neue Farbe `rrggbb`; nur für ein RGB-Farb-Widget, geprüft nach (a)–(c).
pub fn farbe_patch(quelle: &Source, angebot: &Angebot, hex: &str) -> Result<Patch, PatchFehler> {
    let rgb = hex_lesen(hex).ok_or_else(|| PatchFehler::UngueltigerWert(hex.into()))?;
    wert_patch(quelle, angebot, &angebot.art, &Wert::Farbe(Farbe::Rgb(rgb)))
}

/// Patch für einen neuen Wert der Art `art`; geprüft nach (a)–(c).
pub fn wert_patch(
    quelle: &Source,
    angebot: &Angebot,
    art: &WidgetArt,
    wert: &Wert,
) -> Result<Patch, PatchFehler> {
    if let Some(g) = &angebot.gesperrt {
        return Err(PatchFehler::Gesperrt(g.clone()));
    }
    // Geschrieben wird nur mit der Art des Angebots (sonst z. B. RGB statt verlangtem CMYK).
    if *art != angebot.art {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "Widget-Art {art} passt nicht zum Angebot ({angebot})",
            art = art.name(),
            angebot = angebot.widget
        )));
    }
    let literal = schreiben(art, wert)?;
    let patch = match (&angebot.ort, wert) {
        // Inhalt: ersetzt wird nur, was zwischen den Klammern steht.
        (Ort::Ersetzen { bereich }, Wert::Inhalt(markup)) => Patch {
            bereich: bereich.clone(),
            text: markup.clone(),
        },
        (Ort::Ersetzen { bereich }, _) => Patch {
            bereich: bereich.clone(),
            text: literal,
        },
        (Ort::Anhaengen { pos }, _) => Patch {
            bereich: *pos..*pos,
            text: literal,
        },
        (Ort::Einfuegen { pos, vorne, hinten }, _) => Patch {
            bereich: *pos..*pos,
            text: format!("{vorne}{}: {literal}{hinten}", angebot.parameter),
        },
    };
    pruefen(quelle, angebot, art, &patch, wert)?;
    Ok(patch)
}

/// Wendet einen Patch auf Text an.
pub fn anwenden(text: &str, patch: &Patch) -> String {
    format!(
        "{}{}{}",
        &text[..patch.bereich.start],
        patch.text,
        &text[patch.bereich.end..]
    )
}

/// Prüft die Invarianten (a)–(c) für `patch` auf `quelle`.
pub fn pruefen(
    quelle: &Source,
    angebot: &Angebot,
    art: &WidgetArt,
    patch: &Patch,
    wert: &Wert,
) -> Result<(), PatchFehler> {
    let neu = aussen_pruefen(quelle, patch)?;
    let ende_neu = patch.bereich.start + patch.text.len();
    // ... und am selben Aufruf steht jetzt genau dieser Wert (endet vor `hinten`, z. B. ")").
    let wert_ende = match &angebot.ort {
        Ort::Einfuegen { hinten, .. } => ende_neu - hinten.len(),
        Ort::Ersetzen { .. } | Ort::Anhaengen { .. } => ende_neu,
    };
    // Ein ergänzter oder angehängter Inhaltsblock wird innerhalb seiner Klammern gelesen.
    let wert_ende = match (&angebot.ort, wert) {
        (Ort::Einfuegen { .. } | Ort::Anhaengen { .. }, Wert::Inhalt(_)) => wert_ende - 1,
        _ => wert_ende,
    };
    let aufruf_neu = angebot.aufruf.start..angebot.aufruf.end + ende_neu - patch.bereich.end;
    let parameter = [WidgetParameter {
        funktion: angebot.funktion.clone(),
        parameter: angebot.parameter.clone(),
        art: art.clone(),
        position: angebot.position,
    }];
    let wieder = angebote(&neu, aufruf_neu.start, &parameter);
    let passt = wieder.iter().any(|w| {
        w.aufruf == aufruf_neu
            && matches!(&w.ort, Ort::Ersetzen { bereich } if bereich.end == wert_ende)
            && w.wert.as_ref() == Some(wert)
    });
    if !passt {
        return Err(PatchFehler::Invariante(
            "(b) Aufruf trägt danach nicht genau den neuen Wert",
        ));
    }
    Ok(())
}

/// (a) kein Byte außerhalb des Bereichs, (c) kein neuer Fehlerknoten, (b) die Blätter vor und
/// nach dem Bereich unverändert. Liefert die neue Quelle für die Prüfung der Stelle selbst.
pub(crate) fn aussen_pruefen(quelle: &Source, patch: &Patch) -> Result<Source, PatchFehler> {
    let alt = quelle.text();
    let neu_text = anwenden(alt, patch);
    let ende_neu = patch.bereich.start + patch.text.len();

    // (a) kein Byte außerhalb des Bereichs geändert
    if neu_text[..patch.bereich.start] != alt[..patch.bereich.start]
        || neu_text[ende_neu..] != alt[patch.bereich.end..]
    {
        return Err(PatchFehler::Invariante(
            "(a) Bytes außerhalb des Bereichs geändert",
        ));
    }

    let neu = Source::detached(neu_text);
    // (c) kein neuer Fehlerknoten
    if fehler(neu.root()) > fehler(quelle.root()) {
        return Err(PatchFehler::Invariante("(c) neuer Fehlerknoten"));
    }

    // (b) Blätter vor und nach dem Bereich unverändert ...
    let blaetter_alt = blaetter(quelle.root());
    let blaetter_neu = blaetter(neu.root());
    let vor = |b: &[(usize, SyntaxKind, String)], grenze: usize| -> Vec<(SyntaxKind, String)> {
        b.iter()
            .filter(|(o, _, t)| o + t.len() <= grenze)
            .map(|(_, k, t)| (*k, t.clone()))
            .collect()
    };
    let nach = |b: &[(usize, SyntaxKind, String)], grenze: usize| -> Vec<(SyntaxKind, String)> {
        b.iter()
            .filter(|(o, _, _)| *o >= grenze)
            .map(|(_, k, t)| (*k, t.clone()))
            .collect()
    };
    if vor(&blaetter_alt, patch.bereich.start) != vor(&blaetter_neu, patch.bereich.start)
        || nach(&blaetter_alt, patch.bereich.end) != nach(&blaetter_neu, ende_neu)
    {
        return Err(PatchFehler::Invariante(
            "(b) Syntaxbaum außerhalb des Bereichs geändert",
        ));
    }
    Ok(neu)
}

pub(crate) fn fehler(knoten: &SyntaxNode) -> usize {
    usize::from(knoten.kind() == SyntaxKind::Error) + knoten.children().map(fehler).sum::<usize>()
}

/// Blätter mit Byte-Anfang, Art und Text.
fn blaetter(wurzel: &SyntaxNode) -> Vec<(usize, SyntaxKind, String)> {
    fn gehen(k: &SyntaxNode, offset: usize, aus: &mut Vec<(usize, SyntaxKind, String)>) {
        if k.children().len() == 0 {
            // Leere Knoten (z. B. `Markup` in `[]`) tragen keinen Text und zählen nicht.
            if !k.is_empty() {
                aus.push((offset, k.kind(), k.leaf_text().to_string()));
            }
            return;
        }
        let mut o = offset;
        for c in k.children() {
            gehen(c, o, aus);
            o += c.len();
        }
    }
    let mut aus = Vec::new();
    gehen(wurzel, 0, &mut aus);
    aus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funktion_passt_faelle() {
        for (aufruf, funktion) in [
            ("hinweis", "hinweis"),
            ("modul.hinweis", "hinweis"),
            ("cetz.draw.line", "cetz.draw.line"),
            ("draw.line", "cetz.draw.line"),
            ("cetz.draw.line", "line"),
        ] {
            assert!(funktion_passt(aufruf, funktion), "{aufruf} / {funktion}");
        }
        for (aufruf, funktion) in [
            ("line", "cetz.draw.line"),
            ("mydraw.line", "cetz.draw.line"),
        ] {
            assert!(!funktion_passt(aufruf, funktion), "{aufruf} / {funktion}");
        }
    }

    #[test]
    fn ebene_nennt_katalogname() {
        let p = vec![WidgetParameter {
            funktion: "cetz.draw.line".into(),
            parameter: "anfang".into(),
            art: WidgetArt::Punkt,
            position: Some(0),
        }];
        let t = "#cetz.draw.line((0, 0), (1, 1))";
        let q = Source::detached(t);
        let e = ebenen(&q, t.find("(0").unwrap(), &p);
        assert_eq!(e[0].funktion, "cetz.draw.line");
        let t = "#line(length: 1cm)";
        let q = Source::detached(t);
        assert!(ebenen(&q, t.find("length").unwrap(), &p).is_empty());
    }

    fn hinweis() -> Vec<WidgetParameter> {
        vec![WidgetParameter {
            funktion: "hinweis".into(),
            parameter: "farbe".into(),
            art: WidgetArt::Farbe {
                raum: Farbraum::Rgb,
            },
            position: None,
        }]
    }

    fn patch_text(text: &str, cursor: &str, hex: &str) -> Result<String, PatchFehler> {
        let q = Source::detached(text);
        let a = angebote(&q, text.find(cursor).unwrap(), &hinweis());
        let a = a.first().expect("Angebot");
        farbe_patch(&q, a, hex).map(|p| anwenden(text, &p))
    }

    #[test]
    fn cmyk_wird_nicht_still_rgb() {
        let cmyk = vec![WidgetParameter {
            position: None,
            funktion: "kasten".into(),
            parameter: "farbe".into(),
            art: WidgetArt::Farbe {
                raum: Farbraum::Cmyk,
            },
        }];
        let t = "#kasten(farbe: cmyk(0%, 50%, 100%, 0%))[x]";
        let q = Source::detached(t);
        let a = &angebote(&q, t.find('x').unwrap(), &cmyk)[0];
        assert!(farbe_patch(&q, a, "112233").is_err());
        // Eine Art, die nicht zum Angebot passt, wird nie geschrieben.
        let rgb = WidgetArt::Farbe {
            raum: Farbraum::Rgb,
        };
        assert!(matches!(
            wert_patch(&q, a, &rgb, &Wert::Farbe(Farbe::Rgb([1, 2, 3]))),
            Err(PatchFehler::UngueltigerWert(_))
        ));
        let neu = Wert::Farbe(Farbe::Cmyk([10, 20, 30, 40]));
        let p = wert_patch(&q, a, &a.art, &neu).unwrap();
        assert_eq!(
            anwenden(t, &p),
            "#kasten(farbe: cmyk(10%, 20%, 30%, 40%))[x]"
        );
    }

    #[test]
    fn ersetzt_nur_das_literal() {
        let t = "Text\n#hinweis(farbe: rgb(\"#E8F1F2\"), art: \"info\")[Inhalt *fett*]\nmehr";
        assert_eq!(
            patch_text(t, "Inhalt", "#00aa33").unwrap(),
            "Text\n#hinweis(farbe: rgb(\"#00aa33\"), art: \"info\")[Inhalt *fett*]\nmehr"
        );
    }

    #[test]
    fn liest_kurzformen_und_luma() {
        let q = Source::detached("#hinweis(farbe: luma(230))[x] #hinweis(farbe: rgb(\"#abc\"))[y]");
        let a = angebote(&q, 25, &hinweis());
        assert_eq!(a[0].wert, Some(Wert::Farbe(Farbe::Luma(230))));
        let b = angebote(&q, q.text().find('y').unwrap(), &hinweis());
        assert_eq!(b[0].wert, Some(Wert::Farbe(Farbe::Rgb([0xaa, 0xbb, 0xcc]))));
    }

    #[test]
    fn ergaenzt_fehlendes_argument() {
        assert_eq!(
            patch_text("#hinweis[Text]", "Text", "112233").unwrap(),
            "#hinweis(farbe: rgb(\"#112233\"))[Text]"
        );
        assert_eq!(
            patch_text("#hinweis(art: \"merke\")[Text]", "Text", "112233").unwrap(),
            "#hinweis(farbe: rgb(\"#112233\"), art: \"merke\")[Text]"
        );
        assert_eq!(
            patch_text("#hinweis()[Text]", "Text", "112233").unwrap(),
            "#hinweis(farbe: rgb(\"#112233\"))[Text]"
        );
    }

    #[test]
    fn berechnete_werte_sind_gesperrt() {
        let t = "#let c = red\n#hinweis(farbe: c.lighten(50%))[Text]";
        let q = Source::detached(t);
        let a = &angebote(&q, t.find("Text").unwrap(), &hinweis())[0];
        assert_eq!(a.quelltext.as_deref(), Some("c.lighten(50%)"));
        assert!(a.gesperrt.is_some());
        assert!(matches!(
            farbe_patch(&q, a, "112233"),
            Err(PatchFehler::Gesperrt(_))
        ));
    }

    #[test]
    fn innerster_passender_aufruf_auch_in_verschachteltem_inhalt() {
        let t = "#hinweis(farbe: luma(200))[außen #text(fill: red)[innen]]";
        let q = Source::detached(t);
        let a = angebote(&q, t.find("innen").unwrap(), &hinweis());
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].wert, Some(Wert::Farbe(Farbe::Luma(200))));
    }

    #[test]
    fn ausserhalb_kein_angebot_und_ungueltiger_wert() {
        let t = "Fließtext #hinweis[x]";
        let q = Source::detached(t);
        assert!(angebote(&q, 2, &hinweis()).is_empty());
        let a = &angebote(&q, t.find("[x]").unwrap() + 1, &hinweis())[0];
        assert!(matches!(
            farbe_patch(&q, a, "zz0000"),
            Err(PatchFehler::UngueltigerWert(_))
        ));
    }
}
