//! Eigene Textbausteine: Lesen, Fehlerfälle, Anhängen.

use tftn_katalog::textbausteine::{VORLAGE, anhaengen, im_ordner, lesen};
use tftn_katalog::{Herkunft, Katalog};
use tftn_manifest::Modus;

const ZWEI: &str = r#"# Meine Sätze
[[baustein]]
titel = "Procedere"
gruppe = "Befund"
einfuegen = '''
Kontrolle in ${feld:Wochen:6} Wochen.
${cursor}'''

# Grußformel für Briefe
[[baustein]]
id = "gruss"
titel = "Grußformel"
kuerzel = "Mod-Alt-g"
einfuegen = "Mit freundlichen Grüßen"
"#;

#[test]
fn liest_titel_gruppe_kuerzel_und_felder() {
    let g = lesen(ZWEI);
    assert_eq!(g.meldung(), None);
    assert_eq!(g.eintraege.len(), 2);
    let p = &g.eintraege[0];
    assert_eq!(p.id, "eigene/befund-procedere");
    assert_eq!(p.titel, "Procedere");
    assert_eq!(p.gruppe.as_deref(), Some("Befund"));
    assert_eq!(p.herkunft, Herkunft::Eigene);
    assert_eq!(p.modus, vec![Modus::Markup]);
    assert!(p.funktion.is_none());
    let e = p.schnipsel.einsetzen("");
    assert_eq!(e.text, "Kontrolle in 6 Wochen.\n");
    assert_eq!(e.felder.len(), 1);
    assert_eq!(e.felder[0].name, "Wochen");
    assert_eq!(e.cursor, Some(e.text.len()));
    let gruss = &g.eintraege[1];
    assert_eq!(gruss.id, "eigene/gruss");
    assert_eq!(gruss.kuerzel.as_deref(), Some("Mod-Alt-g"));
}

#[test]
fn fehlerhafte_datei_meldet_zeile_und_bringt_nichts() {
    let g = lesen("[[baustein]]\ntitel = \"A\"\neinfuegen = \"x\n");
    assert!(g.eintraege.is_empty());
    let f = g.fehler.expect("Fehler");
    assert!(f.starts_with("textbausteine.toml, Zeile 3"), "{f}");
}

#[test]
fn fehlerhafter_eintrag_wird_uebersprungen_die_anderen_bleiben() {
    let text = format!(
        "{ZWEI}\n[[baustein]]\ntitel = \"Ohne Text\"\n\n[[baustein]]\ntitel = \"Kaputt\"\neinfuegen = \"${{feld:offen\"\n\n[[baustein]]\nid = \"gruss\"\ntitel = \"Doppelt\"\neinfuegen = \"y\"\n"
    );
    let g = lesen(&text);
    assert!(g.fehler.is_none());
    assert_eq!(g.eintraege.len(), 2, "{:?}", g.warnungen);
    assert_eq!(g.warnungen.len(), 3, "{:?}", g.warnungen);
    assert!(
        g.warnungen[0].contains("Baustein 3 („Ohne Text“)")
            && g.warnungen[0].contains("einfuegen fehlt"),
        "{}",
        g.warnungen[0]
    );
    assert!(g.warnungen[1].contains("„Kaputt“"), "{}", g.warnungen[1]);
    assert!(g.warnungen[2].contains("doppelt"), "{}", g.warnungen[2]);
    assert!(g.meldung().unwrap().contains(" · "));
}

