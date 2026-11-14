//! Schnipsel mit Platzhaltern: `${auswahl}`, `${cursor}`, `${feld:Name}`, `${feld:Name:Vorgabe}`.
//! Jedes andere `${` ist gewöhnlicher Text; so kollidiert nichts mit Typst (`#{ … }`, `$…$`).

use std::ops::Range;
use tftn_sprachen::tr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Teil {
    Text(String),
    Feld {
        name: String,
        vorgabe: Option<String>,
    },
    Auswahl,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schnipsel {
    pub teile: Vec<Teil>,
}

/// Fehler beim Lesen; `stelle` ist das Byte des `$` im Schnipsel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchnipselFehler {
    pub stelle: usize,
    pub grund: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feld {
    pub name: String,
    pub bereich: Range<usize>,
}

/// Eingesetzter Text mit Feldern in Tab-Reihenfolge; Stellen in Bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eingesetzt {
    pub text: String,
    pub felder: Vec<Feld>,
    pub cursor: Option<usize>,
}

/// Wie [`Eingesetzt`], Stellen aber in UTF-16-Einheiten (für CodeMirror).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EingesetztUtf16 {
    pub text: String,
    pub felder: Vec<Feld>,
    pub cursor: Option<usize>,
}

/// Name des Feldes, das `${auswahl}` ohne markierten Text wird.
pub const AUSWAHL_FELD: &str = "Inhalt";

impl Schnipsel {
    pub fn lesen(roh: &str) -> Result<Schnipsel, SchnipselFehler> {
        let mut teile = Vec::new();
        let mut text = String::new();
        let mut rest = roh;
        while let Some(i) = rest.find("${") {
            let stelle = roh.len() - rest.len() + i;
            let nach = &rest[i + 2..];
            let (teil, laenge) = if let Some(n) = nach.strip_prefix("auswahl}") {
                (Teil::Auswahl, nach.len() - n.len())
            } else if let Some(n) = nach.strip_prefix("cursor}") {
                (Teil::Cursor, nach.len() - n.len())
            } else if let Some(n) = nach.strip_prefix("feld:") {
                // Name und Vorgabe enden am ersten `}`; `{` oder ein Zeilenumbruch davor heißt,
                // das Feld wurde nicht geschlossen (sonst würde ein späteres `}` es verschlucken).
                let ende = n
                    .find(['}', '{', '\n'])
                    .filter(|&e| n.as_bytes()[e] == b'}')
                    .ok_or_else(|| SchnipselFehler {
                        stelle,
                        grund: tr!("`${feld:` ohne schließendes `}`"),
                    })?;
                let (name, vorgabe) = match n[..ende].split_once(':') {
                    Some((name, vorgabe)) => (name, Some(vorgabe.to_string())),
                    None => (&n[..ende], None),
                };
                if name.is_empty() {
                    return Err(SchnipselFehler {
                        stelle,
                        grund: tr!("Feld ohne Namen"),
                    });
                }
                let feld = Teil::Feld {
                    name: name.to_string(),
                    vorgabe,
                };
                (feld, "feld:".len() + ende + 1)
            } else {
                // Kein Platzhalter: `${` bleibt Text.
                text.push_str(&rest[..i + 2]);
                rest = nach;
                continue;
            };
            text.push_str(&rest[..i]);
            if !text.is_empty() {
                teile.push(Teil::Text(std::mem::take(&mut text)));
            }
            teile.push(teil);
            rest = &nach[laenge..];
        }
        text.push_str(rest);
        if !text.is_empty() {
            teile.push(Teil::Text(text));
        }
        Ok(Schnipsel { teile })
    }

    /// Liest ein Schnipsel aus `typst_ide` (`apply`): `${}` ist ein leeres Feld, `${text}` und
    /// `${<Ziffern>:text}` ein Feld mit `text`. Ein Feld endet am ersten `}`; ein `${` ohne `}` und
    /// `#{` bleiben Text.
    pub fn aus_ide(apply: &str) -> Schnipsel {
        let mut teile = Vec::new();
        let mut text = String::new();
        let mut rest = apply;
        while let Some(i) = rest.find("${") {
            let Some(ende) = rest[i + 2..].find('}') else {
                break;
            };
            let inhalt = &rest[i + 2..i + 2 + ende];
            let ziffern = inhalt.bytes().take_while(u8::is_ascii_digit).count();
            let name = match &inhalt[ziffern..] {
                _ if ziffern == 0 => inhalt,
                "" => "",
                n => n.strip_prefix(':').unwrap_or(inhalt),
            };
            text.push_str(&rest[..i]);
            if !text.is_empty() {
                teile.push(Teil::Text(std::mem::take(&mut text)));
            }
            teile.push(Teil::Feld {
                name: name.to_string(),
                vorgabe: None,
            });
            rest = &rest[i + 2 + ende + 1..];
        }
        text.push_str(rest);
        if !text.is_empty() {
            teile.push(Teil::Text(text));
        }
        Schnipsel { teile }
    }

