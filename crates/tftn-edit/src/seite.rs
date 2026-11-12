//! Seiteneinstellungen: Format, Schrift, Sprache als `#set`-Regeln auf der obersten Ebene.
//!
//! Gelesen und geändert wird die jeweils erste `set page`- bzw. `set text`-Regel. Eine Vorlage, die das
//! ganze Dokument per `#show: …` übernimmt, legt das selbst fest; dann ist nichts einstellbar.

use serde::{Deserialize, Serialize};
use tftn_sprachen::tr;
use typst_syntax::ast;
use typst_syntax::{LinkedNode, Source, SyntaxKind};

use crate::{Patch, PatchFehler, anwenden, fehler};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SeitenStand {
    /// Eine Vorlage legt Seite und Schrift fest.
    pub gesperrt: bool,
    pub format: Option<String>,
    pub quer: bool,
    pub schrift: Option<String>,
    pub groesse: Option<f64>,
    pub sprache: Option<String>,
    pub region: Option<String>,
    pub raender: Option<Raender>,
    pub doppelseitig: bool,
    pub spalten: u32,
    pub blocksatz: bool,
    pub zeilenabstand: Option<f64>,
    pub nummeriert: bool,
    pub kopf: Zeile,
    pub fuss: Zeile,
    /// Kopf- und Fußzeilen brauchen den Import des leeren Dokumenttyps.
    pub zeilen_moeglich: bool,
}

/// Seitenränder in cm; `innen`/`aussen` sind bei einseitigem Satz links/rechts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Raender {
    pub oben: f64,
    pub unten: f64,
    pub innen: f64,
    pub aussen: f64,
}

/// Kopf- oder Fußzeile; `{seite}` und `{kapitel}` setzt der Dokumenttyp ein.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zeile {
    pub innen: String,
    pub mitte: String,
    pub aussen: String,
    /// Auch auf der ersten Seite zeigen.
    pub erste: bool,
}

