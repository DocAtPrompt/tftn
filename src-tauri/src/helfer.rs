//! Kern der Helfer-Befehle (PLAN-M2 AP6): Fach „Hier“, Werte setzen, Tabellenraster, Marken.
//! Reine Funktionen auf `Source` und `Katalog` mit Byte-Positionen hinein und UTF-16-Positionen
//! hinaus, damit sie ohne App testbar sind. Die Befehle in `lib.rs` prüfen nur die Revision und
//! rechnen die Positionen der Anfrage um.

use std::ops::Range;

use serde::Serialize;
use tftn_edit::{
    Angebot, MarkupStelle, Ort, Patch, Tabelle, TabellenAktion, Wert, WidgetArt, WidgetParameter,
};
use tftn_katalog::{Katalog, WidgetEintrag};
use tftn_sprachen::tr;
use typst::syntax::Source;

#[derive(Debug, Serialize)]
pub struct HierJs {
    pub rev: u64,
    pub ebenen: Vec<EbeneJs>,
    pub markup: Vec<MarkupJs>,
    pub tabelle: Option<TabelleJs>,
}

#[derive(Debug, Serialize)]
pub struct EbeneJs {
    pub baustein: String,
    pub funktion: String,
    pub von: usize,
    pub bis: usize,
    pub felder: Vec<FeldJs>,
}

#[derive(Debug, Serialize)]
pub struct FeldJs {
    pub parameter: String,
    pub titel: String,
    pub hilfe: Option<String>,
    pub standard: Option<String>,
    pub art: WidgetArt,
    pub wert: Option<Wert>,
    pub quelltext: Option<String>,
    pub gesperrt: Option<String>,
    pub fehlt: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct MarkupJs {
    pub art: &'static str,
    pub ebene: Option<usize>,
    pub aufzaehlung: Option<bool>,
    pub block: Option<bool>,
    pub schluessel: Option<String>,
    pub zusatz: Option<String>,
}

/// Die Tabelle am Cursor. `von`/`bis` ist der Aufruf; bei einer gesperrten Tabelle die
/// Cursorstelle (der Aufruf ist dann nicht bekannt).
#[derive(Debug, Serialize)]
pub struct TabelleJs {
    pub von: usize,
    pub bis: usize,
    pub tabelle: Option<Tabelle>,
    pub gesperrt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarkeJs {
    pub pos: usize,
    pub aufruf: usize,
    pub parameter: String,
    pub baustein: String,
    pub titel: String,
    pub gesperrt: Option<String>,
    pub fehlt: bool,
}

/// `Err("veraltet")`, wenn sich der Text seit der Anfrage geändert hat.
pub fn rev_pruefen(aktuell: u64, angefragt: u64) -> Result<(), String> {
    if aktuell == angefragt {
        Ok(())
    } else {
        Err("veraltet".into())
    }
}

fn utf16(quelle: &Source, byte: usize) -> usize {
    quelle.lines().byte_to_utf16(byte).unwrap_or(0)
}

fn widget_liste(katalog: &Katalog) -> (Vec<WidgetEintrag>, Vec<WidgetParameter>) {
    let w = katalog.widgets();
    let p = w.iter().map(|w| w.parameter.clone()).collect();
    (w, p)
}

fn eintrag<'a>(widgets: &'a [WidgetEintrag], a: &Angebot) -> Option<&'a WidgetEintrag> {
    widgets
        .iter()
        .find(|w| w.parameter.funktion == a.funktion && w.parameter.parameter == a.parameter)
}

/// Sperrtext von `tftn_edit` für einen fehlenden Wert, den man nur im Text ergänzen kann (dort
/// ebenso mit `tr!` gebildet).
fn fehlt_text(g: &str) -> bool {
    g == tr!("fehlt – im Text ergänzen")
}

fn fehlt(a: &Angebot) -> bool {
    matches!(a.ort, Ort::Einfuegen { .. } | Ort::Anhaengen { .. })
        || a.gesperrt.as_deref().is_some_and(fehlt_text)
}

fn feld(widgets: &[WidgetEintrag], a: Angebot) -> FeldJs {
    let e = eintrag(widgets, &a);
    FeldJs {
        titel: e.map_or_else(|| a.parameter.clone(), |e| e.titel.clone()),
        hilfe: e.and_then(|e| e.hilfe.clone()),
        standard: e.and_then(|e| e.standard.clone()),
        fehlt: fehlt(&a),
        parameter: a.parameter,
        art: a.art,
        wert: a.wert,
        quelltext: a.quelltext,
        gesperrt: a.gesperrt,
    }
}

