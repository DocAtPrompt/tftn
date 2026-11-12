//! Index lesen und durchsuchen (PLAN-M2 AP9 Task 2).

use tftn_pakete::Index;

fn typst() -> semver::Version {
    semver::Version::new(0, 15, 1)
}

#[test]
fn index_nachsichtig() {
    let json = br#"[
        {"name": "a", "version": "1.0.0", "description": "gut"},
        {"name": "ohne-version"},
        {"name": "Gross", "version": "1.0.0"},
        {"name": "b", "version": "0.2.0", "foo": [1, 2], "keywords": ["k"]},
        {"name": "a", "version": "1.0.0", "description": "doppelt"}
    ]"#;
    let index = Index::lesen(json).unwrap();
    let namen: Vec<_> = index
        .eintraege
        .iter()
        .map(|e| format!("{} {}", e.name, e.version))
        .collect();
    assert_eq!(namen, vec!["a 1.0.0", "b 0.2.0"]);
    assert_eq!(index.eintraege[0].description.as_deref(), Some("gut"));
    assert_eq!(index.eintraege[1].keywords, vec!["k"]);
    assert_eq!(index.uebersprungen, 2);
    assert!(Index::lesen(b"kein json").is_err());
    assert!(Index::lesen(b"{}").is_err());
}

#[test]
fn suche_rang_und_compiler() {
    let json = br#"[
        {"name": "plot", "version": "1.0.0"},
        {"name": "cetz-plot", "version": "0.1.0"},
        {"name": "cetz", "version": "0.4.2", "compiler": "0.14.0", "license": "LGPL-3.0"},
        {"name": "cetz", "version": "0.5.0", "compiler": "0.16.0"},
        {"name": "zeichnen", "version": "1.0.0", "keywords": ["CeTZ"]},
        {"name": "x", "version": "1.0.0", "description": "nutzt CeTZ"},
        {"name": "neu", "version": "2.0.0", "compiler": "0.16.0", "description": "cetz"}
    ]"#;
    let index = Index::lesen(json).unwrap();
    let treffer: Vec<_> = index
        .suchen("cetz", &typst(), 50)
        .into_iter()
        .map(|e| format!("{} {}", e.name, e.version))
        .collect();
    assert_eq!(
        treffer,
        vec!["cetz 0.4.2", "cetz-plot 0.1.0", "zeichnen 1.0.0", "x 1.0.0"]
    );
    assert_eq!(
        index.suchen("CETZ", &typst(), 50)[0].license.as_deref(),
        Some("LGPL-3.0")
    );
    assert!(index.suchen("", &typst(), 50).is_empty());
    assert!(index.suchen("   ", &typst(), 50).is_empty());
    let zwei: Vec<_> = index
        .suchen("cetz plot", &typst(), 50)
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(zwei, vec!["cetz-plot"]);
    assert_eq!(index.suchen("cetz", &typst(), 2).len(), 2);
}

#[test]
fn index_neueste() {
    let i = Index::lesen(
        br#"[{"name":"a","version":"0.1.0","compiler":"0.13.0"},
             {"name":"a","version":"0.3.0","compiler":"0.15.0"},
             {"name":"a","version":"0.4.0","compiler":"0.16.0"},
             {"name":"b","version":"1.0.0"}]"#,
    )
    .unwrap();
    let typst = semver::Version::new(0, 15, 1);
    assert_eq!(i.neueste("a", &typst).unwrap().version.to_string(), "0.3.0");
    assert_eq!(i.neueste("b", &typst).unwrap().version.to_string(), "1.0.0");
    assert!(i.neueste("c", &typst).is_none());
}