impl Default for Zeile {
    fn default() -> Self {
        Self {
            innen: String::new(),
            mitte: String::new(),
            aussen: String::new(),
            erste: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "art", content = "wert", rename_all = "kebab-case")]
pub enum SeitenFeld {
    Format(String),
    Quer(bool),
    Schrift(String),
    Groesse(f64),
    Sprache {
        lang: String,
        region: Option<String>,
    },
    Raender {
        oben: f64,
        unten: f64,
        innen: f64,
        aussen: f64,
        doppelseitig: bool,
    },
    Spalten(u32),
    Blocksatz(bool),
    Zeilenabstand(f64),
    Nummerierung(bool),
    Kopf(Zeile),
    Fuss(Zeile),
}

const GROESSE: std::ops::RangeInclusive<f64> = 4.0..=144.0;

fn oberste<'a>(wurzel: &'a LinkedNode<'a>) -> impl Iterator<Item = LinkedNode<'a>> {
    wurzel.children()
}

fn regel<'a>(wurzel: &'a LinkedNode<'a>, name: &str) -> Option<LinkedNode<'a>> {
    oberste(wurzel).find(|k| {
        k.cast::<ast::SetRule>()
            .is_some_and(|r| matches!(r.target(), ast::Expr::Ident(i) if i.as_str() == name))
    })
}

fn gesperrt(wurzel: &LinkedNode) -> bool {
    oberste(wurzel).any(|k| {
        k.cast::<ast::ShowRule>()
            .is_some_and(|s| s.selector().is_none())
    })
}

fn argument<'a>(k: &'a LinkedNode<'a>, name: &str) -> Option<ast::Expr<'a>> {
    let r = k.get().cast::<ast::SetRule>()?;
    r.args().items().find_map(|a| match a {
        ast::Arg::Named(n) if n.name().as_str() == name => Some(n.expr()),
        _ => None,
    })
}

fn text_von(e: Option<ast::Expr>) -> Option<String> {
    match e? {
        ast::Expr::Str(s) => Some(s.get().to_string()),
        _ => None,
    }
}

/// Die Einstellungen des Dokuments; was kein einfacher Wert ist, bleibt `None`.
pub fn seite_lesen(quelle: &Source) -> SeitenStand {
    let wurzel = LinkedNode::new(quelle.root());
    let mut s = SeitenStand {
        gesperrt: gesperrt(&wurzel),
        ..SeitenStand::default()
    };
    if let Some(k) = regel(&wurzel, "page") {
        s.format = text_von(argument(&k, "paper"));
        s.quer = matches!(argument(&k, "flipped"), Some(ast::Expr::Bool(b)) if b.get());
    }
    if let Some(k) = regel(&wurzel, "text") {
        s.schrift = text_von(argument(&k, "font"));
        s.sprache = text_von(argument(&k, "lang"));
        s.region = text_von(argument(&k, "region"));
        if let Some(ast::Expr::Numeric(n)) = argument(&k, "size")
            && let (wert, ast::Unit::Pt) = n.get()
        {
            s.groesse = Some(wert);
        }
    }
    s.spalten = 1;
    s.kopf = Zeile::default();
    s.fuss = Zeile::default();
    if let Some(k) = regel(&wurzel, "page") {
        let (raender, doppelseitig) = raender_lesen(argument(&k, "margin"));
        s.raender = raender;
        s.doppelseitig = doppelseitig;
        if let Some(ast::Expr::Int(n)) = argument(&k, "columns") {
            s.spalten = u32::try_from(n.get()).unwrap_or(1);
        }
        s.kopf = zeile_lesen(argument(&k, "header"));
        s.fuss = zeile_lesen(argument(&k, "footer"));
    }
    if let Some(k) = regel(&wurzel, "par") {
        s.blocksatz = matches!(argument(&k, "justify"), Some(ast::Expr::Bool(b)) if b.get());
        if let Some(ast::Expr::Numeric(n)) = argument(&k, "leading")
            && let (wert, ast::Unit::Em) = n.get()
        {
            s.zeilenabstand = Some(wert);
        }
    }
    s.nummeriert = regel(&wurzel, "heading").is_some_and(|k| argument(&k, "numbering").is_some());
    s.zeilen_moeglich = hat_zeilen(&wurzel);
    s
}

/// Länge in cm, wenn sie als einfache cm- oder mm-Angabe dasteht.
fn zentimeter(e: ast::Expr) -> Option<f64> {
    match e {
        ast::Expr::Numeric(n) => match n.get() {
            (w, ast::Unit::Cm) => Some(w),
            (w, ast::Unit::Mm) => Some(w / 10.0),
            _ => None,
        },
        _ => None,
    }
}

fn raender_lesen(e: Option<ast::Expr>) -> (Option<Raender>, bool) {
    match e {
        Some(ast::Expr::Dict(d)) => {
            let wert = |namen: &[&str]| {
                d.items().find_map(|i| match i {
                    ast::DictItem::Named(n) if namen.contains(&n.name().as_str()) => {
                        zentimeter(n.expr())
                    }
                    _ => None,
                })
            };
            let doppelseitig = wert(&["inside", "outside"]).is_some();
            let r = (|| {
                Some(Raender {
                    oben: wert(&["top"])?,
                    unten: wert(&["bottom"])?,
                    innen: wert(&["inside", "left"])?,
                    aussen: wert(&["outside", "right"])?,
                })
            })();
            (r, doppelseitig)
        }
        Some(e) => (
            zentimeter(e).map(|w| Raender {
                oben: w,
                unten: w,
                innen: w,
                aussen: w,
            }),
            false,
        ),
        None => (None, false),
    }
}

fn zeile_lesen(e: Option<ast::Expr>) -> Zeile {
    let mut z = Zeile::default();
    let Some(ast::Expr::FuncCall(c)) = e else {
        return z;
    };
    for a in c.args().items() {
        if let ast::Arg::Named(n) = a {
            match (n.name().as_str(), n.expr()) {
                ("innen", ast::Expr::Str(s)) => z.innen = s.get().to_string(),
                ("mitte", ast::Expr::Str(s)) => z.mitte = s.get().to_string(),
                ("aussen", ast::Expr::Str(s)) => z.aussen = s.get().to_string(),
                ("erste", ast::Expr::Bool(b)) => z.erste = b.get(),
                _ => {}
            }
        }
    }
    z
}

fn hat_zeilen(wurzel: &LinkedNode) -> bool {
    oberste(wurzel).any(|k| {
        k.kind() == SyntaxKind::ModuleImport
            && k.get()
                .cast::<ast::ModuleImport>()
                .is_some_and(|i| matches!(i.source(), ast::Expr::Str(s) if s.get().starts_with("@local/tftn-leer:")))
    })
}

fn zeile_text(funktion: &str, z: &Zeile) -> Option<String> {
    let mut args = Vec::new();
    for (name, wert) in [
        ("innen", &z.innen),
        ("mitte", &z.mitte),
        ("aussen", &z.aussen),
    ] {
        if !wert.is_empty() {
            args.push(format!("{name}: {}", zeichenkette(wert)));
        }
    }
    if args.is_empty() {
        return None;
    }
    if !z.erste {
        args.push("erste: false".to_string());
    }
    Some(format!("{funktion}({})", args.join(", ")))
}

fn zahl_im_bereich(
    w: f64,
    bereich: std::ops::RangeInclusive<f64>,
    was: &str,
) -> Result<(), PatchFehler> {
    if bereich.contains(&w) {
        Ok(())
    } else {
        Err(PatchFehler::UngueltigerWert(tr!(
            "{was} muss zwischen {von} und {bis} liegen.",
            was,
            von = bereich.start(),
            bis = bereich.end()
        )))
    }
}

fn zeichenkette(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn nicht_leer(s: &str, was: &str) -> Result<(), PatchFehler> {
    if s.trim().is_empty() {
        return Err(PatchFehler::UngueltigerWert(tr!("{was} fehlt", was)));
    }
    Ok(())
}

/// Hinter der letzten Importzeile der obersten Ebene, sonst ganz oben; (Stelle, Zeilenumbruch davor nötig).
fn einfuegestelle(quelle: &Source, wurzel: &LinkedNode) -> (usize, bool) {
    let Some(ende) = oberste(wurzel)
        .filter(|k| k.kind() == SyntaxKind::ModuleImport)
        .map(|k| k.range().end)
        .last()
    else {
        return (0, false);
    };
    match quelle.text()[ende..].find('\n') {
        Some(n) => (ende + n + 1, false),
        None => (quelle.text().len(), true),
    }
}

fn regel_patch(quelle: &Source, name: &str, neu: &[(&str, Option<String>)]) -> Patch {
    let wurzel = LinkedNode::new(quelle.root());
    let Some(k) = regel(&wurzel, name) else {
        let args: Vec<String> = neu
            .iter()
            .filter_map(|(n, w)| w.as_ref().map(|w| format!("{n}: {w}")))
            .collect();
        if args.is_empty() {
            return Patch {
                bereich: 0..0,
                text: String::new(),
            };
        }
        let (stelle, umbruch) = einfuegestelle(quelle, &wurzel);
        return Patch {
            bereich: stelle..stelle,
            text: format!(
                "{}#set {name}({})\n",
                if umbruch { "\n" } else { "" },
                args.join(", ")
            ),
        };
    };
    let mut args: Vec<String> = Vec::new();
    let mut erledigt: Vec<&str> = Vec::new();
    if let Some(liste) = k.children().find(|c| c.kind() == SyntaxKind::Args) {
        for a in liste.children() {
            if matches!(
                a.kind(),
                SyntaxKind::LeftParen | SyntaxKind::RightParen | SyntaxKind::Comma
            ) || a.kind().is_trivia()
            {
                continue;
            }
            let eigener = a
                .cast::<ast::Named>()
                .map(|n| n.name().as_str().to_string())
                .and_then(|n| neu.iter().find(|(m, _)| *m == n));
            match eigener {
                Some((n, w)) => {
                    erledigt.push(n);
                    if let Some(w) = w {
                        args.push(format!("{n}: {w}"));
                    }
                }
                None => args.push(quelle.text()[a.range()].to_string()),
            }
        }
    }
    for (n, w) in neu {
        if !erledigt.contains(n)
            && let Some(w) = w
        {
            args.push(format!("{n}: {w}"));
        }
    }
    if args.is_empty() {
        // Nichts mehr gesetzt: die ganze Zeile weg.
        let von = k.range().start.saturating_sub(1);
        let mut bis = k.range().end;
        if quelle.text()[bis..].starts_with('\n') {
            bis += 1;
        }
        return Patch {
            bereich: von..bis,
            text: String::new(),
        };
    }
    Patch {
        bereich: k.range(),
        text: format!("set {name}({})", args.join(", ")),
    }
}

/// Patch für eine Einstellung: ändert die vorhandene Regel oder legt sie oben neu an.
pub fn seite_patch(quelle: &Source, feld: &SeitenFeld) -> Result<Patch, PatchFehler> {
    if gesperrt(&LinkedNode::new(quelle.root())) {
        return Err(PatchFehler::Gesperrt(tr!(
            "Diese Vorlage legt Seite und Schrift selbst fest."
        )));
    }
    let patch = match feld {
        SeitenFeld::Format(f) => {
            nicht_leer(f, &tr!("Das Format"))?;
            regel_patch(quelle, "page", &[("paper", Some(zeichenkette(f)))])
        }
        SeitenFeld::Quer(q) => regel_patch(
            quelle,
            "page",
            &[("flipped", q.then(|| "true".to_string()))],
        ),
        SeitenFeld::Schrift(s) => {
            nicht_leer(s, &tr!("Die Schrift"))?;
            regel_patch(quelle, "text", &[("font", Some(zeichenkette(s)))])
        }
        SeitenFeld::Groesse(g) => {
            if !GROESSE.contains(g) {
                return Err(PatchFehler::UngueltigerWert(tr!(
                    "Die Größe muss zwischen {von} und {bis} pt liegen.",
                    von = GROESSE.start(),
                    bis = GROESSE.end()
                )));
            }
            regel_patch(quelle, "text", &[("size", Some(format!("{g}pt")))])
        }
        SeitenFeld::Sprache { lang, region } => {
            nicht_leer(lang, &tr!("Die Sprache"))?;
            regel_patch(
                quelle,
                "text",
                &[
                    ("lang", Some(zeichenkette(lang))),
                    ("region", region.as_deref().map(zeichenkette)),
                ],
            )
        }
        SeitenFeld::Raender {
            oben,
            unten,
            innen,
            aussen,
            doppelseitig,
        } => {
            for w in [oben, unten, innen, aussen] {
                zahl_im_bereich(*w, 0.0..=15.0, &tr!("Ein Rand (cm)"))?;
            }
            let (i, a) = if *doppelseitig {
                ("inside", "outside")
            } else {
                ("left", "right")
            };
            regel_patch(
                quelle,
                "page",
                &[(
                    "margin",
                    Some(format!(
                        "(top: {oben}cm, bottom: {unten}cm, {i}: {innen}cm, {a}: {aussen}cm)"
                    )),
                )],
            )
        }
        SeitenFeld::Spalten(n) => {
            zahl_im_bereich(f64::from(*n), 1.0..=4.0, &tr!("Die Spaltenzahl"))?;
            regel_patch(
                quelle,
                "page",
                &[("columns", (*n > 1).then(|| n.to_string()))],
            )
        }
        SeitenFeld::Blocksatz(b) => {
            regel_patch(quelle, "par", &[("justify", b.then(|| "true".to_string()))])
        }
        SeitenFeld::Zeilenabstand(w) => {
            zahl_im_bereich(*w, 0.1..=3.0, &tr!("Der Zeilenabstand"))?;
            regel_patch(quelle, "par", &[("leading", Some(format!("{w}em")))])
        }
        SeitenFeld::Nummerierung(b) => regel_patch(
            quelle,
            "heading",
            &[("numbering", b.then(|| "\"1.1\"".to_string()))],
        ),
        SeitenFeld::Kopf(z) | SeitenFeld::Fuss(z) => {
            if !hat_zeilen(&LinkedNode::new(quelle.root())) {
                return Err(PatchFehler::Gesperrt(tr!(
                    "Kopf- und Fußzeilen gibt es erst in neu angelegten Dokumenten."
                )));
            }
            let (arg, funktion) = if matches!(feld, SeitenFeld::Kopf(_)) {
                ("header", "kopfzeile")
            } else {
                ("footer", "fusszeile")
            };
            regel_patch(quelle, "page", &[(arg, zeile_text(funktion, z))])
        }
    };
    let neu = Source::detached(anwenden(quelle.text(), &patch));
    if fehler(neu.root()) > fehler(quelle.root()) {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "Dieser Wert passt hier nicht."
        )));
    }
    Ok(patch)
}