    /// Setzt `auswahl` und die Vorgaben ein; Bereiche in Bytes von `text`.
    ///
    /// Steht vor `${auswahl}` in seiner Zeile nur eine Listenmarke, wird jede Zeile der Auswahl ein
    /// Punkt (vorhandene Marken werden ersetzt). Enthält die Auswahl ```` ``` ````, werden die Zäune
    /// des Schnipsels länger als die längste Backtick-Folge der Auswahl. Ohne `${auswahl}` füllt die
    /// Auswahl das erste Feld bzw. bleibt vor dem Schnipsel stehen.
    pub fn einsetzen(&self, auswahl: &str) -> Eingesetzt {
        let mut aus = Eingesetzt {
            text: String::new(),
            felder: Vec::new(),
            cursor: None,
        };
        let zaun = backtick_folgen(auswahl)
            .map(|(_, n)| n)
            .max()
            .filter(|&n| n >= 3)
            .map(|n| "`".repeat(n + 1));
        // Ohne `${auswahl}` geht die Markierung nicht verloren: Sie füllt das erste Feld, und gibt es
        // keines, bleibt sie stehen und das Schnipsel folgt dahinter.
        let mut fuer_feld = None;
        if !auswahl.is_empty() && !self.teile.contains(&Teil::Auswahl) {
            if self.teile.iter().any(|t| matches!(t, Teil::Feld { .. })) {
                fuer_feld = Some(auswahl);
            } else {
                aus.text.push_str(auswahl);
            }
        }
        for teil in &self.teile {
            match teil {
                Teil::Text(t) => match &zaun {
                    Some(z) => aus.text.push_str(&zaeune_ersetzen(t, z)),
                    None => aus.text.push_str(t),
                },
                Teil::Feld { name, vorgabe } => {
                    let von = aus.text.len();
                    let wert = fuer_feld.take().or(vorgabe.as_deref()).unwrap_or(name);
                    aus.text.push_str(wert);
                    aus.felder.push(Feld {
                        name: name.clone(),
                        bereich: von..aus.text.len(),
                    });
                }
                Teil::Auswahl if auswahl.is_empty() => aus.felder.push(Feld {
                    name: AUSWAHL_FELD.into(),
                    bereich: aus.text.len()..aus.text.len(),
                }),
                Teil::Auswahl => {
                    let zeile = &aus.text[aus.text.rfind('\n').map_or(0, |i| i + 1)..];
                    let eingesetzt = match ist_listenmarke(zeile) {
                        true => als_liste(auswahl, zeile),
                        false => auswahl.to_string(),
                    };
                    aus.text.push_str(&eingesetzt);
                }
                Teil::Cursor => aus.cursor = Some(aus.text.len()),
            }
        }
        aus
    }
}

/// Folgen von Backticks in `text`: (Start-Byte, Länge).
fn backtick_folgen(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let b = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < b.len() && b[i] != b'`' {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i] == b'`' {
            i += 1;
        }
        (i > start).then_some((start, i - start))
    })
}

/// Ersetzt jede Folge von mindestens drei Backticks durch `zaun`.
fn zaeune_ersetzen(text: &str, zaun: &str) -> String {
    let mut aus = String::new();
    let mut bis = 0;
    for (start, n) in backtick_folgen(text).filter(|&(_, n)| n >= 3) {
        aus.push_str(&text[bis..start]);
        aus.push_str(zaun);
        bis = start + n;
    }
    aus.push_str(&text[bis..]);
    aus
}

/// Länge einer Listenmarke (`- `, `+ `, `<Zahl>. `) am Anfang von `text`.
fn marke_laenge(text: &str) -> Option<usize> {
    let ziffern = text.bytes().take_while(u8::is_ascii_digit).count();
    let nach = if ziffern > 0 {
        text[ziffern..].strip_prefix('.')?
    } else {
        text.strip_prefix(['-', '+'])?
    };
    nach.starts_with(' ').then(|| text.len() - nach.len() + 1)
}

