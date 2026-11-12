//! Holer gegen den lokalen Prüfserver (PLAN-M2 AP9 Task 4). Kein Netz.

mod pruefserver;

use std::path::Path;
use std::process::Command;

use pruefserver::*;
use tftn_holer::{Erlaubnis, Netz, adresse_pruefen, index_holen};
use tftn_pakete::{PaketSpec, Zwischenspeicher};

/// Startet das Programm; gibt Exit-Code und die JSON-Zeile zurück.
fn lauf(args: &[&str], env: &[(&str, &str)]) -> (i32, serde_json::Value) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_tftn-holer"));
    c.args(args);
    for (n, w) in env {
        c.env(n, w);
    }
    let aus = c.output().unwrap();
    let text = String::from_utf8_lossy(&aus.stdout);
    let zeile = text.lines().next().unwrap_or("null");
    let json = serde_json::from_str(zeile)
        .unwrap_or_else(|e| panic!("keine JSON-Zeile ({e}): {text:?} / {:?}", aus.stderr));
    (aus.status.code().unwrap_or(-1), json)
}

fn zs_pfad(o: &Path) -> String {
    o.join("zs").display().to_string()
}

#[test]
fn adresse_pruefen_faelle() {
    let e = Erlaubnis::Universe;
    assert_eq!(e.basis(), "https://packages.typst.org");
    assert!(adresse_pruefen("https://packages.typst.org/preview/index.json", &e).is_ok());
    assert!(adresse_pruefen("https://packages.typst.org/preview/cetz-0.4.2.tar.gz", &e).is_ok());
    for falsch in [
        "http://packages.typst.org/preview/index.json",
        "https://packages.typst.org:8443/preview/index.json",
        "https://packages.typst.org.example.com/preview/index.json",
        "https://example.com/preview/index.json",
        "https://u@packages.typst.org/preview/index.json",
        "https://127.0.0.1/preview/index.json",
        "file:///etc/passwd",
        "https://packages.typst.org",
        "https://packages.typst.org/preview/../x",
        "https://packages.typst.org/preview/index.json?x=1",
    ] {
        let f = adresse_pruefen(falsch, &e).expect_err(falsch);
        assert!(
            f.to_string().contains("packages.typst.org"),
            "{falsch}: {f}"
        );
    }
}

#[test]
fn weiterleitung_bricht_ab() {
    let a = Server::starten();
    let b = Server::starten();
    b.index(&[("tftn-probe", "0.1.0")]);
    a.weg(
        "/preview/index.json",
        Antwort {
            status: 302,
            kopf: vec![("Location".into(), format!("{}/preview/index.json", b.basis))],
            rumpf: Rumpf::Bytes(Vec::new()),
        },
    );
    let o = testordner("weiterleitung");
    let zs = Zwischenspeicher {
        wurzel: o.join("zs"),
    };
    let e = Erlaubnis::Pruefserver(a.basis.clone());
    let fehler =
        index_holen(&Netz::neu(e.clone()), &e, &zs).expect_err("Weiterleitung muss abbrechen");
    assert!(fehler.to_string().contains("Weiterleitung"), "{fehler}");
    assert_eq!(b.verbindungen(), 0);
    assert!(!zs.index_pfad().exists());

    // Dasselbe über das Programm
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "index",
            "--zwischenspeicher",
            &zs_pfad(&o),
        ],
        &[],
    );
    assert_eq!(code, 1, "{json}");
    assert_eq!(json["ok"], false);
    assert_eq!(b.verbindungen(), 0);
}

#[test]
fn proxy_wird_nicht_genutzt() {
    let a = Server::starten();
    let b = Server::starten();
    a.index(&[("tftn-probe", "0.1.0")]);
    let o = testordner("proxy");
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "index",
            "--zwischenspeicher",
            &zs_pfad(&o),
        ],
        &[
            ("HTTPS_PROXY", &b.basis),
            ("HTTP_PROXY", &b.basis),
            ("ALL_PROXY", &b.basis),
            ("https_proxy", &b.basis),
            ("http_proxy", &b.basis),
        ],
    );
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["index"]["pakete"], 1);
    assert_eq!(b.verbindungen(), 0);
    assert_eq!(a.verbindungen(), 1);
}

