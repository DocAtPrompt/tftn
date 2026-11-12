//! Patch-Klasse Tabellenstruktur (PLAN-M2 AP5): Lesen als Raster, Ändern am Modell, Schreiben und
//! Prüfen. Standard 1 000 Fälle je Eigenschaft; mehr mit `PROPTEST_CASES=20000`.

use proptest::prelude::*;
use tftn_edit::{
    Patch, PatchFehler, Tabelle, TabellenAktion, TabellenStelle, ZellInhalt, Zelle, anwenden,
    tabelle_am, tabelle_patch,
};
use typst_syntax::Source;

fn konfig() -> ProptestConfig {
    let faelle = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    ProptestConfig {
        cases: faelle,
        ..ProptestConfig::default()
    }
}

/// Die Tabelle am ersten Vorkommen von `marke` (Cursor an deren Anfang).
fn am(text: &str, marke: &str) -> Option<Result<TabellenStelle, String>> {
    let byte = text.find(marke).expect("Marke fehlt");
    tabelle_am(&Source::detached(text.to_string()), byte)
}

fn lesen(text: &str, marke: &str) -> Tabelle {
    am(text, marke)
        .expect("keine Tabelle")
        .expect("Tabelle nicht rein")
        .tabelle
}

fn grund(text: &str, marke: &str) -> String {
    match am(text, marke).expect("keine Tabelle") {
        Ok(_) => panic!("Tabelle gilt als rein: {text}"),
        Err(g) => g,
    }
}

fn markup(t: &str) -> ZellInhalt {
    ZellInhalt::Markup(t.into())
}

fn zelle(zeile: usize, spalte: usize, zeilen: usize, spalten: usize, inhalt: ZellInhalt) -> Zelle {
    Zelle {
        zeile,
        spalte,
        zeilen,
        spalten,
        inhalt,
        weitere: vec![],
    }
}

/// (zeile, spalte, zeilen, spalten) je Zelle
fn lagen(t: &Tabelle) -> Vec<(usize, usize, usize, usize)> {
    t.zellen
        .iter()
        .map(|z| (z.zeile, z.spalte, z.zeilen, z.spalten))
        .collect()
}

// ---------------------------------------------------------------- Task 1: Lesen

#[test]
fn liest_einfache_tabelle() {
    let t = lesen("#table(columns: 2, [a], [b], [c])", "b]");
    assert_eq!(t.spalten, 2);
    assert_eq!(t.zeilen, 2);
    assert_eq!(t.spuren, None);
    assert!(t.weitere.is_empty());
    assert_eq!(
        t.zellen,
        vec![
            zelle(0, 0, 1, 1, markup("a")),
            zelle(0, 1, 1, 1, markup("b")),
            zelle(1, 0, 1, 1, markup("c")),
            zelle(1, 1, 1, 1, markup("")),
        ]
    );
    assert_eq!(t.zelle_bei(1, 1), Some(3));
    assert_eq!(t.zelle_bei(2, 0), None);
}

#[test]
fn liest_spuren_und_weitere() {
    let text =
        "#figure(\n  table(columns: (1fr, auto), stroke: 0.5pt, [a], [b]),\n  caption: [T],\n)";
    let s = am(text, "a]").unwrap().unwrap();
    assert_eq!(
        &text[s.aufruf.clone()],
        "table(columns: (1fr, auto), stroke: 0.5pt, [a], [b])"
    );
    let t = s.tabelle;
    assert_eq!(t.spalten, 2);
    assert_eq!(t.zeilen, 1);
    assert_eq!(t.spuren, Some(vec!["1fr".to_string(), "auto".to_string()]));
    assert_eq!(t.weitere, vec!["stroke: 0.5pt".to_string()]);
}

#[test]
fn ohne_columns_oder_auto_eine_spalte() {
    for text in ["#table([a], [b])", "#table(columns: auto, [a], [b])"] {
        let t = lesen(text, "a]");
        assert_eq!((t.spalten, t.zeilen), (1, 2), "{text}");
    }
    let t = lesen("#table(columns: 2)", "columns");
    assert_eq!((t.spalten, t.zeilen), (2, 0));
    assert!(t.zellen.is_empty());
}

#[test]
fn platzierung_wie_typst() {
    let t = lesen(
        "#table(columns: 3, table.cell(rowspan: 2)[a], [b], [c], [d], [e])",
        "b]",
    );
    assert_eq!(
        lagen(&t),
        vec![
            (0, 0, 2, 1),
            (0, 1, 1, 1),
            (0, 2, 1, 1),
            (1, 1, 1, 1),
            (1, 2, 1, 1)
        ]
    );
    assert_eq!(t.zeilen, 2);

    let t = lesen(
        "#table(columns: 2, table.cell(colspan: 2, rowspan: 2)[a], [b])",
        "b]",
    );
    // Die Zelle füllt die ganze Breite: der Zähler springt um colspan * rowspan.
    assert_eq!(lagen(&t), vec![(0, 0, 2, 2), (2, 0, 1, 1), (2, 1, 1, 1)]);
    assert_eq!(t.zeilen, 3);

    let g = grund("#table(columns: 2, [a], table.cell(colspan: 2)[b])", "a]");
    assert!(g.contains("Fehler in den Zellen"), "{g}");
}

