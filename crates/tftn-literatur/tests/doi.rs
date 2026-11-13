//! Literatur aus DOI: BibTeX von Crossref bzw. DataCite wird ein lesbarer Eintrag mit eigenem
//! Schlüssel; eine DOI, die schon da ist, kommt nicht doppelt.

use std::path::PathBuf;

use tftn_literatur::Literatur;
use tftn_literatur::doi::{anhaengen, doi_eintrag, schluessel_stamm};

const CROSSREF: &str = " @article{LeCun_2015, title={Deep learning}, volume={521}, ISSN={1476-4687}, \
url={http://dx.doi.org/10.1038/nature14539}, DOI={10.1038/nature14539}, number={7553}, journal={Nature}, \
publisher={Springer Science and Business Media LLC}, author={LeCun, Yann and Bengio, Yoshua and Hinton, Geoffrey}, \
year={2015}, month=May, pages={436–444} }\n";

const DATACITE: &str = "@misc{https://doi.org/10.5281/zenodo.3509134,
  doi = {10.5281/ZENODO.3509134},
  url = {https://zenodo.org/doi/10.5281/zenodo.3509134},
  author = {The pandas development team, },
  keywords = {python, data science, flexible, pandas, alignment, data analysis},
  title = {pandas-dev/pandas: Pandas},
  publisher = {Zenodo},
  year = {2026},
  copyright = {BSD 3-Clause \"New\" or \"Revised\" License}
}";

fn ordner(name: &str) -> PathBuf {
    let o = std::env::temp_dir().join(format!("tftn-doi-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    o
}

fn laden(datei: &PathBuf) -> Literatur {
    let (lit, fehler) = Literatur::laden(std::slice::from_ref(datei));
    assert!(fehler.is_empty(), "{fehler:?}");
    lit
}

#[test]
fn schluessel_aus_name_und_jahr() {
    assert_eq!(schluessel_stamm("LeCun, Yann and Bengio, Yoshua"), "lecun");
    assert_eq!(schluessel_stamm("Lüddecke, Jörg"), "lueddecke");
    assert_eq!(schluessel_stamm("van der Berg, Anna"), "berg");
    assert_eq!(schluessel_stamm("Anna van der Berg"), "berg");
    assert_eq!(schluessel_stamm("{World Health Organization}"), "world");
    assert_eq!(schluessel_stamm("Žižek, Slavoj"), "zizek");
    assert_eq!(schluessel_stamm(""), "quelle");
}

#[test]
fn crossref_eintrag_landet_lesbar_in_der_bib() {
    let o = ordner("crossref");
    let bib = o.join("literatur.bib");
    std::fs::write(
        &bib,
        "@book{alt2020, title={Alt}, author={Alt, A.}, year={2020}}\n",
    )
    .unwrap();
    let e = doi_eintrag(CROSSREF, &laden(&bib), "10.1038/nature14539").unwrap();
    assert_eq!(e.schluessel, "lecun2015");
    let text = e.text.expect("neu");
    assert!(
        text.starts_with("@article{lecun2015,\n  author = {LeCun, Yann"),
        "{text}"
    );
    assert!(text.ends_with("\n}\n"), "{text}");
    anhaengen(&bib, &text).unwrap();
    let lit = laden(&bib);
    assert_eq!(lit.len(), 2);
    let t = &lit.suchen("lecun2015", 1)[0];
    assert_eq!(t.schluessel, "lecun2015");
    assert_eq!(t.titel, "Deep learning");
    assert_eq!(t.jahr, Some(2015));
    assert_eq!(t.doi.as_deref(), Some("10.1038/nature14539"));
    assert!(t.autoren.contains("LeCun"), "{}", t.autoren);
    let inhalt = std::fs::read_to_string(&bib).unwrap();
    assert!(
        inhalt.starts_with("@book{alt2020"),
        "Vorhandenes bleibt vorn"
    );
    assert!(
        inhalt.contains("}\n\n@article{lecun2015,"),
        "Leerzeile dazwischen: {inhalt}"
    );
    assert!(inhalt.ends_with("}\n"));
}

#[test]
fn datacite_eintrag_mit_organisation() {
    let o = ordner("datacite");
    let bib = o.join("literatur.bib");
    let e = doi_eintrag(DATACITE, &laden_oder_leer(&bib), "10.5281/zenodo.3509134").unwrap();
    assert_eq!(e.schluessel, "pandas2026");
    anhaengen(&bib, &e.text.unwrap()).unwrap();
    let lit = laden(&bib);
    let t = &lit.suchen("pandas2026", 1)[0];
    assert_eq!(t.titel, "pandas-dev/pandas: Pandas");
    assert_eq!(t.jahr, Some(2026));
}

fn laden_oder_leer(datei: &PathBuf) -> Literatur {
    Literatur::laden(std::slice::from_ref(datei)).0
}

#[test]
fn vorhandene_doi_nicht_doppelt_und_schluessel_frei() {
    let o = ordner("doppelt");
    let bib = o.join("literatur.bib");
    std::fs::write(
        &bib,
        "@article{lecun2015, title={Anderes}, author={LeCun, Y.}, year={2015}}\n\
         @article{dl, title={Deep learning}, author={LeCun, Y.}, year={2015}, doi={10.1038/NATURE14539}}\n",
    )
    .unwrap();
    let e = doi_eintrag(CROSSREF, &laden(&bib), "10.1038/nature14539").unwrap();
    assert_eq!(e.schluessel, "dl");
    assert!(e.text.is_none());

    std::fs::write(
        &bib,
        "@article{lecun2015, title={A}, author={LeCun, Y.}, year={2015}}\n\
         @article{lecun2015a, title={B}, author={LeCun, Y.}, year={2015}}\n",
    )
    .unwrap();
    let e = doi_eintrag(CROSSREF, &laden(&bib), "10.1038/nature14539").unwrap();
    assert_eq!(e.schluessel, "lecun2015b");
}

#[test]
fn kaputtes_oder_mehrfaches_bibtex_abgelehnt() {
    let leer = Literatur::default();
    assert!(doi_eintrag("<html>", &leer, "10.1/x").is_err());
    assert!(
        doi_eintrag(
            "@article{a, title={A}}\n@article{b, title={B}}",
            &leer,
            "10.1/x"
        )
        .is_err()
    );
}

/// Der geschriebene Eintrag setzt Typst wirklich: Zitat und Literaturverzeichnis.
#[test]
fn typst_setzt_den_eintrag() {
    let o = ordner("typst");
    let bib = o.join("literatur.bib");
    let e = doi_eintrag(CROSSREF, &Literatur::default(), "10.1038/nature14539").unwrap();
    anhaengen(&bib, &e.text.unwrap()).unwrap();
    std::fs::write(
        o.join("tftn.toml"),
        "tftn = 1\ntypst = \"0.15.1\"\nhaupt = \"main.typ\"\nzeitstempel = 2026-10-31T00:00:00Z\n",
    )
    .unwrap();
    std::fs::write(
        o.join("main.typ"),
        "Siehe @lecun2015.\n#bibliography(\"literatur.bib\")\n",
    )
    .unwrap();
    let typen = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../types");
    tftn_core::schriftvorrat_setzen(vec![typen.join("tftn-leer/schriften")]);
    let p = tftn_core::Projekt::oeffnen(&o).unwrap();
    let k = tftn_core::kompilieren(&tftn_core::ProjektWorld::neu(&p).unwrap());
    assert!(k.meldungen.iter().all(|m| !m.fehler), "{:?}", k.meldungen);
    let text = tftn_core::seitenzeilen(&k.dokument.unwrap())
        .concat()
        .join(" ");
    assert!(
        text.contains("LeCun") && text.contains("Deep learning"),
        "{text}"
    );
}