#[test]
fn index_und_paket_ueber_programm() {
    let a = Server::starten();
    a.index(&[("tftn-probe", "0.1.0"), ("tftn-probe-basis", "0.1.0")]);
    let probe = a.paket("tftn-probe", "0.1.0");
    let basis = a.paket("tftn-probe-basis", "0.1.0");
    let o = testordner("programm");
    let zs_text = zs_pfad(&o);
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "index",
            "--zwischenspeicher",
            &zs_text,
        ],
        &[],
    );
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["index"]["pakete"], 2);
    assert_eq!(json["index"]["uebersprungen"], 0);

    let args = [
        "--pruefserver",
        &a.basis,
        "paket",
        "--zwischenspeicher",
        &zs_text,
        "@preview/tftn-probe:0.1.0",
    ];
    let (code, json) = lauf(&args, &[]);
    assert_eq!(code, 0, "{json}");
    let pakete = json["pakete"].as_array().unwrap();
    assert_eq!(pakete.len(), 2, "{json}");
    assert_eq!(pakete[0]["spec"], "@preview/tftn-probe:0.1.0");
    assert_eq!(pakete[0]["sha256"], tftn_pakete::sha256_hex(&probe));
    assert_eq!(pakete[1]["spec"], "@preview/tftn-probe-basis:0.1.0");
    assert_eq!(pakete[1]["sha256"], tftn_pakete::sha256_hex(&basis));
    assert!(pakete.iter().all(|p| p["neu"] == true));
    let zs = Zwischenspeicher {
        wurzel: o.join("zs"),
    };
    for s in [
        "@preview/tftn-probe:0.1.0",
        "@preview/tftn-probe-basis:0.1.0",
    ] {
        assert!(zs.ordner(&PaketSpec::lesen(s).unwrap()).is_some(), "{s}");
    }

    let vorher = a.verbindungen();
    let (code, json) = lauf(&args, &[]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(
        a.verbindungen(),
        vorher,
        "zweiter Aufruf aus dem Zwischenspeicher"
    );
    assert!(
        json["pakete"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["neu"] == false)
    );
}

#[test]
fn abweichende_pruefsumme_bricht_ab() {
    let a = Server::starten();
    a.paket("tftn-probe", "0.1.0");
    let basis = a.paket("tftn-probe-basis", "0.1.0");
    let o = testordner("pruefsumme");
    let zs_text = zs_pfad(&o);
    let null = format!("@preview/tftn-probe-basis:0.1.0=sha256:{}", "0".repeat(64));
    let args = [
        "--pruefserver",
        &a.basis,
        "paket",
        "--zwischenspeicher",
        &zs_text,
        "--erwartet",
        &null,
        "@preview/tftn-probe:0.1.0",
    ];
    let (code, json) = lauf(&args, &[]);
    assert_eq!(code, 1, "{json}");
    let fehler = json["fehler"].as_str().unwrap();
    assert!(fehler.contains(&"0".repeat(64)), "{fehler}");
    assert!(
        fehler.contains(&tftn_pakete::sha256_hex(&basis)),
        "{fehler}"
    );
    let zs = Zwischenspeicher {
        wurzel: o.join("zs"),
    };
    assert!(
        zs.ordner(&PaketSpec::lesen("@preview/tftn-probe-basis:0.1.0").unwrap())
            .is_none()
    );

    // Liegt das Paket schon im Zwischenspeicher, wird mit dessen .sha256 verglichen.
    let ohne = [
        "--pruefserver",
        &a.basis,
        "paket",
        "--zwischenspeicher",
        &zs_text,
        "@preview/tftn-probe:0.1.0",
    ];
    assert_eq!(lauf(&ohne, &[]).0, 0);
    let (code, json) = lauf(&args, &[]);
    assert_eq!(code, 1, "{json}");
    assert!(json["fehler"].as_str().unwrap().contains(&"0".repeat(64)));
}