#[test]
fn fehlende_datei_ist_kein_fehler() {
    let o = std::env::temp_dir().join(format!("tftn-textbausteine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    let g = im_ordner(&o);
    assert!(g.eintraege.is_empty() && g.meldung().is_none());
    std::fs::write(o.join("textbausteine.toml"), ZWEI).unwrap();
    assert_eq!(im_ordner(&o).eintraege.len(), 2);
    std::fs::remove_dir_all(&o).unwrap();
}

#[test]
fn anhaengen_erhaelt_kommentare_und_reihenfolge() {
    let text = "Befund unauffällig.\nKeine Therapie nötig.";
    let (neu, id) = anhaengen(Some(ZWEI), "Procedere", Some("Befund"), text).unwrap();
    // Alles bisherige bleibt Zeichen für Zeichen stehen.
    assert!(neu.starts_with(ZWEI), "{neu}");
    assert_eq!(id, "procedere");
    let g = lesen(&neu);
    assert_eq!(g.meldung(), None);
    let ids: Vec<_> = g.eintraege.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "eigene/befund-procedere",
            "eigene/gruss",
            "eigene/procedere"
        ]
    );
    let e = &g.eintraege[2];
    assert_eq!(e.schnipsel.einsetzen("").text, text);
    assert_eq!(e.gruppe.as_deref(), Some("Befund"));
    // Mehrzeilig lesbar abgelegt.
    assert!(
        neu.contains("einfuegen = '''\nBefund unauffällig."),
        "{neu}"
    );

    // Gleicher Titel noch einmal: neue Id, nichts überschrieben.
    let (noch, id2) = anhaengen(Some(&neu), "Procedere", None, "Anderes").unwrap();
    assert_eq!(id2, "procedere-2");
    assert!(noch.starts_with(&neu));
    assert_eq!(lesen(&noch).eintraege.len(), 4);
}

#[test]
fn anhaengen_ohne_datei_beginnt_mit_vorlage() {
    let (neu, _) = anhaengen(None, "Gruß", None, "Mit freundlichen Grüßen").unwrap();
    assert!(neu.starts_with(VORLAGE));
    let g = lesen(&neu);
    assert_eq!(g.eintraege.len(), 1);
    assert_eq!(g.eintraege[0].titel, "Gruß");
}

#[test]
fn anhaengen_ohne_zeilenende_und_mit_sonderzeichen() {
    let alt = "[[baustein]]\ntitel = \"A\"\neinfuegen = \"a\"";
    for text in [
        "Zitat: '''drei''' und \"doppelt\"\nzweite Zeile",
        "endet mit Apostroph'\nja'",
        "Tab\tund Backslash \\ und ${kein Feld}",
        "einzeilig mit ${feld:Name} zum Ausfüllen",
    ] {
        let (neu, id) = anhaengen(Some(alt), "B", None, text).unwrap();
        assert!(neu.starts_with(alt));
        let g = lesen(&neu);
        assert_eq!(g.meldung(), None, "{neu}");
        let e = g
            .eintraege
            .iter()
            .find(|e| e.id == format!("eigene/{id}"))
            .unwrap();
        assert_eq!(
            e.schnipsel,
            tftn_katalog::Schnipsel::lesen(text).unwrap(),
            "{neu}"
        );
    }
}

#[test]
fn anhaengen_an_fehlerhafte_datei_wird_verweigert() {
    let kaputt = "[[baustein]\ntitel = 1\n";
    let f = anhaengen(Some(kaputt), "A", None, "Text").unwrap_err();
    assert!(f.contains("reparieren"), "{f}");
    assert!(anhaengen(Some(ZWEI), "  ", None, "Text").is_err());
    assert!(anhaengen(Some(ZWEI), "A", None, " \n ").is_err());
    // Ein Feld, das nicht schließt, wird nicht gespeichert.
    assert!(anhaengen(Some(ZWEI), "A", None, "x ${feld:offen").is_err());
}

#[test]
fn katalog_nimmt_eigene_auf() {
    let mut k = Katalog::default();
    let mut w = Vec::new();
    k.eigene_hinzufuegen(lesen(ZWEI).eintraege, &mut w);
    k.eigene_hinzufuegen(lesen(ZWEI).eintraege, &mut w);
    assert_eq!(k.eintraege().len(), 2);
    assert_eq!(w.len(), 2, "doppelte Ids werden gemeldet");
    assert_eq!(k.finden("eigene/gruss").unwrap().herkunft.art(), "eigene");
}
