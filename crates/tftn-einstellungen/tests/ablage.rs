//! Ablage der Einstellungen (PLAN-M2 AP3, Task 1).

use std::fs;
use std::path::{Path, PathBuf};

use tftn_einstellungen::{
    EINSTELLUNGEN, Rechner, TASTEN, kuerzel_setzen, ort, tasten_lesen, umziehen,
};

/// Eigener leerer Ordner unter dem Temp-Verzeichnis; wird beim Verlassen gelöscht.
struct Tmp(PathBuf);

impl Tmp {
    fn neu(name: &str) -> Tmp {
        let p =
            std::env::temp_dir().join(format!("tftn-einstellungen-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn p(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rechner_ohne_datei_ist_standard() {
    let t = Tmp::neu("rechner");
    let r = Rechner::laden(t.p());
    assert_eq!(
        r,
        Rechner {
            ordner: None,
            ganz_offline: false,
            sprache: None,
        }
    );
    let gesetzt = Rechner {
        ordner: Some(t.p().join("tresor")),
        ganz_offline: true,
        sprache: Some("en".into()),
    };
    gesetzt.speichern(t.p()).unwrap();
    assert_eq!(Rechner::laden(t.p()), gesetzt);
}

#[test]
fn ort_ohne_wahl_ist_app_ordner() {
    let t = Tmp::neu("ohne-wahl");
    let o = ort(t.p(), &Rechner::default());
    assert_eq!(o.ordner, t.p());
    assert!(!o.synchronisiert);
    assert!(o.hinweis.unwrap().contains("nicht synchronisiert"));
}

#[test]
fn ort_fehlender_ordner() {
    let t = Tmp::neu("fehlt");
    let weg = t.p().join("gibt-es-nicht");
    let r = Rechner {
        ordner: Some(weg.clone()),
        ganz_offline: false,
        sprache: None,
    };
    let o = ort(t.p(), &r);
    assert_eq!(o.ordner, t.p());
    assert!(!o.synchronisiert);
    assert!(o.hinweis.unwrap().contains("nicht gefunden"));
    assert!(!weg.exists());
}

#[test]
fn ort_gewaehlt() {
    let t = Tmp::neu("gewaehlt");
    let tresor = t.p().join("tresor");
    fs::create_dir_all(&tresor).unwrap();
    let r = Rechner {
        ordner: Some(tresor.clone()),
        ganz_offline: false,
        sprache: None,
    };
    let o = ort(t.p(), &r);
    assert_eq!(o.ordner, tresor);
    assert!(o.synchronisiert);
    assert_eq!(o.hinweis, None);
}

#[test]
fn setzen_und_lesen() {
    let t = Tmp::neu("setzen");
    let o = t.p();
    assert!(tasten_lesen(o).kuerzel.is_empty());
    assert_eq!(tasten_lesen(o).fehler, None);
    kuerzel_setzen(o, "typst/fussnote", Some("Mod-Alt-f")).unwrap();
    let l = tasten_lesen(o);
    assert_eq!(l.fehler, None);
    assert_eq!(l.kuerzel.len(), 1);
    assert_eq!(l.kuerzel["typst/fussnote"], "Mod-Alt-f");
    assert!(
        fs::read_to_string(o.join(TASTEN))
            .unwrap()
            .starts_with("# TFTN-Tastenkürzel")
    );
    kuerzel_setzen(o, "typst/fussnote", Some("")).unwrap();
    assert_eq!(tasten_lesen(o).kuerzel["typst/fussnote"], "");
    kuerzel_setzen(o, "typst/fussnote", None).unwrap();
    assert!(tasten_lesen(o).kuerzel.is_empty());
}

#[test]
fn kommentare_und_fremde_bleiben() {
    let t = Tmp::neu("kommentare");
    let o = t.p();
    let vorher = "# meine Notiz\n[kuerzel]\n\"anderer-typ/kasten\" = \"Mod-Shift-k\" # bleibt\n\"helfer/zitat\" = \"Mod-Shift-e\"\n";
    fs::write(o.join(TASTEN), vorher).unwrap();
    kuerzel_setzen(o, "helfer/zitat", Some("Mod-Shift-y")).unwrap();
    let nachher = fs::read_to_string(o.join(TASTEN)).unwrap();
    let erwartet = vorher.replace("Mod-Shift-e", "Mod-Shift-y");
    assert_eq!(nachher, erwartet);
    let l = tasten_lesen(o);
    assert_eq!(l.kuerzel["anderer-typ/kasten"], "Mod-Shift-k");
}

#[test]
fn fehlerhafte_datei_wird_nicht_ueberschrieben() {
    let t = Tmp::neu("fehlerhaft");
    let o = t.p();
    let kaputt = "[kuerzel\n\"x\" = ";
    fs::write(o.join(TASTEN), kaputt).unwrap();
    let l = tasten_lesen(o);
    assert!(l.kuerzel.is_empty());
    assert!(l.fehler.unwrap().contains("Zeile"));
    assert!(kuerzel_setzen(o, "helfer/zitat", Some("Mod-Shift-y")).is_err());
    assert_eq!(fs::read_to_string(o.join(TASTEN)).unwrap(), kaputt);
}

#[test]
fn falscher_werttyp_wird_genannt() {
    let t = Tmp::neu("werttyp");
    let o = t.p();
    fs::write(o.join(TASTEN), "[kuerzel]\n\"a\" = 3\n\"b\" = \"Mod-b\"\n").unwrap();
    let l = tasten_lesen(o);
    assert_eq!(l.kuerzel.len(), 1);
    assert!(l.warnung.unwrap().contains("a"));
    // Ein einzelner falscher Wert sperrt das Umbelegen nicht.
    assert_eq!(l.fehler, None);
    kuerzel_setzen(o, "b", Some("Mod-k")).unwrap();
    assert_eq!(tasten_lesen(o).kuerzel["b"], "Mod-k");
}

#[test]
fn umzug_kopiert_nur_fehlende() {
    let t = Tmp::neu("umzug");
    let von = t.p().join("von");
    let nach = t.p().join("nach");
    fs::create_dir_all(&von).unwrap();
    fs::create_dir_all(&nach).unwrap();
    fs::write(von.join(TASTEN), "[kuerzel]\n\"a\" = \"Mod-b\"\n").unwrap();
    fs::write(von.join(EINSTELLUNGEN), "# leer\n").unwrap();
    fs::write(nach.join(TASTEN), "[kuerzel]\n\"a\" = \"Mod-k\"\n").unwrap();
    let kopiert = umziehen(&von, &nach).unwrap();
    assert_eq!(kopiert, vec![EINSTELLUNGEN.to_string()]);
    assert_eq!(
        fs::read_to_string(nach.join(TASTEN)).unwrap(),
        "[kuerzel]\n\"a\" = \"Mod-k\"\n"
    );
    assert_eq!(
        fs::read_to_string(nach.join(EINSTELLUNGEN)).unwrap(),
        "# leer\n"
    );
}