#[test]
fn abhaengigkeit_fehlt_nichts_halbes() {
    let a = Server::starten();
    a.paket("tftn-probe", "0.1.0");
    let o = testordner("fehlt");
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "paket",
            "--zwischenspeicher",
            &zs_pfad(&o),
            "@preview/tftn-probe:0.1.0",
        ],
        &[],
    );
    assert_eq!(code, 1, "{json}");
    let fehler = json["fehler"].as_str().unwrap();
    assert!(fehler.contains("nicht gefunden"), "{fehler}");
    assert!(
        fehler.contains("@preview/tftn-probe-basis:0.1.0"),
        "{fehler}"
    );
    let preview = o.join("zs/preview");
    for name in std::fs::read_dir(&preview).unwrap() {
        for e in std::fs::read_dir(name.unwrap().path()).unwrap() {
            let n = e.unwrap().file_name().to_string_lossy().into_owned();
            assert!(!n.starts_with(".tmp"), "Rest {n}");
        }
    }
}

#[test]
fn zu_gross_bricht_ab() {
    let a = Server::starten();
    a.weg(
        "/preview/tftn-probe-0.1.0.tar.gz",
        Antwort {
            status: 200,
            kopf: Vec::new(),
            rumpf: Rumpf::Nullen(65 << 20),
        },
    );
    let o = testordner("gross");
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "paket",
            "--zwischenspeicher",
            &zs_pfad(&o),
            "@preview/tftn-probe:0.1.0",
        ],
        &[],
    );
    assert_eq!(code, 1, "{json}");
    assert!(
        json["fehler"].as_str().unwrap().contains("zu groß"),
        "{json}"
    );
}

#[test]
fn falscher_aufruf() {
    let a = Server::starten();
    let o = testordner("aufruf");
    let (code, json) = lauf(&[], &[]);
    assert_eq!(code, 2, "{json}");
    assert_eq!(json["ok"], false);
    let (code, json) = lauf(&["paket", "@preview/x:1.0.0"], &[]);
    assert_eq!(code, 2, "ohne Zwischenspeicher: {json}");
    let (code, json) = lauf(
        &[
            "--pruefserver",
            &a.basis,
            "paket",
            "--zwischenspeicher",
            &zs_pfad(&o),
            "@preview/X:1",
        ],
        &[],
    );
    assert_eq!(code, 1, "{json}");
    assert!(json["fehler"].as_str().unwrap().contains("X"), "{json}");
    assert_eq!(a.verbindungen(), 0);
}

#[test]
fn github_nur_die_zwei_release_adressen() {
    let e = Erlaubnis::Universe;
    for gut in [
        "https://api.github.com/repos/typst/typst/releases/latest",
        "https://api.github.com/repos/DocAtPrompt/tftn/releases/latest",
    ] {
        assert!(adresse_pruefen(gut, &e).is_ok(), "{gut}");
    }
    for falsch in [
        "https://api.github.com/repos/typst/typst/releases",
        "https://api.github.com/repos/typst/typst/releases/latest?x=1",
        "https://api.github.com/repos/anders/tftn/releases/latest",
        "https://api.github.com/repos/DocAtPrompt/tftn/releases/latest/../x",
        "http://api.github.com/repos/typst/typst/releases/latest",
        "https://github.com/typst/typst/releases/latest",
        "https://packages.typst.org/repos/typst/typst/releases/latest-X",
    ] {
        assert!(adresse_pruefen(falsch, &e).is_err(), "{falsch}");
    }
}

#[test]
fn versionen_ueber_programm() {
    let a = Server::starten();
    a.weg(
        "/repos/typst/typst/releases/latest",
        ok(br#"{"tag_name": "v0.16.0", "name": "Version 0.16.0"}"#.to_vec()),
    );
    let (code, json) = lauf(&["--pruefserver", &a.basis, "versionen"], &[]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["versionen"]["typst"], "v0.16.0");
    assert!(json["versionen"]["tftn"].is_null(), "{json}");
    let fehler = json["versionen"]["fehler"].as_array().unwrap();
    assert_eq!(fehler.len(), 1, "{json}");
    assert!(fehler[0].as_str().unwrap().contains("TFTN"), "{json}");
    assert_eq!(a.verbindungen(), 2);
}