#[test]
fn zellarten() {
    let t = lesen(
        "#table(columns: 3, [x], \"s\", image(\"a.png\"), table.cell(fill: red, colspan: 2)[y])",
        "x]",
    );
    assert_eq!(t.zellen[0].inhalt, markup("x"));
    assert_eq!(t.zellen[1].inhalt, ZellInhalt::Ausdruck("\"s\"".into()));
    assert_eq!(
        t.zellen[2].inhalt,
        ZellInhalt::Ausdruck("image(\"a.png\")".into())
    );
    assert_eq!(
        (
            t.zellen[3].zeile,
            t.zellen[3].spalte,
            t.zellen[3].zeilen,
            t.zellen[3].spalten
        ),
        (1, 0, 1, 2)
    );
    assert_eq!(t.zellen[3].inhalt, markup("y"));
    assert_eq!(t.zellen[3].weitere, vec!["fill: red".to_string()]);

    let t = lesen("#table(columns: 2, table.cell(colspan: 2, [z]))", "z]");
    assert_eq!(t.zellen, vec![zelle(0, 0, 1, 2, markup("z"))]);

    // Inhaltsblöcke hinter der Klammer sind auch Zellen.
    let t = lesen("#table(columns: 2)[a][b]", "a]");
    assert_eq!(t.zellen.len(), 2);
    assert_eq!(t.zellen[1].inhalt, markup("b"));
}

#[test]
fn nicht_reine_tabellen_sind_gesperrt() {
    let faelle = [
        ("#table(columns: 2, ..zellen)", "`..`"),
        (
            "#table(columns: 2, table.header([a], [b]), [c], [d])",
            "table.header",
        ),
        (
            "#table(columns: 2, [a], [b], table.footer([c], [d]))",
            "table.footer",
        ),
        ("#table(columns: 2, [a], table.hline(), [b])", "table.hline"),
        ("#table(columns: 2, [a], table.vline(), [b])", "table.vline"),
        ("#table(columns: 2, table.cell(x: 1)[a])", "fester Position"),
        ("#table(columns: 2, table.cell(colspan: n)[a])", "berechnet"),
        ("#table(columns: 2, table.cell(rowspan: 0)[a])", "berechnet"),
        ("#table(columns: n, [a])", "berechnet"),
        ("#table(columns: (auto,) * 2, [a])", "berechnet"),
        ("#table(columns: 0, [a])", "berechnet"),
        (
            "#table(columns: 1, table.cell(colspan: 2)[a])",
            "Fehler in den Zellen",
        ),
        (
            "#table(columns: 2, table.cell(rowspan: 2)[a], [b], table.cell(colspan: 2)[c])",
            "Fehler in den Zellen",
        ),
        (
            "#table(columns: 2, [a], // Kopf\n [b])",
            "Kommentar in der Tabelle",
        ),
        (
            "#table(columns: 2, [a], /* k */ [b])",
            "Kommentar in der Tabelle",
        ),
        (
            "#table([a], columns: 2, [b])",
            "benanntes Argument hinter den Zellen",
        ),
        ("#table(columns: 2, [a], [b)", "Syntaxfehler"),
        // Abschlussprüfung I1: Kommentar im Wert eines Arguments, das neu geschrieben wird
        (
            "#table(columns: /* zwei */ 2, [a], [b])",
            "Kommentar in der Tabelle",
        ),
        (
            "#table(\n  columns: // zwei\n    2,\n  [a], [b],\n)",
            "Kommentar in der Tabelle",
        ),
        (
            "#table(columns: 2, table.cell(colspan: /* c */ 2)[a])",
            "Kommentar in der Tabelle",
        ),
        (
            "#table(columns: 2, table.cell(rowspan: // r\n 2)[a], [b])",
            "Kommentar in der Tabelle",
        ),
        // Abschlussprüfung I2: Variablen und Felder können Spannen oder Linien tragen
        ("#table(columns: 2, z, [b])", "berechnet"),
        ("#table(columns: 2, linien.oben, [b])", "berechnet"),
        (
            "#table(columns: 2, std.table.header([a], [b]), [c], [d])",
            "table.header",
        ),
        (
            "#table(columns: 2, std.table.cell(colspan: 2)[a])",
            "berechnet",
        ),
    ];
    for (text, erwartet) in faelle {
        let g = grund(text, "columns").to_string();
        assert!(g.contains(erwartet), "{text}: {g}");
        assert!(
            g.contains("im Text bearbeiten") || g.contains("Syntaxfehler"),
            "{text}: {g}"
        );
    }
}

#[test]
fn innerste_tabelle() {
    let text = "#table(columns: 2, [außen], [#table(columns: 1, [innen])])";
    let innen = lesen(text, "innen");
    assert_eq!((innen.spalten, innen.zellen.len()), (1, 1));
    assert_eq!(innen.zellen[0].inhalt, markup("innen"));
    let aussen = lesen(text, "außen");
    assert_eq!(aussen.spalten, 2);
    assert_eq!(
        aussen.zellen[1].inhalt,
        markup("#table(columns: 1, [innen])")
    );
}

#[test]
fn keine_tabelle() {
    assert!(am("#grid(columns: 2, [a], [b])", "a]").is_none());
    assert!(am("Nur Text hier.", "Text").is_none());
    assert!(am("#table.cell[a]", "a]").is_none());
}

// ---------------------------------------------------------------- Task 2: Ändern am Modell

fn aendern(text: &str, aktion: TabellenAktion) -> Result<Tabelle, PatchFehler> {
    lesen(text, "table").aendern(&aktion)
}

fn fehlertext(e: Result<Tabelle, PatchFehler>) -> String {
    match e {
        Err(PatchFehler::UngueltigerWert(t)) => t,
        andere => panic!("erwartet UngueltigerWert, bekommen {andere:?}"),
    }
}

fn inhalte(t: &Tabelle) -> Vec<&str> {
    t.zellen.iter().map(|z| z.inhalt.text()).collect()
}