/// Fach „Hier“: alle umschließenden Bausteine, die Markup-Stellen und die Tabelle am Cursor.
pub fn hier_js(
    quelle: &Source,
    katalog: &Katalog,
    byte: usize,
    rev: u64,
    ist_quelle: &dyn Fn(&str) -> bool,
) -> HierJs {
    let (widgets, liste) = widget_liste(katalog);
    let ebenen = tftn_edit::ebenen(quelle, byte, &liste)
        .into_iter()
        .map(|e| EbeneJs {
            baustein: e
                .angebote
                .first()
                .and_then(|a| eintrag(&widgets, a))
                .map_or_else(|| e.funktion.clone(), |w| w.baustein.clone()),
            funktion: e.funktion,
            von: utf16(quelle, e.aufruf.start),
            bis: utf16(quelle, e.aufruf.end),
            felder: e.angebote.into_iter().map(|a| feld(&widgets, a)).collect(),
        })
        .collect();
    let markup = tftn_edit::markup_stellen(quelle, byte)
        .into_iter()
        .filter_map(|s| match s {
            MarkupStelle::Ueberschrift { ebene, .. } => Some(MarkupJs {
                ebene: Some(ebene),
                ..markup_js("ueberschrift")
            }),
            MarkupStelle::Liste { aufzaehlung, .. } => Some(MarkupJs {
                aufzaehlung: Some(aufzaehlung),
                ..markup_js("liste")
            }),
            MarkupStelle::Formel { block, .. } => Some(MarkupJs {
                block: Some(block),
                ..markup_js("formel")
            }),
            // Nur echte Quellen: Ein `@abb-1` auf eine Abbildung hat keine Seitenangabe.
            MarkupStelle::Zitat {
                schluessel, zusatz, ..
            } => ist_quelle(&schluessel).then(|| MarkupJs {
                schluessel: Some(schluessel),
                zusatz,
                ..markup_js("zitat")
            }),
        })
        .collect();
    let tabelle = tftn_edit::tabelle_am(quelle, byte).map(|t| match t {
        Ok(s) => TabelleJs {
            von: utf16(quelle, s.aufruf.start),
            bis: utf16(quelle, s.aufruf.end),
            tabelle: Some(s.tabelle),
            gesperrt: None,
        },
        Err(g) => TabelleJs {
            von: utf16(quelle, byte),
            bis: utf16(quelle, byte),
            tabelle: None,
            gesperrt: Some(g),
        },
    });
    HierJs {
        rev,
        ebenen,
        markup,
        tabelle,
    }
}

/// Patch für `wert` am Parameter `parameter` des Aufrufs, der bei `aufruf` (Byte) beginnt.
pub fn wert_patch_js(
    quelle: &Source,
    katalog: &Katalog,
    aufruf: usize,
    parameter: &str,
    wert: &Wert,
) -> Result<Patch, String> {
    let (_, liste) = widget_liste(katalog);
    let ebene = tftn_edit::ebenen(quelle, aufruf, &liste)
        .into_iter()
        .find(|e| e.aufruf.start == aufruf)
        .ok_or_else(|| tr!("kein Baustein an dieser Stelle"))?;
    let angebot = ebene
        .angebote
        .into_iter()
        .find(|a| a.parameter == parameter)
        .ok_or_else(|| tr!("kein passender Parameter an dieser Stelle"))?;
    tftn_edit::wert_patch(quelle, &angebot, &angebot.art, wert).map_err(|e| e.to_string())
}

fn markup_js(art: &'static str) -> MarkupJs {
    MarkupJs {
        art,
        ebene: None,
        aufzaehlung: None,
        block: None,
        schluessel: None,
        zusatz: None,
    }
}

