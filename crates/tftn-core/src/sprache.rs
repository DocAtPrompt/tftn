//! Dokumentsprache für die Rechtschreibung: die Sprache, in der der meiste Text gesetzt ist. So
//! zählt auch, was ein Dokumenttyp in seiner Vorlage festlegt.

use std::collections::HashMap;

use typst::layout::{Frame, FrameItem};
use typst_layout::PagedDocument;

/// `lang` und `region` mit den meisten gesetzten Zeichen; `None` ohne Text.
pub fn hauptsprache(doc: &PagedDocument) -> Option<(String, Option<String>)> {
    fn sammeln(frame: &Frame, aus: &mut HashMap<(String, Option<String>), usize>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => sammeln(&g.frame, aus),
                FrameItem::Text(t) => {
                    let schluessel = (
                        t.lang.as_str().to_string(),
                        t.region.map(|r| r.as_str().to_string()),
                    );
                    *aus.entry(schluessel).or_default() += t.text.chars().count();
                }
                _ => {}
            }
        }
    }
    let mut zaehler = HashMap::new();
    for seite in doc.pages() {
        sammeln(&seite.frame, &mut zaehler);
    }
    zaehler
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(s, _)| s)
}