/// `zeile` besteht nur aus Einrückung und einer Listenmarke.
fn ist_listenmarke(zeile: &str) -> bool {
    let ohne = zeile.trim_start_matches([' ', '\t']);
    marke_laenge(ohne) == Some(ohne.len())
}

/// Jede nicht leere Zeile wird ein Punkt mit dem Präfix `praefix` (die erste hat ihn schon im
/// Schnipsel); gemeinsame Einrückung fällt weg, vorhandene Marken werden ersetzt.
fn als_liste(auswahl: &str, praefix: &str) -> String {
    let einrueckung = |z: &str| z.len() - z.trim_start_matches([' ', '\t']).len();
    let gemeinsam = auswahl
        .lines()
        .filter(|z| !z.trim().is_empty())
        .map(einrueckung)
        .min()
        .unwrap_or(0);
    auswahl
        .split('\n')
        .enumerate()
        .map(|(i, z)| {
            if z.trim().is_empty() {
                return String::new();
            }
            let z = &z[gemeinsam..];
            let (ws, rest) = z.split_at(einrueckung(z));
            let rest = &rest[marke_laenge(rest).unwrap_or(0)..];
            if i == 0 {
                rest.to_string()
            } else {
                format!("{ws}{praefix}{rest}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl Eingesetzt {
    /// Dieselben Bereiche in UTF-16-Einheiten (für CodeMirror).
    pub fn utf16(&self) -> EingesetztUtf16 {
        let u = |byte: usize| self.text[..byte].encode_utf16().count();
        EingesetztUtf16 {
            text: self.text.clone(),
            felder: self
                .felder
                .iter()
                .map(|f| Feld {
                    name: f.name.clone(),
                    bereich: u(f.bereich.start)..u(f.bereich.end),
                })
                .collect(),
            cursor: self.cursor.map(u),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn felder_vorgaben_und_auswahl() {
        let s = Schnipsel::lesen(
            "#figure(image(\"${feld:Datei:bilder/bild.png}\"), caption: [${feld:Bildunterschrift}])",
        )
        .unwrap();
        let e = s.einsetzen("");
        assert_eq!(
            e.text,
            "#figure(image(\"bilder/bild.png\"), caption: [Bildunterschrift])"
        );
        assert_eq!(
            e.felder
                .iter()
                .map(|f| &e.text[f.bereich.clone()])
                .collect::<Vec<_>>(),
            ["bilder/bild.png", "Bildunterschrift"]
        );
    }

    #[test]
    fn auswahl_leer_wird_feld_inhalt() {
        let e = Schnipsel::lesen("#footnote[${auswahl}]")
            .unwrap()
            .einsetzen("");
        assert_eq!(e.text, "#footnote[]");
        assert_eq!(e.felder[0].name, "Inhalt");
        assert_eq!(e.felder[0].bereich, 10..10);
        let e = Schnipsel::lesen("#footnote[${auswahl}]")
            .unwrap()
            .einsetzen("Quelle");
        assert_eq!(e.text, "#footnote[Quelle]");
        assert!(e.felder.is_empty());
    }

    #[test]
    fn typst_code_und_dollar_bleiben_text() {
        let e = Schnipsel::lesen("#{ let x = 1 } $${feld:Formel:x}$ ${unbekannt}")
            .unwrap()
            .einsetzen("");
        assert_eq!(e.text, "#{ let x = 1 } $x$ ${unbekannt}");
        assert_eq!(&e.text[e.felder[0].bereich.clone()], "x");
    }

    #[test]
    fn cursor_und_utf16() {
        let e = Schnipsel::lesen("Ä😀 ${feld:Titel}${cursor}")
            .unwrap()
            .einsetzen("");
        assert_eq!(e.felder[0].bereich, 7..12); // Bytes: Ä=2, 😀=4, Leerzeichen=1
        assert_eq!(e.cursor, Some(12));
        // Eigener Typ: Byte- und UTF-16-Stellen lassen sich nicht verwechseln.
        let u: EingesetztUtf16 = e.utf16();
        assert_eq!(u.felder[0].bereich, 4..9); // UTF-16: Ä=1, 😀=2, Leerzeichen=1
        assert_eq!(u.cursor, Some(9));
    }

    #[test]
    fn offener_platzhalter_ist_fehler() {
        let f = Schnipsel::lesen("= ${feld:Titel").unwrap_err();
        assert_eq!(f.stelle, 2);
    }

    #[test]
    fn offener_platzhalter_vor_spaeterem_schluss_ist_fehler() {
        for (roh, stelle) in [
            ("= ${feld:Titel ${cursor}", 2),
            ("#hinweis[${feld:Titel] ${auswahl}", 9),
            ("${feld:Code:#{1+1}}", 0),
            ("${feld:A\n} und weiter", 0),
        ] {
            let f = Schnipsel::lesen(roh).unwrap_err();
            assert_eq!(f.stelle, stelle, "{roh:?}");
        }
    }

    #[test]
    fn leerer_feldname_ist_fehler_vorgabe_mit_doppelpunkt_nicht() {
        assert!(Schnipsel::lesen("${feld:}").is_err());
        assert!(Schnipsel::lesen("${feld::x}").is_err());
        let e = Schnipsel::lesen("${feld:Adresse:https://}")
            .unwrap()
            .einsetzen("");
        assert_eq!(e.text, "https://");
        assert_eq!(e.felder[0].name, "Adresse");
        assert_eq!(e.felder[0].bereich, 0..8);
    }

    fn ein(roh: &str, auswahl: &str) -> String {
        Schnipsel::lesen(roh).unwrap().einsetzen(auswahl).text
    }

    #[test]
    fn liste_je_zeile_ein_punkt() {
        assert_eq!(ein("- ${auswahl}", "a\nb\n\nc"), "- a\n- b\n\n- c");
    }

    #[test]
    fn aufzaehlung_ersetzt_vorhandene_marken() {
        assert_eq!(ein("+ ${auswahl}", "- a\n- b"), "+ a\n+ b");
        assert_eq!(ein("+ ${auswahl}", "1. a\n2. b"), "+ a\n+ b");
    }

    #[test]
    fn gemeinsame_einrueckung_faellt_relative_bleibt() {
        assert_eq!(ein("- ${auswahl}", "  a\n    b"), "- a\n  - b");
    }

    #[test]
    fn fussnote_mehrzeilig_unveraendert() {
        assert_eq!(ein("#footnote[${auswahl}]", "a\nb"), "#footnote[a\nb]");
    }

    #[test]
    fn codeblock_mit_zaun_in_auswahl() {
        let e = Schnipsel::lesen("```${feld:Sprache:}\n${auswahl}\n```")
            .unwrap()
            .einsetzen("x\n```\ny");
        assert_eq!(e.text, "````\nx\n```\ny\n````");
        assert_eq!(e.felder[0].bereich, 4..4);
        assert!(!typst_syntax::parse(&e.text).diagnosis().either());
    }

    #[test]
    fn ide_schnipsel() {
        let e = Schnipsel::aus_ide("figure(\n  ${}\n)").einsetzen("");
        assert_eq!(
            (e.text.as_str(), e.felder[0].bereich.clone()),
            ("figure(\n  \n)", 10..10)
        );
        let e = Schnipsel::aus_ide("${x}_${2:2}").einsetzen("");
        assert_eq!(e.text, "x_2");
        assert_eq!(
            e.felder
                .iter()
                .map(|f| f.bereich.clone())
                .collect::<Vec<_>>(),
            [0..1, 2..3]
        );
        let e = Schnipsel::aus_ide("(${a: 1, b: 2})").einsetzen("");
        assert_eq!(
            (e.text.as_str(), e.felder[0].bereich.clone()),
            ("(a: 1, b: 2)", 1..11)
        );
        assert_eq!(Schnipsel::aus_ide("#{ x }").einsetzen("").text, "#{ x }");
    }

    #[test]
    fn markierung_ohne_auswahl_platzhalter_bleibt_erhalten() {
        // Erstes Feld bekommt die Markierung.
        let e = Schnipsel::lesen("= ${feld:Titel}")
            .unwrap()
            .einsetzen("Einleitung");
        assert_eq!(e.text, "= Einleitung");
        assert_eq!(e.felder[0].bereich, 2..12);
        // Ohne Feld bleibt sie stehen, das Schnipsel folgt.
        let e = Schnipsel::lesen("#pagebreak()${cursor}")
            .unwrap()
            .einsetzen("Text");
        assert_eq!(e.text, "Text#pagebreak()");
        assert_eq!(e.cursor, Some(16));
    }
}
