//! `tr!` mit gesetzter Sprache (eigenes Testprogramm, weil die Sprache für den ganzen Prozess gilt).

use tftn_sprachen::{Katalog, ist_deutsch, kennung, setzen, tr};

#[test]
fn tr_folgt_der_gesetzten_sprache() {
    let doi = "10.1/x";
    assert_eq!(tr!("Speichern"), "Speichern");
    assert_eq!(tr!("DOI {doi} fehlt.", doi), "DOI 10.1/x fehlt.");
    assert!(ist_deutsch());
    assert_eq!(kennung(), "de");

    setzen(Some(
        Katalog::lesen(
            "en",
            "[texte]\n\"Speichern\" = \"Save\"\n\"DOI {doi} fehlt.\" = \"DOI {doi} is missing.\"\n",
        )
        .unwrap(),
    ));
    assert_eq!(tr!("Speichern"), "Save");
    assert_eq!(tr!("DOI {doi} fehlt.", doi), "DOI 10.1/x is missing.");
    assert_eq!(
        tr!(
            "{n} von {m}",
            n = 2,
            m = std::path::Path::new("a/b").display()
        ),
        "2 von a/b",
        "ohne Übersetzung deutsch, Werte eingesetzt"
    );
    assert_eq!(kennung(), "en");
    setzen(None);
    assert_eq!(tr!("Speichern"), "Speichern");
}
