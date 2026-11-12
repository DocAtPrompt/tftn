//! typstyle-Stabilität S1–S3 (PLAN-M1 4.2) mit typstyle-core 0.15.2, das auf typst-syntax 0.15.1
//! aufbaut. typstyle ist nur Werkzeug, nie automatisch beim Speichern. Dazu: Die Vorlagen in
//! `typen/` sind formatiert eingecheckt (ersetzt `typstyle --check` in der CI).
//!
//! S2 braucht die typst-CLI in der Projektversion (TFTN_TYPST oder PATH).

use std::path::{Path, PathBuf};

use tftn_core::Projekt;
use tftn_edit::{
    MarkupAktion, Ort, Wert, WidgetArt, WidgetParameter, angebote, anwenden, farbe_patch,
    formel_patch, markup_patch, markup_stellen, wert_patch, zitat_patch,
};
use tftn_export::{TypstCli, exportieren};
use typst_syntax::Source;
use typstyle_core::{Config, LineEnding, Typstyle};

/// Wie die Quellen im Repo: 2 Leerzeichen, LF, Zeilen bis 100 Zeichen (Vorgabe von typstyle: 80).
fn fmt(text: &str) -> String {
    let konfig = Config {
        max_width: 100,
        tab_spaces: 2,
        line_ending: LineEnding::Lf,
        ..Config::default()
    };
    Typstyle::new(konfig)
        .format_text(text)
        .render()
        .expect("typstyle: Syntaxfehler")
}

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Alle `.typ`-Dateien unter `ordner`, ohne `_renders/`, `_lokal/`.
fn typ_dateien(ordner: &Path) -> Vec<PathBuf> {
    let mut liste = Vec::new();
    for e in std::fs::read_dir(ordner).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap().to_string_lossy().starts_with('_') {
            continue;
        }
        if p.is_dir() {
            liste.extend(typ_dateien(&p));
        } else if p.extension().is_some_and(|e| e == "typ") {
            liste.push(p);
        }
    }
    liste.sort();
    liste
}

fn alle_quellen() -> Vec<PathBuf> {
    let w = wurzel();
    let mut liste = typ_dateien(&w.join("typen"));
    liste.extend(typ_dateien(&w.join("tests/korpus")));
    assert!(liste.len() >= 4, "{liste:?}");
    liste
}

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().unwrap().to_string_lossy().starts_with('_') {
            continue;
        }
        let ziel = nach.join(p.file_name().unwrap());
        if p.is_dir() {
            kopieren(&p, &ziel)
        } else {
            std::fs::copy(&p, &ziel).unwrap();
        }
    }
}

fn farbe() -> [WidgetParameter; 1] {
    [WidgetParameter {
        position: None,
        funktion: "hinweis".into(),
        parameter: "farbe".into(),
        art: tftn_edit::WidgetArt::Farbe {
            raum: tftn_edit::Farbraum::Rgb,
        },
    }]
}

/// S1: `fmt(fmt(x)) == fmt(x)` für alle Korpusdateien und Vorlagen.
#[test]
fn s1_idempotent() {
    for datei in alle_quellen() {
        let einmal = fmt(&std::fs::read_to_string(&datei).unwrap());
        assert_eq!(fmt(&einmal), einmal, "{}", datei.display());
    }
}

/// Die Vorlagen in `typen/` sind so eingecheckt, wie typstyle sie ausgibt.
#[test]
fn vorlagen_sind_formatiert() {
    for datei in typ_dateien(&wurzel().join("typen")) {
        let text = std::fs::read_to_string(&datei).unwrap();
        assert!(
            fmt(&text) == text,
            "{} ist nicht formatiert (typstyle-core 0.15.2, Breite 100)",
            datei.display()
        );
    }
}

/// S2: Der Testartikel ergibt mit allen Quellen (samt mitgelieferten Paketen) formatiert
/// dasselbe PDF, Byte für Byte.
#[test]
fn s2_bedeutung_bleibt() {
    let korpus = wurzel().join("tests/korpus/testartikel");
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let roh = tmp.join("s2-roh");
    let formatiert = tmp.join("s2-formatiert");
    for o in [&roh, &formatiert] {
        let _ = std::fs::remove_dir_all(o);
        kopieren(&korpus, o);
    }
    let mut geaendert = 0;
    for datei in typ_dateien(&formatiert) {
        let text = std::fs::read_to_string(&datei).unwrap();
        let neu = fmt(&text);
        geaendert += (neu != text) as usize;
        std::fs::write(&datei, neu).unwrap();
    }
    assert!(
        geaendert > 0,
        "typstyle hat nichts geändert, Test prüft nichts"
    );

    let mut pdfs = Vec::new();
    for o in [&roh, &formatiert] {
        let projekt = Projekt::oeffnen(o).unwrap();
        let cli = TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI nötig");
        let ziel = o.join("_renders/s2.pdf");
        std::fs::create_dir_all(ziel.parent().unwrap()).unwrap();
        exportieren(&projekt, &cli, &ziel, &[]).unwrap();
        pdfs.push(std::fs::read(&ziel).unwrap());
    }
    assert!(pdfs[0] == pdfs[1], "PDF ändert sich durch typstyle");
}

