//! Was beim Einfügen eines Bausteins außer seinem Schnipsel nötig ist (PLAN-M2 AP2): die Importzeile
//! eines Universe-Pakets, das im Projekt liegt, aber in der Datei noch nicht importiert wird.

use typst_syntax::ast;
use typst_syntax::{LinkedNode, Source};

use crate::{Eintrag, Herkunft};

/// Text, der an `byte` mit eingefügt wird (im selben Undo-Schritt wie das Schnipsel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Einfuegung {
    pub byte: usize,
    pub text: String,
}

/// Die nötigen Importzeilen für `eintrag` in `quelle`, falls er aus einem Paket stammt: alle, deren
/// Namen die Datei noch nicht bindet. Importiert die Datei das Paket schon in anderer Form, nimmt
/// die neue Zeile dieselbe Version.
pub fn import_fuer(quelle: &Source, eintrag: &Eintrag) -> Option<Einfuegung> {
    let Herkunft::Paket {
        name, import, auch, ..
    } = &eintrag.herkunft
    else {
        return None;
    };
    let zeilen: Vec<String> = std::iter::once((name, import))
        .chain(auch.iter().map(|(n, z)| (n, z)))
        .filter_map(|(n, z)| fehlende_zeile(quelle, n, z))
        .collect();
    if zeilen.is_empty() {
        return None;
    }
    Some(match letzte_importzeile(quelle) {
        Some(byte) => Einfuegung {
            byte,
            text: format!("\n{}", zeilen.join("\n")),
        },
        None => Einfuegung {
            byte: 0,
            text: format!("{}\n", zeilen.join("\n")),
        },
    })
}

/// Was eine Importanweisung bindet.
enum Bindung {
    Modul,
    Alles,
    Namen(Vec<String>),
}

struct Import {
    spec: String,
    umbenannt: Option<String>,
    bindung: Bindung,
}

fn import_lesen(imp: ast::ModuleImport) -> Option<Import> {
    let ast::Expr::Str(s) = imp.source() else {
        return None;
    };
    Some(Import {
        spec: s.get().to_string(),
        umbenannt: imp.new_name().map(|n| n.get().to_string()),
        bindung: match imp.imports() {
            None => Bindung::Modul,
            Some(ast::Imports::Wildcard) => Bindung::Alles,
            Some(ast::Imports::Items(items)) => Bindung::Namen(
                items
                    .iter()
                    .map(|i| i.bound_name().get().to_string())
                    .collect(),
            ),
        },
    })
}

/// Bindet einer der Importe `vorhanden` alles, was `gesucht` bindet?
fn gedeckt(vorhanden: &[Import], name: &str, gesucht: &Import) -> bool {
    if let Some(u) = &gesucht.umbenannt {
        return vorhanden.iter().any(|d| d.umbenannt.as_ref() == Some(u));
    }
    match &gesucht.bindung {
        Bindung::Modul => vorhanden.iter().any(|d| match &d.umbenannt {
            Some(u) => u == name,
            None => matches!(d.bindung, Bindung::Modul),
        }),
        Bindung::Alles => vorhanden
            .iter()
            .any(|d| matches!(d.bindung, Bindung::Alles)),
        Bindung::Namen(namen) => namen.iter().all(|n| {
            vorhanden.iter().any(|d| match &d.bindung {
                Bindung::Alles => true,
                Bindung::Namen(v) => v.contains(n),
                Bindung::Modul => false,
            })
        }),
    }
}

/// Die Zeile für `name`, wenn die Datei das nicht schon bindet, ggf. mit der Version der Datei.
fn fehlende_zeile(quelle: &Source, name: &str, zeile: &str) -> Option<String> {
    let gesucht = LinkedNode::new(&typst_syntax::parse(zeile))
        .children()
        .find_map(|k| k.get().cast::<ast::ModuleImport>().and_then(import_lesen))?;
    let anfang = format!("@preview/{name}:");
    let vorhanden: Vec<Import> = LinkedNode::new(quelle.root())
        .children()
        .filter_map(|k| k.get().cast::<ast::ModuleImport>().and_then(import_lesen))
        .filter(|i| i.spec.starts_with(&anfang))
        .collect();
    if gedeckt(&vorhanden, name, &gesucht) {
        return None;
    }
    Some(match vorhanden.first() {
        Some(i) => zeile.replace(&gesucht.spec, &i.spec),
        None => zeile.to_string(),
    })
}