/// Patch für das Zitat bei `byte` (PLAN-M2 AP11). `schluessel: None` behält die Quelle, `seite`
/// wird ohne Quellenwechsel wie beim Einfügen gelesen („12“ → „S. 12“), leer oder `None` entfernt
/// die Angabe.
pub fn zitat_patch_js(
    quelle: &Source,
    aktuell: u64,
    rev: u64,
    byte: usize,
    schluessel: Option<&str>,
    seite: Option<&str>,
) -> Result<Patch, String> {
    rev_pruefen(aktuell, rev)?;
    let stelle = tftn_edit::markup_stellen(quelle, byte)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Zitat { .. }))
        .ok_or_else(|| tr!("An dieser Stelle steht kein Zitat."))?;
    let MarkupStelle::Zitat {
        schluessel: bisher, ..
    } = &stelle
    else {
        unreachable!();
    };
    // Beim Quellenwechsel bleibt die Angabe wörtlich, etwa „2. Aufl.“.
    let zusatz = match schluessel {
        Some(_) => seite.map(str::to_string),
        None => tftn_literatur::ergaenzung(seite),
    };
    tftn_edit::zitat_patch(
        quelle,
        &stelle,
        schluessel.unwrap_or(bisher),
        zusatz.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Patch für `aktion` an der Tabelle, deren Aufruf bei `aufruf` (Byte) beginnt.
pub fn tabelle_patch_js(
    quelle: &Source,
    aufruf: usize,
    aktion: &TabellenAktion,
) -> Result<Patch, String> {
    let stelle = match tftn_edit::tabelle_am(quelle, aufruf) {
        Some(Ok(s)) if s.aufruf.start == aufruf => s,
        Some(Err(g)) => return Err(g),
        _ => return Err(tr!("keine Tabelle an dieser Stelle")),
    };
    tftn_edit::tabelle_patch(quelle, &stelle, aktion).map_err(|e| e.to_string())
}

/// Marken B: alle Widgets der Aufrufe im Bereich (Bytes).
pub fn marken_js(quelle: &Source, katalog: &Katalog, bereich: Range<usize>) -> Vec<MarkeJs> {
    let (widgets, liste) = widget_liste(katalog);
    tftn_edit::angebote_im_bereich(quelle, bereich, &liste)
        .into_iter()
        .map(|a| {
            let e = eintrag(&widgets, &a);
            // Hinter dem Wert; fehlt er, dort, wo er eingefügt würde (hinter „(“), sonst am Ende
            // des Aufrufs (fehlender Rumpf, fehlendes positionales Argument).
            let pos = match &a.ort {
                Ort::Ersetzen { bereich } if !fehlt(&a) => bereich.end,
                Ort::Einfuegen { pos, .. } if *pos > a.aufruf.start => *pos,
                _ => a.aufruf.end,
            };
            MarkeJs {
                pos: utf16(quelle, pos),
                aufruf: utf16(quelle, a.aufruf.start),
                baustein: e.map_or_else(|| a.funktion.clone(), |e| e.baustein.clone()),
                titel: e.map_or_else(|| a.parameter.clone(), |e| e.titel.clone()),
                fehlt: fehlt(&a),
                parameter: a.parameter,
                gesperrt: a.gesperrt,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tftn_core::Projekt;

    fn katalog() -> Katalog {
        let projekt = Projekt::oeffnen(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/korpus/testartikel"),
        )
        .unwrap();
        let (k, w) = Katalog::bauen(&tftn_katalog::Quellen {
            projekt: Some(&projekt),
            beschreibungen: &[],
        });
        assert!(w.is_empty(), "{w:?}");
        k
    }

    fn q(t: &str) -> Source {
        Source::detached(t)
    }

    #[test]
    fn hier_verschachtelt() {
        let t = "#figure(image(\"a.png\"), caption: [x])";
        let h = hier_js(&q(t), &katalog(), t.find("a.png").unwrap(), 7, &|_| false);
        assert_eq!(h.rev, 7);
        let namen: Vec<&str> = h.ebenen.iter().map(|e| e.baustein.as_str()).collect();
        assert_eq!(namen, ["Bild", "Abbildung"]);
        let bild = &h.ebenen[0];
        assert_eq!((bild.von, bild.bis), (8, 22));
        let pfad = bild.felder.iter().find(|f| f.parameter == "pfad").unwrap();
        assert_eq!(pfad.wert, Some(Wert::Text("a.png".into())));
        assert!(!pfad.fehlt);
        let breite = bild.felder.iter().find(|f| f.parameter == "width").unwrap();
        assert!(breite.fehlt);
        assert_eq!(breite.titel, "Breite");
        let caption = &h.ebenen[1].felder[0];
        assert_eq!(caption.parameter, "caption");
        assert_eq!(caption.wert, Some(Wert::Inhalt("x".into())));
        assert!(h.tabelle.is_none());
    }

    #[test]
    fn hier_tabelle_und_markup() {
        let k = katalog();
        let t = "#table(columns: 2, [a], [b])";
        let h = hier_js(&q(t), &k, t.find("b]").unwrap(), 1, &|_| false);
        let tab = h.tabelle.unwrap();
        assert_eq!(tab.tabelle.unwrap().zellen.len(), 2);
        assert_eq!((tab.von, tab.bis), (1, t.len()));

        let t = "= Titel\n";
        let h = hier_js(&q(t), &k, 3, 1, &|_| false);
        assert_eq!(
            h.markup,
            [MarkupJs {
                art: "ueberschrift",
                ebene: Some(1),
                aufzaehlung: None,
                block: None,
                schluessel: None,
                zusatz: None,
            }]
        );

        let t = "#table(columns: 2, ..x)";
        let h = hier_js(&q(t), &k, 3, 1, &|_| false);
        assert!(h.tabelle.unwrap().gesperrt.unwrap().contains("`..`"));
    }

    #[test]
    fn hier_zeigt_zitat_in_fussnote() {
        let t = "#footnote[Siehe @mei2020[S. 3] dazu]";
        let h = hier_js(&q(t), &katalog(), t.find("mei").unwrap(), 1, &|k| {
            k == "mei2020"
        });
        assert_eq!(h.ebenen[0].baustein, "Fußnote");
        assert_eq!(
            h.markup,
            [MarkupJs {
                art: "zitat",
                ebene: None,
                aufzaehlung: None,
                block: None,
                schluessel: Some("mei2020".into()),
                zusatz: Some("S. 3".into()),
            }]
        );
    }

    #[test]
    fn verweis_ohne_quelle_keine_zeile() {
        let t = "Siehe @abb-1 dazu";
        let h = hier_js(&q(t), &katalog(), t.find("abb").unwrap(), 1, &|k| {
            k == "mei2020"
        });
        assert_eq!(h.markup, []);
    }

    #[test]
    fn zitat_aendern_veraltet() {
        let t = "Text @a mehr";
        let p = zitat_patch_js(&q(t), 5, 4, t.find('@').unwrap(), None, Some("3"));
        assert_eq!(p.unwrap_err(), "veraltet");
    }

    #[test]
    fn zitat_seite_regel() {
        let t = "Text @a[S. 3] mehr";
        let byte = t.find('@').unwrap();
        let neu = |schluessel, seite| {
            tftn_edit::anwenden(
                t,
                &zitat_patch_js(&q(t), 1, 1, byte, schluessel, seite).unwrap(),
            )
        };
        assert_eq!(neu(None, Some("12")), "Text @a[S. 12] mehr");
        assert_eq!(neu(None, Some("")), "Text @a mehr");
        assert_eq!(neu(None, None), "Text @a mehr");
        assert_eq!(neu(Some("b"), Some("S. 3")), "Text @b[S. 3] mehr");
        // Beim Quellenwechsel bleibt die Angabe wörtlich.
        assert_eq!(neu(Some("b"), Some("2. Aufl.")), "Text @b[2. Aufl.] mehr");
        let kein = zitat_patch_js(&q("Text ohne"), 1, 1, 2, None, Some("3"));
        assert_eq!(kein.unwrap_err(), "An dieser Stelle steht kein Zitat.");
    }

    #[test]
    fn seite_feld_aus_json() {
        use tftn_edit::SeitenFeld as F;
        let f = |j| serde_json::from_value::<F>(j).unwrap();
        assert_eq!(
            f(serde_json::json!({"art": "format", "wert": "a5"})),
            F::Format("a5".into())
        );
        assert_eq!(
            f(serde_json::json!({"art": "sprache", "wert": {"lang": "de", "region": null}})),
            F::Sprache {
                lang: "de".into(),
                region: None
            }
        );
    }

    #[test]
    fn wert_setzen_trifft_den_aufruf() {
        let t = "#hinweis(art: \"info\")[a] #hinweis(art: \"info\")[b]";
        let zweiter = t.rfind("hinweis").unwrap();
        let p = wert_patch_js(
            &q(t),
            &katalog(),
            zweiter,
            "art",
            &Wert::Text("warnung".into()),
        )
        .unwrap();
        assert!(p.bereich.start > zweiter);
        assert_eq!(
            tftn_edit::anwenden(t, &p),
            "#hinweis(art: \"info\")[a] #hinweis(art: \"warnung\")[b]"
        );
    }

    #[test]
    fn wert_setzen_veraltet() {
        assert_eq!(rev_pruefen(3, 3), Ok(()));
        assert_eq!(rev_pruefen(4, 3), Err("veraltet".into()));
    }

    #[test]
    fn marken_im_bereich() {
        let t = "#hinweis(art: \"info\")[a] und #hinweis(art: \"info\", farbe: luma(1))[b]";
        let m = marken_js(&q(t), &katalog(), 0..t.len());
        let paare: Vec<(&str, bool)> = m.iter().map(|m| (m.parameter.as_str(), m.fehlt)).collect();
        assert_eq!(
            paare,
            [
                ("art", false),
                ("farbe", true),
                ("art", false),
                ("farbe", false)
            ]
        );
        let erste_farbe = &m[1];
        assert_eq!(erste_farbe.pos, t.find('(').unwrap() + 1);
        assert_eq!(m[0].pos, t.find(")[a]").unwrap());
        assert_eq!(m[0].baustein, "Hinweiskasten");
    }
}