/// Wendet für jeden `#hinweis` in `text` (formatiert) Farb-Patches an; Ergebnis je Fall:
/// (Ort, ob `patch(fmt(x))` formatiert bleibt).
fn s3_faelle(text: &str) -> Vec<(Ort, bool)> {
    let quelle = Source::detached(text.to_string());
    let mut aus = Vec::new();
    for (pos, _) in text.match_indices("#hinweis") {
        for a in angebote(&quelle, pos + 2, &farbe()) {
            for hex in ["fde8c8", "000000", "e8f1f2"] {
                let neu = anwenden(text, &farbe_patch(&quelle, &a, hex).unwrap());
                aus.push((a.ort.clone(), fmt(&neu) == neu));
            }
        }
    }
    aus
}

/// S3: `fmt(patch(fmt(x))) == patch(fmt(x))`: Ein Widget-Patch auf formatiertem Code bleibt
/// formatiert. Gilt beim Ersetzen (Literal gleich lang, A4) und beim Ergänzen, solange die Zeile
/// in 100 Zeichen passt.
#[test]
fn s3_patch_bleibt_formatiert() {
    let w = wurzel();
    let artikel =
        fmt(&std::fs::read_to_string(w.join("tests/korpus/testartikel/main.typ")).unwrap());
    let ersetzen = s3_faelle(&artikel);
    assert!(!ersetzen.is_empty());
    for (ort, gut) in &ersetzen {
        assert!(matches!(ort, Ort::Ersetzen { .. }) && *gut, "{ort:?}");
    }
    for kurz in ["#hinweis[Kurz.]\n", "#hinweis(art: \"merke\")[Kurz.]\n"] {
        let faelle = s3_faelle(&fmt(kurz));
        assert!(!faelle.is_empty());
        for (ort, gut) in faelle {
            assert!(
                matches!(ort, Ort::Einfuegen { .. }) && gut,
                "{kurz:?} {ort:?}"
            );
        }
    }
}

/// Bekannte Grenze von S3 (KONZEPT Anhang B): Wird das Argument ergänzt und die Zeile dadurch
/// länger als 100 Zeichen, bricht typstyle den Aufruf um. Der Patch bleibt richtig, nur nicht mehr
/// formatiert; typstyle läuft nie automatisch. Schlägt dieser Test fehl, ist die Grenze weg.
#[test]
fn s3_grenze_ergaenzen_in_langer_zeile() {
    let vorlage = fmt(&std::fs::read_to_string(
        wurzel().join("typen/tftn-fachartikel/vorlage/main.typ"),
    )
    .unwrap());
    let faelle = s3_faelle(&vorlage);
    assert!(!faelle.is_empty());
    for (ort, gut) in faelle {
        assert!(matches!(ort, Ort::Einfuegen { .. }) && !gut, "{ort:?}");
    }
}

/// Patcht in `text` (formatiert) den Parameter `parameter` am Aufruf, in dem `cursor` steht;
/// `true`, wenn das Ergebnis formatiert bleibt.
fn s3_wert(
    text: &str,
    cursor: &str,
    funktion: &str,
    parameter: &str,
    art: WidgetArt,
    wert: Wert,
) -> bool {
    let q = Source::detached(text.to_string());
    let p = [WidgetParameter {
        position: None,
        funktion: funktion.into(),
        parameter: parameter.into(),
        art: art.clone(),
    }];
    let a = angebote(&q, text.find(cursor).unwrap(), &p);
    let neu = anwenden(text, &wert_patch(&q, &a[0], &art, &wert).unwrap());
    fmt(&neu) == neu
}

/// S3 für die Literal-Arten aus AP4: Ersetzen in kurzen Zeilen bleibt formatiert.
#[test]
fn s3_literal_arten() {
    let t = fmt("#kasten(art: \"info\", titel: \"Alt\", breite: 2pt, sichtbar: true)[Kurz.]\n");
    let wahl = WidgetArt::Wahl { werte: vec![] };
    assert!(s3_wert(
        &t,
        "Kurz",
        "kasten",
        "art",
        wahl,
        Wert::Text("warnung".into())
    ));
    let text = WidgetArt::Text { mehrzeilig: false };
    assert!(s3_wert(
        &t,
        "Kurz",
        "kasten",
        "titel",
        text,
        Wert::Text("Neuer „Titel“".into())
    ));
    let laenge = WidgetArt::Laenge { einheiten: vec![] };
    let l = Wert::Laenge {
        betrag: 1.5,
        einheit: "em".into(),
    };
    assert!(s3_wert(&t, "Kurz", "kasten", "breite", laenge, l));
    assert!(s3_wert(
        &t,
        "Kurz",
        "kasten",
        "sichtbar",
        WidgetArt::Schalter,
        Wert::Schalter(false)
    ));
}

