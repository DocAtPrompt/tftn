//! Widget-Arten (KONZEPT 5.4) und ihre Werte: Typst-Literale lesen und schreiben (PLAN-M2 AP4).
//!
//! `lesen` erkennt nur reine Literale; alles andere (Variablen, Rechnungen) ist „berechnet“.
//! `schreiben` prüft den Wert gegen die Zusatzschlüssel der Art und zum Schluss, dass das Literal
//! wieder genau diesen Wert ergibt (P5 zur Laufzeit).

use serde::{Deserialize, Serialize};
use tftn_sprachen::tr;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::{SyntaxKind, SyntaxNode, parse_code};

use crate::PatchFehler;

/// Größter Betrag, den Zahl-, Längen- und Prozent-Widgets schreiben (ohne Exponentenschreibweise).
pub const ZAHL_GRENZE: f64 = 1e15;

const LAENGEN: [&str; 5] = ["pt", "mm", "cm", "in", "em"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Farbraum {
    Rgb,
    Cmyk,
}

/// Widget-Art eines Parameters mit ihren Zusatzschlüsseln. Für die Oberfläche adjazent getaggt:
/// `{"art": "zahl", "mit": {"min": …}}`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "art", content = "mit", rename_all = "kebab-case")]
pub enum WidgetArt {
    Text {
        mehrzeilig: bool,
    },
    Zahl {
        min: Option<f64>,
        max: Option<f64>,
        ganzzahl: bool,
    },
    Laenge {
        einheiten: Vec<String>,
    },
    Prozent {
        min: Option<f64>,
        max: Option<f64>,
    },
    Schalter,
    Wahl {
        werte: Vec<String>,
    },
    Farbe {
        raum: Farbraum,
    },
    Datum,
    Label,
    Datei {
        endungen: Vec<String>,
    },
    /// Paar aus Zahlen oder Längen, z. B. `(1pt, 2pt)`
    Punkt,
    /// Array von Literalen der Art `element`
    Liste {
        element: Box<WidgetArt>,
    },
    /// Inhaltsblock `[…]`; `rumpf`: der Block hinter der Argumentliste
    Inhalt {
        rumpf: bool,
    },
    /// Verweis auf das Tabellenraster (Patch-Klasse Tabellenstruktur, AP5)
    Tabelle,
    Unbekannt(String),
}

impl WidgetArt {
    pub fn name(&self) -> &str {
        match self {
            Self::Text { .. } => "text",
            Self::Zahl { .. } => "zahl",
            Self::Laenge { .. } => "laenge",
            Self::Prozent { .. } => "prozent",
            Self::Schalter => "schalter",
            Self::Wahl { .. } => "wahl",
            Self::Farbe { .. } => "farbe",
            Self::Datum => "datum",
            Self::Label => "label",
            Self::Datei { .. } => "datei",
            Self::Punkt => "punkt",
            Self::Liste { .. } => "liste",
            Self::Inhalt { .. } => "inhalt",
            Self::Tabelle => "tabelle",
            Self::Unbekannt(n) => n,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Farbe {
    Rgb([u8; 3]),
    /// in ganzen Prozent 0..=100
    Cmyk([u8; 4]),
    Luma(u8),
}

impl Farbe {
    /// `rrggbb` für RGB und Grau; `None` für CMYK.
    pub fn hex(&self) -> Option<String> {
        match self {
            Self::Rgb([r, g, b]) => Some(format!("{r:02x}{g:02x}{b:02x}")),
            Self::Luma(l) => Some(format!("{l:02x}{l:02x}{l:02x}")),
            Self::Cmyk(_) => None,
        }
    }
}

/// Ein Wert, wie ihn ein Widget zeigt und setzt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "art", content = "wert", rename_all = "kebab-case")]
pub enum Wert {
    /// `text`, `wahl`, `datei`
    Text(String),
    Zahl(f64),
    Laenge {
        betrag: f64,
        einheit: String,
    },
    Prozent(f64),
    Schalter(bool),
    Farbe(Farbe),
    Datum {
        jahr: i32,
        monat: u8,
        tag: u8,
    },
    /// ohne spitze Klammern
    Label(String),
    Punkt(Vec<Wert>),
    Liste(Vec<Wert>),
    /// Markup innerhalb der Klammern
    Inhalt(String),
}

/// Liest den Wert eines reinen Literals; `None`, wenn `knoten` berechnet ist oder nicht passt.
pub fn lesen(art: &WidgetArt, knoten: &SyntaxNode) -> Option<Wert> {
    let expr = knoten.cast::<ast::Expr>()?;
    match art {
        WidgetArt::Text { .. } | WidgetArt::Wahl { .. } | WidgetArt::Datei { .. } => match expr {
            ast::Expr::Str(s) => Some(Wert::Text(s.get().to_string())),
            _ => None,
        },
        WidgetArt::Zahl { .. } => zahl(expr).map(Wert::Zahl),
        WidgetArt::Laenge { .. } => laenge(expr),
        WidgetArt::Prozent { .. } => match numerisch(expr)? {
            (p, ast::Unit::Percent) => Some(Wert::Prozent(p)),
            _ => None,
        },
        WidgetArt::Schalter => match expr {
            ast::Expr::Bool(b) => Some(Wert::Schalter(b.get())),
            _ => None,
        },
        WidgetArt::Farbe { .. } => farbe(knoten).map(Wert::Farbe),
        WidgetArt::Datum => datum(expr),
        WidgetArt::Label => match expr {
            ast::Expr::Label(l) => Some(Wert::Label(l.get().to_string())),
            _ => None,
        },
        WidgetArt::Punkt => {
            let teile = array(expr)?;
            let werte: Option<Vec<Wert>> = teile
                .iter()
                .map(|e| laenge(*e).or_else(|| zahl(*e).map(Wert::Zahl)))
                .collect();
            werte.filter(|w| w.len() == 2).map(Wert::Punkt)
        }
        WidgetArt::Liste { element } => {
            let teile = array(expr)?;
            teile
                .iter()
                .map(|e| lesen(element, e.to_untyped()))
                .collect::<Option<Vec<_>>>()
                .map(Wert::Liste)
        }
        WidgetArt::Inhalt { .. } => match expr {
            ast::Expr::ContentBlock(_) => {
                let markup = knoten.children().find(|k| k.kind() == SyntaxKind::Markup)?;
                Some(Wert::Inhalt(markup.full_text().to_string()))
            }
            _ => None,
        },
        WidgetArt::Tabelle | WidgetArt::Unbekannt(_) => None,
    }
}

/// Das Literal für `wert`; geprüft gegen die Zusatzschlüssel der Art und durch Zurücklesen.
pub fn schreiben(art: &WidgetArt, wert: &Wert) -> Result<String, PatchFehler> {
    let ungueltig = |grund: String| Err(PatchFehler::UngueltigerWert(grund));
    let literal = match (art, wert) {
        (WidgetArt::Tabelle, _) => {
            return Err(PatchFehler::Gesperrt(tr!("Tabellen ändert das Raster")));
        }
        (WidgetArt::Unbekannt(n), _) => {
            return Err(PatchFehler::Gesperrt(tr!("unbekannte Widget-Art {n}", n)));
        }
        (WidgetArt::Text { mehrzeilig }, Wert::Text(t)) => {
            if !mehrzeilig && t.contains(['\n', '\r']) {
                return ungueltig(tr!("Zeilenumbruch in einzeiligem Text"));
            }
            text_literal(t)
        }
        (WidgetArt::Wahl { werte }, Wert::Text(t)) => {
            if !werte.is_empty() && !werte.contains(t) {
                return ungueltig(tr!(
                    "{wert} ist keiner der Werte {werte}",
                    wert = format!("{t:?}"),
                    werte = format!("{werte:?}")
                ));
            }
            text_literal(t)
        }
        (WidgetArt::Datei { endungen }, Wert::Text(t)) => {
            let endung = std::path::Path::new(t)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase());
            if !endungen.is_empty()
                && !endung.is_some_and(|e| endungen.iter().any(|x| x.to_lowercase() == e))
            {
                return ungueltig(tr!(
                    "{datei}: Endung nicht in {endungen}",
                    datei = t,
                    endungen = format!("{endungen:?}")
                ));
            }
            text_literal(t)
        }
        (WidgetArt::Zahl { min, max, ganzzahl }, Wert::Zahl(z)) => {
            betrag_pruefen(*z)?;
            grenzen_pruefen(*z, *min, *max)?;
            if *ganzzahl {
                if z.fract() != 0.0 {
                    return ungueltig(tr!("{z} ist keine ganze Zahl", z));
                }
                format!("{}", *z as i64)
            } else {
                zahl_text(*z)
            }
        }
        (WidgetArt::Laenge { einheiten }, Wert::Laenge { betrag, einheit }) => {
            betrag_pruefen(*betrag)?;
            let erlaubt = if einheiten.is_empty() {
                LAENGEN.contains(&einheit.as_str())
            } else {
                einheiten.contains(einheit) && LAENGEN.contains(&einheit.as_str())
            };
            if !erlaubt {
                return ungueltig(tr!("Einheit {einheit} nicht erlaubt", einheit));
            }
            format!("{}{einheit}", zahl_text(*betrag))
        }
        (WidgetArt::Prozent { min, max }, Wert::Prozent(p)) => {
            betrag_pruefen(*p)?;
            grenzen_pruefen(*p, *min, *max)?;
            format!("{}%", zahl_text(*p))
        }
        (WidgetArt::Schalter, Wert::Schalter(b)) => b.to_string(),
        (WidgetArt::Farbe { raum }, Wert::Farbe(f)) => match (raum, f) {
            (Farbraum::Rgb, Farbe::Rgb([r, g, b])) => format!("rgb(\"#{r:02x}{g:02x}{b:02x}\")"),
            (Farbraum::Rgb, Farbe::Luma(l)) => format!("luma({l})"),
            (Farbraum::Cmyk, Farbe::Cmyk(c)) => {
                if c.iter().any(|&x| x > 100) {
                    return ungueltig(tr!("CMYK über 100 %: {wert}", wert = format!("{c:?}")));
                }
                format!("cmyk({}%, {}%, {}%, {}%)", c[0], c[1], c[2], c[3])
            }
            _ => {
                return ungueltig(tr!(
                    "{farbe} passt nicht zum Farbraum {raum}",
                    farbe = format!("{f:?}"),
                    raum = format!("{raum:?}")
                ));
            }
        },
        (WidgetArt::Datum, Wert::Datum { jahr, monat, tag }) => {
            if !gueltiger_tag(*jahr, *monat, *tag) {
                return ungueltig(tr!(
                    "{jahr}-{monat}-{tag} ist kein Kalendertag",
                    jahr,
                    monat,
                    tag
                ));
            }
            format!("datetime(year: {jahr}, month: {monat}, day: {tag})")
        }
        (WidgetArt::Label, Wert::Label(l)) => format!("<{l}>"),
        (WidgetArt::Punkt, Wert::Punkt(teile)) if teile.len() == 2 => {
            let t: Result<Vec<String>, _> = teile
                .iter()
                .map(|w| match w {
                    Wert::Laenge { .. } => schreiben(&WidgetArt::Laenge { einheiten: vec![] }, w),
                    Wert::Zahl(_) => schreiben(
                        &WidgetArt::Zahl {
                            min: None,
                            max: None,
                            ganzzahl: false,
                        },
                        w,
                    ),
                    _ => Err(PatchFehler::UngueltigerWert(tr!(
                        "{wert} im Punkt",
                        wert = format!("{w:?}")
                    ))),
                })
                .collect();
            let t = t?;
            format!("({}, {})", t[0], t[1])
        }
        (WidgetArt::Liste { element }, Wert::Liste(teile)) => {
            let t = teile
                .iter()
                .map(|w| schreiben(element, w))
                .collect::<Result<Vec<_>, _>>()?;
            match t.len() {
                0 => "()".into(),
                1 => format!("({},)", t[0]),
                _ => format!("({})", t.join(", ")),
            }
        }
        (WidgetArt::Inhalt { .. }, Wert::Inhalt(m)) => format!("[{m}]"),
        _ => {
            return ungueltig(tr!(
                "{wert} passt nicht zur Widget-Art {art}",
                wert = format!("{wert:?}"),
                art = art.name()
            ));
        }
    };
    // P5 zur Laufzeit: Das Literal parst fehlerfrei als ein Ausdruck und ergibt genau den Wert.
    let wurzel = parse_code(&literal);
    let ausdruck: Vec<&SyntaxNode> = wurzel
        .children()
        .filter(|k| !k.kind().is_trivia())
        .collect();
    let zurueck = match ausdruck.as_slice() {
        [a] if wurzel.errors_and_warnings().0.is_empty() => lesen(art, a),
        _ => None,
    };
    if zurueck.as_ref() != Some(wert) {
        return ungueltig(tr!("{literal} ergibt nicht wieder den Wert", literal));
    }
    Ok(literal)
}

/// Liest `rgb("#rgb")`, `rgb("#rrggbb")`, `rgb(r, g, b)` (ganze Zahlen) und `luma(n)` als `rrggbb`.
pub fn farbe_lesen(knoten: &SyntaxNode) -> Option<String> {
    farbe(knoten)?.hex()
}

/// Das Literal, das das Farb-Widget für `rrggbb` schreibt.
pub fn farbe_literal(hex: &str) -> Result<String, PatchFehler> {
    let rgb = hex_lesen(hex).ok_or_else(|| PatchFehler::UngueltigerWert(hex.into()))?;
    Ok(format!(
        "rgb(\"#{:02x}{:02x}{:02x}\")",
        rgb[0], rgb[1], rgb[2]
    ))
}

/// `rrggbb` (mit oder ohne `#`, groß oder klein) als Bytes.
pub fn hex_lesen(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

fn text_literal(t: &str) -> String {
    let mut aus = String::from("\"");
    for c in t.chars() {
        match c {
            '\\' => aus.push_str("\\\\"),
            '"' => aus.push_str("\\\""),
            '\n' => aus.push_str("\\n"),
            '\r' => aus.push_str("\\r"),
            '\t' => aus.push_str("\\t"),
            c if c.is_control() => aus.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => aus.push(c),
        }
    }
    aus.push('"');
    aus
}

/// Kürzeste exakte Darstellung ohne Exponent. Ganze Werte ohne `.0`, also als Int: Typst nimmt
/// Int auch für Float-Parameter, aber nicht umgekehrt (`columns: 3.0` wäre ein Fehler).
fn zahl_text(x: f64) -> String {
    format!("{x}")
}

fn betrag_pruefen(x: f64) -> Result<(), PatchFehler> {
    if !x.is_finite() || x.abs() > ZAHL_GRENZE {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "{x}: außerhalb von ±{grenze}",
            x,
            grenze = ZAHL_GRENZE
        )));
    }
    Ok(())
}

fn grenzen_pruefen(x: f64, min: Option<f64>, max: Option<f64>) -> Result<(), PatchFehler> {
    if min.is_some_and(|m| x < m) || max.is_some_and(|m| x > m) {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "{x} außerhalb von {min}..{max}",
            x,
            min = format!("{min:?}"),
            max = format!("{max:?}")
        )));
    }
    Ok(())
}