#[test]
fn einfuegen_mitten_durch_verbundene_zelle() {
    use TabellenAktion as A;
    // a: (0,0) 2×1, b: (0,1), c: (1,1)
    let t = aendern(
        "#table(columns: 2, table.cell(rowspan: 2)[a], [b], [c])",
        A::ZeileEinfuegen { vor: 1 },
    )
    .unwrap();
    assert_eq!(t.zeilen, 3);
    assert_eq!(
        lagen(&t),
        vec![(0, 0, 3, 1), (0, 1, 1, 1), (1, 1, 1, 1), (2, 1, 1, 1)]
    );
    assert_eq!(inhalte(&t), vec!["a", "b", "", "c"]);

    // a: (0,0) 1×2, b: (1,0), c: (1,1)
    let t = aendern(
        "#table(columns: 2, table.cell(colspan: 2)[a], [b], [c])",
        A::SpalteEinfuegen { vor: 1 },
    )
    .unwrap();
    assert_eq!(t.spalten, 3);
    assert_eq!(
        lagen(&t),
        vec![(0, 0, 1, 3), (1, 0, 1, 1), (1, 1, 1, 1), (1, 2, 1, 1)]
    );
    assert_eq!(inhalte(&t), vec!["a", "b", "", "c"]);
}

#[test]
fn loeschen_verkleinert_spannen() {
    use TabellenAktion as A;
    // a: (0,0) 1×2, b: (0,2), c: (1,0), d: (1,1), e: (1,2)
    let t = aendern(
        "#table(columns: 3, table.cell(colspan: 2)[a], [b], [c], [d], [e])",
        A::SpalteLoeschen { spalte: 1 },
    )
    .unwrap();
    assert_eq!((t.spalten, t.zeilen), (2, 2));
    assert_eq!(
        lagen(&t),
        vec![(0, 0, 1, 1), (0, 1, 1, 1), (1, 0, 1, 1), (1, 1, 1, 1)]
    );
    assert_eq!(inhalte(&t), vec!["a", "b", "c", "e"]);

    // Die verbundene Zelle beginnt in der gelöschten Zeile: sie rückt nach und behält den Inhalt.
    let t = aendern(
        "#table(columns: 2, table.cell(rowspan: 2)[a], [b], [c])",
        A::ZeileLoeschen { zeile: 0 },
    )
    .unwrap();
    assert_eq!(t.zeilen, 1);
    assert_eq!(lagen(&t), vec![(0, 0, 1, 1), (0, 1, 1, 1)]);
    assert_eq!(inhalte(&t), vec!["a", "c"]);
}

#[test]
fn letzte_zeile_und_spalte_bleiben() {
    use TabellenAktion as A;
    let t = fehlertext(aendern("#table([a])", A::ZeileLoeschen { zeile: 0 }));
    assert_eq!(t, "die letzte Zeile bleibt");
    let t = fehlertext(aendern("#table([a])", A::SpalteLoeschen { spalte: 0 }));
    assert_eq!(t, "die letzte Spalte bleibt");
}

#[test]
fn index_ausserhalb() {
    use TabellenAktion as A;
    let text = "#table(columns: 2, [a], [b])";
    for a in [
        A::ZeileEinfuegen { vor: 2 },
        A::SpalteEinfuegen { vor: 3 },
        A::ZeileLoeschen { zeile: 1 },
        A::SpalteVerschieben { von: 0, nach: 2 },
        A::Verbinden {
            zeile: 0,
            spalte: 1,
            zeilen: 1,
            spalten: 2,
        },
        A::Trennen {
            zeile: 1,
            spalte: 0,
        },
        A::Zelltext {
            zeile: 0,
            spalte: 2,
            inhalt: "x".into(),
        },
    ] {
        assert_eq!(fehlertext(aendern(text, a)), "Index außerhalb der Tabelle");
    }
}

#[test]
fn verschieben() {
    use TabellenAktion as A;
    let t = aendern(
        "#table(columns: 2, [a], [b], [c], [d], [e], [f])",
        A::ZeileVerschieben { von: 0, nach: 2 },
    )
    .unwrap();
    assert_eq!(inhalte(&t), vec!["c", "d", "e", "f", "a", "b"]);

    let e = aendern(
        "#table(columns: 2, table.cell(rowspan: 2)[a], [b], [c], [d], [e])",
        A::ZeileVerschieben { von: 0, nach: 2 },
    );
    assert_eq!(fehlertext(e), "verbundene Zellen im Weg – erst trennen");

    let t = aendern(
        "#table(columns: (1fr, 2cm), [a], [b])",
        A::SpalteVerschieben { von: 0, nach: 1 },
    )
    .unwrap();
    assert_eq!(t.spuren, Some(vec!["2cm".to_string(), "1fr".to_string()]));
    assert_eq!(inhalte(&t), vec!["b", "a"]);
}