/// S3 für Inhalt: `caption` und Rumpf mit einzeiligem Markup.
#[test]
fn s3_inhalt() {
    let t = fmt("#figure(image(\"a.png\"), caption: [Alt])\n\nSatz#footnote[Fuß].\n");
    let caption = WidgetArt::Inhalt { rumpf: false };
    assert!(s3_wert(
        &t,
        "a.png",
        "figure",
        "caption",
        caption,
        Wert::Inhalt("Neue *Unterschrift*".into())
    ));
    let rumpf = WidgetArt::Inhalt { rumpf: true };
    assert!(s3_wert(
        &t,
        "Fuß",
        "footnote",
        "body",
        rumpf,
        Wert::Inhalt("Andere Fußnote".into())
    ));
}

/// S3 für die Markup-Helfer: Ebene, Liste ↔ Aufzählung, Formel.
#[test]
fn s3_markup_helfer() {
    let eins = fmt("= Titel\n\nText.\n\n- a\n- b\n\nEs gilt $x^2$.\n");
    let zwei = fmt("== Titel\n\n+ a\n+ b\n\nEs gilt $ a + b $.\n");
    for (t, cursor, aktion) in [
        (&eins, "Titel", MarkupAktion::EbenePlus),
        (&eins, "a\n", MarkupAktion::ListeUmschalten),
        (&eins, "x^2", MarkupAktion::FormelUmschalten),
        (&zwei, "Titel", MarkupAktion::EbeneMinus),
        (&zwei, "a\n", MarkupAktion::ListeUmschalten),
        (&zwei, "a + b", MarkupAktion::FormelUmschalten),
    ] {
        let t = t.clone();
        let q = Source::detached(t.clone());
        let stelle = markup_stellen(&q, t.find(cursor).unwrap()).remove(0);
        let neu = anwenden(&t, &markup_patch(&q, &stelle, aktion).unwrap());
        assert_eq!(fmt(&neu), neu, "{aktion:?}");
    }
}

/// S3 für das Formelfeld und das Ändern von Zitaten (PLAN-M2 AP10, AP11).
#[test]
fn s3_formel_und_zitat() {
    use tftn_edit::MarkupStelle as S;
    let t = fmt("= Titel\n\nEs gilt $x^2$ und\n\n$ a + b $\n\nText @alt[S. 3] und @neu mehr.\n");
    let stelle = |q: &Source, cursor: &str, zitat: bool| {
        markup_stellen(q, t.find(cursor).unwrap())
            .into_iter()
            .find(|s| matches!(s, S::Zitat { .. }) == zitat)
            .unwrap()
    };
    let q = Source::detached(t.clone());
    let patches = [
        formel_patch(&q, &stelle(&q, "x^2", false), "frac(a, b)").unwrap(),
        formel_patch(&q, &stelle(&q, "a + b", false), "sum_(i=1)^n i").unwrap(),
        zitat_patch(&q, &stelle(&q, "@alt", true), "alt", Some("S. 12")).unwrap(),
        zitat_patch(&q, &stelle(&q, "@alt", true), "alt", None).unwrap(),
        zitat_patch(&q, &stelle(&q, "@neu", true), "neu", Some("Kap. 2")).unwrap(),
        zitat_patch(&q, &stelle(&q, "@alt", true), "anders", Some("S. 3")).unwrap(),
    ];
    for p in patches {
        let neu = anwenden(&t, &p);
        assert_eq!(fmt(&neu), neu, "{}", p.text);
    }
}

/// S3 für die Patch-Klasse Tabellenstruktur (PLAN-M2 AP5): auf formatiertem Code bleibt das
/// Ergebnis jeder Aktion formatiert.
#[test]
fn s3_tabellenstruktur() {
    use tftn_edit::{TabellenAktion as A, tabelle_am, tabelle_patch};
    let t = fmt(
        "#figure(\n  table(\n    columns: 2,\n    [Kopf 1], [Kopf 2],\n    [Zelle], [],\n    [a], [b],\n  ),\n  caption: [T],\n)\n",
    );
    for aktion in [
        A::ZeileEinfuegen { vor: 1 },
        A::SpalteEinfuegen { vor: 1 },
        A::ZeileLoeschen { zeile: 1 },
        A::SpalteLoeschen { spalte: 0 },
        A::ZeileVerschieben { von: 2, nach: 0 },
        A::SpalteVerschieben { von: 1, nach: 0 },
        A::Verbinden {
            zeile: 0,
            spalte: 0,
            zeilen: 1,
            spalten: 2,
        },
        A::Zelltext {
            zeile: 1,
            spalte: 1,
            inhalt: "neu".into(),
        },
    ] {
        let q = Source::detached(t.clone());
        let stelle = tabelle_am(&q, t.find("table").unwrap()).unwrap().unwrap();
        let neu = anwenden(&t, &tabelle_patch(&q, &stelle, &aktion).unwrap());
        assert_eq!(fmt(&neu), neu, "{aktion:?}");
    }
}
