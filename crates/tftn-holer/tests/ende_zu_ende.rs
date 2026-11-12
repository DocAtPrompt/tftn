//! Ende zu Ende (PLAN-M2 AP9, „fertig, wenn“): Ein Projekt mit geholtem Paket baut ohne
//! Zwischenspeicher und ohne Netz bytegleich zur Gegenprobe.
//!
//! Braucht die typst-CLI in der Version aus tftn.toml (TFTN_TYPST oder PATH).

mod pruefserver;

use std::collections::BTreeMap;
use std::path::Path;

use pruefserver::*;
use tftn_core::{Projekt, ProjektWorld, kompilieren};
use tftn_export::{STANDARD_FRIST, TypstCli, ausfuehren, exportieren};
use tftn_holer::{Erlaubnis, Netz, paket_holen};
use tftn_pakete::{PaketSpec, Zwischenspeicher, uebernehmen};

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let e = e.unwrap();
        let ziel = nach.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            kopieren(&e.path(), &ziel);
        } else {
            std::fs::copy(e.path(), ziel).unwrap();
        }
    }
}

#[test]
fn geholtes_paket_baut_ohne_zwischenspeicher_bytegleich() {
    let o = testordner("ende-zu-ende");
    let testartikel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    let ohne = o.join("ohne");
    let mit = o.join("mit");
    kopieren(&testartikel, &ohne);
    kopieren(&testartikel, &mit);
    let haupt = mit.join("main.typ");
    let text = std::fs::read_to_string(&haupt).unwrap();
    std::fs::write(
        &haupt,
        format!(
            "#import \"@preview/tftn-probe:0.1.0\": probe\n{text}\n#probe[Text aus dem geholten Paket]\n"
        ),
    )
    .unwrap();

    // Holen über den Prüfserver in einen Zwischenspeicher, dann übernehmen.
    let server = Server::starten();
    server.paket("tftn-probe", "0.1.0");
    server.paket("tftn-probe-basis", "0.1.0");
    let zs = Zwischenspeicher {
        wurzel: o.join("zs"),
    };
    let e = Erlaubnis::Pruefserver(server.basis.clone());
    let spec = PaketSpec::lesen("@preview/tftn-probe:0.1.0").unwrap();
    let (geholt, warnungen) =
        paket_holen(&Netz::neu(e.clone()), &e, &zs, &spec, &BTreeMap::new()).unwrap();
    assert!(warnungen.is_empty(), "{warnungen:?}");
    let pakete: Vec<_> = geholt.into_iter().map(|g| (g.spec, g.sha256)).collect();
    uebernehmen(&mit, &zs, &pakete).unwrap();

    // Ohne Zwischenspeicher weiter
    std::fs::remove_dir_all(&zs.wurzel).unwrap();

    let projekt = Projekt::oeffnen(&mit).unwrap();
    assert_eq!(projekt.konfig.pakete.len(), 2);
    let world = ProjektWorld::neu(&projekt).unwrap();
    let ergebnis = kompilieren(&world);
    assert!(world.fehlende_pakete().is_empty());
    assert!(
        ergebnis.meldungen.iter().all(|m| !m.fehler),
        "{:?}",
        ergebnis.meldungen
    );

    let cli =
        TypstCli::finden(&projekt.konfig.typst).expect("typst-CLI in der Projektversion nötig");
    let export = o.join("export.pdf");
    let gegenprobe = o.join("gegenprobe.pdf");
    exportieren(&projekt, &cli, &export, &[]).unwrap();
    ausfuehren(&projekt, &cli, &gegenprobe, &[], STANDARD_FRIST).unwrap();
    let pdf = std::fs::read(&export).unwrap();
    assert_eq!(pdf, std::fs::read(&gegenprobe).unwrap());

    // Das Paket hat etwas zum Dokument beigetragen.
    let ohne_projekt = Projekt::oeffnen(&ohne).unwrap();
    let ohne_pdf = o.join("ohne.pdf");
    exportieren(&ohne_projekt, &cli, &ohne_pdf, &[]).unwrap();
    assert_ne!(pdf, std::fs::read(&ohne_pdf).unwrap());
}