#[test]
fn verbinden_und_trennen() {
    use TabellenAktion as A;
    let t = aendern(
        "#table(columns: 2, [a], [b], [c], [])",
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 2,
            spalten: 2,
        },
    )
    .unwrap();
    assert_eq!(t.zellen, vec![zelle(0, 0, 2, 2, markup("a b c"))]);

    let t = t
        .aendern(&A::Trennen {
            zeile: 1,
            spalte: 1,
        })
        .unwrap();
    assert_eq!(
        t.zellen,
        vec![
            zelle(0, 0, 1, 1, markup("a b c")),
            zelle(0, 1, 1, 1, markup("")),
            zelle(1, 0, 1, 1, markup("")),
            zelle(1, 1, 1, 1, markup("")),
        ]
    );
    let t = fehlertext(t.aendern(&A::Trennen {
        zeile: 0,
        spalte: 0,
    }));
    assert_eq!(t, "Zelle ist nicht verbunden");

    let e = aendern(
        "#table(columns: 3, table.cell(colspan: 2)[a], [b], [c], [d], [e])",
        A::Verbinden {
            zeile: 0,
            spalte: 1,
            zeilen: 1,
            spalten: 2,
        },
    );
    assert_eq!(fehlertext(e), "Bereich schneidet verbundene Zellen");

    let e = aendern(
        "#table(columns: 2, table.cell(colspan: 2)[a])",
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
    );
    assert_eq!(fehlertext(e), "Bereich ist nur eine Zelle");

    let e = aendern(
        "#table(columns: 2, \"s\", image(\"a.png\"))",
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
    );
    assert_eq!(
        fehlertext(e),
        "mehrere Zellen mit berechnetem Inhalt – erst im Text zusammenführen"
    );

    let t = aendern(
        "#table(columns: 2, \"s\", [])",
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
    )
    .unwrap();
    assert_eq!(
        t.zellen,
        vec![zelle(0, 0, 1, 2, ZellInhalt::Ausdruck("\"s\"".into()))]
    );

    // Weitere Argumente übernimmt die Zelle oben links.
    let t = aendern(
        "#table(columns: 2, table.cell(fill: red)[a], [b])",
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
    )
    .unwrap();
    assert_eq!(t.zellen[0].weitere, vec!["fill: red".to_string()]);
    assert_eq!(t.zellen[0].inhalt, markup("a b"));
}

#[test]
fn zelltext_im_modell() {
    use TabellenAktion as A;
    let t = aendern(
        "#table(columns: 2, table.cell(colspan: 2)[a], \"s\")",
        A::Zelltext {
            zeile: 0,
            spalte: 1,
            inhalt: "neu".into(),
        },
    )
    .unwrap();
    assert_eq!(t.zellen[0], zelle(0, 0, 1, 2, markup("neu")));
    let t = t
        .aendern(&A::Zelltext {
            zeile: 1,
            spalte: 0,
            inhalt: "x".into(),
        })
        .unwrap();
    assert_eq!(t.zellen[1].inhalt, markup("x"));
}

#[test]
fn einfuegen_in_spalten_ohne_columns() {
    let t = aendern(
        "#table([a], [b])",
        TabellenAktion::SpalteEinfuegen { vor: 1 },
    )
    .unwrap();
    assert_eq!((t.spalten, t.zeilen), (2, 2));
    assert_eq!(t.spuren, None);
    assert_eq!(inhalte(&t), vec!["a", "", "b", ""]);
}

// ---- Generator: reine Tabellen als Modell (auch für Task 3)

const MARKUP: [&str; 10] = [
    "Text",
    "*fett*",
    "$x$",
    "\\]",
    "Größe",
    "😀",
    "#emph[e]",
    "a, b)",
    "#table(columns: 1, [i])",
    "",
];
const AUSDRUECKE: [&str; 2] = ["\"s, t\"", "image(\"a.png\")"];

fn inhalt_aus(n: usize) -> ZellInhalt {
    let n = n % (MARKUP.len() + AUSDRUECKE.len() + 3);
    if n < MARKUP.len() {
        markup(MARKUP[n])
    } else if n < MARKUP.len() + AUSDRUECKE.len() {
        ZellInhalt::Ausdruck(AUSDRUECKE[n - MARKUP.len()].into())
    } else {
        markup("") // leere Zellen häufiger
    }
}

/// Belegt das Raster in Lesereihenfolge mit Zellen, Spannen 1–2 soweit frei und im Rand.
fn modell() -> impl Strategy<Value = Tabelle> {
    (
        1usize..=5,
        1usize..=5,
        proptest::collection::vec((0u8..8, 0usize..100), 25),
        any::<bool>(),
        proptest::collection::vec(0usize..3, 5),
        0usize..4,
    )
        .prop_map(
            |(spalten, zeilen, wahl, mit_spuren, spurwahl, weitere_wahl)| {
                let mut belegt = vec![vec![false; spalten]; zeilen];
                let mut zellen = Vec::new();
                let mut w = wahl.into_iter();
                for z in 0..zeilen {
                    for s in 0..spalten {
                        if belegt[z][s] {
                            continue;
                        }
                        let (bits, inhalt) = w.next().unwrap_or((0, 0));
                        let mut cs = 1 + usize::from(bits & 1);
                        let mut rs = 1 + usize::from(bits & 2 != 0);
                        let frei = |cs: usize, rs: usize, b: &Vec<Vec<bool>>| {
                            z + rs <= zeilen
                                && s + cs <= spalten
                                && (z..z + rs).all(|zz| (s..s + cs).all(|ss| !b[zz][ss]))
                        };
                        if !frei(cs, rs, &belegt) {
                            cs = 1;
                        }
                        if !frei(cs, rs, &belegt) {
                            rs = 1;
                        }
                        for reihe in belegt.iter_mut().skip(z).take(rs) {
                            reihe[s..s + cs].fill(true);
                        }
                        let weitere = if bits & 4 != 0 {
                            vec!["fill: red".to_string()]
                        } else {
                            vec![]
                        };
                        zellen.push(Zelle {
                            zeile: z,
                            spalte: s,
                            zeilen: rs,
                            spalten: cs,
                            inhalt: inhalt_aus(inhalt),
                            weitere,
                        });
                    }
                }
                let spuren = mit_spuren.then(|| {
                    (0..spalten)
                        .map(|i| ["1fr", "auto", "2cm"][spurwahl[i]].to_string())
                        .collect()
                });
                let weitere = match weitere_wahl {
                    0 => vec![],
                    1 => vec!["stroke: 0.5pt".to_string()],
                    2 => vec!["align: center".to_string()],
                    _ => vec!["stroke: 0.5pt".to_string(), "align: center".to_string()],
                };
                Tabelle {
                    spalten,
                    zeilen,
                    spuren,
                    weitere,
                    zellen,
                }
            },
        )
}