fn gueltiger_tag(jahr: i32, monat: u8, tag: u8) -> bool {
    let schalt = (jahr % 4 == 0 && jahr % 100 != 0) || jahr % 400 == 0;
    let tage = match monat {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if schalt => 29,
        2 => 28,
        _ => return false,
    };
    (1..=tage).contains(&tag)
}

/// Ganze Zahl oder Float, auch mit unärem Minus.
fn zahl(expr: ast::Expr) -> Option<f64> {
    match expr {
        ast::Expr::Int(i) => Some(i.get() as f64),
        ast::Expr::Float(f) => Some(f.get()),
        ast::Expr::Unary(u) if u.op() == ast::UnOp::Neg => {
            matches!(u.expr(), ast::Expr::Int(_) | ast::Expr::Float(_))
                .then(|| zahl(u.expr()).map(|z| -z))
                .flatten()
        }
        _ => None,
    }
}

/// Zahl mit Einheit, auch mit unärem Minus.
fn numerisch(expr: ast::Expr) -> Option<(f64, ast::Unit)> {
    match expr {
        ast::Expr::Numeric(n) => Some(n.get()),
        ast::Expr::Unary(u) if u.op() == ast::UnOp::Neg => match u.expr() {
            ast::Expr::Numeric(n) => {
                let (v, e) = n.get();
                Some((-v, e))
            }
            _ => None,
        },
        _ => None,
    }
}