/// Ende der Zeile mit dem letzten Import der obersten Ebene.
fn letzte_importzeile(quelle: &Source) -> Option<usize> {
    let text = quelle.text();
    LinkedNode::new(quelle.root())
        .children()
        .filter(|k| k.get().cast::<ast::ModuleImport>().is_some())
        .last()
        .map(|k| {
            let ende = k.range().end;
            text[ende..].find('\n').map_or(text.len(), |i| ende + i)
        })
}

/// Wo und wie `import` für das Universe-Paket `name` eingefügt wird: hinter dem letzten Import der
/// obersten Ebene, sonst am Dateianfang. `None`, wenn die Datei `name` schon importiert (gleich
/// welche Version). Auch „Paket hinzufügen“ (PLAN-M2 AP9) nutzt das.
pub fn importzeile(quelle: &Source, name: &str, import: &str) -> Option<Einfuegung> {
    let gesucht = format!("@preview/{name}:");
    let text = quelle.text();
    let mut letzte_zeile = None;
    // Oberste Ebene: `#import …` ist ein Kind der Wurzel (nach dem `#`).
    for kind in LinkedNode::new(quelle.root()).children() {
        let Some(imp) = kind.get().cast::<ast::ModuleImport>() else {
            continue;
        };
        if let ast::Expr::Str(s) = imp.source()
            && s.get().starts_with(&gesucht)
        {
            return None;
        }
        let ende = kind.range().end;
        letzte_zeile = Some(text[ende..].find('\n').map_or(text.len(), |i| ende + i));
    }
    Some(match letzte_zeile {
        Some(byte) => Einfuegung {
            byte,
            text: format!("\n{import}"),
        },
        None => Einfuegung {
            byte: 0,
            text: format!("{import}\n"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Herkunft, Schnipsel};

    const IMPORT: &str = "#import \"@preview/demo:0.1.3\": kasten";

    fn eintrag(herkunft: Herkunft) -> Eintrag {
        Eintrag {
            id: "@preview/demo/kasten".into(),
            titel: "Kasten".into(),
            gruppe: None,
            herkunft,
            funktion: Some("kasten".into()),
            schnipsel: Schnipsel::lesen("#kasten[${auswahl}]").unwrap(),
            modus: vec![tftn_manifest::Modus::Markup],
            kuerzel: None,
            parameter: Default::default(),
        }
    }

    fn paket() -> Eintrag {
        eintrag(Herkunft::Paket {
            name: "demo".into(),
            version: semver::Version::new(0, 1, 3),
            import: IMPORT.into(),
            auch: Vec::new(),
        })
    }

    /// Fügt ein und prüft, dass kein Fehlerknoten entsteht.
    fn eingefuegt(text: &str, e: &Einfuegung) -> String {
        let mut neu = text.to_string();
        neu.insert_str(e.byte, &e.text);
        assert!(!typst_syntax::parse(&neu).diagnosis().either(), "{neu:?}");
        neu
    }

    #[test]
    fn ohne_import_am_dateianfang() {
        let e = import_fuer(&Source::detached("Text"), &paket()).unwrap();
        assert_eq!(
            e,
            Einfuegung {
                byte: 0,
                text: format!("{IMPORT}\n")
            }
        );
        eingefuegt("Text", &e);
    }

    #[test]
    fn nach_der_letzten_importzeile() {
        let text = "// k\n#import \"@local/x:0.1.0\": *\n#show: x\n";
        let e = import_fuer(&Source::detached(text), &paket()).unwrap();
        assert_eq!(e.byte, text.find("\n#show").unwrap());
        assert_eq!(e.text, format!("\n{IMPORT}"));
        assert_eq!(
            eingefuegt(text, &e),
            format!("// k\n#import \"@local/x:0.1.0\": *\n{IMPORT}\n#show: x\n")
        );
    }

    #[test]
    fn paket_schon_importiert() {
        let text = "#import \"@preview/demo:0.1.0\": *\n";
        assert_eq!(import_fuer(&Source::detached(text), &paket()), None);
    }

    #[test]
    fn typst_baustein_braucht_nichts() {
        assert_eq!(
            import_fuer(&Source::detached("Text"), &eintrag(Herkunft::Typst)),
            None
        );
    }

    fn mit_import(import: &str) -> Eintrag {
        eintrag(Herkunft::Paket {
            name: "demo".into(),
            version: semver::Version::new(0, 1, 3),
            import: import.into(),
            auch: Vec::new(),
        })
    }

    /// Review Focus 2: Ist das Paket anders importiert, wird die fehlende Bindung ergänzt, und zwar
    /// mit der Version, die die Datei schon benutzt.
    #[test]
    fn import_bindung_faelle() {
        const MODUL: &str = "#import \"@preview/demo:0.1.3\"";
        let e = mit_import(MODUL);
        let fuer = |text: &str| import_fuer(&Source::detached(text), &e);
        assert_eq!(fuer("Text").unwrap().text, format!("{MODUL}\n"));
        assert_eq!(fuer("#import \"@preview/demo:0.2.0\"\n"), None);
        assert_eq!(fuer("#import \"@preview/demo:0.2.0\" as demo\n"), None);
        for form in [": kasten", ": *", " as d"] {
            let text = format!("#import \"@preview/demo:0.2.0\"{form}\nText");
            let f = fuer(&text).unwrap_or_else(|| panic!("{form}"));
            assert_eq!(f.text, "\n#import \"@preview/demo:0.2.0\"", "{form}");
            eingefuegt(&text, &f);
        }
        const NAMEN: &str = "#import \"@preview/demo:0.1.3\": kasten";
        let e = mit_import(NAMEN);
        let fuer = |text: &str| import_fuer(&Source::detached(text), &e);
        assert_eq!(fuer("#import \"@preview/demo:0.2.0\": *\n"), None);
        assert_eq!(fuer("#import \"@preview/demo:0.2.0\": kasten, x\n"), None);
        assert_eq!(
            fuer("#import \"@preview/demo:0.2.0\": anderes\n")
                .unwrap()
                .text,
            "\n#import \"@preview/demo:0.2.0\": kasten"
        );
    }

    #[test]
    fn mehrere_zeilen_in_einer_einfuegung() {
        let e = eintrag(Herkunft::Paket {
            name: "plot".into(),
            version: semver::Version::new(0, 1, 0),
            import: "#import \"@preview/plot:0.1.0\"".into(),
            auch: vec![("demo".into(), "#import \"@preview/demo:0.1.3\"".into())],
        });
        let text = "#import \"@local/a:0.1.0\": *\nText";
        let f = import_fuer(&Source::detached(text), &e).unwrap();
        assert_eq!(
            eingefuegt(text, &f),
            "#import \"@local/a:0.1.0\": *\n#import \"@preview/plot:0.1.0\"\n#import \"@preview/demo:0.1.3\"\nText"
        );
        let schon = "#import \"@preview/demo:0.1.3\"\nText";
        let f = import_fuer(&Source::detached(schon), &e).unwrap();
        assert_eq!(f.text, "\n#import \"@preview/plot:0.1.0\"");
    }

    const ZEILE: &str = "#import \"@preview/tftn-probe:0.1.0\"";

    #[test]
    fn importzeile_nach_import() {
        let text = "#import \"@local/a:0.1.0\": x\nText\n";
        let e = importzeile(&Source::detached(text), "tftn-probe", ZEILE).unwrap();
        assert_eq!(
            eingefuegt(text, &e),
            format!("#import \"@local/a:0.1.0\": x\n{ZEILE}\nText\n")
        );
    }

    #[test]
    fn importzeile_am_anfang() {
        let e = importzeile(&Source::detached("Text"), "tftn-probe", ZEILE).unwrap();
        assert_eq!(eingefuegt("Text", &e), format!("{ZEILE}\nText"));
    }

    #[test]
    fn importzeile_schon_da() {
        let text = "#import \"@preview/tftn-probe:0.0.9\": *\n";
        assert_eq!(
            importzeile(&Source::detached(text), "tftn-probe", ZEILE),
            None
        );
        // Ein Paket mit gleichem Anfang ist ein anderes.
        let text = "#import \"@preview/tftn-probe-basis:0.1.0\": *\n";
        assert!(importzeile(&Source::detached(text), "tftn-probe", ZEILE).is_some());
    }
}