/// Eine Aktion mit Indizes im gültigen Bereich (Fehler wie „verbundene Zellen im Weg“ möglich).
fn aktion(t: &Tabelle) -> impl Strategy<Value = TabellenAktion> + use<> {
    use TabellenAktion as A;
    let (n, m) = (t.zeilen, t.spalten);
    prop_oneof![
        (0..=n).prop_map(|vor| A::ZeileEinfuegen { vor }),
        (0..=m).prop_map(|vor| A::SpalteEinfuegen { vor }),
        (0..n).prop_map(|zeile| A::ZeileLoeschen { zeile }),
        (0..m).prop_map(|spalte| A::SpalteLoeschen { spalte }),
        (0..n, 0..n).prop_map(|(von, nach)| A::ZeileVerschieben { von, nach }),
        (0..m, 0..m).prop_map(|(von, nach)| A::SpalteVerschieben { von, nach }),
        (0..n, 0..m, 1usize..=3, 1usize..=3).prop_map(move |(zeile, spalte, h, b)| {
            A::Verbinden {
                zeile,
                spalte,
                zeilen: h.min(n - zeile),
                spalten: b.min(m - spalte),
            }
        }),
        (0..n, 0..m).prop_map(|(zeile, spalte)| A::Trennen { zeile, spalte }),
        (0..n, 0..m, 0..MARKUP.len()).prop_map(|(zeile, spalte, i)| A::Zelltext {
            zeile,
            spalte,
            inhalt: MARKUP[i].to_string(),
        }),
    ]
}

/// Jedes Feld genau einmal belegt, Zellen in Lesereihenfolge, Spuren passend.
fn stimmig(t: &Tabelle) -> Result<(), String> {
    let mut feld = vec![vec![0u8; t.spalten]; t.zeilen];
    for z in &t.zellen {
        if z.zeilen == 0 || z.spalten == 0 {
            return Err(format!("leere Spanne {z:?}"));
        }
        for zz in z.zeile..z.zeile + z.zeilen {
            for ss in z.spalte..z.spalte + z.spalten {
                *feld
                    .get_mut(zz)
                    .and_then(|r| r.get_mut(ss))
                    .ok_or(format!("außerhalb {z:?}"))? += 1;
            }
        }
    }
    if feld.iter().flatten().any(|f| *f != 1) {
        return Err(format!("Felder nicht genau einmal belegt: {feld:?}"));
    }
    let summe: usize = t.zellen.iter().map(|z| z.zeilen * z.spalten).sum();
    if summe != t.zeilen * t.spalten {
        return Err("Zellzahl passt nicht".into());
    }
    if !t
        .zellen
        .windows(2)
        .all(|w| (w[0].zeile, w[0].spalte) < (w[1].zeile, w[1].spalte))
    {
        return Err("nicht in Lesereihenfolge".into());
    }
    if t.spuren.as_ref().is_some_and(|s| s.len() != t.spalten) {
        return Err("Spuren passen nicht zur Spaltenzahl".into());
    }
    Ok(())
}

/// Nicht leere Inhalte als sortierte Liste (Multimenge).
fn multimenge<'a>(zellen: impl Iterator<Item = &'a Zelle>) -> Vec<ZellInhalt> {
    let mut v: Vec<ZellInhalt> = zellen
        .filter(|z| !z.inhalt.ist_leer())
        .map(|z| z.inhalt.clone())
        .collect();
    v.sort_by(|a, b| a.text().cmp(b.text()));
    v
}

proptest! {
    #![proptest_config(konfig())]

    #[test]
    fn p_modell((t, a) in modell().prop_flat_map(|t| {
        let a = aktion(&t);
        (Just(t), a)
    })) {
        use TabellenAktion as A;
        prop_assert_eq!(stimmig(&t), Ok(()), "Generator");
        match t.aendern(&a) {
            Err(PatchFehler::UngueltigerWert(_)) => {}
            Err(e) => prop_assert!(false, "falscher Fehler {e:?}"),
            Ok(neu) => {
                prop_assert_eq!(stimmig(&neu), Ok(()), "{:?}", neu);
                let (dz, ds): (isize, isize) = match &a {
                    A::ZeileEinfuegen { .. } => (1, 0),
                    A::SpalteEinfuegen { .. } => (0, 1),
                    A::ZeileLoeschen { .. } => (-1, 0),
                    A::SpalteLoeschen { .. } => (0, -1),
                    _ => (0, 0),
                };
                prop_assert_eq!(neu.zeilen as isize, t.zeilen as isize + dz);
                prop_assert_eq!(neu.spalten as isize, t.spalten as isize + ds);
                prop_assert_eq!(&neu.weitere, &t.weitere);
                let vorher = multimenge(t.zellen.iter());
                let nachher = multimenge(neu.zellen.iter());
                let erwartet = match &a {
                    A::ZeileLoeschen { zeile } => multimenge(
                        t.zellen.iter().filter(|z| !(z.zeile == *zeile && z.zeilen == 1)),
                    ),
                    A::SpalteLoeschen { spalte } => multimenge(
                        t.zellen.iter().filter(|z| !(z.spalte == *spalte && z.spalten == 1)),
                    ),
                    A::Zelltext { zeile, spalte, inhalt } => {
                        let i = t.zelle_bei(*zeile, *spalte).unwrap();
                        let mut z = t.zellen.clone();
                        z[i].inhalt = ZellInhalt::Markup(inhalt.clone());
                        multimenge(z.iter())
                    }
                    A::Verbinden { zeile, spalte, zeilen, spalten } => {
                        let drin = |z: &Zelle| {
                            z.zeile >= *zeile && z.zeile + z.zeilen <= zeile + zeilen
                                && z.spalte >= *spalte && z.spalte + z.spalten <= spalte + spalten
                        };
                        let innen: Vec<&Zelle> = t.zellen.iter().filter(|z| drin(z)).collect();
                        let voll: Vec<&ZellInhalt> = innen
                            .iter()
                            .filter(|z| !z.inhalt.ist_leer())
                            .map(|z| &z.inhalt)
                            .collect();
                        let mut rest: Vec<Zelle> =
                            t.zellen.iter().filter(|z| !drin(z)).cloned().collect();
                        let neu_inhalt = match voll.as_slice() {
                            [] => markup(""),
                            [eins] => (*eins).clone(),
                            viele => markup(
                                &viele.iter().map(|i| i.text()).collect::<Vec<_>>().join(" "),
                            ),
                        };
                        rest.push(zelle(0, 0, 1, 1, neu_inhalt));
                        multimenge(rest.iter())
                    }
                    _ => vorher.clone(),
                };
                prop_assert_eq!(nachher, erwartet);
            }
        }
    }
}