fn laenge(expr: ast::Expr) -> Option<Wert> {
    let (betrag, einheit) = numerisch(expr)?;
    let einheit = match einheit {
        ast::Unit::Pt => "pt",
        ast::Unit::Mm => "mm",
        ast::Unit::Cm => "cm",
        ast::Unit::In => "in",
        ast::Unit::Em => "em",
        _ => return None,
    };
    Some(Wert::Laenge {
        betrag,
        einheit: einheit.into(),
    })
}

/// Elemente eines Arrays ohne Spread.
fn array(expr: ast::Expr) -> Option<Vec<ast::Expr>> {
    let ast::Expr::Array(a) = expr else {
        return None;
    };
    a.items()
        .map(|i| match i {
            ast::ArrayItem::Pos(e) => Some(e),
            ast::ArrayItem::Spread(_) => None,
        })
        .collect()
}

/// Positionale und benannte Argumente eines Aufrufs `name(…)` ohne Rumpf.
fn aufruf<'a>(expr: ast::Expr<'a>, name: &str) -> Option<Vec<ast::Arg<'a>>> {
    let ast::Expr::FuncCall(f) = expr else {
        return None;
    };
    match f.callee() {
        ast::Expr::Ident(i) if i.get() == name => Some(f.args().items().collect()),
        _ => None,
    }
}

