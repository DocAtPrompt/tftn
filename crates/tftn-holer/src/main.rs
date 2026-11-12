//! Kommandozeile des Holers (PLAN-M2 AP9). Die App startet ihn auf Klick; er antwortet mit genau
//! einer JSON-Zeile auf stdout und beendet sich. Exit 0 = ok, 1 = Fehler, 2 = falscher Aufruf.
//!
//! ```text
//! tftn-holer index --zwischenspeicher <ordner>
//! tftn-holer paket --zwischenspeicher <ordner> [--erwartet <spec>=sha256:<hex>]… <spec>
//! tftn-holer versionen
//! tftn-holer doi <doi>
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use tftn_holer::{
    Erlaubnis, Netz, doi_holen, doi_lesen, index_holen, paket_holen, versionen_holen,
};
use tftn_pakete::{PaketSpec, Zwischenspeicher};

/// Der Holer beendet sich spätestens nach dieser Zeit selbst (Projektregel Hintergrundprozesse).
const FRIST: Duration = Duration::from_secs(600);

const AUFRUF: &str = "Aufruf: tftn-holer versionen | tftn-holer doi <doi> | tftn-holer index --zwischenspeicher <ordner> | \
     tftn-holer paket --zwischenspeicher <ordner> [--erwartet <spec>=sha256:<hex>]… <spec>";

fn main() {
    // Meldungen in der Sprache der Oberfläche (die App gibt die Datei mit).
    if let Some(p) = std::env::var_os("TFTN_SPRACHDATEI") {
        tftn_sprachen::setzen(tftn_sprachen::Katalog::laden(std::path::Path::new(&p)).ok());
    }
    std::thread::spawn(|| {
        std::thread::sleep(FRIST);
        println!(
            "{}",
            json!({"ok": false, "fehler": tftn_sprachen::tr!("Frist von {sekunden} s abgelaufen", sekunden = FRIST.as_secs())})
        );
        std::process::exit(1);
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    match lauf(&args) {
        Ok(wert) => {
            println!("{wert}");
            std::process::exit(0);
        }
        Err((code, text)) => {
            println!("{}", json!({"ok": false, "fehler": text}));
            std::process::exit(code);
        }
    }
}

fn aufruf(text: impl Into<String>) -> (i32, String) {
    (2, format!("{} – {AUFRUF}", text.into()))
}

fn lauf(args: &[String]) -> Result<Value, (i32, String)> {
    // Ohne das Feature `pruefserver` bleiben beide unverändert.
    #[allow(unused_mut)]
    let mut rest = args;
    #[allow(unused_mut)]
    let mut erlaubnis = Erlaubnis::Universe;
    #[cfg(feature = "pruefserver")]
    if rest.first().map(String::as_str) == Some("--pruefserver") {
        let basis = rest
            .get(1)
            .ok_or_else(|| aufruf("--pruefserver ohne Adresse"))?;
        erlaubnis = Erlaubnis::Pruefserver(basis.clone());
        rest = &rest[2..];
    }
    let (befehl, rest) = rest.split_first().ok_or_else(|| aufruf("kein Befehl"))?;
    if befehl.starts_with("--") {
        return Err(aufruf(format!("unbekannte Option {befehl}")));
    }
    let mut zwischenspeicher: Option<PathBuf> = None;
    let mut erwartet = BTreeMap::new();
    let mut angaben = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        match a.as_str() {
            "--zwischenspeicher" | "--erwartet" => {
                let wert = rest
                    .get(i + 1)
                    .ok_or_else(|| aufruf(format!("{a} ohne Wert")))?;
                if a == "--zwischenspeicher" {
                    zwischenspeicher = Some(PathBuf::from(wert));
                } else {
                    let (s, sha) = wert.split_once('=').ok_or_else(|| {
                        aufruf(format!("--erwartet {wert}: erwartet <spec>=sha256:<hex>"))
                    })?;
                    let s = PaketSpec::lesen(s).map_err(|e| aufruf(e.to_string()))?;
                    erwartet.insert(s, sha.to_string());
                }
                i += 2;
            }
            o if o.starts_with("--") => return Err(aufruf(format!("unbekannte Option {o}"))),
            _ => {
                angaben.push(a.clone());
                i += 1;
            }
        }
    }
    let netz = Netz::neu(erlaubnis.clone());
    if befehl == "versionen" {
        if !angaben.is_empty() || zwischenspeicher.is_some() {
            return Err(aufruf("versionen nimmt keine weiteren Angaben"));
        }
        let v = versionen_holen(&netz, &erlaubnis);
        return Ok(json!({
            "ok": true,
            "versionen": {"typst": v.typst, "tftn": v.tftn, "fehler": v.fehler}
        }));
    }
    if befehl == "doi" {
        let [angabe] = angaben.as_slice() else {
            return Err(aufruf("doi braucht genau eine DOI"));
        };
        if zwischenspeicher.is_some() {
            return Err(aufruf("doi nimmt keinen Zwischenspeicher"));
        }
        let doi = doi_lesen(angabe)
            .ok_or_else(|| (1, tftn_sprachen::tr!("„{angabe}“ ist keine DOI.", angabe)))?;
        let bibtex = doi_holen(&netz, &erlaubnis, &doi).map_err(|e| (1, e.to_string()))?;
        return Ok(json!({"ok": true, "doi": doi, "bibtex": bibtex}));
    }
    let zs = Zwischenspeicher {
        wurzel: zwischenspeicher.ok_or_else(|| aufruf("--zwischenspeicher fehlt"))?,
    };
    let fehler = |e: tftn_holer::HolerFehler| (1, e.to_string());
    match befehl.as_str() {
        "index" => {
            if !angaben.is_empty() {
                return Err(aufruf("index nimmt keine Paketangabe"));
            }
            let index = index_holen(&netz, &erlaubnis, &zs).map_err(fehler)?;
            Ok(json!({
                "ok": true,
                "index": {"pakete": index.eintraege.len(), "uebersprungen": index.uebersprungen}
            }))
        }
        "paket" => {
            let [angabe] = angaben.as_slice() else {
                return Err(aufruf("paket braucht genau eine Paketangabe"));
            };
            let spec = PaketSpec::lesen(angabe).map_err(|e| (1, e.to_string()))?;
            let (geholt, warnungen) =
                paket_holen(&netz, &erlaubnis, &zs, &spec, &erwartet).map_err(fehler)?;
            let pakete: Vec<Value> = geholt
                .iter()
                .map(|g| json!({"spec": g.spec.to_string(), "sha256": g.sha256, "neu": g.neu}))
                .collect();
            Ok(json!({"ok": true, "pakete": pakete, "warnungen": warnungen}))
        }
        andere => Err(aufruf(format!("unbekannter Befehl {andere}"))),
    }
}