// ---------------------------------------------------------------- Task 3: Schreiben und Patch

fn patch(text: &str, marke: &str, aktion: &TabellenAktion) -> Result<Patch, PatchFehler> {
    let q = Source::detached(text.to_string());
    let stelle = tabelle_am(&q, text.find(marke).unwrap()).unwrap().unwrap();
    tabelle_patch(&q, &stelle, aktion)
}

fn neu(text: &str, marke: &str, aktion: TabellenAktion) -> String {
    anwenden(text, &patch(text, marke, &aktion).expect("Patch abgelehnt"))
}

const FIGUR: &str = "#figure(
  table(
    columns: 2,
    [Kopf 1], [Kopf 2],
    [Zelle], [],
  ),
  caption: [T],
)";

#[test]
fn schreibt_feste_form() {
    use TabellenAktion as A;
    assert_eq!(
        neu(FIGUR, "table", A::ZeileEinfuegen { vor: 1 }),
        "#figure(
  table(
    columns: 2,
    [Kopf 1], [Kopf 2],
    [], [],
    [Zelle], [],
  ),
  caption: [T],
)"
    );
    assert_eq!(
        neu(FIGUR, "table", A::SpalteEinfuegen { vor: 2 }),
        "#figure(
  table(
    columns: 3,
    [Kopf 1], [Kopf 2], [],
    [Zelle], [], [],
  ),
  caption: [T],
)"
    );
    assert_eq!(
        neu(
            FIGUR,
            "table",
            A::Verbinden {
                zeile: 0,
                spalte: 0,
                zeilen: 1,
                spalten: 2
            }
        ),
        "#figure(
  table(
    columns: 2,
    table.cell(colspan: 2)[Kopf 1 Kopf 2],
    [Zelle], [],
  ),
  caption: [T],
)"
    );
}

#[test]
fn einzeilig_bleibt_einzeilig() {
    let text = "Vor #table(columns: 2, [a], [b]) nach";
    assert_eq!(
        neu(text, "table", TabellenAktion::SpalteLoeschen { spalte: 1 }),
        "Vor #table(columns: 1, [a]) nach"
    );
    // Spuren, weitere Argumente und verbundene Zellen
    let text = "#table(columns: (1fr, 2cm), stroke: none, [a], [b], [c], [d])";
    assert_eq!(
        neu(
            text,
            "table",
            TabellenAktion::Verbinden {
                zeile: 0,
                spalte: 0,
                zeilen: 2,
                spalten: 1
            }
        ),
        "#table(columns: (1fr, 2cm), stroke: none, table.cell(rowspan: 2)[a c], [b], [d])"
    );
    assert_eq!(
        neu(text, "table", TabellenAktion::SpalteLoeschen { spalte: 0 }),
        "#table(columns: (2cm,), stroke: none, [b], [d])"
    );
    // ohne columns: kommt dazu, sobald es mehr als eine Spalte gibt
    assert_eq!(
        neu(
            "#table([a])",
            "table",
            TabellenAktion::SpalteEinfuegen { vor: 1 }
        ),
        "#table(columns: 2, [a], [])"
    );
    assert_eq!(
        neu(
            "#table(columns: auto, [a])",
            "table",
            TabellenAktion::ZeileEinfuegen { vor: 1 }
        ),
        "#table(columns: auto, [a], [])"
    );
}

#[test]
fn unberuehrte_zeilen_bleiben() {
    use TabellenAktion as A;
    let text = "Absatz davor.\n\n#table(\n  columns: 3,\n  [a], [b], [c],\n  [d], [e], [f],\n  [g], [h], [i],\n  [j], [k], [l],\n)\n\nDanach.\n";
    let alt: Vec<&str> = text.lines().collect();

    let n = neu(
        text,
        "table",
        A::Zelltext {
            zeile: 2,
            spalte: 1,
            inhalt: "neu".into(),
        },
    );
    let n: Vec<&str> = n.lines().collect();
    assert_eq!(n.len(), alt.len());
    let anders: Vec<usize> = (0..alt.len()).filter(|i| alt[*i] != n[*i]).collect();
    assert_eq!(anders.len(), 1);
    assert_eq!(n[anders[0]], "  [g], [neu], [i],");

    let n = neu(text, "table", A::ZeileEinfuegen { vor: 2 });
    let n: Vec<&str> = n.lines().collect();
    let mut erwartet = alt.clone();
    erwartet.insert(6, "  [], [], [],");
    assert_eq!(n, erwartet);
}

