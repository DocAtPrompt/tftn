//! Eigenschaftstests P1, P2, P7 (PLAN-M1 4.1) für den Spiegel.
//! Standard 1 000 Fälle je Eigenschaft; mehr mit `PROPTEST_CASES=100000`.

use proptest::prelude::*;
use tftn_core::spiegel::{Aenderung, Spiegel, SyncFehler, pruefsumme};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};

fn konfig() -> ProptestConfig {
    let faelle = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    ProptestConfig {
        cases: faelle,
        ..ProptestConfig::default()
    }
}

fn id() -> FileId {
    RootedPath::new(VirtualRoot::Project, VirtualPath::new("main.typ").unwrap()).intern()
}

/// Text aus Zeichen, die in Typst-Quellen vorkommen, plus Umlaute und Zeichen außerhalb der BMP.
/// Ohne `\r`: Der Editor liefert nur `\n` (siehe Modul `spiegel`).
fn zeichen() -> impl Strategy<Value = char> {
    prop_oneof![
        8 => proptest::char::range('a', 'z'),
        2 => Just(' '),
        2 => Just('\n'),
        3 => proptest::sample::select(vec!['ä', 'ö', 'ü', 'Ä', 'ß', 'é', '„', '“', '–', '€']),
        1 => proptest::sample::select(vec!['😀', '𝔸', '🇦']),
        4 => proptest::sample::select(vec![
            '#', '*', '_', '$', '=', '[', ']', '{', '}', '(', ')', '"', '\\', '/', '`', '@', '<', '>', '-', ':', ',',
        ]),
    ]
}

fn text(max: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(zeichen(), 0..max).prop_map(|v| v.into_iter().collect())
}

fn utf16(s: &str, zeichen: usize) -> usize {
    s.chars().take(zeichen).map(char::len_utf16).sum()
}

/// Änderungen wie CodeMirror sie liefert: aufsteigend, nicht überlappend, auf Zeichengrenzen.
fn transaktion(alt: String) -> impl Strategy<Value = (String, Vec<Aenderung>)> {
    let n = alt.chars().count();
    (proptest::collection::vec((0..=n, 0..=n, text(6)), 1..4)).prop_map(move |roh| {
        let mut punkte: Vec<(usize, usize, String)> = roh
            .into_iter()
            .map(|(a, b, t)| (a.min(b), a.max(b), t))
            .collect();
        punkte.sort_by_key(|p| (p.0, p.1));
        let mut aenderungen = Vec::new();
        let mut ende = 0;
        for (von, bis, t) in punkte {
            if von < ende {
                continue;
            }
            aenderungen.push(Aenderung {
                von: utf16(&alt, von),
                bis: utf16(&alt, bis),
                text: t,
            });
            ende = bis;
        }
        (alt.clone(), aenderungen)
    })
}

/// So wendet der Editor die Änderungen an (Modell der UI-Seite).
fn modell_anwenden(alt: &str, aenderungen: &[Aenderung]) -> String {
    let einheiten: Vec<u16> = alt.encode_utf16().collect();
    let mut neu = Vec::new();
    let mut pos = 0;
    for a in aenderungen {
        neu.extend_from_slice(&einheiten[pos..a.von]);
        neu.extend(a.text.encode_utf16());
        pos = a.bis;
    }
    neu.extend_from_slice(&einheiten[pos..]);
    String::from_utf16(&neu).unwrap()
}

proptest! {
    #![proptest_config(konfig())]

    /// P1: UTF-8 ↔ UTF-16 ist für jede Zeichengrenze umkehrbar.
    #[test]
    fn p1_offsets_umkehrbar(s in text(80)) {
        let quelle = Source::detached(s.clone());
        let z = quelle.lines();
        for (b, _) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
            let u = z.byte_to_utf16(b).unwrap();
            prop_assert_eq!(z.utf16_to_byte(u), Some(b));
        }
        prop_assert_eq!(z.len_utf16(), s.encode_utf16().count());
    }

    /// P2: Inkrementell geändert = vollständig neu geparst (ohne Spans verglichen).
    #[test]
    fn p2_inkrementell_gleich_vollstaendig((alt, aenderungen) in text(120).prop_flat_map(transaktion)) {
        let mut spiegel = Spiegel::neu(id(), &alt);
        spiegel.anwenden(0, &aenderungen).unwrap();
        let neu = modell_anwenden(&alt, &aenderungen);
        prop_assert_eq!(spiegel.text(), neu.as_str());
        let frisch = Source::detached(neu);
        prop_assert!(spiegel.quelle().root().spanless_eq(frisch.root()));
    }

    /// P7: Zufällige Transaktionsfolgen, auch mit veralteter Revision, halten Editor und Spiegel gleich.
    #[test]
    fn p7_sync(start in text(60), schritte in proptest::collection::vec((any::<u8>(), text(6), any::<u16>(), any::<u16>()), 1..40)) {
        let mut editor = start.clone();
        let mut editor_rev = 0u64;
        let mut spiegel = Spiegel::neu(id(), &start);
        for (art, einfuegen, a, b) in schritte {
            let n = editor.chars().count();
            let (von, bis) = {
                let (x, y) = (a as usize % (n + 1), b as usize % (n + 1));
                (x.min(y), x.max(y))
            };
            let aenderung = Aenderung { von: utf16(&editor, von), bis: utf16(&editor, bis), text: einfuegen };
            let neu = modell_anwenden(&editor, std::slice::from_ref(&aenderung));
            // Jede achte Transaktion beruht auf einer veralteten Revision (z. B. Widget-Patch).
            let basis = if art % 8 == 0 { editor_rev.wrapping_add(7) } else { editor_rev };
            editor = neu;
            match spiegel.anwenden(basis, &[aenderung]) {
                Ok(rev) => editor_rev = rev,
                Err(SyncFehler::Revision { .. }) => editor_rev = spiegel.ersetzen(&editor),
                Err(e) => prop_assert!(false, "unerwartet: {e}"),
            }
            prop_assert_eq!(spiegel.rev(), editor_rev);
            prop_assert_eq!(spiegel.pruefsumme(), pruefsumme(&editor));
        }
        prop_assert_eq!(spiegel.text(), editor.as_str());
    }
}