/// Nur positionale Argumente.
fn positional<'a>(args: &[ast::Arg<'a>]) -> Option<Vec<ast::Expr<'a>>> {
    args.iter()
        .map(|a| match a {
            ast::Arg::Pos(e) => Some(*e),
            _ => None,
        })
        .collect()
}

fn farbe(knoten: &SyntaxNode) -> Option<Farbe> {
    let expr = knoten.cast::<ast::Expr>()?;
    let pos = positional;
    let byte = |e: &ast::Expr| match e {
        ast::Expr::Int(i) => u8::try_from(i.get()).ok(),
        _ => None,
    };
    if let Some(args) = aufruf(expr, "rgb") {
        let args = pos(&args)?;
        return match args.as_slice() {
            [ast::Expr::Str(s)] => {
                let t = s.get();
                let hex = t.strip_prefix('#')?;
                match hex.len() {
                    3 => {
                        let lang: String = hex.chars().flat_map(|c| [c, c]).collect();
                        hex_lesen(&lang).map(Farbe::Rgb)
                    }
                    6 => hex_lesen(hex).map(Farbe::Rgb),
                    _ => None, // mit Transparenz: nicht über das Widget
                }
            }
            [r, g, b] => Some(Farbe::Rgb([byte(r)?, byte(g)?, byte(b)?])),
            _ => None,
        };
    }
    if let Some(args) = aufruf(expr, "luma") {
        return match pos(&args)?.as_slice() {
            [l] => byte(l).map(Farbe::Luma),
            _ => None,
        };
    }
    if let Some(args) = aufruf(expr, "cmyk") {
        let werte: Option<Vec<u8>> = pos(&args)?
            .into_iter()
            .map(|e| match numerisch(e)? {
                (p, ast::Unit::Percent) if p.fract() == 0.0 && (0.0..=100.0).contains(&p) => {
                    Some(p as u8)
                }
                _ => None,
            })
            .collect();
        return match werte?.as_slice() {
            [c, m, y, k] => Some(Farbe::Cmyk([*c, *m, *y, *k])),
            _ => None,
        };
    }
    None
}

fn datum(expr: ast::Expr) -> Option<Wert> {
    let args = aufruf(expr, "datetime")?;
    let (mut jahr, mut monat, mut tag) = (None, None, None);
    for a in args {
        let ast::Arg::Named(n) = a else {
            return None;
        };
        let ast::Expr::Int(i) = n.expr() else {
            return None;
        };
        let v = i.get();
        match n.name().get().as_str() {
            "year" => jahr = Some(i32::try_from(v).ok()?),
            "month" => monat = Some(u8::try_from(v).ok()?),
            "day" => tag = Some(u8::try_from(v).ok()?),
            _ => return None,
        }
    }
    Some(Wert::Datum {
        jahr: jahr?,
        monat: monat?,
        tag: tag?,
    })
}