#[test]
fn zelltext_minimal() {
    use TabellenAktion as A;
    let text = "#table(columns: 2, [alt], \"s\")";
    let p = patch(
        text,
        "table",
        &A::Zelltext {
            zeile: 0,
            spalte: 0,
            inhalt: "neu".into(),
        },
    )
    .unwrap();
    assert_eq!(&text[p.bereich.clone()], "alt");
    assert_eq!(p.text, "neu");
    let p = patch(
        text,
        "table",
        &A::Zelltext {
            zeile: 0,
            spalte: 1,
            inhalt: "neu".into(),
        },
    )
    .unwrap();
    assert_eq!(&text[p.bereich.clone()], "\"s\"");
    assert_eq!(p.text, "[neu]");
    for kaputt in ["a]b", "a[b", "#emph["] {
        let e = patch(
            text,
            "table",
            &A::Zelltext {
                zeile: 0,
                spalte: 0,
                inhalt: kaputt.into(),
            },
        );
        assert!(e.is_err(), "{kaputt}: {e:?}");
    }
}

#[test]
fn zelltext_in_luecke() {
    assert_eq!(
        neu(
            "#table(columns: 2, [a], [b], [c])",
            "table",
            TabellenAktion::Zelltext {
                zeile: 1,
                spalte: 1,
                inhalt: "d".into()
            }
        ),
        "#table(columns: 2, [a], [b], [c], [d])"
    );
    assert_eq!(
        neu(
            "#table(\n  columns: 2,\n  [a], [b],\n  [c],\n)",
            "table",
            TabellenAktion::Zelltext {
                zeile: 1,
                spalte: 1,
                inhalt: "d".into()
            }
        ),
        "#table(\n  columns: 2,\n  [a], [b],\n  [c], [d],\n)"
    );
}

/// Schreibt ein Modell als Quelltext, unabhängig vom Code unter Test.
/// `form`: 0 einzeilig, 1 eine Zeile je Rasterzeile, 2 Umbrüche nach `umbrueche`.
fn quelltext(t: &Tabelle, form: u8, umbrueche: &[bool], ohne_columns: bool) -> String {
    let mut teile: Vec<String> = Vec::new();
    match &t.spuren {
        Some(s) if s.len() == 1 => teile.push(format!("columns: ({},)", s[0])),
        Some(s) => teile.push(format!("columns: ({})", s.join(", "))),
        None if t.spalten == 1 && ohne_columns => {}
        None => teile.push(format!("columns: {}", t.spalten)),
    }
    teile.extend(t.weitere.iter().cloned());
    let zelle = |z: &Zelle| {
        let mut args = z.weitere.clone();
        // andere Reihenfolge als der Schreiber in TFTN
        if z.zeilen > 1 {
            args.push(format!("rowspan: {}", z.zeilen));
        }
        if z.spalten > 1 {
            args.push(format!("colspan: {}", z.spalten));
        }
        match (&z.inhalt, args.is_empty()) {
            (ZellInhalt::Markup(m), true) => format!("[{m}]"),
            (ZellInhalt::Ausdruck(a), true) => a.clone(),
            (ZellInhalt::Markup(m), false) => format!("table.cell({})[{m}]", args.join(", ")),
            (ZellInhalt::Ausdruck(a), false) => format!("table.cell({}, {a})", args.join(", ")),
        }
    };
    let mut zeilen: Vec<Vec<String>> = vec![Vec::new(); t.zeilen];
    for z in &t.zellen {
        zeilen[z.zeile].push(zelle(z));
    }
    match form {
        0 => {
            teile.extend(zeilen.into_iter().flatten());
            format!("table({})", teile.join(", "))
        }
        1 => {
            let mut aus = String::from("table(\n");
            for b in &teile {
                aus.push_str(&format!("  {b},\n"));
            }
            for z in zeilen.into_iter().filter(|z| !z.is_empty()) {
                aus.push_str(&format!("  {},\n", z.join(", ")));
            }
            aus.push(')');
            aus
        }
        _ => {
            teile.extend(zeilen.into_iter().flatten());
            let mut aus = String::from("table(");
            for (i, x) in teile.iter().enumerate() {
                aus.push_str(x);
                if i + 1 < teile.len() {
                    aus.push(',');
                    aus.push_str(if umbrueche.get(i).copied().unwrap_or(false) {
                        "\n    "
                    } else {
                        " "
                    });
                }
            }
            aus.push(')');
            aus
        }
    }
}

fn umgebung() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            "Text ",
            "Größe und Maß ",
            "*fett* ",
            "$x^2 + y$ ",
            "= Titel\n",
            "\n\n",
            "// Kommentar\n",
            "#figure(image(\"b.png\"), caption: [Bild])\n",
            "😀 ",
            "- Punkt\n",
            "#footnote[Fuß] ",
        ]),
        0..6,
    )
    .prop_map(|v| v.concat())
}

/// Quelltext mit einer reinen Tabelle, ihr erwartetes Modell und der Anfang des Aufrufs.
fn fall() -> impl Strategy<Value = (String, usize, Tabelle)> {
    (
        modell(),
        0u8..3,
        proptest::collection::vec(any::<bool>(), 40),
        any::<bool>(),
        any::<bool>(),
        umgebung(),
        umgebung(),
    )
        .prop_map(|(t, form, umbrueche, ohne_columns, figur, vor, nach)| {
            let tab = quelltext(&t, form, &umbrueche, ohne_columns);
            let (kopf, fuss) = if figur {
                ("#figure(\n  ", ",\n  caption: [T],\n)")
            } else {
                ("#", "")
            };
            let text = format!("{vor}{kopf}{tab}{fuss}\n{nach}");
            let start = vor.len() + kopf.len();
            (text, start, t)
        })
}

fn fehler(k: &typst_syntax::SyntaxNode) -> usize {
    usize::from(k.kind() == typst_syntax::SyntaxKind::Error)
        + k.children().map(fehler).sum::<usize>()
}

