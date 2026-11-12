//! Literatur aus DOI: Crossref zuerst, dann DataCite; nur diese zwei festen Adressen.

use std::cell::RefCell;
use std::collections::HashMap;

use tftn_holer::{
    Abruf, CROSSREF, DATACITE, Erlaubnis, HolerFehler, adresse_pruefen, doi_adressen, doi_holen,
    doi_lesen,
};

#[derive(Default)]
struct Falsch {
    antworten: HashMap<String, Result<Vec<u8>, u16>>,
    gefragt: RefCell<Vec<String>>,
}

impl Abruf for Falsch {
    fn holen(&self, url: &str, _max: u64) -> Result<Vec<u8>, HolerFehler> {
        adresse_pruefen(url, &Erlaubnis::Universe)?;
        self.gefragt.borrow_mut().push(url.to_string());
        match self.antworten.get(url) {
            Some(Ok(d)) => Ok(d.clone()),
            Some(Err(s)) => Err(HolerFehler::Status(*s, url.into())),
            None => Err(HolerFehler::NichtGefunden(format!("{url} nicht gefunden"))),
        }
    }
}

#[test]
fn doi_aus_verschiedenen_schreibweisen() {
    for e in [
        "10.1038/nature14539",
        " https://doi.org/10.1038/nature14539 ",
        "http://dx.doi.org/10.1038/nature14539",
        "doi:10.1038/nature14539",
        "DOI: 10.1038/nature14539",
        "https://www.doi.org/10.1038/nature14539",
    ] {
        assert_eq!(doi_lesen(e).as_deref(), Some("10.1038/nature14539"), "{e}");
    }
    assert_eq!(
        doi_lesen("10.1002/(SICI)1097-4571(199806)49:8<693::AID-ASI4>3.0.CO;2-0").as_deref(),
        Some("10.1002/(SICI)1097-4571(199806)49:8<693::AID-ASI4>3.0.CO;2-0")
    );
    for nein in [
        "",
        "nature14539",
        "10.12/abc",
        "10.1038/",
        "10.1038/a b",
        "11.1038/x",
        "Smith 2020",
    ] {
        assert_eq!(doi_lesen(nein), None, "{nein}");
    }
}

#[test]
fn adressen_kodiert_und_erlaubt() {
    let e = Erlaubnis::Universe;
    let [cr, dc] = doi_adressen(
        "10.1002/(SICI)1097-4571(199806)49:8<693::AID-ASI4>3.0.CO;2-0",
        &e,
    );
    assert_eq!(
        cr,
        format!(
            "{CROSSREF}/works/10.1002/%28SICI%291097-4571%28199806%2949%3A8%3C693%3A%3AAID-ASI4%3E3.0.CO%3B2-0/transform/application/x-bibtex"
        )
    );
    assert!(dc.starts_with(&format!(
        "{DATACITE}/dois/application/x-bibtex/10.1002/%28SICI"
    )));
    adresse_pruefen(&cr, &e).unwrap();
    adresse_pruefen(&dc, &e).unwrap();
}

#[test]
fn fremde_adressen_bei_crossref_und_datacite_abgelehnt() {
    let e = Erlaubnis::Universe;
    for nein in [
        format!("{CROSSREF}/works?query=x"),
        format!("{CROSSREF}/works/10.1038/x"),
        format!("{CROSSREF}/works/10.1038/../../members/transform/application/x-bibtex"),
        format!("{CROSSREF}/works/10.1038/a b/transform/application/x-bibtex"),
        format!("{CROSSREF}/works/11.1038/x/transform/application/x-bibtex"),
        format!("{DATACITE}/dois/10.1038/x"),
        format!("{DATACITE}/dois/application/x-bibtex/10.1038/x?y=1"),
        format!("{DATACITE}/dois/application/x-bibtex/10.1038/x%zz"),
        "https://doi.org/10.1038/x".to_string(),
        "http://api.crossref.org/works/10.1038/x/transform/application/x-bibtex".to_string(),
    ] {
        assert!(adresse_pruefen(&nein, &e).is_err(), "{nein}");
    }
}

#[test]
fn crossref_zuerst() {
    let e = Erlaubnis::Universe;
    let [cr, _] = doi_adressen("10.1038/nature14539", &e);
    let mut f = Falsch::default();
    f.antworten.insert(
        cr.clone(),
        Ok(b"@article{LeCun_2015, title={Deep learning}}".to_vec()),
    );
    assert_eq!(
        doi_holen(&f, &e, "10.1038/nature14539").unwrap(),
        "@article{LeCun_2015, title={Deep learning}}"
    );
    assert_eq!(*f.gefragt.borrow(), [cr]);
}

#[test]
fn datacite_wenn_crossref_nichts_kennt() {
    let e = Erlaubnis::Universe;
    let [cr, dc] = doi_adressen("10.5281/zenodo.3509134", &e);
    let mut f = Falsch::default();
    f.antworten
        .insert(dc.clone(), Ok(b"@misc{x, title={Pandas}}".to_vec()));
    assert_eq!(
        doi_holen(&f, &e, "10.5281/zenodo.3509134").unwrap(),
        "@misc{x, title={Pandas}}"
    );
    assert_eq!(*f.gefragt.borrow(), [cr, dc]);
}

#[test]
fn nirgends_gefunden_oder_fehler() {
    let e = Erlaubnis::Universe;
    let f = Falsch::default();
    let t = doi_holen(&f, &e, "10.1234/gibtsnicht")
        .unwrap_err()
        .to_string();
    assert!(
        t.contains("10.1234/gibtsnicht") && t.contains("Crossref") && t.contains("DataCite"),
        "{t}"
    );
    // Ein anderer Fehler bei Crossref bricht ab, statt DataCite zu fragen.
    let [cr, _] = doi_adressen("10.1038/x", &e);
    let mut f = Falsch::default();
    f.antworten.insert(cr, Err(503));
    assert!(matches!(
        doi_holen(&f, &e, "10.1038/x"),
        Err(HolerFehler::Status(503, _))
    ));
    assert_eq!(f.gefragt.borrow().len(), 1);
    // Keine Literatur, sondern etwas anderes
    let [cr, _] = doi_adressen("10.1038/y", &e);
    let mut f = Falsch::default();
    f.antworten.insert(cr, Ok(b"<html>".to_vec()));
    assert!(doi_holen(&f, &e, "10.1038/y").is_err());
}
