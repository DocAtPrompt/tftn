//! Sprachdateien lesen, Sprache wählen, Platzhalter einsetzen, Texte im Code finden.

use std::path::PathBuf;

use tftn_sprachen::quellen::{aus_html, aus_rust, aus_ts, platzhalter, sprachdatei, toml_text};
use tftn_sprachen::{Katalog, einsetzen, verfuegbar, waehlen};

fn ordner(name: &str) -> PathBuf {
    let o = std::env::temp_dir().join(format!("tftn-sprachen-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    o
}

#[test]
fn datei_lesen_leeres_bleibt_deutsch() {
    let k = Katalog::lesen(
        "en",
        "[sprache]\nname = \"English\"\n[texte]\n\"Speichern\" = \"Save\"\n\"Öffnen\" = \"\"\n",
    )
    .unwrap();
    assert_eq!(k.name, "English");
    assert_eq!(k.texte.get("Speichern").map(String::as_str), Some("Save"));
    assert!(!k.texte.contains_key("Öffnen"));
    assert!(Katalog::lesen("en", "[texte\n").is_err());
}

#[test]
fn platzhalter_einsetzen() {
    let w = [("doi", "10.1/x".to_string()), ("n", "3".to_string())];
    assert_eq!(
        einsetzen("DOI {doi}: {n} Treffer", &w),
        "DOI 10.1/x: 3 Treffer"
    );
    assert_eq!(einsetzen("{unbekannt} {doi}", &w), "{unbekannt} 10.1/x");
    assert_eq!(
        einsetzen("#set text(lang: \"de\") { x }", &w),
        "#set text(lang: \"de\") { x }"
    );
    assert_eq!(
        platzhalter("{a} und {b} { c } {1x}")
            .into_iter()
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn sprache_waehlen() {
    let o = ordner("waehlen");
    let eigene = ordner("waehlen-eigene");
    std::fs::write(o.join("en.toml"), "[sprache]\nname = \"English\"\n").unwrap();
    std::fs::write(o.join("fr.toml"), "[sprache]\nname = \"Français\"\n").unwrap();
    std::fs::write(
        eigene.join("fr.toml"),
        "[sprache]\nname = \"Français (eigen)\"\n",
    )
    .unwrap();
    std::fs::write(
        eigene.join("vorlage.toml"),
        "[sprache]\nname = \"Vorlage\"\n",
    )
    .unwrap();
    std::fs::write(eigene.join("de.toml"), "").unwrap();
    std::fs::write(eigene.join("kaputt.toml"), "[[[").unwrap();
    let da = verfuegbar(&[o.clone(), eigene.clone()]);
    let namen: Vec<_> = da
        .iter()
        .map(|s| (s.kennung.as_str(), s.name.as_str()))
        .collect();
    assert_eq!(namen, [("en", "English"), ("fr", "Français (eigen)")]);

    let k = |w: Option<&str>, sys: &str| waehlen(w, sys, &da).map(|s| s.kennung.clone());
    assert_eq!(k(None, "de-AT"), None);
    assert_eq!(k(None, "en-US"), Some("en".into()));
    assert_eq!(k(None, "fr-CH"), Some("fr".into()));
    assert_eq!(k(None, "ja-JP"), Some("en".into()), "unbekannt: Englisch");
    assert_eq!(k(Some("de"), "en-US"), None);
    assert_eq!(k(Some("fr"), "de-AT"), Some("fr".into()));
    assert_eq!(
        k(Some("xx"), "de-AT"),
        None,
        "fehlende Datei: wie das System"
    );
}

#[test]
fn texte_in_rust() {
    let q = r#"
        let a = tr!("Speichern");
        let b = tftn_sprachen::tr!(
            "Die DOI {doi} kennen \
             weder Crossref noch DataCite.",
            doi
        );
        // tr!("Kommentar")
        let c = format!("nicht");
        let d = tr!("Zeile\n\"zwei\"");
    "#;
    assert_eq!(
        aus_rust(q),
        [
            "Speichern",
            "Die DOI {doi} kennen weder Crossref noch DataCite.",
            "Zeile\n\"zwei\""
        ]
    );
}

#[test]
fn texte_in_typescript() {
    let q = "const a = t(\"Speichern\");\nconst b = t('Öffnen …', { n });\nconst c = t(`Zwei\nZeilen`);\n\
             const d = t(name);\nconst e = n(\"Fett\");\nconst f = this.t(\"nein\"); const g = fit(\"nein\");\n\
             // t(\"Kommentar\")\nconst h = t(`mit ${x}`);\n";
    let t = aus_ts(q);
    assert_eq!(t.texte, ["Speichern", "Öffnen …", "Zwei\nZeilen", "Fett"]);
    assert_eq!(t.offen, [5, 9]);
}

#[test]
fn texte_in_html() {
    let q = "<html><head><title>TFTN</title></head><body>\n<button id=\"a\" title=\"Neues Dokument &amp; mehr\">Vorlage …</button>\n\
             <span id=\"datei\">…</span><nav aria-label=\"Gliederung\"></nav>\n<script src=\"x\">nein()</script></body></html>";
    assert_eq!(
        aus_html(q),
        ["Neues Dokument & mehr", "Vorlage …", "Gliederung"]
    );
}

#[test]
fn sprachdatei_schreiben_und_lesen() {
    let o = ordner("schreiben");
    std::fs::create_dir_all(o.join("ui/src")).unwrap();
    std::fs::write(o.join("ui/src/a.ts"), "t(\"Eins\"); t(\"Zwei \\\"x\\\"\");").unwrap();
    std::fs::write(o.join("ui/src/b.ts"), "t(\"Eins\"); t(\"Drei {n}\");").unwrap();
    std::fs::write(o.join("ui/src/b.test.ts"), "t(\"Test\");").unwrap();
    let b = tftn_sprachen::quellen::bestand(&o);
    assert_eq!(b.texte.len(), 3);
    let alt = [("Eins".to_string(), "One".to_string())].into();
    let text = sprachdatei("Kopf\n\nzwei", "English", &b, &alt);
    assert!(
        text.starts_with("# Kopf\n#\n# zwei\n\n[sprache]\nname = \"English\"\n"),
        "{text}"
    );
    assert!(
        text.contains("# ui/src/a.ts\n\"Eins\" = \"One\"\n\"Zwei \\\"x\\\"\" = \"\"\n"),
        "{text}"
    );
    assert!(
        text.contains("# ui/src/b.ts\n\"Drei {n}\" = \"\"\n"),
        "{text}"
    );
    let k = Katalog::lesen("en", &text).unwrap();
    assert_eq!(k.texte.len(), 1);
    assert_eq!(toml_text("a\tb\u{1}"), "\"a\\tb\\u0001\"");
}