proptest! {
    #![proptest_config(konfig())]

    #[test]
    fn p_lesen((text, start, t) in fall()) {
        let q = Source::detached(text.clone());
        let stelle = tabelle_am(&q, start).expect("keine Tabelle");
        prop_assert!(stelle.is_ok(), "{:?}\n{}", stelle.as_ref().err(), text);
        let stelle = stelle.unwrap();
        prop_assert_eq!(stelle.aufruf.start, start);
        prop_assert_eq!(stelle.tabelle, t, "{}", text);
    }

    #[test]
    fn p_aendern(((text, start, t), a) in fall().prop_flat_map(|f| {
        let a = aktion(&f.2);
        (Just(f), a)
    })) {
        let q = Source::detached(text.clone());
        let stelle = tabelle_am(&q, start).unwrap().unwrap();
        match tabelle_patch(&q, &stelle, &a) {
            Err(PatchFehler::UngueltigerWert(_)) => {}
            Err(e) => prop_assert!(false, "{e:?} bei {a:?}\n{text}"),
            Ok(p) => {
                prop_assert!(
                    p.bereich.start >= stelle.aufruf.start && p.bereich.end <= stelle.aufruf.end
                );
                let n = anwenden(&text, &p);
                prop_assert_eq!(&n[..p.bereich.start], &text[..p.bereich.start]);
                prop_assert_eq!(&n[p.bereich.start + p.text.len()..], &text[p.bereich.end..]);
                let nq = Source::detached(n.clone());
                prop_assert!(fehler(nq.root()) <= fehler(q.root()), "{}", n);
                let wieder = tabelle_am(&nq, start).unwrap().unwrap();
                prop_assert_eq!(wieder.tabelle, t.aendern(&a).unwrap(), "{}", n);
            }
        }
    }
}

#[test]
fn ergebnisse_kompilieren() {
    use TabellenAktion as A;
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let projekt = tftn_core::Projekt::oeffnen(&wurzel.join("tests/korpus/testartikel")).unwrap();
    let cli = tftn_export::TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI nötig");

    let quelle = "#table(columns: 3, table.cell(rowspan: 2)[a], [b], [c], [d], [e], [f], [g], [h])";
    let aktionen = [
        A::ZeileEinfuegen { vor: 1 },
        A::SpalteEinfuegen { vor: 1 },
        A::ZeileLoeschen { zeile: 0 },
        A::SpalteLoeschen { spalte: 0 },
        A::SpalteVerschieben { von: 2, nach: 1 },
        A::Verbinden {
            zeile: 0,
            spalte: 1,
            zeilen: 2,
            spalten: 2,
        },
        A::Trennen {
            zeile: 1,
            spalte: 0,
        },
        A::Zelltext {
            zeile: 2,
            spalte: 2,
            inhalt: "*neu*".into(),
        },
    ];
    let mut datei = String::new();
    for a in &aktionen {
        datei.push_str(&neu(quelle, "table", a.clone()));
        datei.push_str("\n\n");
    }
    for a in [
        A::ZeileEinfuegen { vor: 1 },
        A::SpalteEinfuegen { vor: 2 },
        A::ZeileVerschieben { von: 1, nach: 0 },
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 2,
            spalten: 2,
        },
    ] {
        datei.push_str(&neu(FIGUR, "table", a));
        datei.push_str("\n\n");
    }
    let tmp = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("tabellen");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("t.typ"), &datei).unwrap();
    let aus = std::process::Command::new(&cli.pfad)
        .args(["compile", "t.typ", "t.pdf"])
        .current_dir(&tmp)
        .output()
        .unwrap();
    assert!(
        aus.status.success(),
        "typst: {}\n{datei}",
        String::from_utf8_lossy(&aus.stderr)
    );
}

// ---------------------------------------------------------------- Task 4: Widget-Art tabelle

#[test]
fn widget_tabelle_zeigt_auf_raster() {
    use tftn_edit::{Ort, Wert, WidgetArt, WidgetParameter, angebote, wert_patch};
    let p = [WidgetParameter {
        position: None,
        funktion: "datentabelle".into(),
        parameter: "daten".into(),
        art: WidgetArt::Tabelle,
    }];
    let angebot = |text: &str| {
        let q = Source::detached(text.to_string());
        let mut a = angebote(&q, text.find("datentabelle").unwrap(), &p);
        assert_eq!(a.len(), 1, "{text}");
        (q, a.remove(0))
    };

    let rein = "#datentabelle(daten: table(columns: 2, [a], [b]))";
    let (q, a) = angebot(rein);
    assert_eq!(a.gesperrt, None);
    assert_eq!(a.wert, None);
    let Ort::Ersetzen { bereich } = &a.ort else {
        panic!("{:?}", a.ort)
    };
    assert_eq!(&rein[bereich.clone()], "table(columns: 2, [a], [b])");
    assert_eq!(
        tabelle_am(&q, bereich.start).unwrap().unwrap().aufruf,
        bereich.clone()
    );
    assert_eq!(
        wert_patch(&q, &a, &WidgetArt::Tabelle, &Wert::Text("x".into())),
        Err(PatchFehler::Gesperrt("Tabellen ändert das Raster".into()))
    );

    let (_, a) = angebot("#datentabelle(daten: table(columns: 2, ..x))");
    assert!(
        a.gesperrt.as_deref().unwrap().contains("`..`"),
        "{:?}",
        a.gesperrt
    );
    let (_, a) = angebot("#datentabelle(daten: x)");
    assert_eq!(
        a.gesperrt.as_deref(),
        Some("berechnet – im Text bearbeiten")
    );
    let (_, a) = angebot("#datentabelle(titel: [T])");
    assert_eq!(
        a.gesperrt.as_deref(),
        Some("Tabelle fehlt – im Text ergänzen")
    );
}
